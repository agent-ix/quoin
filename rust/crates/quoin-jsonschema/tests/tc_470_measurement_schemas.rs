// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The committed measurement schema documents parse and name themselves.
//!
//! Trace: FR-100-AC-4
//! Provenance: quoin#479

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_jsonschema::MeasurementSchema;
use serde_json::Value;

/// Both vendored documents parse, and the census is not measuring nothing.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_every_vendored_schema_parses() {
    for schema in MeasurementSchema::ALL {
        let document = schema
            .document()
            .unwrap_or_else(|e| panic!("{schema} must parse: {e}"));
        assert_eq!(
            document.get("$id").and_then(Value::as_str),
            Some(schema.id()),
            "{schema} must carry the $id this crate reports for it"
        );
    }
}
