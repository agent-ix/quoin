// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The typed payload boundary consumes the linked engine's wire contract.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::PathBuf;

use ix_trace_rs::trace;
use quoin_quire::{
    CoveragePayload, ModuleSelection, PayloadLimit, ScopeRoot, ids, payload, properties,
};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("repo")
}

fn fixture_modules(root: &PathBuf) -> ModuleSelection {
    ModuleSelection::Closed(vec![
        ids::ModuleRoot::open(root).expect("fixture module exists"),
    ])
}

#[trace("FR-029-AC-2")]
#[test]
fn tc_379_120_linked_engine_coverage_and_properties_payloads_validate() {
    let root = fixture_root();
    let scope = ScopeRoot::open(&root).expect("fixture scope exists");
    let modules = fixture_modules(&root);

    let coverage = quoin_quire::coverage::compute(&quoin_quire::coverage::Request {
        scope: scope.clone(),
        modules: modules.clone(),
    })
    .expect("fixture coverage computes");
    let coverage_bytes = serde_json::to_vec(&CoveragePayload::from_report(coverage.report))
        .expect("the linked engine coverage report serializes");
    payload::from_slice::<CoveragePayload>(
        "fixture coverage",
        "CoverageReport",
        &coverage_bytes,
        PayloadLimit::DEFAULT,
    )
    .expect("the linked engine coverage report validates as its own type");

    let properties = quoin_quire::properties::classify(&properties::Request {
        documents: properties::documents_under_spec(&scope).expect("fixture spec exists"),
        scope,
        modules,
        archetype: None,
    })
    .expect("fixture properties classify");
    let properties_bytes =
        serde_json::to_vec(&properties.report).expect("the properties report serializes");
    payload::from_slice::<properties::Report>(
        "fixture properties",
        "PropertiesReport",
        &properties_bytes,
        PayloadLimit::DEFAULT,
    )
    .expect("the properties report validates as its published Rust type");
}

#[trace("FR-029-AC-3")]
#[test]
fn tc_379_121_missing_fields_and_closed_engine_enums_name_the_failing_path() {
    let missing = payload::from_slice::<properties::Report>(
        "properties",
        "PropertiesReport",
        br#"{"documents":[{}]}"#,
        PayloadLimit::DEFAULT,
    )
    .expect_err("a required document field is missing");
    assert!(missing.to_string().contains("document"), "{missing}");

    let root = fixture_root();
    let outcome = quoin_quire::properties::classify(&properties::Request {
        documents: properties::documents_under_spec(
            &ScopeRoot::open(&root).expect("fixture scope exists"),
        )
        .expect("fixture spec exists"),
        scope: ScopeRoot::open(&root).expect("fixture scope exists"),
        modules: fixture_modules(&root),
        archetype: None,
    })
    .expect("fixture properties classify");
    let mut value = serde_json::to_value(outcome.report).expect("report serializes");
    value["documents"][0]["criteria"][0]["shape"] = serde_json::json!("future-shape");
    let invalid_enum = serde_json::to_vec(&value).expect("mutated report serializes");
    let error = payload::from_slice::<properties::Report>(
        "properties",
        "PropertiesReport",
        &invalid_enum,
        PayloadLimit::DEFAULT,
    )
    .expect_err("a value outside the closed engine enum is rejected");
    let message = error.to_string();
    assert!(message.contains("shape"), "{message}");
    assert!(message.contains("future-shape"), "{message}");

    // The linked engine types intentionally accept additive object fields.
    // This keeps an older consumer compatible with a newer publisher.
    let additive = payload::from_slice::<properties::Report>(
        "properties",
        "PropertiesReport",
        br#"{"documents":[],"future_engine_field":true}"#,
        PayloadLimit::DEFAULT,
    )
    .expect("an additive engine field remains consumable");
    assert!(additive.documents.is_empty());
}

#[trace("FR-029-AC-4")]
#[test]
fn tc_379_122_unreadable_json_is_a_named_payload_violation() {
    let error = payload::from_slice::<CoveragePayload>(
        "coverage artifact",
        "CoverageReport",
        b"{",
        PayloadLimit::DEFAULT,
    )
    .expect_err("malformed JSON must be reported");
    assert_eq!(error.code(), quoin_quire::ErrorCode::PayloadNotJson);
    let message = error.to_string();
    assert!(message.contains("coverage artifact"), "{message}");
    assert!(message.contains("JSON"), "{message}");
}

#[trace("FR-029-CON-3")]
#[test]
fn tc_379_123_diagnostic_reasons_follow_the_linked_engine_open_contract() {
    let root = fixture_root();
    let outcome = quoin_quire::coverage::compute(&quoin_quire::coverage::Request {
        scope: ScopeRoot::open(&root).expect("fixture scope exists"),
        modules: fixture_modules(&root),
    })
    .expect("fixture coverage computes");
    let mut value = serde_json::to_value(CoveragePayload::from_report(outcome.report))
        .expect("coverage report serializes");
    value["diagnostics"] = serde_json::json!([{
        "declaration": "future-declaration",
        "reason": "future-engine-reason",
        "message": "the publisher added a diagnostic reason"
    }]);
    let bytes = serde_json::to_vec(&value).expect("mutated coverage report serializes");
    let report = payload::from_slice::<CoveragePayload>(
        "coverage",
        "CoverageReport",
        &bytes,
        PayloadLimit::DEFAULT,
    )
    .expect("the consumer accepts an open diagnostic reason vocabulary");
    assert_eq!(report.report.diagnostics[0].reason, "future-engine-reason");
}
