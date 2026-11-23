// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Read execution provenance for the Test Result export (FR-185-AC-11/12).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "the fixture is valid and assertions should identify the failed step"
)]

use base64::Engine as _;
use ix_trace_rs::trace;
use quoin_evidence::export::provenance::{PAYLOAD_TYPE, ProvenanceError, read_provenance};
use serde_json::{Value, json};

const SUITES: &[u8] = b"# suites\n";

fn envelope(payload: &Value) -> Vec<u8> {
    json!({
        "payloadType": PAYLOAD_TYPE,
        "payload": base64::engine::general_purpose::STANDARD.encode(payload.to_string()),
        "signatures": [{ "sig": "AAAA" }]
    })
    .to_string()
    .into_bytes()
}

fn payload() -> Value {
    json!({
        "version": 2,
        "sourceRevision": "0123456789abcdef0123456789abcdef01234567",
        "suiteDeclarationsSha256": quoin_store::attachment::sha256_hex(SUITES),
        "testMatrix": [
            { "document": "spec/functional/FR-1.md", "id": "FR-1", "criteria": [
                { "id": "FR-1-AC-1", "status": "tagged" },
                { "id": "FR-1-AC-2", "status": "untagged" },
                { "id": "FR-1-AC-3", "status": "retired" }
            ] }
        ],
        "runs": [
            { "suite": "SUITE-001", "recordPath": "runs/SUITE-001/a.json", "sha256": "a".repeat(64) },
            { "suite": "SUITE-002", "recordPath": "runs/SUITE-002/a.json", "sha256": null }
        ]
    })
}

#[trace("FR-185-AC-12")]
#[test]
fn tc_1792_010_valid_provenance_yields_population_and_runs() {
    let provenance = read_provenance(&envelope(&payload()), SUITES).expect("valid");
    assert_eq!(
        provenance
            .required
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["FR-1-AC-1", "FR-1-AC-2"]
    );
    assert_eq!(provenance.runs.len(), 2);
    assert!(provenance.runs[1].sha256.is_none());
}

#[trace("FR-185-AC-11", "FR-185-AC-12")]
#[test]
fn tc_1792_011_each_defect_refuses() {
    let mutate = |edit: &dyn Fn(&mut Value)| {
        let mut changed = payload();
        edit(&mut changed);
        read_provenance(&envelope(&changed), SUITES)
    };
    assert!(matches!(
        read_provenance(&envelope(&payload()), b"changed"),
        Err(ProvenanceError::SuiteDeclarationsMismatch)
    ));
    assert!(matches!(
        mutate(&|p| p["testMatrix"] = json!([])),
        Err(ProvenanceError::EmptyMatrix)
    ));
    assert!(mutate(&|p| p["version"] = json!(1)).is_err());
    assert!(mutate(&|p| p["extra"] = json!(1)).is_err());
    assert!(mutate(&|p| p["testMatrix"] = json!([{ "id": "x" }])).is_err());
    assert!(mutate(&|p| p["testMatrix"] = json!([{ "criteria": [{ "id": "x" }] }])).is_err());
    assert!(matches!(
        mutate(&|p| p["testMatrix"] = json!([{ "criteria": [{ "id": "x", "status": "retired" }] }])),
        Err(ProvenanceError::EmptyMatrix)
    ));
    assert!(mutate(&|p| p["sourceRevision"] = json!("main")).is_err());
    assert!(
        mutate(&|p| p["sourceRevision"] = json!("0123456789ABCDEF0123456789ABCDEF01234567"))
            .is_err()
    );
    assert!(mutate(&|p| p["runs"][0]["recordPath"] = json!("../escape.json")).is_err());
    assert!(mutate(&|p| p["runs"][0]["recordPath"] = json!("/abs.json")).is_err());
    assert!(mutate(&|p| p["runs"][0]["sha256"] = json!("ABC")).is_err());
    assert!(mutate(&|p| p["runs"][1]["suite"] = json!("SUITE-001")).is_err());

    assert!(read_provenance(b"not json", SUITES).is_err());
    // Each envelope defect alone, over an otherwise valid payload.
    let valid: Value = serde_json::from_slice(&envelope(&payload())).unwrap();
    let mut wrong_type = valid.clone();
    wrong_type["payloadType"] = json!("application/json");
    assert!(read_provenance(wrong_type.to_string().as_bytes(), SUITES).is_err());
    let mut not_base64 = valid.clone();
    not_base64["payload"] = json!("%%%");
    assert!(read_provenance(not_base64.to_string().as_bytes(), SUITES).is_err());
    let mut no_signatures = valid;
    no_signatures.as_object_mut().unwrap().remove("signatures");
    assert!(read_provenance(no_signatures.to_string().as_bytes(), SUITES).is_err());
}
