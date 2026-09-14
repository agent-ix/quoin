// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The captured oracle corpus, and the one way to turn a case into inputs.
//!
//! `tests/goldens/graph-analysis.json` was written once by a capture script
//! run against the retained `src/graph-analysis/`. Both are deleted at HEAD
//! (quoin#500) and neither was ever consulted at test time (FR-101-AC-5); the
//! corpus records where each one lived, as `<revision>:<path>`, in its own
//! `provenance` block. Two tests read it, so the reading lives here rather
//! than twice.
//!
//! Provenance: quoin#385, quoin#500

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test-support module is compiled into several tests, each of which uses part of \
              it; and in a test, a panic IS the failure report"
)]

use std::path::Path;

use quoin_graph_analysis::model::binding::{BindingInput, parse_bindings_file};
use quoin_graph_analysis::{
    GraphAnalysisInput, parse_accepted_premises, parse_assurance_export, parse_audit_envelope,
};
use serde_json::Value;

/// The captured corpus, whole.
pub(crate) fn corpus() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
        .join("graph-analysis.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display())),
    )
    .expect("the captured corpus is JSON")
}

/// The four inputs of one analysis case, read through this crate's own
/// contracts.
///
/// The case records the inputs as JSON values, exactly as the oracle held
/// them; they are re-serialised here so that the same parsers the loader uses
/// read them. Reading them any other way would test a path no caller has.
pub(crate) fn input(case: &Value) -> GraphAnalysisInput {
    let id = case["id"].as_str().expect("a case id");
    let raw = &case["input"];
    let assurance = parse_assurance_export(&raw["assurance"].to_string())
        .unwrap_or_else(|error| panic!("{id}: the captured export is valid: {error}"));
    let premises = parse_accepted_premises(&raw["premises"].to_string())
        .unwrap_or_else(|error| panic!("{id}: the captured premises are valid: {error}"));
    let audit = parse_audit_envelope(&raw["audit"].to_string())
        .unwrap_or_else(|error| panic!("{id}: the captured audit is valid: {error}"));
    GraphAnalysisInput {
        assurance,
        premises,
        audit,
        bindings: bindings(id, &raw["bindings"]),
    }
}

/// The `{ availability, bindings | reason }` shape the oracle recorded.
pub(crate) fn bindings(id: &str, value: &Value) -> BindingInput {
    let reason = || {
        value["reason"]
            .as_str()
            .unwrap_or_else(|| panic!("{id}: an unavailable store records a reason"))
            .to_owned()
    };
    match value["availability"].as_str() {
        Some("available") => BindingInput::Available(
            parse_bindings_file(&serde_json::json!({
                "schemaVersion": quoin_store::STORE_SCHEMA_VERSION,
                "bindings": value["bindings"].clone(),
            }))
            .unwrap_or_else(|error| panic!("{id}: the captured bindings are valid: {error}")),
        ),
        Some("absent") => BindingInput::Absent { reason: reason() },
        Some("unreadable") => BindingInput::Unreadable { reason: reason() },
        other => panic!("{id}: unknown bindings availability {other:?}"),
    }
}
