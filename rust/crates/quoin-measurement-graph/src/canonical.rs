// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The crate's only route from a value to canonical JSON text.
//!
//! # There is no JSON writer here
//!
//! Three writers exist in this workspace and all three are `quoin-store`'s:
//! [`quoin_store::canonical_json`] (the evidence store's pretty form, which
//! `src/store/canonical.ts` states), [`quoin_store::canonical_bytes`] (RFC 8785
//! JCS, compact) and [`quoin_store::canonical_json_bytes`]. This module calls
//! them. It does not add a fourth, and no `serde_json` *text* writer is reached
//! for anywhere in this crate — `serde_json::to_value` produces a value, never
//! bytes, and the bytes are always the store's.
//!
//! The crossing between [`serde_json::Value`] and [`quoin_store::JsonValue`] is
//! likewise not re-declared: it is
//! [`quoin_measurement::json_bridge`], the one crossing the workspace has.
//!
//! # Two forms, because the retained code uses two
//!
//! [`pretty_text`] is `canonicalJson(value)` from `src/store/canonical.ts` —
//! two-space indent, trailing newline — which `graph-adapters.ts` uses for
//! premise comparison, for the duplicate-partition key and for the observation
//! sort order.
//!
//! [`compact_bytes`] is the identity form `graphQualityObservationId` digests.
//! See [`crate::quality::identity`] for the one measured difference between it
//! and the retained spelling.

use quoin_measurement::json_bridge;
use quoin_store::{canonical_bytes, canonical_json};
use serde_json::Value;

use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};

fn store_value(value: &Value) -> Result<quoin_store::JsonValue> {
    json_bridge::from_serde(value)
        .map_err(|error| GraphAdapterError::new(GraphAdapterErrorCode::Store, error.to_string()))
}

/// The evidence store's pretty canonical text, with the trailing newline kept.
///
/// # Errors
///
/// [`GraphAdapterErrorCode::Store`] for a value the store cannot hold, which is
/// a number no IEEE-754 double can express.
pub fn pretty_text(value: &Value) -> Result<String> {
    Ok(canonical_json(&store_value(value)?)?)
}

/// The same text with its trailing newline removed, as `.trimEnd()` leaves it.
///
/// # Errors
///
/// As [`pretty_text`].
pub fn pretty_text_trimmed(value: &Value) -> Result<String> {
    Ok(pretty_text(value)?.trim_end().to_owned())
}

/// The RFC 8785 compact canonical bytes.
///
/// # Errors
///
/// As [`pretty_text`].
pub fn compact_bytes(value: &Value) -> Result<Vec<u8>> {
    Ok(canonical_bytes(&store_value(value)?)?)
}
