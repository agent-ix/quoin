// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Criterion-level contract checks for the campaign-native conformance adapter.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::SuiteId;
use quoin_evidence::adapters::{PROTOCOL, parse_contract_conformance};
use quoin_evidence::types::{Outcome, RunRecord, STORE_SCHEMA_VERSION};

const REAL: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-real.jsonl");
const REAL_TRACES: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-traces-real.jsonl");
const INVENTORY: &str = include_str!("../../../../tests/fixtures/evidence/README.md");

#[trace("FR-069-AC-1")]
#[test]
fn tc_fr069_real_conformance_sample_keys_each_fixture_by_corpus_operation_and_fixture() {
    let entries = parse_contract_conformance(REAL).unwrap().entries;
    let identities: Vec<&str> = entries.iter().map(|entry| entry.symbol.as_str()).collect();

    assert_eq!(identities.len(), 5);
    assert_eq!(
        identities,
        [
            "contract-v0.1::package::package-constructs",
            "contract-v0.1::package::package-invalid-namespace",
            "contract-v0.1::expression::expression-boolean-literal",
            "contract-v0.1::expression::expression-invalid-wire",
            "contract-v0.1::coverage::coverage-shallow",
        ]
    );
    assert!(entries.iter().all(|entry| entry.outcome == Outcome::Pass));
}

#[trace("FR-069-AC-9")]
#[test]
fn tc_fr069_real_trace_ids_keep_their_order_and_legacy_rows_remain_unbound() {
    let supplied = parse_contract_conformance(REAL_TRACES).unwrap().entries;
    assert_eq!(
        supplied[0].trace_ids.as_deref().unwrap(),
        [
            "FR-011-AC-1",
            "FR-012-AC-2",
            "FR-012-AC-4",
            "FR-012-AC-6",
            "FR-014-AC-3",
            "FR-016-AC-1",
            "FR-018-AC-1",
        ]
    );

    let legacy = format!(
        r#"{{"protocol":"{PROTOCOL}","corpus_id":"legacy","operation":"check","fixture_id":"f","status":"match"}}"#
    );
    let legacy_entry = parse_contract_conformance(&legacy)
        .unwrap()
        .entries
        .remove(0);
    assert!(legacy_entry.trace_ids.is_none());
}

#[trace("FR-069-AC-10")]
#[test]
fn tc_fr069_bad_trace_ids_after_a_valid_row_refuse_the_whole_input() {
    let valid = REAL.lines().next().unwrap();
    for shape in [
        r#""wrong""#,
        "[]",
        "[1]",
        r#"[""]"#,
        r#"["  "]"#,
        r#"["FR-1-AC-1","FR-1-AC-1"]"#,
    ] {
        let malformed = format!(
            r#"{{"protocol":"{PROTOCOL}","corpus_id":"c","operation":"o","fixture_id":"f","status":"match","trace_ids":{shape}}}"#
        );
        let input = format!("{valid}\n{malformed}");
        let error = parse_contract_conformance(&input).unwrap_err().to_string();
        assert!(error.contains("line 2 trace_ids"), "{shape}: {error}");
        // The adapter is a pure parser: returning Err means no partial result
        // exists for the caller to write to the evidence store.
    }
}

#[trace("FR-069-CON-2")]
#[test]
fn tc_fr069_adapter_output_uses_the_existing_run_record_schema() {
    let entry = parse_contract_conformance(REAL_TRACES)
        .unwrap()
        .entries
        .remove(0);
    let record = RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("contract-conformance-fixture"),
        tool: "filament-contract-conformance".to_owned(),
        evidence_kind: Some("test".to_owned()),
        timestamp: "2026-11-26T00:00:00Z".to_owned(),
        entries: vec![entry],
    };
    let serialized = serde_json::to_value(record).unwrap();
    let record_object = serialized.as_object().unwrap();
    assert_eq!(
        record_object.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "entries",
            "evidenceKind",
            "schemaVersion",
            "suite",
            "timestamp",
            "tool"
        ]
    );
    let entry_object = serialized["entries"][0].as_object().unwrap();
    assert!(entry_object.contains_key("traceIds"));
    assert!(entry_object.contains_key("symbol"));
    assert!(entry_object.contains_key("outcome"));
    assert_eq!(entry_object.len(), 3);
}

#[trace("FR-069-AC-7", "FR-069-CON-4")]
#[test]
fn tc_fr069_inventory_names_the_real_producer_and_captured_samples() {
    assert!(INVENTORY.contains(
        "filament-contract-conformance run --corpus corpus/contract-v0.1 --schemas schemas"
    ));
    assert!(INVENTORY.contains("contract-conformance-real.jsonl"));
    assert!(INVENTORY.contains("contract-conformance-traces-real.jsonl"));
    assert!(INVENTORY.contains("agent-ix/filament-contract-ir`"));
    assert!(INVENTORY.contains("`ce1970fb`"));
}
