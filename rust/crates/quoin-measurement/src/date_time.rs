// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The ONE RFC 3339 grammar in this domain.
//!
//! # Two grammars went in; one comes out
//!
//! The retained TypeScript has two, and they disagree:
//!
//! | implementation | separator | zulu |
//! |---|---|---|
//! | `src/measurement/date-time.ts:2` | `[Tt]` | `[Zz]` |
//! | `src/measurement/intervention.ts:349` | `T` | `Z` |
//!
//! `"2026-01-01t00:00:00z"` is therefore a valid `observed_at` in an
//! operational record and an invalid one in an intervention record, today, on
//! `origin/main` (quoin#440). The ruling recorded in the Stage 6 plan §5 is
//! that the two unify on the **permissive** grammar, for three reasons:
//!
//! 1. RFC 3339 §5.6 says `NOTE: ISO 8601 defines date and time separated by
//!    "T". Applications using this syntax may choose ... to use a lower case
//!    "t"`, and says the same of `z`. The permissive grammar is the conformant
//!    one; the strict one is narrower than the standard it is named after.
//! 2. Widening is the only direction that cannot invalidate a record that
//!    validates today. Narrowing could invalidate records in stores this
//!    repository cannot enumerate.
//! 3. It is already the behaviour of eight of the nine fields that carry
//!    `format: "date-time"` in the operational schema.
//!
//! So this module ports `date-time.ts`, and the strict grammar's narrowing is
//! kept as an **observation on the parsed value** rather than as a second
//! parser: [`Rfc3339DateTime::narrowed_by_the_strict_grammar`] is true exactly
//! for the inputs `intervention.ts:349` rejected and this module accepts.
//! `tests/tc_468_date_time.rs` asserts that disagreement on `…t…z` directly.
//!
//! # Why this is not the seventh instant reader
//!
//! quoin already carries six hand-rolled instant validators across
//! `quoin-assurance`, `quoin-change-assurance` and `quoin-evidence`, accepting
//! three different languages, all returning `bool` so every caller re-parses
//! the string it just validated (Stage 6 plan §13.1). This one returns a
//! **value**, the way `quoin_store::RawFileSha256Digest::parse_stored` does.
//! Its eventual home is the shared instant reader that ticket asks for, modelled
//! on `quoin-store`'s named digest domains; until that exists, this is the one
//! grammar in the measurement domain and nothing here may add a second.

use crate::error::{MeasurementError, MeasurementErrorCode};

/// One RFC 3339 date-time, parsed.
///
/// Holding this value *is* the proof that the text satisfied the grammar and
/// the calendar, so no caller re-parses it.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Rfc3339DateTime {
    text: String,
    epoch_millis: i64,
    lowercase_separator: bool,
    lowercase_zulu: bool,
}

impl Rfc3339DateTime {
    /// Parse one RFC 3339 date-time, rejecting `Date.parse`'s date-only forms.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::DateTimeInvalid`] when the text does not match
    /// the grammar, or matches it and names a day the calendar does not have.
    pub fn parse(value: &str) -> Result<Self, MeasurementError> {
        Self::try_parse(value)
            .ok_or_else(|| MeasurementError::new(MeasurementErrorCode::DateTimeInvalid, value))
    }

    /// The text exactly as it was parsed.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Milliseconds since the Unix epoch, as `Date.parse` reports them.
    #[must_use]
    pub const fn epoch_millis(&self) -> i64 {
        self.epoch_millis
    }

    /// Whether the strict grammar at `intervention.ts:349` would have refused
    /// this value.
    ///
    /// True exactly when a lowercase `t` separator or a lowercase `z` zulu
    /// designator was used — the one difference between the two retained
    /// implementations, and the whole of the narrowing this port declares.
    #[must_use]
    pub const fn narrowed_by_the_strict_grammar(&self) -> bool {
        self.lowercase_separator || self.lowercase_zulu
    }

