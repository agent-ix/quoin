// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The strict I-JSON reader, mirroring the oracle's `parseStrictJson`.
//!
//! This is deliberately not a general JSON reader. It refuses everything the
//! retained evidence must never contain, because a permissive reader would
//! accept a document whose re-serialization is a *different* document:
//!
//! * a byte-order mark;
//! * duplicate member names (the second silently wins in a permissive reader,
//!   so the digest depends on which one the reader kept);
//! * unescaped C0 controls in strings;
//! * unpaired surrogate escapes (`\uD800` with no `\uDC00` after it), which
//!   have no UTF-8 spelling and therefore no canonical bytes;
//! * numbers that overflow to an infinity;
//! * trailing content after the top-level value.
//!
//! Numbers are read as IEEE-754 doubles, exactly as the oracle does. That is a
//! frozen decision, not an oversight — see `COMPATIBILITY.md`.

use crate::error::StoreError;
use crate::json::MAX_NESTING_DEPTH;
use crate::json::value::{JsonObject, JsonValue};

/// Read a strict I-JSON document from UTF-8 bytes.
///
/// # Errors
///
/// Refuses input that is not UTF-8, and then as [`parse_strict_json_str`].
pub fn parse_strict_json(input: &[u8]) -> Result<JsonValue, StoreError> {
    let text = std::str::from_utf8(input).map_err(|_| StoreError::JsonNotUtf8)?;
    parse_strict_json_str(text)
}

/// Read a strict I-JSON document from text.
///
/// # Errors
///
/// Refuses a leading byte-order mark, anything the strict I-JSON grammar
/// rejects, a non-finite number, a duplicate member name, trailing content
/// after the document, and nesting past [`MAX_NESTING_DEPTH`].
pub fn parse_strict_json_str(text: &str) -> Result<JsonValue, StoreError> {
    if text.starts_with('\u{FEFF}') {
        return Err(StoreError::JsonByteOrderMark);
    }
    let mut parser = Parser {
        text,
        offset: 0,
        depth: 0,
    };
    let value = parser.value()?;
    parser.space();
    if parser.offset != text.len() {
        return Err(StoreError::JsonTrailingContent {
            offset: parser.offset,
        });
    }
    Ok(value)
}

/// One element of a string literal: a literal character, or one UTF-16 code
/// unit produced by a `\u` escape. They are kept apart because surrogate
/// pairing is defined over code units, not over characters.
enum Element {
    Character(char),
    CodeUnit(u16),
}

struct Parser<'a> {
    text: &'a str,
    offset: usize,
    /// Nesting depth of the value currently being read. Bounded by
    /// [`MAX_NESTING_DEPTH`]; see that constant for why a bound exists at all.
    depth: usize,
}

