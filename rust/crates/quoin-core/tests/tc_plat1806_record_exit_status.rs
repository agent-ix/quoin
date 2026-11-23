// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `evidence.record` exit status for unmatched ids and unbound runs, and the
//! pytest symbol mapping, through the production runtime.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;

use ix_cli_kit::exit::Outcome;
use ix_trace_rs::trace;
use quoin_core::protocol::Response;
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

fn record(adapter: Option<&str>, tool: &str, results: &str) -> (tempfile::TempDir, Response) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("spec/evidence")).unwrap();
    let mut request = json!({
        "repo": dir.path(),
        "suite": "SUITE-1",
        "tool": tool,
        "timestamp": "2026-01-01T00:00:00Z",
        "results": results,
        "obligations": [{"id": "FR-1-AC-1", "statement": "s"}],
    });
    if let Some(adapter) = adapter {
        request["adapter"] = json!(adapter);
    }
    let response = dispatch("evidence.record", &request, &RuntimeSettings::default()).unwrap();
    (dir, response)
}

fn warning(response: &Response) -> Option<&str> {
    response.payload["warning"].as_str()
}

fn bindings_exist(dir: &tempfile::TempDir) -> bool {
    dir.path().join("spec/evidence/bindings.json").exists()
}

fn run_written(dir: &tempfile::TempDir) -> bool {
    dir.path().join("spec/evidence/runs/SUITE-1").is_dir()
}

/// An unmatched id exits 1 with one byte-ordered warning line; the run record
/// and the matched id's binding are published first.
#[trace("FR-030-AC-32")]
#[test]
fn tc_1806_an_unmatched_trace_id_exits_1_after_publishing() {
    let results = r#"{"entries":[
        {"symbol":"a","outcome":"pass","traceIds":["FR-1-AC-1","ZZ-1","AA-1"]}]}"#;
    let (dir, response) = record(None, "custom", results);
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(
        warning(&response),
        Some("warning: unmatched trace ids: AA-1, ZZ-1")
    );
    assert_eq!(response.payload["unmatched"], json!(["AA-1", "ZZ-1"]));
    assert_eq!(response.payload["bound"], json!(["FR-1-AC-1"]));
    assert!(run_written(&dir));
    assert!(bindings_exist(&dir));
}

/// An id carried only by a failing entry is still unmatched.
#[trace("FR-030-AC-32")]
#[test]
fn tc_1806_an_unmatched_id_on_a_failing_entry_exits_1() {
    let results = r#"{"entries":[{"symbol":"a","outcome":"fail","traceIds":["ZZ-1"]}]}"#;
    let (_dir, response) = record(None, "custom", results);
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(
        warning(&response),
        Some("warning: unmatched trace ids: ZZ-1")
    );
}

/// A test run with entries and no trace id exits 1 with the one AC-33 line,
/// after the run record is written.
#[trace("FR-030-AC-33")]
#[test]
fn tc_1806_a_run_with_no_trace_id_exits_1_naming_the_entry_count() {
    let results =
        r#"{"entries":[{"symbol":"a","outcome":"pass"},{"symbol":"b","outcome":"pass"}]}"#;
    let (dir, response) = record(None, "custom", results);
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(
        warning(&response),
        Some("warning: no entry carries a trace id: 2 entries recorded, none can bind")
    );
    assert!(run_written(&dir));
}

/// An empty `traceIds` array carries no id, so the run is still unbound.
#[trace("FR-030-AC-33")]
#[test]
fn tc_1806_an_empty_trace_id_array_does_not_count_as_carrying_one() {
    let results = r#"{"entries":[{"symbol":"a","outcome":"pass","traceIds":[]}]}"#;
    let (_dir, response) = record(None, "custom", results);
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(
        warning(&response),
        Some("warning: no entry carries a trace id: 1 entries recorded, none can bind")
    );
}

/// Contract-conformance rows from a legacy producer carry no `trace_ids`.
#[trace("FR-030-AC-33")]
#[test]
fn tc_1806_contract_conformance_rows_without_trace_ids_exit_1() {
    let row = r#"{"protocol":"filament.contract.conformance-jsonl/v1","corpus_id":"c","operation":"op","fixture_id":"f","status":"match"}"#;
    let (dir, response) = record(
        Some("contract-conformance"),
        "filament-contract-conformance 1",
        row,
    );
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(
        warning(&response),
        Some("warning: no entry carries a trace id: 1 entries recorded, none can bind")
    );
    assert!(run_written(&dir));
}

/// An agent-eval scenario id is its own trace id: an id no obligation states
/// is unmatched (AC-32), and one an obligation states exits 0.
#[trace("FR-030-AC-32", "FR-030-AC-33")]
#[test]
fn tc_1806_agent_eval_scenario_ids_are_trace_ids() {
    let (_dir, response) = record(
        Some("agent-eval"),
        "cli-agent-evals 1",
        r#"{"results":[{"id":"TC-EV-1","ok":true}]}"#,
    );
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(
        warning(&response),
        Some("warning: unmatched trace ids: TC-EV-1")
    );

    let (_dir, response) = record(
        Some("agent-eval"),
        "cli-agent-evals 1",
        r#"{"results":[{"id":"FR-1-AC-1","ok":true}]}"#,
    );
    assert_eq!(response.outcome, Outcome::Ok);
    assert_eq!(response.payload["bound"], json!(["FR-1-AC-1"]));
}

