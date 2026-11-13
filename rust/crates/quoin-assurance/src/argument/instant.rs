// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! JavaScript's trimmed set and the authored instant grammar (quoin#436).
//!
//! Two readings the host language decides rather than the schema: what
//! `String.prototype.trim` removes, and which instants `Date.parse`
//! accepts. Both are transcribed here once and shared, because a second
//! transcription is a second chance to disagree with the oracle. The instant
//! grammar itself lives in [`quoin_store::instant`] (PLAT-91).

use quoin_store::instant::{Instant, InstantGrammar};

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
    Instant::parse(value, InstantGrammar::CalendarInstant)?.epoch_millis()
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
        assert_ne!("\u{FEFF}".trim().len(), 0);

        // U+0085: JavaScript keeps it, Rust trims it. A naive port REFUSES an
        // owner the retained implementation accepts.
        assert!(!js_trim_is_empty("\u{0085}"));
        assert_eq!("\u{0085}".trim().len(), 0);

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
