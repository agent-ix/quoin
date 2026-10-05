// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Adapt store values to the shared RFC 8785 encoder without implementing
//! JSON encoding or member ordering locally.

use crate::error::StoreError;
use crate::json::{MAX_NESTING_DEPTH, value::JsonValue};
use quire_canonical::{Limits, Writer};

/// The RFC 8785 canonical text of `value`.
///
/// # Errors
/// The store depth refusal or a typed shared encoder refusal.
pub fn canonicalize_jcs(value: &JsonValue) -> Result<String, StoreError> {
    String::from_utf8(canonical_bytes(value)?).map_err(|_| StoreError::JsonNotUtf8)
}

/// The bytes shared by every canonical-domain digest.
///
/// # Errors
/// As [`canonicalize_jcs`]. No partial output is returned on refusal.
pub fn canonical_bytes(value: &JsonValue) -> Result<Vec<u8>, StoreError> {
    enum Frame<'a> {
        Value(&'a JsonValue, usize),
        Array(std::slice::Iter<'a, JsonValue>, usize),
        Object(
            std::collections::btree_map::Iter<'a, String, JsonValue>,
            usize,
        ),
    }
    fn encoding(source: quire_canonical::Error) -> StoreError {
        StoreError::CanonicalEncoding { source }
    }
    let mut output = Vec::new();
    let mut writer = Writer::new(&mut output, Limits::new(u64::MAX));
    let mut stack = vec![Frame::Value(value, 0)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Value(value, depth) => {
                if depth > MAX_NESTING_DEPTH {
                    return Err(StoreError::JsonNestingTooDeep {
                        limit: MAX_NESTING_DEPTH,
                        offset: 0,
                    });
                }
                match value {
                    JsonValue::Null => writer.null().map_err(encoding)?,
                    JsonValue::Bool(value) => writer.bool(*value).map_err(encoding)?,
                    JsonValue::Number(value) => writer.number(value.get()).map_err(encoding)?,
                    JsonValue::String(value) => writer.string(value).map_err(encoding)?,
                    JsonValue::Array(items) => {
                        writer.begin_array().map_err(encoding)?;
                        stack.push(Frame::Array(items.iter(), depth + 1));
                    }
                    JsonValue::Object(object) => {
                        writer.begin_object().map_err(encoding)?;
                        stack.push(Frame::Object(object.into_iter(), depth + 1));
                    }
                }
            }
            Frame::Array(mut items, depth) => {
                if let Some(item) = items.next() {
                    stack.push(Frame::Array(items, depth));
                    stack.push(Frame::Value(item, depth));
                } else {
                    writer.end_array().map_err(encoding)?;
                }
            }
            Frame::Object(mut members, depth) => {
                if let Some((name, value)) = members.next() {
                    writer.name(name).map_err(encoding)?;
                    stack.push(Frame::Object(members, depth));
                    stack.push(Frame::Value(value, depth));
                } else {
                    writer.end_object().map_err(encoding)?;
                }
            }
        }
    }
    writer.finish().map_err(encoding)?;
    Ok(output)
}