/// Failing, erroring and skipped entries with stated ids bind nothing and are
/// not an error; a run of zero entries is not either.
#[trace("FR-030-AC-34")]
#[test]
fn tc_1806_a_run_whose_stated_ids_all_failed_exits_0() {
    let results = r#"{"entries":[
        {"symbol":"a","outcome":"fail","traceIds":["FR-1-AC-1"]},
        {"symbol":"b","outcome":"error","traceIds":["FR-1-AC-1"]},
        {"symbol":"c","outcome":"skip","traceIds":["FR-1-AC-1"]}]}"#;
    let (_dir, response) = record(None, "custom", results);
    assert_eq!(response.outcome, Outcome::Ok);
    assert_eq!(response.payload["bound"], json!([]));
    assert_eq!(warning(&response), None);

    let (_dir, response) = record(None, "custom", r#"{"entries":[]}"#);
    assert_eq!(response.outcome, Outcome::Ok);
    assert_eq!(warning(&response), None);
}

/// Recordings that bind nothing by design exit 0 when no id is unmatched.
#[trace("FR-030-AC-35")]
#[test]
fn tc_1806_cargo_mutants_and_an_undischarging_sbom_exit_0() {
    let sbom = r#"{"bomFormat":"CycloneDX","components":[{"name":"a"}]}"#;
    let (_dir, response) = record(Some("sbom"), "syft 1.0.0", sbom);
    assert_eq!(response.outcome, Outcome::Ok, "{:?}", response.payload);

    let mutants = r#"{"outcomes":[{"scenario":{"Mutant":{"name":"x","file":"src/a.rs","function":{"function_name":"f"}}},"summary":"CaughtMutant"},{"scenario":"Baseline","summary":"Success"}]}"#;
    let (_dir, response) = record(Some("cargo-mutants"), "cargo-mutants 1", mutants);
    assert_eq!(response.outcome, Outcome::Ok, "{:?}", response.payload);
}

fn junit(classname: &str, name: &str, trace: Option<&str>) -> String {
    let property = trace.map_or_else(String::new, |ids| {
        format!(r#"<properties><property name="trace" value="{ids}"/></properties>"#)
    });
    format!(
        r#"<testsuite><testcase classname="{classname}" name="{name}">{property}</testcase></testsuite>"#
    )
}

fn stored_symbols(dir: &tempfile::TempDir) -> Vec<String> {
    let runs = dir.path().join("spec/evidence/runs/SUITE-1");
    let file = fs::read_dir(runs).unwrap().next().unwrap().unwrap().path();
    let run: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    run["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["symbol"].as_str().unwrap().to_owned())
        .collect()
}

/// A case recorded under a pytest tool is stored under the Python extractor's
/// symbol; under another tool the `::` rule applies.
#[trace("FR-033-AC-16")]
#[test]
fn tc_1806_a_pytest_case_is_stored_under_the_python_symbol() {
    let cases = [
        ("tests.test_bolus", "test_basic", "test_basic"),
        ("tests.test_bolus.TestDose", "test_a[1]", "TestDose.test_a"),
        ("tests.TestA.Inner", "test_b", "TestA.Inner.test_b"),
        ("t.TestA", "test_a[x[1]]", "TestA.test_a"),
        ("test_basic", "test_basic", "test_basic"),
        ("", "test_x", "test_x"),
    ];
    for (classname, name, symbol) in cases {
        let (dir, _) = record(None, "Pytest 8.3.2", &junit(classname, name, None));
        assert_eq!(stored_symbols(&dir), [symbol], "{classname} {name}");
    }
    let (dir, _) = record(
        None,
        "junit",
        &junit("tests.test_bolus.TestDose", "test_a", None),
    );
    assert_eq!(
        stored_symbols(&dir),
        ["tests::test_bolus::TestDose::test_a"]
    );
}

/// A pytest case with no `trace` property has no ids; a run of such cases is
/// an AC-33 error, and a case whose id no obligation states is unmatched.
#[trace("FR-033-AC-17")]
#[test]
fn tc_1806_a_pytest_case_without_a_trace_property_records_no_ids() {
    let (dir, response) = record(None, "pytest 8", &junit("t.TestA", "test_a", None));
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(
        warning(&response),
        Some("warning: no entry carries a trace id: 1 entries recorded, none can bind")
    );
    let runs = dir.path().join("spec/evidence/runs/SUITE-1");
    let file = fs::read_dir(runs).unwrap().next().unwrap().unwrap().path();
    assert!(!fs::read_to_string(file).unwrap().contains("traceIds"));

    let (_dir, response) = record(
        None,
        "pytest 8",
        &junit("t.TestA", "test_a", Some("NOPE-1")),
    );
    assert_eq!(response.payload["unmatched"], json!(["NOPE-1"]));
    assert_eq!(
        warning(&response),
        Some("warning: unmatched trace ids: NOPE-1")
    );
}
