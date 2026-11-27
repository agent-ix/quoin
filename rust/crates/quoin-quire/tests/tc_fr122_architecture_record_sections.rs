// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-122 architecture record-section acceptance tests.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "invalid fixture evidence is a test failure"
)]

#[path = "support/record_sections.rs"]
mod record_sections;

use ix_trace_rs::trace;
use quire_rs::semantic::compile_documents;
use record_sections::*;

#[trace("FR-122-AC-1")]
#[test]
fn architecture_provider_exposes_and_policy_sections_map_to_valid_records() {
    for id in [
        "provider",
        "exposes",
        "delivery",
        "versioning",
        "registration-stability",
    ] {
        assert_fixture_record(id);
    }
    assert_error(
        "registration-bullet-list",
        "semantic.feature-not-extractable",
        "registration",
    );
}

#[trace("FR-122-AC-2")]
#[test]
fn architecture_routes_map_and_unknown_operations_are_refused() {
    let (case, output) = assert_fixture_record("routes");
    assert_eq!(output["record"]["routes"], case["record"]["routes"]);
    assert_error(
        "route-operation-unknown",
        "semantic.invalid-model-cell",
        "route-operation-unknown",
    );
}

#[trace("FR-122-AC-3")]
#[test]
fn associated_types_are_typed_refs_and_contract_fences_are_not_sources() {
    let (case, output) = run_fixture("associated-types");
    assert_eq!(
        output["record"]["associated_types"],
        case["record"]["associated_types"]
    );
    let (_, schema, documents) = fixture_case("associated-types");
    let validator = compile_documents(&schema["properties"]["associated_types"], &documents)
        .expect("TypeRef[] schema compiles");
    assert!(
        validator.is_valid(&output["record"]["associated_types"]),
        "associated_types validates against TypeRef[]"
    );
    let (_, fence) = run_fixture("contract-fence-not-a-source");
    assert!(fence["record"].get("associated_types").is_none());
    assert!(
        fence["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
}

#[trace("FR-122-AC-4")]
#[test]
fn architecture_layout_fence_maps_and_refuses_bad_size_references() {
    let (case, output) = assert_fixture_record("layout-fence");
    assert_eq!(output["record"]["endianness"], "little");
    assert_eq!(output["record"]["records"], case["record"]["records"]);
    assert_eq!(output["record"]["records"][0]["tag"], "0x21");
    assert_eq!(
        output["record"]["records"][1]["fields"][1]["size"],
        "payload_len"
    );
    for (id, reason) in [
        ("layout-size-unknown-field", "size-unknown-field"),
        ("layout-size-ambiguous", "size-ambiguous"),
    ] {
        assert_error(id, "semantic.invalid-model-cell", reason);
    }
    let (_, prose) = run_fixture("endianness-section-is-prose");
    assert!(prose["record"].get("endianness").is_none());
    assert!(
        prose["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
}

#[trace("FR-122-AC-5")]
#[test]
fn architecture_thresholds_extract_optional_values_and_require_status() {
    let (case, output) = assert_fixture_record("thresholds-row");
    let threshold = &output["record"]["thresholds"][0];
    assert_eq!(threshold["on_exceeded"]["status"], 429);
    assert_eq!(threshold["on_exceeded"]["retry_after"], true);
    assert!(threshold["on_exceeded"].get("behavior").is_none());
    assert_eq!(output["record"]["thresholds"], case["record"]["thresholds"]);
    assert_error(
        "thresholds-status-required",
        "semantic.invalid-model-cell",
        "status-required",
    );
}

#[trace("FR-122-AC-6")]
#[test]
fn architecture_requires_relations_are_typed_and_bullet_sections_are_prose() {
    let (case, output) = run_fixture("typed-relation-vs-requires-section");
    assert_eq!(
        output["requires"], case["typedRelations"]["requires"],
        "{output:#?}"
    );
    assert!(output["record"].get("requires").is_none());
    assert!(
        output["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
}

#[trace("FR-122-AC-7")]
#[test]
fn every_architecture_object_schema_property_has_one_mapping_owner() {
    assert_schema_ownership("architecture", "spec-objects-architecture", 31, 113, false);
}