impl<'a> Parser<'a> {
    fn bytes(&self) -> &'a [u8] {
        self.text.as_bytes()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes().get(self.offset).copied()
    }

    fn take(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.offset += 1;
        }
    }

    fn value(&mut self) -> Result<JsonValue, StoreError> {
        self.space();
        match self.peek() {
            Some(b'{') => self.nested(Self::object),
            Some(b'[') => self.nested(Self::array),
            Some(b'"') => self.string().map(JsonValue::String),
            Some(b't') => self.literal("true").map(|()| JsonValue::Bool(true)),
            Some(b'f') => self.literal("false").map(|()| JsonValue::Bool(false)),
            Some(b'n') => self.literal("null").map(|()| JsonValue::Null),
            _ => self.number(),
        }
    }

    /// Read one composite value, charging it against the nesting budget.
    ///
    /// Only arrays and objects recurse, so the budget is charged here and
    /// nowhere else. Exceeding it is a refusal, not an abort: without it a
    /// document of 40,000 `[` ends this process with SIGABRT rather than with
    /// a diagnostic.
    fn nested(
        &mut self,
        read: fn(&mut Self) -> Result<JsonValue, StoreError>,
    ) -> Result<JsonValue, StoreError> {
        if self.depth >= MAX_NESTING_DEPTH {
            return Err(StoreError::JsonNestingTooDeep {
                limit: MAX_NESTING_DEPTH,
                offset: self.offset,
            });
        }
        self.depth += 1;
        let value = read(self);
        self.depth -= 1;
        value
    }

    fn object(&mut self) -> Result<JsonValue, StoreError> {
        self.offset += 1;
        self.space();
        let mut object = JsonObject::new();
        if self.take(b'}') {
            return Ok(JsonValue::Object(object));
        }
        loop {
            self.space();
            if self.peek() != Some(b'"') {
                return Err(StoreError::JsonObjectKeyNotString {
                    offset: self.offset,
                });
            }
            let name_offset = self.offset;
            let name = self.string()?;
            if object.contains(&name) {
                return Err(StoreError::JsonDuplicateName {
                    name,
                    offset: name_offset,
                });
            }
            self.space();
            if !self.take(b':') {
                return Err(StoreError::JsonExpectedColon {
                    offset: self.offset,
                });
            }
            let value = self.value()?;
            object.insert(name, value)?;
            self.space();
            if self.take(b'}') {
                return Ok(JsonValue::Object(object));
            }
            if !self.take(b',') {
                return Err(StoreError::JsonExpectedObjectSeparator {
                    offset: self.offset,
                });
            }
        }
    }

    fn array(&mut self) -> Result<JsonValue, StoreError> {
        self.offset += 1;
        self.space();
        let mut items = Vec::new();
        if self.take(b']') {
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.space();
            if self.take(b']') {
                return Ok(JsonValue::Array(items));
            }
            if !self.take(b',') {
                return Err(StoreError::JsonExpectedArraySeparator {
                    offset: self.offset,
                });
            }
        }
    }

    /// Match one of the three JSON literals at the cursor.
    ///
    /// The comparison is on BYTES. Slicing `self.text` here would panic
    /// whenever `offset + token.len()` lands inside a multi-byte character —
    /// `n\u{e9}\u{e9}` is five bytes, long enough to pass a length guard and
    /// split the second `\u{e9}` at byte 4. `<[u8]>::get` returns `None` for
    /// a short input instead, and byte equality is exactly literal equality
    /// because every token is ASCII.
    fn literal(&mut self, token: &'static str) -> Result<(), StoreError> {
        let end = self.offset + token.len();
        if self.bytes().get(self.offset..end) != Some(token.as_bytes()) {
            return Err(StoreError::JsonInvalidLiteral {
                expected: token,
                offset: self.offset,
            });
        }
        self.offset = end;
        Ok(())
    }

    fn string(&mut self) -> Result<String, StoreError> {
        let opened_at = self.offset;
        self.offset += 1;
        let mut out = String::new();
        let mut pending_high: Option<(u16, usize)> = None;
        loop {
            let element_offset = self.offset;
            let Some(byte) = self.peek() else {
                return Err(StoreError::JsonUnterminatedString { offset: opened_at });
            };
            if byte == b'"' {
                if let Some((_, offset)) = pending_high {
                    return Err(StoreError::JsonLoneHighSurrogate { offset });
                }
                self.offset += 1;
                return Ok(out);
            }
            if byte < 0x20 {
                return Err(StoreError::JsonUnescapedControl {
                    code: u32::from(byte),
                    offset: element_offset,
                });
            }
            let element = if byte == b'\\' {
                self.escape(opened_at)?
            } else {
                let character = self.text[self.offset..]
                    .chars()
                    .next()
                    .ok_or(StoreError::JsonUnterminatedString { offset: opened_at })?;
                self.offset += character.len_utf8();
                Element::Character(character)
            };
            match (pending_high.take(), element) {
                (Some((high, _)), Element::CodeUnit(low)) if is_low_surrogate(low) => {
                    out.push(combine_surrogates(high, low));
                }
                (Some((_, offset)), _) => {
                    return Err(StoreError::JsonLoneHighSurrogate { offset });
                }
                (None, Element::CodeUnit(unit)) if is_high_surrogate(unit) => {
                    pending_high = Some((unit, element_offset));
                }
                (None, Element::CodeUnit(unit)) if is_low_surrogate(unit) => {
                    return Err(StoreError::JsonLoneLowSurrogate {
                        offset: element_offset,
                    });
                }
                (None, Element::CodeUnit(unit)) => {
                    // Not a surrogate, so `from_u32` cannot fail.
                    out.push(char::from_u32(u32::from(unit)).unwrap_or('\u{FFFD}'));
                }
                (None, Element::Character(character)) => out.push(character),
            }
        }
    }

    fn escape(&mut self, opened_at: usize) -> Result<Element, StoreError> {
        let escape_offset = self.offset;
        self.offset += 1;
        let Some(kind) = self.peek() else {
            return Err(StoreError::JsonUnterminatedString { offset: opened_at });
        };
        self.offset += 1;
        let simple = match kind {
            b'"' => Some('"'),
            b'\\' => Some('\\'),
            b'/' => Some('/'),
            b'b' => Some('\u{0008}'),
            b'f' => Some('\u{000C}'),
            b'n' => Some('\u{000A}'),
            b'r' => Some('\u{000D}'),
            b't' => Some('\u{0009}'),
            _ => None,
        };
        if let Some(character) = simple {
            return Ok(Element::Character(character));
        }
        if kind != b'u' {
            return Err(StoreError::JsonInvalidEscape {
                offset: escape_offset,
            });
        }
        let end = self.offset + 4;
        let digits =
            self.text
                .as_bytes()
                .get(self.offset..end)
                .ok_or(StoreError::JsonInvalidEscape {
                    offset: escape_offset,
                })?;
        let mut unit: u16 = 0;
        for digit in digits {
            let nibble = (*digit as char)
                .to_digit(16)
                .ok_or(StoreError::JsonInvalidEscape {
                    offset: escape_offset,
                })?;
            // Four hex digits cannot exceed u16::MAX, so the shift cannot lose
            // information; `u16::try_from` states that rather than assuming it.
            unit = (unit << 4)
                | u16::try_from(nibble).map_err(|_| StoreError::JsonInvalidEscape {
                    offset: escape_offset,
                })?;
        }
        self.offset = end;
        Ok(Element::CodeUnit(unit))
    }

    fn number(&mut self) -> Result<JsonValue, StoreError> {
        let start = self.offset;
        let mut cursor = self.offset;
        let bytes = self.bytes();
        if bytes.get(cursor) == Some(&b'-') {
            cursor += 1;
        }
        match bytes.get(cursor) {
            Some(b'0') => cursor += 1,
            Some(byte) if byte.is_ascii_digit() => {
                while matches!(bytes.get(cursor), Some(byte) if byte.is_ascii_digit()) {
                    cursor += 1;
                }
            }
            _ => return Err(StoreError::JsonExpectedValue { offset: start }),
        }
        // The fraction is taken only when at least one digit follows the point,
        // mirroring the oracle's `(?:\.\d+)?`: `1.` parses as `1` and then
        // fails as trailing content, it is not a malformed number.
        if bytes.get(cursor) == Some(&b'.')
            && matches!(bytes.get(cursor + 1), Some(byte) if byte.is_ascii_digit())
        {
            cursor += 1;
            while matches!(bytes.get(cursor), Some(byte) if byte.is_ascii_digit()) {
                cursor += 1;
            }
        }
        if matches!(bytes.get(cursor), Some(b'e' | b'E')) {
            let mut lookahead = cursor + 1;
            if matches!(bytes.get(lookahead), Some(b'+' | b'-')) {
                lookahead += 1;
            }
            if matches!(bytes.get(lookahead), Some(byte) if byte.is_ascii_digit()) {
                cursor = lookahead;
                while matches!(bytes.get(cursor), Some(byte) if byte.is_ascii_digit()) {
                    cursor += 1;
                }
            }
        }
        let literal = &self.text[start..cursor];
        let value: f64 = literal
            .parse()
            .map_err(|_| StoreError::JsonExpectedValue { offset: start })?;
        if !value.is_finite() {
            return Err(StoreError::JsonNumberNotFinite {
                literal: literal.to_owned(),
                offset: start,
            });
        }
        self.offset = cursor;
        JsonValue::number(value).map_err(|_| StoreError::JsonNumberNotFinite {
            literal: literal.to_owned(),
            offset: start,
        })
    }
}

