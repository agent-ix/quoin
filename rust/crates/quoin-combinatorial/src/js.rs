// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The three ECMAScript string primitives the retained implementation leans on.
//!
//! Ported for quoin#382 and reused by quoin#383. Each one is a place where the
//! obvious Rust spelling is *not* the retained behaviour, so each is written
//! once here rather than open-coded per call site.
//!
//! # `\s` is not `char::is_whitespace`
//!
//! ECMAScript's `WhiteSpace` production is `<TAB> <VT> <FF> <SP> <NBSP>
//! <ZWNBSP>` plus the Unicode `Zs` category, and a `RegExp` `\s` is that union
//! with `LineTerminator` (`LF CR LS PS`). Rust's `\s` in the `regex` crate is
//! the Unicode `White_Space` property, which does **not** contain `U+FEFF`
//! (ZWNBSP) — and `char::is_whitespace` does not contain it either. A statement
//! carrying a byte-order mark mid-string would therefore split into different
//! dimension values on the two sides. [`is_js_whitespace`] and
//! [`JS_WHITESPACE_CLASS`] are the retained set, exactly.
//!
//! # `<` on strings is UTF-16, not scalar-value, order
//!
//! The retained comparator is `a === b ? 0 : a < b ? -1 : 1`, and
//! `Array.prototype.sort`'s default comparator is UTF-16 lexicographic. Rust's
//! `str: Ord` compares Unicode scalar values, which disagrees above the BMP.
//! [`cmp_js`] is `quoin_store::json::order::cmp_utf16`, called — this crate
//! does not mint a second ordering.

use std::cmp::Ordering;

pub use quoin_store::json::order::cmp_utf16 as cmp_js;

/// Every character ECMAScript's `RegExp` `\s` matches, as a `regex` class body.
///
/// Written as a class **body** without brackets so a caller can negate it
/// (`[^…(]`) as well as use it positively (`[…]`). `U+FEFF` is the member Rust's
/// own `\s` lacks; it is why this constant exists rather than `\s`.
pub const JS_WHITESPACE_CLASS: &str =
    r"\t\n\x0B\f\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}";

/// The same set as UTF-8 bytes, for a `regex::bytes` pattern.
///
/// `regex::bytes` patterns in this workspace are compiled with Unicode syntax
/// on (so `\x{2264}` is a character, not a byte), which is why this is the same
/// spelling as [`JS_WHITESPACE_CLASS`]; it is named separately so a reader of a
/// byte pattern is not left wondering whether the class is byte-wise.
pub const JS_WHITESPACE_BYTES_CLASS: &str = JS_WHITESPACE_CLASS;

/// Is this character one ECMAScript's `\s` and `String.prototype.trim` accept?
#[must_use]
pub const fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{9}'
            | '\u{a}'
            | '\u{b}'
            | '\u{c}'
            | '\u{d}'
            | '\u{20}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `String.prototype.trim`, exactly.
#[must_use]
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_whitespace)
}

/// The retained `compare(a, b)` — `a === b ? 0 : a < b ? -1 : 1`.
///
/// Kept as its own name because the retained source spells it that way in both
/// `audit.ts` and `advise.ts`, each with the same comment: plain comparison and
/// never `localeCompare`, because a report that depends on the runtime's ICU
/// data is not reproducible (agent-ix/quoin#106).
#[must_use]
pub fn compare(left: &str, right: &str) -> Ordering {
    cmp_js(left, right)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{cmp_js, is_js_whitespace, js_trim};
    use std::cmp::Ordering;

    #[test]
    fn the_byte_order_mark_is_whitespace_here_and_is_not_in_rust() {
        // The one member of ECMAScript's `\s` that Rust's own definitions lack.
        // If this ever agrees with `char::is_whitespace`, the reason this
        // module exists has gone away and it should be deleted, not adjusted.
        assert!(is_js_whitespace('\u{feff}'));
        assert!(!'\u{feff}'.is_whitespace());
        assert_eq!(js_trim("\u{feff} a \u{feff}"), "a");
    }

    #[test]
    fn the_ordering_is_utf16_and_not_scalar_value() {
        // U+10000 is one scalar above U+FFFD, and two surrogate code units
        // (D800 DC00) below it. Rust's `str: Ord` says Greater; UTF-16 says
        // Less. The retained `<` says Less.
        assert_eq!(cmp_js("\u{10000}", "\u{fffd}"), Ordering::Less);
        assert_eq!("\u{10000}".cmp("\u{fffd}"), Ordering::Greater);
    }

    #[test]
    fn trim_removes_line_terminators_as_well_as_spaces() {
        assert_eq!(js_trim("\u{2028}\u{2029}\r\n\t x \u{3000}"), "x");
        assert_eq!(js_trim(""), "");
        assert_eq!(js_trim("\u{a0}"), "");
    }
}
