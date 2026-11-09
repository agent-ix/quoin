// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The evidence store's on-disk form: `canonicalJson`.
//!
//! Two-space indent, `": "` between name and value, one trailing newline. It
//! is **not** JCS and the two must never be confused: RFC 8785 is the
//! canonical-bytes form. This is what `spec/evidence/**.json` contains so that a pull
//! request diff of the store *is* the per-PR delta.
//!
//! The member order is the subtle part. The oracle is
//! `JSON.stringify(sortKeys(value), null, 2)`, and `sortKeys` builds a fresh
//! JavaScript object by inserting the sorted names one at a time. Insertion
//! does not preserve that order: ECMAScript emits array-index names first in
//! ascending numeric order, then the rest in insertion order. A store object
//! keyed by numbers — `{"10": …, "2": …}` — therefore serializes as `2` before
//! `10`, which is neither Rust's `BTreeMap` order nor JCS order. This is a storage presentation policy, never the canonical identity order.

use crate::error::StoreError;
use crate::json::MAX_NESTING_DEPTH;
use crate::json::escape::write_json_string;
use crate::json::value::JsonValue;

const INDENT: &str = "  ";

/// The evidence store's canonical text for `value`, trailing newline included.
///
/// # Errors
///
/// [`StoreError::JsonNestingTooDeep`] when `value` nests deeper than
/// [`MAX_NESTING_DEPTH`]. This writer recurses, so the budget is what keeps a
/// value built in memory from aborting the process.
pub fn canonical_json(value: &JsonValue) -> Result<String, StoreError> {
    let mut out = String::new();
    write(&mut out, value, 0)?;
    out.push('\n');
    Ok(out)
}

/// The evidence store's canonical bytes for `value`.
///
/// # Errors
///
/// As [`canonical_json`].
pub fn canonical_json_bytes(value: &JsonValue) -> Result<Vec<u8>, StoreError> {
    canonical_json(value).map(String::into_bytes)
}

fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str(INDENT);
    }
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
        JsonValue::Number(number) => out.push_str(&crate::json::canonical::canonicalize_jcs(
            &JsonValue::Number(*number),
        )?),
        JsonValue::String(text) => write_json_string(out, text),
        JsonValue::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return Ok(());
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                push_indent(out, depth + 1);
                write(out, item, depth + 1)?;
            }
            out.push('\n');
            push_indent(out, depth);
            out.push(']');
        }
        JsonValue::Object(object) => {
            if object.is_empty() {
                out.push_str("{}");
                return Ok(());
            }
            out.push_str("{\n");
            for (index, name) in storage_names(object.names()).into_iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                push_indent(out, depth + 1);
                write_json_string(out, name);
                out.push_str(": ");
                // `name` came from this object's own key set.
                if let Some(member) = object.get(name) {
                    write(out, member, depth + 1)?;
                }
            }
            out.push('\n');
            push_indent(out, depth);
            out.push('}');
        }
    }
    Ok(())
}

// JSON.stringify presents canonical array-index names numerically first.
// This affects readable storage only; the shared encoder owns identity order.
fn storage_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    fn array_index(name: &str) -> Option<u32> {
        let value: u32 = name.parse().ok()?;
        (value < u32::MAX && value.to_string() == name).then_some(value)
    }
    let mut names: Vec<_> = names.collect();
    names.sort_unstable_by(
        |left, right| match (array_index(left), array_index(right)) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.encode_utf16().cmp(right.encode_utf16()),
        },
    );
    names
}

#[cfg(test)]
mod tests {
    #[test]
    fn storage_indices_remain_presentation_order_not_identity_order() {
        assert_eq!(
            super::storage_names(
                [
                    "10",
                    "2",
                    "01",
                    "4294967294",
                    "4294967295",
                    "\u{10000}",
                    "\u{fffd}"
                ]
                .into_iter()
            ),
            [
                "2",
                "10",
                "4294967294",
                "01",
                "4294967295",
                "\u{10000}",
                "\u{fffd}"
            ]
        );
    }
}
