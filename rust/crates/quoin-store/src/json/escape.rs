// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! String escaping, `JSON.stringify` exactly.
//!
//! RFC 8785 §3.2.2.2 and `JSON.stringify` agree: escape `"` and `\`, use the
//! two-character forms for `\b \t \n \f \r`, use `\u00xx` with **lowercase**
//! hex digits for every other C0 control, and emit everything else — including
//! `U+007F` and every non-ASCII character — as literal UTF-8.
//!
//! No Unicode normalization is applied. JCS explicitly does not normalize, so
//! a name spelled `e` + `U+0301` and a name spelled `U+00E9` are two different
//! members with two different digests. Normalizing would silently merge
//! records that the retained evidence holds apart.

/// Append the canonical JSON string literal for `text`, quotes included.
pub fn write_json_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{0009}' => out.push_str("\\t"),
            '\u{000A}' => out.push_str("\\n"),
            '\u{000C}' => out.push_str("\\f"),
            '\u{000D}' => out.push_str("\\r"),
            control if (control as u32) < 0x20 => {
                use std::fmt::Write as _;
                // Infallible: writing into a String cannot fail.
                let _ = write!(out, "\\u{:04x}", control as u32);
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// The canonical JSON string literal for `text`, quotes included.
#[must_use]
pub fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    write_json_string(&mut out, text);
    out
}

#[cfg(test)]
mod tests {
    use super::json_string;

    #[test]
    fn tc_380_control_characters_use_the_short_forms_then_lowercase_hex() {
        assert_eq!(
            json_string("\u{8}\u{9}\u{a}\u{c}\u{d}\u{1f}\u{0}"),
            "\"\\b\\t\\n\\f\\r\\u001f\\u0000\""
        );
    }

    #[test]
    fn tc_380_del_and_non_ascii_are_emitted_literally() {
        assert_eq!(json_string("\u{7f}"), "\"\u{7f}\"");
        assert_eq!(json_string("é😀"), "\"é😀\"");
    }

    #[test]
    fn tc_380_only_quote_and_backslash_are_escaped_among_printables() {
        assert_eq!(json_string("a\"b\\c/d"), "\"a\\\"b\\\\c/d\"");
    }
}
