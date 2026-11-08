// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one timestamp reader, with a named grammar per language (PLAT-91).
//!
//! Several textual forms look alike and answer different questions: is it a
//! date, a zoned instant, an instant a calendar actually has? They were
//! written independently in five crates, each faithful to the TypeScript check
//! it replaced, and each returned a `bool` so every caller re-parsed what it
//! had just validated. They are collected here, as [`InstantGrammar`]
//! variants, and a caller names the one it reads. The accepted language of each
//! variant is byte-for-byte the language of the reader it replaced; collapsing
//! two of them would admit a record that is refused today or the reverse.
//!
//! | grammar | replaced | accepts |
//! |---|---|---|
//! | [`InstantGrammar::DateOrInstant`] | `trust::is_instant` | `YYYY-MM-DD`, or `YYYY-MM-DDTHH:MM:SS` plus any suffix; shape only |
//! | [`InstantGrammar::ZonedInstant`] | `assurance_records::is_iso_instant` | `YYYY-MM-DDTHH:MM:SS[.f+](Z\|+HH:MM)`; shape only |
//! | [`InstantGrammar::CalendarInstant`] | `quoin-assurance` `is_instant` | zoned instant, upper-case `T`/`Z`, ranges and calendar checked |
//! | [`InstantGrammar::CalendarInstantEndOfDay`] | `quoin-change-assurance` `is_a_real_instant` | `CalendarInstant` plus ECMAScript hour `24` at exactly midnight |
//! | [`InstantGrammar::Rfc3339`] | `quoin-measurement` `Rfc3339DateTime` | `CalendarInstant` plus lower-case `t` and `z` (RFC 3339 section 5.6) |
//!
//! The two shape-only grammars carry no epoch value; the other three do.
//! Holding an [`Instant`] is the proof the text was read in its grammar.

/// Which timestamp language a field is written in.
///
/// Named at every call site: two fields can both be "a timestamp" and not be
/// the same language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstantGrammar {
    /// A calendar date, or a date and a wall-clock time with any suffix.
    DateOrInstant,
    /// A full instant with a mandatory zone offset and nothing after it. Shape only.
    ZonedInstant,
    /// A zoned instant with upper-case `T` and `Z`, whose fields are in range
    /// and name a day the calendar has. Hour `24` is refused.
    CalendarInstant,
    /// [`Self::CalendarInstant`] that also admits hour `24` as exactly midnight.
    CalendarInstantEndOfDay,
    /// [`Self::CalendarInstant`] that also admits a lower-case `t` or `z`.
    Rfc3339,
}

impl InstantGrammar {
    /// Every grammar, so a caller enumerating them cannot miss one.
    pub const ALL: &'static [Self] = &[
        Self::DateOrInstant,
        Self::ZonedInstant,
        Self::CalendarInstant,
        Self::CalendarInstantEndOfDay,
        Self::Rfc3339,
    ];

    /// The grammar's stable code, as a diagnostic or a schema would spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DateOrInstant => "date-or-instant",
            Self::ZonedInstant => "zoned-instant",
            Self::CalendarInstant => "calendar-instant",
            Self::CalendarInstantEndOfDay => "calendar-instant-end-of-day",
            Self::Rfc3339 => "rfc3339",
        }
    }

    /// Read a grammar back from its code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|g| g.as_str() == code)
    }

    /// How a refusal names what was expected.
    #[must_use]
    pub const fn expectation(self) -> &'static str {
        match self {
            Self::DateOrInstant => "must be an ISO-8601 timestamp",
            _ => "must be an ISO-8601 instant",
        }
    }
}

/// A timestamp that has been read, and the grammar it was read in.
///
/// A newtype rather than a `bool` return: a caller holding an `Instant` cannot
/// have skipped the check, and callers take the text and the epoch value from
/// here rather than re-reading the unvalidated input.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    text: String,
    grammar: InstantGrammar,
    epoch_millis: Option<i64>,
}

