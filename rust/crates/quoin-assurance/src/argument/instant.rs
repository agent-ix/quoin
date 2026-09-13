// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! JavaScript's trimmed set and the authored instant grammar (quoin#436).
//!
//! Two readings the host language decides rather than the schema: what
//! `String.prototype.trim` removes, and which instants `Date.parse`
//! accepts. Both are transcribed here once and shared, because a second
//! transcription is a second chance to disagree with the oracle.

/// Is this string empty once JavaScript's `trim` has run?
///
/// **Not `str::trim`.** The two disagree on exactly two code points, and they
/// disagree in OPPOSITE directions, which is why one call to `str::trim` would
/// have been wrong twice:
///
/// | code point | `String.prototype.trim` | `char::is_whitespace` | if ported naively |
/// |---|---|---|---|
/// | `U+FEFF` zero-width no-break space | trims | keeps | port **accepts** what the oracle refuses |
/// | `U+0085` next line | keeps | trims | port **refuses** what the oracle accepts |
///
/// ECMA-262 defines the trimmed set as `WhiteSpace` (TAB, VT, FF, SP, NBSP,
/// ZWNBSP and category `Zs`) plus `LineTerminator` (LF, CR, LS, PS). Unicode's
/// `White_Space` property, which Rust uses, includes `U+0085` and excludes
/// `U+FEFF`. So the set is transcribed here rather than borrowed.
pub(crate) fn js_trim_is_empty(value: &str) -> bool {
    value.chars().all(is_js_whitespace)
}

/// The ECMA-262 trimmed set, in one place.
///
/// Transcribed once and shared, because `renderAuthoredArgument`'s closing
/// `trimEnd()` has to agree with `trim()` exactly — a second transcription is
/// a second chance to omit `U+FEFF` or to admit `U+0085`.
pub(crate) fn is_js_whitespace(c: char) -> bool {
    matches!(c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// `String.prototype.trimEnd`, over the same set as [`js_trim_is_empty`].
pub(crate) fn js_trim_end(value: &str) -> &str {
    value.trim_end_matches(is_js_whitespace)
}

/// One authored instant, validated exactly as the retained `instant()` does.
///
/// Two gates, and the split is the retained code's (quoin#436):
///
/// 1. The module's own shape regex, which is **stricter than RFC 3339 on
///    case** — an uppercase `T` and `Z` only.
/// 2. The ranges, which the retained code delegates to the shared strict
///    reader in `src/measurement/date-time.ts`.
///
/// Before quoin#436 the second gate was `Date.parse`, which does not reject an
/// impossible value — it ROLLS it, so `2026-02-30T00:00:00Z` became March 2
/// and the rolled number then decided whether an assumption was due for
/// review. That was fixed in the retained tree before this port was written,
/// precisely so this function is written against a rule rather than against a
/// rollover.
///
/// A leap second (`:60`) is refused, because both sides refuse it. No date
/// crate's default behaviour matches this combination, which is why it is
/// written out.
pub(crate) fn is_instant(value: &str) -> bool {
    instant_epoch_millis(value).is_some()
}

/// The same instant, as the NUMBER the retained `instant()` returns.
///
/// The retained `instant()` has one return value and two consumers: the
/// parser only asks whether it threw, and `buildAuthoredArgumentView` compares
/// the number against `asOf` to decide whether an assumption is due or an
/// accepted risk has expired. [`is_instant`] is the first consumer and this is
/// the second, over ONE grammar — a second parser for the comparison is
/// exactly the divergence quoin#436 records, where a rolled date decided a
/// reported status.
///
/// Milliseconds since the Unix epoch, matching `Date.parse`, including its
/// truncation of a sub-millisecond fraction: `Date.parse` reads at most three
/// fraction digits and discards the rest, so `.0009` and `.0001` are the same
/// instant on both sides and an ordering built from full precision would not
/// agree with the oracle at a boundary.
pub(crate) fn instant_epoch_millis(value: &str) -> Option<i64> {
    // The shape is pure ASCII, so a multi-byte character can only be a
    // rejection — and establishing that up front makes every `get` below a
    // character boundary by construction rather than by argument.
    if !value.is_ascii() {
        return None;
    }
    let (Some(head), Some(rest)) = (value.get(..19), value.get(19..)) else {
        return None;
    };
    if head.get(4..5) != Some("-")
        || head.get(7..8) != Some("-")
        || head.get(10..11) != Some("T")
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
        let Ok(parsed) = text.parse::<u32>() else {
            return None;
        };
        *slot = parsed;
    }
    let [year, month, day, hour, minute, second] = fields;

    // `(?:\.\d+)?` — a dot with no digit after it is not a fraction.
    let mut tail = rest;
    let mut fraction_millis = 0i64;
    if let Some(fraction) = tail.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        let (Some(remainder), Some(read)) = (fraction.get(digits..), fraction.get(..digits)) else {
            return None;
        };
        if digits == 0 {
            return None;
        }
        let mut scale = 100i64;
        for byte in read.bytes().take(3) {
            fraction_millis += i64::from(byte - b'0') * scale;
            scale /= 10;
        }
        tail = remainder;
    }

    // `Z` or `±HH:MM`, and nothing else. Lowercase `z` is refused: the shared
    // reader admits it and this module never has, so delegating the whole
    // check would have loosened the spelling while tightening the ranges.
    let mut offset_millis = 0i64;
    if tail != "Z" {
        let offset = tail.strip_prefix(['+', '-'])?;
        let (Some(offset_hour), Some(":"), Some(offset_minute)) =
            (offset.get(..2), offset.get(2..3), offset.get(3..))
        else {
            return None;
        };
        let (Ok(offset_hour), Ok(offset_minute)) =
            (offset_hour.parse::<u32>(), offset_minute.parse::<u32>())
        else {
            return None;
        };
        // `parse` admits a leading sign and whitespace; the digit check does
        // not, and the pattern is `\d{2}:\d{2}` exactly.
        if !offset
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b':')
            || offset.len() != 5
            || offset_hour > 23
            || offset_minute > 59
        {
            return None;
        }
        let magnitude = i64::from(offset_hour) * 3_600_000 + i64::from(offset_minute) * 60_000;
        // `+05:30` is five and a half hours AHEAD of UTC, so the UTC instant
        // is that much EARLIER than the digits read.
        offset_millis = if tail.starts_with('-') {
            -magnitude
        } else {
            magnitude
        };
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

    let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
    let seconds =
        days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
    Some(seconds * 1_000 + fraction_millis - offset_millis)
}

