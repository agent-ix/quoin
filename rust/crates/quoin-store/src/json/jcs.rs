// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! RFC 8785 JSON Canonicalization Scheme — the bytes every digest is taken over.
//!
//! Compact: no whitespace anywhere, `,` and `:` bare. Member names sorted by
//! UTF-16 code unit ([`crate::json::order::cmp_utf16`]). Numbers in ECMAScript
//! form ([`crate::json::number`]). Strings escaped as `JSON.stringify`
//! escapes them ([`crate::json::escape`]), with no Unicode normalization.
//!
//! The two conditions the oracle throws on — a non-finite number and a lone
//! surrogate — cannot be represented by [`JsonValue`], so they are refused at
//! the parser or at `JsonNumber::new` and never reach here. One refusal
//! remains: this writer recurses, so it carries the same
//! [`MAX_NESTING_DEPTH`](crate::json::MAX_NESTING_DEPTH) budget the reader
//! does. A value built in memory rather than parsed can exceed it, and
//! refusing is the alternative to aborting the process.

use crate::error::StoreError;
use crate::json::MAX_NESTING_DEPTH;
use crate::json::escape::write_json_string;
use crate::json::number::format_number;
use crate::json::order::jcs_order;
use crate::json::value::JsonValue;

/// The RFC 8785 canonical text of `value`.
///
/// # Errors
///
/// [`StoreError::JsonNestingTooDeep`] when `value` nests deeper than
/// [`MAX_NESTING_DEPTH`].
pub fn canonicalize_jcs(value: &JsonValue) -> Result<String, StoreError> {
    let mut out = String::new();
    write(&mut out, value, 0)?;
    Ok(out)
}

/// The RFC 8785 canonical UTF-8 bytes of `value`.
///
/// This is the *only* byte sequence a canonical-domain digest is ever taken
/// over.
///
/// # Errors
///
/// As [`canonicalize_jcs`].
pub fn canonical_bytes(value: &JsonValue) -> Result<Vec<u8>, StoreError> {
    canonicalize_jcs(value).map(String::into_bytes)
}

fn write(out: &mut String, value: &JsonValue, depth: usize) -> Result<(), StoreError> {
    if depth > MAX_NESTING_DEPTH {
        return Err(StoreError::JsonNestingTooDeep {
            limit: MAX_NESTING_DEPTH,
            offset: 0,
        });
    }
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(number) => out.push_str(&format_number(number.get())),
        JsonValue::String(text) => write_json_string(out, text),
        JsonValue::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write(out, item, depth + 1)?;
            }
            out.push(']');
        }
        JsonValue::Object(object) => {
            out.push('{');
            for (index, name) in jcs_order(object.names()).into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json_string(out, name);
                out.push(':');
                // `name` came from this object's own key set.
                if let Some(member) = object.get(name) {
                    write(out, member, depth + 1)?;
                }
            }
            out.push('}');
        }
    }
    Ok(())
}