impl Instant {
    /// Read `value` in `grammar`, or report that it is not in that language.
    #[must_use]
    pub fn parse(value: &str, grammar: InstantGrammar) -> Option<Self> {
        let epoch_millis = match grammar {
            InstantGrammar::DateOrInstant | InstantGrammar::ZonedInstant => {
                if !accepts_shape(value, grammar) {
                    return None;
                }
                None
            }
            InstantGrammar::CalendarInstant => Some(read_calendar(value, false, false)?),
            InstantGrammar::CalendarInstantEndOfDay => Some(read_calendar(value, false, true)?),
            InstantGrammar::Rfc3339 => Some(read_calendar(value, true, false)?),
        };
        Some(Self {
            text: value.to_owned(),
            grammar,
            epoch_millis,
        })
    }

    /// The timestamp exactly as it was written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The grammar it was read in.
    #[must_use]
    pub const fn grammar(&self) -> InstantGrammar {
        self.grammar
    }

    /// Milliseconds since the Unix epoch, as `Date.parse` reports them
    /// (a fraction is truncated to three digits). `None` for the two
    /// shape-only grammars, which do not check the calendar.
    #[must_use]
    pub const fn epoch_millis(&self) -> Option<i64> {
        self.epoch_millis
    }
}

impl std::fmt::Display for Instant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl From<Instant> for String {
    fn from(value: Instant) -> Self {
        value.text
    }
}

/// The shared calendar-date prefix, then the grammar's own tail (shape only).
fn accepts_shape(value: &str, grammar: InstantGrammar) -> bool {
    let bytes = value.as_bytes();
    let digits_at = |index: usize, count: usize| {
        (index..index + count).all(|position| bytes.get(position).is_some_and(u8::is_ascii_digit))
    };
    let byte_at = |index: usize, byte: u8| bytes.get(index) == Some(&byte);

    // `YYYY-MM-DD`, which both grammars open with. Out-of-range indices read as
    // absent rather than panicking, so a short input simply fails here.
    if !(digits_at(0, 4)
        && byte_at(4, b'-')
        && digits_at(5, 2)
        && byte_at(7, b'-')
        && digits_at(8, 2))
    {
        return false;
    }

    match grammar {
        InstantGrammar::DateOrInstant => {
            if bytes.len() == 10 {
                return true;
            }
            // `THH:MM:SS`, and then whatever the producer appended. The
            // retained V8 parse accepted a zone, a fractional part or neither,
            // and narrowing that here would refuse records already on disk.
            byte_at(10, b'T')
                && digits_at(11, 2)
                && byte_at(13, b':')
                && digits_at(14, 2)
                && byte_at(16, b':')
                && digits_at(17, 2)
        }
        InstantGrammar::ZonedInstant => {
            if !(byte_at(10, b'T')
                && digits_at(11, 2)
                && byte_at(13, b':')
                && digits_at(14, 2)
                && byte_at(16, b':')
                && digits_at(17, 2))
            {
                return false;
            }
            let mut index = 19;
            // `(\.\d+)?` — at least one digit if the dot is there at all.
            if byte_at(index, b'.') {
                index += 1;
                let start = index;
                while digits_at(index, 1) {
                    index += 1;
                }
                if index == start {
                    return false;
                }
            }
            // `(Z|[+-]\d{2}:\d{2})`, anchored: the zone ends the string.
            if byte_at(index, b'Z') {
                return index + 1 == bytes.len();
            }
            if byte_at(index, b'+') || byte_at(index, b'-') {
                return digits_at(index + 1, 2)
                    && byte_at(index + 3, b':')
                    && digits_at(index + 4, 2)
                    && index + 6 == bytes.len();
            }
            false
        }
        InstantGrammar::CalendarInstant
        | InstantGrammar::CalendarInstantEndOfDay
        | InstantGrammar::Rfc3339 => false,
    }
}

