// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use serde_json::{Value, json};

use super::{INTAKE_CONTRACT, ScenarioRefusal, write_scenario_evidence};
use crate::error::{EvidenceError, EvidenceErrorCode};
use crate::source::{EvidenceSource, MemoryEvidence};

fn entry(owner: &str) -> Value {
    json!({"owner": owner, "contractVersion": "v1", "revision": format!("{owner}-rev")})
}

fn full() -> Value {
    json!({
        "contract": INTAKE_CONTRACT,
        "integration": "IT-002",
        "scenario": "IT-002-SC-07a",
        "resolution": {"kind": "full", "entries": [entry("qobs"), entry("qsl")]},
        "valueOwners": [{
            "value": "obs-1", "owner": "qobs",
            "contractVersion": "v1", "revision": "qobs-rev"
        }],
        "outcome": {
            "operation": "assessed",
            "claims": [{"kind": "row", "identity": "r1"}, {"kind": "refusal", "identity": "x1"}]
        },
        "budget": [
            {"dimension": "node", "granted": 10, "consumed": 4},
            {"dimension": "elapsed-time", "consumed": 7}
        ],
        "traceBinding": {"criterion": "IT-002-SC-07a"}
    })
}

fn refusal(document: &Value) -> ScenarioRefusal {
    let mut store = MemoryEvidence::default();
    match write_scenario_evidence(&mut store, document).unwrap_err() {
        EvidenceError::ScenarioRefused { kind, .. } => kind,
        other => panic!("expected a scenario refusal, got {other}"),
    }
}

#[test]
#[trace("FR-130-AC-1", "TC-2200")]
fn a_complete_record_is_filed_under_its_integration_and_scenario() {
    let mut store = MemoryEvidence::default();
    let stored = write_scenario_evidence(&mut store, &full()).unwrap();
    assert!(stored.created);
    assert_eq!(stored.path, "scenarios/IT-002/IT-002-SC-07a.json");
    assert!(store.read(&stored.path).unwrap().is_some());
    let again = write_scenario_evidence(&mut store, &full()).unwrap();
    assert!(!again.created, "identical bytes are idempotent");
}

#[test]
#[trace("FR-130-AC-2", "TC-2201")]
fn partial_and_legacy_shapes_are_admitted_without_a_full_manifest() {
    let mut partial = full();
    partial["resolution"] = json!({
        "kind": "partial",
        "reached": [entry("qobs")],
        "causingEntry": {"owner": "qsl", "contractVersion": "v1", "reason": "unresolved"}
    });
    let mut store = MemoryEvidence::default();
    write_scenario_evidence(&mut store, &partial).unwrap();

    let mut legacy = full();
    legacy["integration"] = json!("IT-001");
    legacy["scenario"] = json!("IT-001-SC-06a");
    legacy["resolution"] = json!({"kind": "legacy-compatibility", "selection": entry("qobs")});
    write_scenario_evidence(&mut store, &legacy).unwrap();
}

#[test]
#[trace("FR-130-AC-3", "TC-2202")]
fn a_wrong_or_absent_contract_version_is_refused_before_the_shape_is_read() {
    let mut wrong = full();
    wrong["contract"] = json!("quoin.protocol-scenario-intake/v2");
    wrong["unmodelled"] = json!(true);
    assert_eq!(refusal(&wrong), ScenarioRefusal::WrongContract);

    let mut absent = full();
    absent.as_object_mut().unwrap().remove("contract");
    assert_eq!(refusal(&absent), ScenarioRefusal::MissingComponent);
}

#[test]
#[trace("FR-130-AC-4", "TC-2203")]
fn a_missing_field_an_unknown_field_and_a_blank_component_are_refused() {
    for field in [
        "valueOwners",
        "outcome",
        "budget",
        "traceBinding",
        "resolution",
    ] {
        let mut document = full();
        document.as_object_mut().unwrap().remove(field);
        let mut store = MemoryEvidence::default();
        let error = write_scenario_evidence(&mut store, &document).unwrap_err();
        assert_eq!(error.code(), EvidenceErrorCode::InvalidRecord, "{field}");
    }
    let mut unknown = full();
    unknown["extra"] = json!(1);
    let mut store = MemoryEvidence::default();
    assert_eq!(
        write_scenario_evidence(&mut store, &unknown)
            .unwrap_err()
            .code(),
        EvidenceErrorCode::InvalidRecord
    );

    let mut blank = full();
    blank["traceBinding"]["criterion"] = json!("  ");
    assert_eq!(refusal(&blank), ScenarioRefusal::MissingComponent);
    let mut no_budget = full();
    no_budget["budget"] = json!([]);
    assert_eq!(refusal(&no_budget), ScenarioRefusal::MissingComponent);
}

