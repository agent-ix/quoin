// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two-way adapter between [`quoin_store::JsonValue`] and
//! [`serde_json::Value`].
//!
//! # Why there are two value types at all
//!
//! They are not redundant. [`quoin_store::JsonValue`] is the *retained
//! evidence* value: every number in it is an IEEE-754 double, member order is
//! ECMAScript own-property order, and [`quoin_store::canonical_json_bytes`] is
//! the only writer whose bytes a retained record can be re-identified by.
//! [`serde_json::Value`] is the *analysis* value: it is what
//! [`quoin_jsonschema`] validates, what [`quoin_yaml`] returns and what
//! `serde` derives against.
//!
//! Operational intake needs both — the schema check and the typed record want
//! the second, the byte-identical write wants the first — so the crossing is
//! declared once, here, rather than at each of the four places that cross it.
//!
//! # Neither direction is a second JSON implementation
//!
//! [`to_serde`] crosses **through [`quoin_store::canonical_json_bytes`]**, the
//! same route `quoin_core::ops::change_assurance::support::to_serde` takes: the
//! store's writer states the bytes and `serde_json` reads them back, so no
//! number, no escape and no member order is decided here.
//!
//! [`from_serde`] cannot take that route — `tc_468_boundary` forbids every
//! `serde_json` serializing entry point in this crate, because a
//! second JSON *writer* is exactly what FR-100-CON-4 refuses. It is a
//! structural walk instead: it moves nodes across and decides nothing, and the
//! bytes are still [`quoin_store::canonical_json_bytes`]'s when they are
//! written.

use quoin_store::{JsonObject, JsonValue, canonical_json_bytes};
use serde_json::Value;

use crate::error::{MeasurementError, MeasurementErrorCode};

/// Read a store value as an analysis value.
///
/// # Errors
///
/// [`MeasurementErrorCode::Store`] when the value cannot be canonicalised —
/// which is a non-finite number, the one thing JSON has no spelling for.
pub fn to_serde(value: &JsonValue) -> Result<Value, MeasurementError> {
    let bytes = canonical_json_bytes(value)?;
    serde_json::from_slice(&bytes).map_err(|error| {
        MeasurementError::new(
            MeasurementErrorCode::Store,
            format!("canonical JSON did not read back: {error}"),
        )
    })
}

/// Read an analysis value as a store value.
///
/// # Errors
///
/// [`MeasurementErrorCode::Store`] for a number no double can hold, which is
/// the only node `serde_json` can express and retained evidence cannot.
pub fn from_serde(value: &Value) -> Result<JsonValue, MeasurementError> {
    Ok(match value {
        Value::Null => JsonValue::Null,
        Value::Bool(flag) => JsonValue::Bool(*flag),
        Value::Number(number) => {
            let as_double = number.as_f64().ok_or_else(|| {
                MeasurementError::new(
                    MeasurementErrorCode::Store,
                    format!("number {number} is not an IEEE-754 double"),
                )
            })?;
            JsonValue::number(as_double)?
        }
        Value::String(text) => JsonValue::string(text.clone()),
        Value::Array(items) => JsonValue::Array(
            items
                .iter()
                .map(from_serde)
                .collect::<Result<Vec<_>, MeasurementError>>()?,
        ),
        Value::Object(members) => {
            let mut object = JsonObject::new();
            for (name, member) in members {
                // `set`, not `insert`: `serde_json::Map` cannot hold a
                // duplicate name, so there is no duplicate to refuse, and
                // `insert`'s refusal would be unreachable.
                object.set(name.clone(), from_serde(member)?);
            }
            JsonValue::Object(object)
        }
    })
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
    use quoin_store::{canonical_json_bytes, parse_strict_json_str};

    use super::{from_serde, to_serde};

    /// The crossing is lossless in the only measure that matters: the bytes.
    #[test]
    fn a_document_survives_both_crossings_byte_for_byte() {
        // Sorted member order, because `canonical_json` writes ECMAScript own
        // property order and the third assertion below compares against it.
        let source = concat!(
            "{\n",
            "  \"large\": 100000000000000000000,\n",
            "  \"nested\": [\n",
            "    null,\n",
            "    true,\n",
            "    {\n",
            "      \"a\": []\n",
            "    }\n",
            "  ],\n",
            "  \"ratio\": 0.5,\n",
            "  \"schema_version\": 1,\n",
            "  \"text\": \"a \u{e9} \u{1f600}\"\n",
            "}\n"
        );
        let stored = parse_strict_json_str(source).unwrap();
        let expected = canonical_json_bytes(&stored).unwrap();
        let crossed = from_serde(&to_serde(&stored).unwrap()).unwrap();
        assert_eq!(canonical_json_bytes(&crossed).unwrap(), expected);
        assert_eq!(String::from_utf8(expected).unwrap(), source);
    }

    /// An integer that crossed does not come back spelled as a float.
    #[test]
    fn an_integer_keeps_its_integer_spelling() {
        let stored = parse_strict_json_str("{\n  \"n\": 1\n}\n").unwrap();
        let crossed = from_serde(&to_serde(&stored).unwrap()).unwrap();
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&crossed).unwrap()).unwrap(),
            "{\n  \"n\": 1\n}\n"
        );
    }
}