/// The calendar-checking reader behind the three calendar grammars.
///
/// `lowercase` admits `t`/`z`; `end_of_day` admits hour `24` as exactly
/// midnight (ECMAScript Date Time String Format), with a zero fraction.
fn read_calendar(value: &str, lowercase: bool, end_of_day: bool) -> Option<i64> {
    // The shape is pure ASCII, so a multi-byte character can only be a
    // rejection, and every `get` below is then on a character boundary.
    if !value.is_ascii() {
        return None;
    }
    let (head, rest) = (value.get(..19)?, value.get(19..)?);
    let separator_ok = |at: usize, upper: &str, lower: &str| {
        let found = head.get(at..at + 1);
        found == Some(upper) || (lowercase && found == Some(lower))
    };
    if head.get(4..5) != Some("-")
        || head.get(7..8) != Some("-")
        || !separator_ok(10, "T", "t")
        || head.get(13..14) != Some(":")
        || head.get(16..17) != Some(":")
    {
        return None;
    }
    let mut fields = [0u32; 6];
    for (slot, (from, to)) in
        fields
            .iter_mut()
            .zip([(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)])
    {
        let text = head.get(from..to)?;
        if !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        *slot = text.parse::<u32>().ok()?;
    }
    let [year, month, day, hour, minute, second] = fields;

    // `(?:\.\d+)?` - a dot with no digit after it is not a fraction.
    let mut tail = rest;
    let mut fraction_millis = 0i64;
    let mut fraction_is_zero = true;
    if let Some(fraction) = tail.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let (read, remainder) = (fraction.get(..digits)?, fraction.get(digits..)?);
        let mut scale = 100i64;
        for byte in read.bytes().take(3) {
            fraction_millis += i64::from(byte - b'0') * scale;
            scale /= 10;
        }
        fraction_is_zero = read.bytes().all(|byte| byte == b'0');
        tail = remainder;
    }

    // `Z` or `+-HH:MM`, and nothing else.
    let mut offset_millis = 0i64;
    if tail != "Z" && !(lowercase && tail == "z") {
        let offset = tail.strip_prefix(['+', '-'])?;
        if offset.len() != 5
            || offset.get(2..3) != Some(":")
            || !offset
                .bytes()
                .enumerate()
                .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
        {
            return None;
        }
        let offset_hour = offset.get(..2)?.parse::<u32>().ok()?;
        let offset_minute = offset.get(3..)?.parse::<u32>().ok()?;
        if offset_hour > 23 || offset_minute > 59 {
            return None;
        }
        let magnitude = i64::from(offset_hour) * 3_600_000 + i64::from(offset_minute) * 60_000;
        // `+05:30` is ahead of UTC, so the UTC instant is earlier than the digits.
        offset_millis = if tail.starts_with('-') {
            -magnitude
        } else {
            magnitude
        };
    }

    let midnight_24 = end_of_day && hour == 24 && minute == 0 && second == 0 && fraction_is_zero;
    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || (hour > 23 && !midnight_24)
        || minute > 59
        || second > 59
    {
        return None;
    }

    let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
    let seconds =
        days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
    Some(seconds * 1_000 + fraction_millis - offset_millis)
}

/// Days in a month, with the full Gregorian leap rule.
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}