/// Days between the Unix epoch and a proleptic Gregorian date.
///
/// Howard Hinnant's `days_from_civil`, which is exact for every year this
/// grammar can spell (`0000`–`9999`) and needs no dependency. The alternative
/// was a date crate, and the workspace adds none for one arithmetic identity.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{is_instant, js_trim_is_empty};

    /// The two code points where `str::trim` would have been wrong, in
    /// opposite directions. See [`super::js_trim_is_empty`].
    #[test]
    fn js_trim_disagrees_with_rust_trim_in_both_directions() {
        // U+FEFF: JavaScript trims it, Rust keeps it. A naive port ACCEPTS an
        // owner the retained implementation refuses.
        assert!(js_trim_is_empty("\u{FEFF}"));
        assert!(!"\u{FEFF}".trim().is_empty());

        // U+0085: JavaScript keeps it, Rust trims it. A naive port REFUSES an
        // owner the retained implementation accepts.
        assert!(!js_trim_is_empty("\u{0085}"));
        assert!("\u{0085}".trim().is_empty());

        // Where they agree, they agree.
        assert!(js_trim_is_empty(" \t\n\u{00A0}\u{3000}"));
        assert!(!js_trim_is_empty("\u{200B}"));
    }

    #[test]
    fn instants_that_name_a_day_or_hour_that_does_not_exist_are_refused() {
        // quoin#436: `Date.parse` rolled these rather than rejecting them.
        assert!(!is_instant("2026-02-30T00:00:00Z"));
        assert!(!is_instant("2026-06-31T00:00:00Z"));
        assert!(!is_instant("2025-02-29T00:00:00Z"));
        assert!(!is_instant("2026-08-15T24:00:00Z"));
        // A leap second, which the retained code refuses and chrono accepts.
        assert!(!is_instant("2026-08-15T23:59:60Z"));
        // An offset out of range.
        assert!(!is_instant("2026-08-15T00:00:00+99:99"));
        // Lowercase, which this module has never accepted.
        assert!(!is_instant("2026-08-15t00:00:00Z"));
        assert!(!is_instant("2026-08-15T00:00:00z"));
        // A dot with no fraction after it.
        assert!(!is_instant("2026-08-15T00:00:00.Z"));

        // Accepted: a real leap day, a fraction, and an in-range offset.
        assert!(is_instant("2028-02-29T00:00:00Z"));
        assert!(is_instant("2026-08-15T00:00:00.000Z"));
        assert!(is_instant("2026-08-15T00:00:00+05:30"));
        assert!(is_instant("2026-08-15T23:59:59-11:00"));
    }
}
