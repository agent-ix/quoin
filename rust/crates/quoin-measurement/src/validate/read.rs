// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading members off a [`JsonValue`], with the retained validator's
//! coercions and no others.
//!
//! # The one coercion that is reproduced
//!
//! `validate.ts:118` and `:137` read digests as `String(value[key] ?? "")`,
//! not as a `typeof === "string"` test. Every non-string that reaches those
//! lines fails the digest pattern anyway — `String(5)` is `"5"`,
//! `String(null)` is `""`, `String({})` is `"[object Object]"` — with one
//! exception recorded in `DIVERGENCE.md`: a single-element array of the right
//! string coerces to that string and passes in TypeScript. This module reads
//! strings as strings, so it refuses that input.

use quoin_store::{JsonObject, JsonValue};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::ids::NonEmptyText;

/// Borrow `value` as an object, or refuse with `code`.
pub(crate) fn object<'a>(
    value: &'a JsonValue,
    code: MeasurementErrorCode,
    subject: &str,
) -> Result<&'a JsonObject, MeasurementError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(MeasurementError::new(code, subject.to_owned())),
    }
}

/// The string at `name`, if it is there and is a string.
pub(crate) fn string<'a>(object: &'a JsonObject, name: &str) -> Option<&'a str> {
    object.get(name).and_then(JsonValue::as_str)
}

/// The number at `name`, if it is there and is a number.
pub(crate) fn number(object: &JsonObject, name: &str) -> Option<f64> {
    object.get(name).and_then(JsonValue::as_f64)
}

/// The boolean at `name`, if it is there and is a boolean.
pub(crate) fn boolean(object: &JsonObject, name: &str) -> Option<bool> {
    match object.get(name) {
        Some(JsonValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

/// The non-empty string at `name`, refusing an absent, non-string or empty one.
///
/// This is `validate.ts:76-84`'s loop, which tests
/// `typeof value[key] !== "string" || value[key].length === 0` and names the
/// key it was reading.
pub(crate) fn non_empty(
    object: &JsonObject,
    name: &str,
    code: MeasurementErrorCode,
    subject: &str,
) -> Result<NonEmptyText, MeasurementError> {
    let Some(text) = string(object, name) else {
        return Err(MeasurementError::new(
            code,
            format!("{subject} requires non-empty `{name}`"),
        ));
    };
    NonEmptyText::parse(text, code, name)
        .map_err(|_| MeasurementError::new(code, format!("{subject} requires non-empty `{name}`")))
}

/// Whether `object` holds a member at `name` at all, however valued.
///
/// `validate.ts:87-90` uses `"scope" in value`, which is satisfied by an
/// explicit `null`. Presence, not truthiness.
pub(crate) fn present(object: &JsonObject, name: &str) -> bool {
    object.contains(name)
}
