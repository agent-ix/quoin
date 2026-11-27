// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-123 business record-section acceptance tests.

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
use record_sections::*;
use serde_json::json;

#[trace("FR-123-AC-1")]
#[test]
fn business_values_extract_enum_values_and_validate() {
    let (case, output) = assert_fixture_record("values-table");
    assert_eq!(output["record"]["values"], case["record"]["values"]);
    assert!(output["record"]["values"][1].get("doc").is_none());
}

#[trace("FR-123-AC-2")]
#[test]
fn business_values_refuse_lists_duplicates_and_invalid_cells() {
    assert_error(
        "values-list-refused",
        "semantic.feature-not-extractable",
        "values",
    );
    assert_error("values-duplicate", "semantic.duplicate-model-entry", "");
    let (_, duplicate) = run_fixture("values-duplicate");
    assert_eq!(
        duplicate["diagnostics"][0]["line"], 13,
        "the duplicate is diagnosed at the second row"
    );
    assert_error("values-invalid", "semantic.invalid-model-cell", "");
}

#[trace("FR-123-AC-3")]
#[test]
fn business_states_transitions_and_steps_validate_without_copying_states() {
    let (_, states) = assert_fixture_record("states-and-transitions");
    assert_eq!(
        states["record"]["transitions"][0]["guard"]["clauseId"],
        "NotCancelled"
    );
    let (case, steps) = assert_fixture_record("steps-copied-states-stay-in-model");
    assert_eq!(steps["record"]["steps"], case["record"]["steps"]);
    assert!(steps["record"].get("states").is_none());
    assert!(steps["model"]["states"].is_array());
}

#[trace("FR-123-AC-4")]
#[test]
fn business_members_and_domain_vocabulary_map_and_refuse_invalid_member_forms() {
    let (case, members) = assert_fixture_record("members-composite-verbs");
    assert_eq!(members["record"]["members"], case["record"]["members"]);
    assert_eq!(members["record"]["members"][0]["composite"], true);
    assert_eq!(members["record"]["members"][1]["composite"], true);
    assert_error(
        "members-member-verb",
        "semantic.invalid-model-cell",
        "member-verb",
    );
    assert_error(
        "members-declared-in-members",
        "semantic.duplicate-model-entry",
        "declared-in-members",
    );
    let (case, vocabulary) = assert_fixture_record("domain-vocabulary");
    assert_eq!(
        vocabulary["record"]["vocabulary"],
        case["record"]["vocabulary"]
    );
    let (case, output, declaration, documents) =
        run_fixture_full("domain-vocabulary-empty-description");
    let empty_description = output;
    assert!(
        empty_description["record"]["vocabulary"][0]
            .get("doc")
            .is_none()
    );
    assert!(
        !record_is_valid(&case, &declaration, &documents),
        "the omitted required description leaves the record invalid"
    );
}

#[trace("FR-123-AC-5")]
#[test]
fn named_model_sections_are_read_once_by_their_locators() {
    for id in [
        "values-table",
        "states-and-transitions",
        "steps-copied-states-stay-in-model",
        "members-composite-verbs",
    ] {
        let (case, output) = run_fixture(id);
        assert_eq!(output["record"], case["record"], "{id}");
        let sources = record_source_keys(&output);
        let expected = case["recordSources"]
            .as_array()
            .expect("fixture source keys");
        let expected = expected
            .iter()
            .map(|key| key.as_str().expect("source key").to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            sources, expected,
            "{id}: each declared section appears once"
        );
    }
}

#[trace("FR-123-AC-6")]
#[test]
fn business_relationship_keys_are_typed_and_emits_persists_headings_are_prose() {
    let (case, output) = run_fixture("persists-and-emits-sections-are-prose");
    assert_eq!(
        output["persists"], case["typedRelations"]["persists"],
        "{output:#?}"
    );
    assert!(output["record"].get("persists").is_none());
    assert!(output["record"].get("emits").is_none());
    assert!(
        output["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    let (case, event) = assert_fixture_record("event-source-section");
    assert_eq!(event["record"]["source"], case["record"]["source"]);
}

#[trace("FR-123-AC-7")]
#[test]
fn business_unmapped_sections_stay_prose() {
    let (_, nested) = run_fixture("nested-entity-owner-is-prose");
    assert!(nested["record"].get("owner").is_none());
    assert!(
        nested["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    let (_, domain) = run_fixture("domain-vocabulary");
    assert!(domain["record"].get("ubiquitous_language").is_none());
}

#[trace("FR-123-AC-8")]
#[test]
fn every_business_object_schema_property_has_one_mapping_or_declared_unmapped_owner() {
    assert_schema_ownership("business", "spec-objects-business", 24, 76, true);
}

#[trace("FR-123-AC-9")]
#[test]
fn business_empty_values_table_is_available_and_empty() {
    let (_, output) = assert_fixture_record("values-header-only");
    assert_eq!(output["record"]["values"], json!([]));
    assert_eq!(
        output["availability"]["recordKeys"]["values"]["state"],
        "available"
    );
}