const fn is_high_surrogate(unit: u16) -> bool {
    unit >= 0xD800 && unit <= 0xDBFF
}

const fn is_low_surrogate(unit: u16) -> bool {
    unit >= 0xDC00 && unit <= 0xDFFF
}

fn combine_surrogates(high: u16, low: u16) -> char {
    let scalar = 0x1_0000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(low) - 0xDC00);
    char::from_u32(scalar).unwrap_or('\u{FFFD}')
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{parse_strict_json, parse_strict_json_str};
    use crate::error::StoreErrorCode;
    use crate::json::MAX_NESTING_DEPTH;

    /// A literal is matched over bytes, so a multi-byte character straddling
    /// the end of the token is a refusal and not a panic.
    ///
    /// `n\u{e9}\u{e9}` is five bytes — long enough to pass a
    /// `len() < offset + 4` guard, and its fourth byte lands inside the second
    /// `\u{e9}`. Slicing `&text[0..4]` there aborts the process with `end byte
    /// index 4 is not a char boundary`. The retained parser refuses the same
    /// input cleanly, so a panic is both a crash and a parity divergence.
    ///
    /// Trace: FR-098-AC-9
    #[test]
    fn tc_380_a_literal_split_by_a_multibyte_character_is_refused_not_panicking() {
        for (token, input) in [
            ("null", "n\u{e9}\u{e9}"),
            ("true", "t\u{e9}\u{e9}"),
            ("false", "f\u{e9}\u{e9}\u{e9}"),
        ] {
            let error = parse_strict_json_str(input)
                .expect_err(&format!("{token} prefix must be refused, never sliced"));
            assert_eq!(error.code(), StoreErrorCode::JsonInvalidLiteral, "{input}");
        }
    }

    /// The same boundary reached from the byte entry point, and one byte short
    /// of each literal so the length guard itself is exercised.
    ///
    /// Trace: FR-098-AC-9
    #[test]
    fn tc_380_a_truncated_literal_is_refused_at_every_length() {
        for token in ["null", "true", "false"] {
            for length in 1..token.len() {
                let error = parse_strict_json(&token.as_bytes()[..length])
                    .expect_err("a truncated literal is not a document");
                assert_eq!(error.code(), StoreErrorCode::JsonInvalidLiteral, "{token}");
            }
        }
    }

    /// Input that is not UTF-8 is refused before any canonicalization runs.
    ///
    /// Trace: FR-098-AC-9
    #[test]
    fn tc_380_non_utf8_input_is_refused_before_parsing() {
        let error = parse_strict_json(&[0x22, 0xFF, 0x22]).expect_err("not UTF-8");
        assert_eq!(error.code(), StoreErrorCode::JsonNotUtf8);
    }

    /// Nesting is bounded, and the refusal is a typed error rather than a
    /// stack overflow.
    ///
    /// An unbounded recursive descent parser aborts the process — `fatal
    /// runtime error: stack overflow`, SIGABRT — on input a caller does not
    /// control. A refusal is a result; an abort is not.
    #[test]
    fn tc_380_nesting_past_the_budget_is_refused_rather_than_overflowing() {
        let deep = format!(
            "{}1{}",
            "[".repeat(MAX_NESTING_DEPTH + 1),
            "]".repeat(MAX_NESTING_DEPTH + 1)
        );
        let error = parse_strict_json_str(&deep).expect_err("past the budget");
        assert_eq!(error.code(), StoreErrorCode::JsonNestingTooDeep);

        // The band that once aborted the process: far past any stack this
        // parser could have survived, and now just an error value.
        let hostile = format!("{}1{}", "[".repeat(60_000), "]".repeat(60_000));
        let error = parse_strict_json_str(&hostile).expect_err("far past the budget");
        assert_eq!(error.code(), StoreErrorCode::JsonNestingTooDeep);
    }

    /// Nesting exactly at the budget is still accepted, so the refusal is a
    /// boundary and not a retreat from it.
    #[test]
    fn tc_380_nesting_at_the_budget_is_accepted() {
        let at_budget = format!(
            "{}1{}",
            "[".repeat(MAX_NESTING_DEPTH),
            "]".repeat(MAX_NESTING_DEPTH)
        );
        let value = parse_strict_json_str(&at_budget).expect("at the budget");
        crate::json::jcs::canonicalize_jcs(&value).expect("both writers reach the same budget");
        crate::json::pretty::canonical_json(&value).expect("both writers reach the same budget");
    }

    /// Both canonical writers carry the same budget as the parser, so no value
    /// the parser accepts can overflow a writer.
    #[test]
    fn tc_380_both_writers_refuse_past_the_budget_they_share_with_the_parser() {
        let mut value = crate::json::value::JsonValue::number(1.0).expect("finite");
        for _ in 0..=MAX_NESTING_DEPTH {
            value = crate::json::value::JsonValue::Array(vec![value]);
        }
        assert_eq!(
            crate::json::jcs::canonicalize_jcs(&value)
                .expect_err("past the budget")
                .code(),
            StoreErrorCode::JsonNestingTooDeep
        );
        assert_eq!(
            crate::json::pretty::canonical_json(&value)
                .expect_err("past the budget")
                .code(),
            StoreErrorCode::JsonNestingTooDeep
        );
    }
}
