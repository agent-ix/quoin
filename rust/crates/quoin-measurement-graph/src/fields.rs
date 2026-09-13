// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a named member out of retained producer output.
//!
//! Ports `asRecord` and `stringField` (`graph-portfolio.ts:780-793`).
//!
//! # Why these are two functions and not `serde`
//!
//! `rawEvidence` is *complete producer output*: this crate does not own its
//! contract and `validate.ts` checks nothing inside it. A `Deserialize` impl
//! would be a claim about its shape, and a claim that failed would refuse a
//! collection the retained code reads happily. So the reads are what the
//! retained code does — "is this an object", "is this member a string" — and
//! anything else reads as absent.

use quoin_store::{JsonObject, JsonValue};

/// `asRecord(value)` (`graph-portfolio.ts:780-784`).
///
/// An array is *not* a record: `typeof [] === "object"` in JavaScript, so the
/// retained guard rules it out by hand and this one rules it out by type.
#[must_use]
pub fn record(value: Option<&JsonValue>) -> Option<&JsonObject> {
    match value {
        Some(JsonValue::Object(object)) => Some(object),
        _ => None,
    }
}

/// `stringField(value, key)` (`graph-portfolio.ts:786-793`).
///
/// A member that is present and is not a string reads as absent, exactly as
/// `typeof value?.[key] === "string"` does.
#[must_use]
pub fn string_field(record: Option<&JsonObject>, name: &str) -> Option<String> {
    record
        .and_then(|record| record.get(name))
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
}

/// The named member of a record, present or not.
#[must_use]
pub fn member<'a>(record: Option<&'a JsonObject>, name: &str) -> Option<&'a JsonValue> {
    record.and_then(|record| record.get(name))
}