#[test]
#[trace("FR-130-AC-5", "TC-2204")]
fn a_cross_wired_attribution_is_refused() {
    let mut owner = full();
    owner["valueOwners"][0]["owner"] = json!("elsewhere");
    assert_eq!(refusal(&owner), ScenarioRefusal::CrossWired);

    let mut revision = full();
    revision["valueOwners"][0]["revision"] = json!("other-rev");
    assert_eq!(refusal(&revision), ScenarioRefusal::CrossWired);

    let mut scenario = full();
    scenario["scenario"] = json!("IT-003-SC-01");
    assert_eq!(refusal(&scenario), ScenarioRefusal::CrossWired);

    let mut traversal = full();
    traversal["scenario"] = json!("IT-002-../x");
    assert_eq!(refusal(&traversal), ScenarioRefusal::CrossWired);

    let mut causing = full();
    causing["resolution"] = json!({
        "kind": "partial",
        "reached": [entry("qobs")],
        "causingEntry": {"owner": "qobs", "contractVersion": "v1", "reason": "x"}
    });
    assert_eq!(refusal(&causing), ScenarioRefusal::CrossWired);
}

#[test]
#[trace("FR-130-AC-6", "TC-2205")]
fn duplicate_entries_values_and_dimensions_are_refused() {
    let mut entries = full();
    entries["resolution"]["entries"] = json!([entry("qobs"), entry("qobs")]);
    assert_eq!(refusal(&entries), ScenarioRefusal::DuplicateComponent);

    let mut values = full();
    let first = values["valueOwners"][0].clone();
    values["valueOwners"] = json!([first.clone(), first]);
    assert_eq!(refusal(&values), ScenarioRefusal::DuplicateComponent);

    let mut dimensions = full();
    dimensions["budget"] = json!([
        {"dimension": "node", "consumed": 1},
        {"dimension": "node", "consumed": 2}
    ]);
    assert_eq!(refusal(&dimensions), ScenarioRefusal::DuplicateComponent);
}

#[test]
#[trace("FR-130-AC-7", "TC-2206")]
fn the_budget_is_per_dimension_and_an_absent_grant_is_not_zero() {
    let mut store = MemoryEvidence::default();
    let stored = write_scenario_evidence(&mut store, &full()).unwrap();
    let text = store.read(&stored.path).unwrap().unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["budget"].as_array().unwrap().len(), 2);
    let elapsed = value["budget"]
        .as_array()
        .unwrap()
        .iter()
        .find(|figure| figure["dimension"] == "elapsed-time")
        .unwrap();
    assert!(elapsed.get("granted").is_none(), "absent stays absent");

    let mut scalar = full();
    scalar["budget"] = json!({"total": 5});
    let mut fresh = MemoryEvidence::default();
    assert_eq!(
        write_scenario_evidence(&mut fresh, &scalar)
            .unwrap_err()
            .code(),
        EvidenceErrorCode::InvalidRecord
    );
}

#[test]
#[trace("FR-130-AC-8", "TC-2207")]
fn a_second_different_record_for_one_scenario_is_refused_and_the_first_kept() {
    let mut store = MemoryEvidence::default();
    let stored = write_scenario_evidence(&mut store, &full()).unwrap();
    let before = store.read(&stored.path).unwrap();

    let mut other = full();
    other["outcome"]["operation"] = json!("refused");
    let error = write_scenario_evidence(&mut store, &other).unwrap_err();
    assert_eq!(error.code(), EvidenceErrorCode::RecordIntegrity);
    assert_eq!(store.read(&stored.path).unwrap(), before);
}

#[test]
fn refusal_spellings_are_distinct() {
    let spellings: std::collections::BTreeSet<_> = ScenarioRefusal::ALL
        .iter()
        .map(|kind| kind.as_str())
        .collect();
    assert_eq!(spellings.len(), ScenarioRefusal::ALL.len());
}