/// Days between the Unix epoch and a proleptic Gregorian date.
///
/// Howard Hinnant's `days_from_civil`, exact for every year this grammar can
/// spell (`0000`-`9999`) and needing no dependency.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Instant, InstantGrammar};

    /// Every grammar survives the round trip through its own code.
    ///
    /// Provenance: quoin#458
    #[test]
    fn tc_458_300_every_grammar_round_trips_through_its_code() {
        assert_eq!(
            InstantGrammar::ALL.len(),
            5,
            "a grammar was added without a case here"
        );
        for grammar in InstantGrammar::ALL {
            assert_eq!(
                InstantGrammar::from_code(grammar.as_str()),
                Some(*grammar),
                "{} did not round trip",
                grammar.as_str()
            );
        }
        assert_eq!(InstantGrammar::from_code("iso8601"), None);
    }

    /// The date-or-instant language, exactly as `trust::is_instant` read it.
    ///
    /// Trace: FR-093-AC-1
    /// Provenance: quoin#456, quoin#458
    #[test]
    fn tc_458_301_the_date_or_instant_grammar_takes_a_bare_date_and_any_suffix() {
        for accepted in [
            "2026-01-01",
            "2026-01-01T00:00:00",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00.123456Z",
            "2026-01-01T00:00:00+05:30",
            // The retained `Date.parse` accepted trailing text on a
            // well-formed prefix, so this reader must too.
            "2026-01-01T00:00:00 (Pacific)",
        ] {
            assert!(
                Instant::parse(accepted, InstantGrammar::DateOrInstant).is_some(),
                "refused {accepted}"
            );
        }
        for refused in [
            "",
            "2026-01",
            "2026-01-0",
            "2026-01-01 00:00:00",
            "2026-01-01T00:00",
            "20260101T000000Z",
            "yesterday",
        ] {
            assert!(
                Instant::parse(refused, InstantGrammar::DateOrInstant).is_none(),
                "accepted {refused}"
            );
        }
    }

    /// The zoned language, exactly as `assurance_records::is_iso_instant` read it.
    ///
    /// Trace: FR-048-AC-3
    /// Provenance: quoin#456, quoin#458
    #[test]
    fn tc_458_302_the_zoned_instant_grammar_requires_a_zone_and_ends_at_it() {
        for accepted in [
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00.1Z",
            "2026-01-01T00:00:00.123456789Z",
            "2026-01-01T00:00:00+05:30",
            "2026-01-01T00:00:00-08:00",
            "2026-01-01T00:00:00.5-08:00",
        ] {
            assert!(
                Instant::parse(accepted, InstantGrammar::ZonedInstant).is_some(),
                "refused {accepted}"
            );
        }
        for refused in [
            "2026-01-01",
            "2026-01-01T00:00:00",
            "2026-01-01T00:00:00.Z",
            "2026-01-01T00:00:00Z ",
            "2026-01-01T00:00:00+0530",
            "2026-01-01T00:00:00+05:300",
            "2026-01-01T00:00:00 (Pacific)",
        ] {
            assert!(
                Instant::parse(refused, InstantGrammar::ZonedInstant).is_none(),
                "accepted {refused}"
            );
        }
    }

    /// The two languages are different, and the difference is the point.
    ///
    /// Anti-tautology: if a future edit collapsed them, this fails rather than
    /// the two tests above both continuing to pass against one grammar.
    ///
    /// Trace: FR-048-AC-3, FR-093-AC-1
    /// Provenance: quoin#458
    #[test]
    fn tc_458_303_the_two_grammars_disagree_in_both_directions() {
        let date_only = "2026-01-01";
        assert!(Instant::parse(date_only, InstantGrammar::DateOrInstant).is_some());
        assert!(Instant::parse(date_only, InstantGrammar::ZonedInstant).is_none());

        let unzoned = "2026-01-01T12:30:45";
        assert!(Instant::parse(unzoned, InstantGrammar::DateOrInstant).is_some());
        assert!(Instant::parse(unzoned, InstantGrammar::ZonedInstant).is_none());
    }

    /// Every grammar against one table of inputs: the languages the five
    /// replaced readers accepted, side by side. A row is
    /// `(input, DateOrInstant, ZonedInstant, CalendarInstant, EndOfDay, Rfc3339)`.
    ///
    /// Provenance: PLAT-91
    #[test]
    fn plat_91_every_grammar_keeps_the_language_of_the_reader_it_replaced() {
        #[rustfmt::skip]
        let table: &[(&str, [bool; 5])] = &[
            ("2026-01-01",                   [true,  false, false, false, false]),
            ("2026-01-01T00:00:00",          [true,  false, false, false, false]),
            ("2026-01-01T00:00:00 (Pacific)", [true, false, false, false, false]),
            ("2026-01-01T00:00:00Z",         [true,  true,  true,  true,  true]),
            ("2026-01-01T00:00:00.123456Z",  [true,  true,  true,  true,  true]),
            ("2026-01-01T00:00:00+05:30",    [true,  true,  true,  true,  true]),
            ("2026-01-01T00:00:00-08:00",    [true,  true,  true,  true,  true]),
            ("2026-01-01T00:00:00.Z",        [true,  false, false, false, false]),
            ("2026-01-01T00:00:00Z ",        [true,  false, false, false, false]),
            ("2026-01-01T00:00:00+0530",     [true,  false, false, false, false]),
            // Shape-only grammars do not look at the calendar.
            ("2026-02-30T00:00:00Z",         [true,  true,  false, false, false]),
            ("2026-13-01T00:00:00Z",         [true,  true,  false, false, false]),
            ("2026-01-01T25:00:00Z",         [true,  true,  false, false, false]),
            ("2026-01-01T23:59:60Z",         [true,  true,  false, false, false]),
            ("2026-01-01T00:00:00+24:00",    [true,  true,  false, false, false]),
            ("2025-02-29T00:00:00Z",         [true,  true,  false, false, false]),
            ("2028-02-29T00:00:00Z",         [true,  true,  true,  true,  true]),
            // ECMAScript hour 24 is midnight and nothing later.
            ("2026-01-01T24:00:00Z",         [true,  true,  false, true,  false]),
            ("2026-01-01T24:00:00.000Z",     [true,  true,  false, true,  false]),
            ("2026-01-01T24:00:01Z",         [true,  true,  false, false, false]),
            ("2026-01-01T24:00:00.001Z",     [true,  true,  false, false, false]),
            // Lower case is RFC 3339's, and no one else's.
            ("2026-01-01t00:00:00Z",         [false,  false, false, false, true]),
            ("2026-01-01T00:00:00z",         [true,  false, false, false, true]),
            ("2026-01-01t00:00:00z",         [false,  false, false, false, true]),
            ("2026-01-01 00:00:00Z",         [false, false, false, false, false]),
            ("",                             [false, false, false, false, false]),
            ("yesterday",                    [false, false, false, false, false]),
            ("2026-01-01T00:00:00\u{e9}",    [true,  false, false, false, false]),
        ];
        for (input, expected) in table {
            for (grammar, want) in InstantGrammar::ALL.iter().zip(expected) {
                assert_eq!(
                    Instant::parse(input, *grammar).is_some(),
                    *want,
                    "{} on {input:?}",
                    grammar.as_str()
                );
            }
        }
    }

    /// The epoch value is `Date.parse`'s: offsets applied, a fraction truncated
    /// to milliseconds, and absent for the shape-only grammars.
    ///
    /// Provenance: PLAT-91
    #[test]
    fn plat_91_the_calendar_grammars_report_date_parse_epoch_millis() {
        let millis = |text: &str, grammar| Instant::parse(text, grammar).unwrap().epoch_millis();
        assert_eq!(
            millis("1970-01-01T00:00:00Z", InstantGrammar::CalendarInstant),
            Some(0)
        );
        assert_eq!(
            millis("2026-01-01T00:00:00+01:00", InstantGrammar::Rfc3339),
            Some(1_767_222_000_000)
        );
        assert_eq!(
            millis("2026-01-01T00:00:00.0019Z", InstantGrammar::CalendarInstant),
            millis("2026-01-01T00:00:00.001Z", InstantGrammar::CalendarInstant)
        );
        assert_eq!(
            millis(
                "2026-01-01T24:00:00Z",
                InstantGrammar::CalendarInstantEndOfDay
            ),
            millis("2026-01-02T00:00:00Z", InstantGrammar::CalendarInstant)
        );
        assert_eq!(
            millis("2026-01-01T00:00:00Z", InstantGrammar::ZonedInstant),
            None
        );
        assert_eq!(millis("2026-01-01", InstantGrammar::DateOrInstant), None);
    }

    /// What was read is what was written, and it remembers which language.
    ///
    /// Trace: FR-048-AC-3
    /// Provenance: quoin#458
    #[test]
    fn tc_458_304_a_read_instant_carries_its_own_text_and_grammar() {
        let read = Instant::parse("2026-01-01T00:00:00Z", InstantGrammar::ZonedInstant).unwrap();
        assert_eq!(read.as_str(), "2026-01-01T00:00:00Z");
        assert_eq!(read.grammar(), InstantGrammar::ZonedInstant);
        assert_eq!(String::from(read), "2026-01-01T00:00:00Z");
    }
}
