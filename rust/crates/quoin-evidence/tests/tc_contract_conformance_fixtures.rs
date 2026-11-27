// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The contract-conformance adapter read against rows captured from the real
//! `filament-contract-conformance` runner (`tests/fixtures/evidence/`).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::Outcome;
use quoin_evidence::adapters::{PROTOCOL, parse_contract_conformance};

const REAL: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-real.jsonl");
const TRACES: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-traces-real.jsonl");
const CRITERIA: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-criteria-real.jsonl");

/// Every captured row declares the adapter's protocol, so the fixtures prove
/// `PROTOCOL` against the producer rather than against itself.
#[test]
fn the_real_rows_declare_the_adapters_protocol() {
    for raw in [REAL, TRACES, CRITERIA] {
        for line in raw.lines() {
            assert!(line.contains(&format!(r#""protocol":"{PROTOCOL}""#)));
        }
    }
}

#[test]
#[trace("FR-069-AC-1")]
fn the_real_corpus_rows_parse_one_entry_per_fixture_and_operation() {
    let entries = parse_contract_conformance(REAL).unwrap().entries;
    let symbols: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    assert_eq!(
        symbols,
        [
            "contract-v0.1::package::package-constructs",
            "contract-v0.1::package::package-invalid-namespace",
            "contract-v0.1::expression::expression-boolean-literal",
            "contract-v0.1::expression::expression-invalid-wire",
            "contract-v0.1::coverage::coverage-shallow",
        ]
    );
    assert!(entries.iter().all(|e| e.outcome == Outcome::Pass));
}

#[test]
#[trace("FR-069-AC-9")]
fn the_real_trace_rows_keep_their_ids_in_order() {
    let traces = parse_contract_conformance(TRACES).unwrap().entries;
    assert_eq!(
        traces[0].trace_ids.as_deref().unwrap(),
        [
            "FR-011-AC-1",
            "FR-012-AC-2",
            "FR-012-AC-4",
            "FR-012-AC-6",
            "FR-014-AC-3",
            "FR-016-AC-1",
            "FR-018-AC-1"
        ]
    );
    let criteria = parse_contract_conformance(CRITERIA).unwrap().entries;
    assert_eq!(
        criteria[0].trace_ids.as_deref().unwrap(),
        ["FR-011-AC-3", "FR-018-AC-1"]
    );
    let legacy = parse_contract_conformance(
        r#"{"protocol":"filament.contract.conformance-jsonl/v1","corpus_id":"contract-v0.1","operation":"package","fixture_id":"legacy-row","status":"match"}"#,
    )
    .unwrap();
    assert_eq!(legacy.entries.len(), 1);
    assert!(legacy.entries[0].trace_ids.is_none());
}

#[test]
#[trace("FR-069-AC-10")]
fn malformed_trace_ids_refuse_the_whole_stream_at_their_line() {
    let valid = r#"{"protocol":"filament.contract.conformance-jsonl/v1","corpus_id":"c","operation":"o","fixture_id":"good","status":"match"}"#;
    for malformed in [
        "\"not-an-array\"",
        "[]",
        "[1]",
        "[\"\"]",
        "[\"  \"]",
        "[\"same\",\"same\"]",
    ] {
        let invalid = format!(
            "{{\"protocol\":\"filament.contract.conformance-jsonl/v1\",\"corpus_id\":\"c\",\"operation\":\"o\",\"fixture_id\":\"bad\",\"status\":\"match\",\"trace_ids\":{malformed}}}"
        );
        let error = parse_contract_conformance(&format!("{valid}\n{invalid}"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("line 2"), "{malformed}: {error}");
        assert!(error.contains("trace_ids"), "{malformed}: {error}");
    }
}

#[test]
#[trace("FR-069-AC-7")]
fn inventory_names_each_native_format_and_its_real_producer_verdict() {
    let inventory = include_str!("../../../../docs/campaign-native-result-inventory.md");
    for row in [
        "| JUnit-compatible test reports",
        "| SARIF",
        "| cargo-mutants",
        "| Audit-script / normalized entries",
        "| Advisory scans",
        "| Measurements",
        "| Contract conformance JSONL",
        "| Differential reports",
        "| tl-mltl live differential outputs",
        "| Corpus reports",
        "| Kani outputs",
        "| Solver analysis reports",
        "| Counterexamples",
        "| Domain-specific proof reports",
    ] {
        let line = inventory
            .lines()
            .find(|line| line.starts_with(row))
            .unwrap_or_else(|| panic!("inventory lacks scope item {row}"));
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        assert!(cells.len() >= 5, "incomplete inventory row: {line}");
        assert!(!cells[2].is_empty(), "row has no named producer: {line}");
        assert!(!cells[3].is_empty(), "row has no recorded verdict: {line}");
    }
    assert!(inventory.contains(
        "filament-contract-conformance run --corpus corpus/contract-v0.1 --schemas schemas"
    ));
    assert!(inventory.contains("tests/fixtures/evidence/contract-conformance-real.jsonl"));
    assert!(inventory.contains("quire-analyze run-package"));
}

#[test]
#[trace("FR-069-CON-2")]
fn adapter_entries_keep_the_existing_run_entry_storage_shape() {
    let entry = parse_contract_conformance(TRACES)
        .unwrap()
        .entries
        .remove(0);
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["symbol"], "contract-v0.1::package::package-constructs");
    assert_eq!(json["outcome"], "pass");
    assert!(json["traceIds"].is_array());
    assert_eq!(
        json.as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["outcome", "symbol", "traceIds"]
    );
}

#[test]
#[trace("FR-069-CON-4")]
fn added_adapter_inventory_identifies_its_producer_and_real_sample() {
    let inventory = include_str!("../../../../docs/campaign-native-result-inventory.md");
    let real_differential_sample = include_str!(
        "../../../../tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json"
    );
    assert!(!real_differential_sample.is_empty());
    assert!(inventory.contains("### `contract-conformance`"));
    assert!(inventory.contains(
        "`filament-contract-conformance run --corpus corpus/contract-v0.1 --schemas schemas`"
    ));
    assert!(inventory.contains("`tests/fixtures/evidence/contract-conformance-real.jsonl`"));
    assert!(inventory.contains("### `differential-report`"));
    assert!(inventory.contains("`quire-analyze run-package`"));
    assert!(
        inventory
            .contains("`tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json`")
    );
}