    fn try_parse(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        // `YYYY-MM-DDThh:mm:ss` is 19 bytes and nothing shorter can match.
        if bytes.len() < 20 {
            return None;
        }
        let year = digits(bytes.get(0..4)?)?;
        if *bytes.get(4)? != b'-' {
            return None;
        }
        let month = digits(bytes.get(5..7)?)?;
        if *bytes.get(7)? != b'-' {
            return None;
        }
        let day = digits(bytes.get(8..10)?)?;
        let separator = *bytes.get(10)?;
        if separator != b'T' && separator != b't' {
            return None;
        }
        let hour = digits(bytes.get(11..13)?)?;
        if *bytes.get(13)? != b':' {
            return None;
        }
        let minute = digits(bytes.get(14..16)?)?;
        if *bytes.get(16)? != b':' {
            return None;
        }
        let second = digits(bytes.get(17..19)?)?;

        let mut cursor = 19;
        let mut millis_of_second = 0_i64;
        if bytes.get(cursor) == Some(&b'.') {
            cursor += 1;
            let start = cursor;
            while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }
            if cursor == start {
                return None;
            }
            // `Date.parse` keeps millisecond resolution and truncates the rest.
            let mut scale = 100_i64;
            for index in start..cursor.min(start + 3) {
                millis_of_second += i64::from(bytes.get(index)? - b'0') * scale;
                scale /= 10;
            }
        }

        let (offset_minutes, lowercase_zulu) = match bytes.get(cursor)? {
            b'Z' | b'z' => {
                let lower = *bytes.get(cursor)? == b'z';
                cursor += 1;
                (0_i64, lower)
            }
            sign @ (b'+' | b'-') => {
                let sign = if *sign == b'-' { -1_i64 } else { 1_i64 };
                cursor += 1;
                let offset_hour = digits(bytes.get(cursor..cursor + 2)?)?;
                if *bytes.get(cursor + 2)? != b':' {
                    return None;
                }
                let offset_minute = digits(bytes.get(cursor + 3..cursor + 5)?)?;
                cursor += 5;
                if offset_hour > 23 || offset_minute > 59 {
                    return None;
                }
                (
                    sign * (i64::from(offset_hour) * 60 + i64::from(offset_minute)),
                    false,
                )
            }
            _ => return None,
        };
        if cursor != bytes.len() {
            return None;
        }

        if !(1..=12).contains(&month)
            || day < 1
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return None;
        }

        let days = days_from_civil(i64::from(year), month, day);
        let epoch_millis =
            ((days * 24 + i64::from(hour)) * 60 + i64::from(minute) - offset_minutes) * 60_000
                + i64::from(second) * 1_000
                + millis_of_second;

        Some(Self {
            text: value.to_owned(),
            epoch_millis,
            lowercase_separator: separator == b't',
            lowercase_zulu,
        })
    }
}

/// Read a fixed-width run of ASCII digits.
fn digits(bytes: &[u8]) -> Option<u16> {
    let mut out: u16 = 0;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add(u16::from(byte - b'0'))?;
    }
    Some(out)
}

/// Whether `year` is a leap year under the proleptic Gregorian calendar.
fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// The length of `month` in `year`, `month` being 1-based.
fn days_in_month(year: u16, month: u16) -> u16 {
    match month {
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to `year-month-day`, Howard Hinnant's `days_from_civil`.
///
/// Written out rather than pulled from a date crate because the workspace pins
/// every dependency exactly and this is nine lines of arithmetic with no
/// timezone database behind it.
fn days_from_civil(year: i64, month: u16, day: u16) -> i64 {
    let month = i64::from(month);
    let day = i64::from(day);
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The civil date `days` after 1970-01-01, Howard Hinnant's `civil_from_days`.
///
/// The exact inverse of [`days_from_civil`], and here rather than in a second
/// module for the reason the module header gives: the calendar arithmetic in
/// this crate lives in one place, and `tc_468_boundary` asserts it.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

/// Render an instant the way JavaScript's `Date.prototype.toISOString` does.
///
/// `github-release-operational.ts:159` writes a computed deadline with
/// `new Date(deadline).toISOString()`, which **always** emits three fractional
/// digits and an upper-case `Z` — `2026-08-29T23:21:00.000Z`, not
/// `2026-08-29T23:21:00Z`. The retained pair under
/// `spec/evidence/operational/pairs/` carries that exact spelling, so the
/// milliseconds are not cosmetic: dropping them changes the bytes of every
/// record this producer writes.
///
/// This is the inverse of [`Rfc3339DateTime::parse`] and deliberately not a
/// second grammar — it produces only the one spelling that grammar's strictest
/// reader accepts.
///
/// # Panics
///
/// Never for an instant this crate can parse. A year outside `0000..=9999` has
/// no `toISOString` spelling in this format and is refused by returning
/// [`None`] rather than by widening the field.
#[must_use]
pub fn to_iso_string(epoch_millis: i64) -> Option<String> {
    let millis_of_day = epoch_millis.rem_euclid(86_400_000);
    let days = epoch_millis.div_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    if !(0..=9_999).contains(&year) {
        return None;
    }
    let second_of_day = millis_of_day / 1_000;
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        second_of_day / 3_600,
        (second_of_day / 60) % 60,
        second_of_day % 60,
        millis_of_day % 1_000,
    ))
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use super::Rfc3339DateTime;

    #[test]
    fn the_epoch_is_zero_and_the_designators_are_upper_case() {
        let parsed = Rfc3339DateTime::parse("1970-01-01T00:00:00Z").unwrap();
        assert_eq!(parsed.epoch_millis(), 0);
        assert!(!parsed.narrowed_by_the_strict_grammar());
    }

    #[test]
    fn an_offset_moves_the_instant_the_way_date_parse_does() {
        // Date.parse("2026-01-01T00:00:00+01:00") === 1767222000000
        assert_eq!(
            Rfc3339DateTime::parse("2026-01-01T00:00:00+01:00")
                .unwrap()
                .epoch_millis(),
            1_767_222_000_000
        );
        // Date.parse("2026-01-01T00:00:00-01:00") === 1767229200000
        assert_eq!(
            Rfc3339DateTime::parse("2026-01-01T00:00:00-01:00")
                .unwrap()
                .epoch_millis(),
            1_767_229_200_000
        );
    }

    #[test]
    fn fractional_seconds_truncate_at_milliseconds() {
        assert_eq!(
            Rfc3339DateTime::parse("1970-01-01T00:00:00.0009Z")
                .unwrap()
                .epoch_millis(),
            0
        );
        assert_eq!(
            Rfc3339DateTime::parse("1970-01-01T00:00:00.5Z")
                .unwrap()
                .epoch_millis(),
            500
        );
    }

    #[test]
    fn a_day_the_calendar_does_not_have_is_refused() {
        assert!(Rfc3339DateTime::parse("2026-02-29T00:00:00Z").is_err());
        assert!(Rfc3339DateTime::parse("2024-02-29T00:00:00Z").is_ok());
        assert!(Rfc3339DateTime::parse("2026-13-01T00:00:00Z").is_err());
        assert!(Rfc3339DateTime::parse("2026-01-01T24:00:00Z").is_err());
    }

    #[test]
    fn date_only_and_trailing_content_are_refused() {
        assert!(Rfc3339DateTime::parse("2026-01-01").is_err());
        assert!(Rfc3339DateTime::parse("2026-01-01T00:00:00Z ").is_err());
        assert!(Rfc3339DateTime::parse("2026-01-01T00:00:00").is_err());
        assert!(Rfc3339DateTime::parse("2026-01-01T00:00:00.Z").is_err());
    }

    /// `to_iso_string` inverts the parser over the whole calendar it accepts,
    /// not over a handful of chosen instants.
    #[test]
    fn rendering_and_parsing_are_inverses_across_the_calendar() {
        // Every leap-year boundary, both epoch directions, and the two
        // spellings `toISOString` fixes: three fractional digits and `Z`.
        for text in [
            "1970-01-01T00:00:00.000Z",
            "1969-12-31T23:59:59.999Z",
            "1900-03-01T00:00:00.000Z",
            "2000-02-29T12:34:56.789Z",
            "2024-02-29T23:59:59.001Z",
            "2026-08-29T23:21:00.000Z",
            "9999-12-31T23:59:59.999Z",
            "0001-01-01T00:00:00.000Z",
        ] {
            let parsed = Rfc3339DateTime::parse(text).unwrap();
            assert_eq!(
                super::to_iso_string(parsed.epoch_millis()).as_deref(),
                Some(text),
                "{text} did not survive the round trip"
            );
        }
    }

    /// A year with no four-digit spelling is refused, not silently widened.
    #[test]
    fn an_instant_outside_the_four_digit_years_has_no_iso_spelling() {
        // 10000-01-01T00:00:00Z, one millisecond past the last renderable day.
        assert_eq!(super::to_iso_string(253_402_300_800_000), None);
        assert_eq!(super::to_iso_string(-62_167_219_200_001), None);
    }
}
