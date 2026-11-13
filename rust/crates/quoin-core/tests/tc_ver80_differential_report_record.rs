// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `evidence.record` over `quire.differential-report/v2` reports (FR-151,
//! TC-001), through the production runtime: what the store holds and what the
//! command prints for each disposition, and that a refusal writes no run.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::Path;

use ix_trace_rs::trace;
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

const SATISFIED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json"
);
const REFUTED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-refuted.json"
);
const DISAGREEMENT: &str =
    include_str!("../../../../tests/fixtures/evidence/differential-report-v2/disagreement.json");
const TIMEOUT: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/inconclusive-timeout.json"
);
const UNSUPPORTED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/unavailable-unsupported-platform.json"
);

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("spec/evidence")).unwrap();
    dir
}

fn request(dir: &Path, tool: &str, results: &str) -> Value {
    json!({
        "repo": dir,
        "suite": "SUITE-SMT",
        "tool": tool,
        "timestamp": "2026-01-01T00:00:00Z",
        "results": results,
    })
}

fn record(dir: &Path, tool: &str, results: &str) -> Value {
    dispatch(
        "evidence.record",
        &request(dir, tool, results),
        &RuntimeSettings::default(),
    )
    .unwrap()
    .payload
}

/// The one run the store holds.
fn the_run(dir: &Path) -> Value {
    let runs = dir.join("spec/evidence/runs/SUITE-SMT");
    let mut files = fs::read_dir(runs).unwrap();
    let file = files.next().unwrap().unwrap().path();
    assert!(files.next().is_none(), "more than one run was written");
    serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap()
}

#[trace(
    "FR-151-AC-1",
    "FR-151-AC-2",
    "FR-151-AC-4",
    "FR-151-AC-5",
    "FR-069-AC-2"
)]
#[test]
fn tc_001_a_decided_report_records_one_run_with_one_entry() {
    for (report, outcome) in [
        (SATISFIED, "pass"),
        (REFUTED, "fail"),
        (TIMEOUT, "error"),
        (UNSUPPORTED, "error"),
    ] {
        let dir = repo();
        let payload = record(dir.path(), "quire-analyze 0.1.0", report);
        assert_eq!(payload["kind"], "run", "{payload}");
        assert!(payload.get("unrepresented").is_none(), "{payload}");
        let run = the_run(dir.path());
        let entries = run["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1, "{run}");
        assert_eq!(entries[0]["outcome"], outcome, "{run}");
        assert!(entries[0].get("traceIds").is_none(), "{run}");
    }
}

#[trace("FR-151-AC-3", "FR-069-AC-6")]
#[test]
fn tc_001_a_disagreement_writes_a_run_with_no_entries_and_prints_the_named_result() {
    let dir = repo();
    let payload = record(dir.path(), "differential-report", DISAGREEMENT);
    assert_eq!(payload["kind"], "run", "{payload}");
    assert_eq!(payload["bound"], json!([]));
    let unrepresented = payload["unrepresented"].as_array().unwrap();
    assert_eq!(unrepresented.len(), 1, "{payload}");
    assert_eq!(unrepresented[0]["state"], "disagreement");
    assert!(
        unrepresented[0]["symbol"]
            .as_str()
            .unwrap()
            .starts_with("consistency::"),
        "{payload}"
    );
    assert_eq!(the_run(dir.path())["entries"], json!([]));
}

#[trace(
    "FR-151-AC-6",
    "FR-151-AC-7",
    "FR-151-AC-8",
    "FR-151-AC-9",
    "FR-151-AC-11"
)]
#[test]
fn tc_001_a_refused_report_writes_no_run() {
    for results in [
        "",
        "[]",
        &SATISFIED.replace("differential-report/v2", "differential-report/v1"),
        &serde_json::to_string_pretty(&serde_json::from_str::<Value>(SATISFIED).unwrap()).unwrap(),
    ] {
        let dir = repo();
        let response = dispatch(
            "evidence.record",
            &request(dir.path(), "quire-analyze 0.1.0", results),
            &RuntimeSettings::default(),
        );
        assert!(response.is_err(), "{results}");
        assert!(!dir.path().join("spec/evidence/runs").exists());
    }
}

#[trace("FR-151-AC-12")]
#[test]
fn tc_001_a_retired_tool_name_does_not_select_the_adapter() {
    let dir = repo();
    let response = dispatch(
        "evidence.record",
        &request(dir.path(), "tl-mltl 0.1.0", SATISFIED),
        &RuntimeSettings::default(),
    );
    assert!(response.is_err());
    assert!(!dir.path().join("spec/evidence/runs").exists());
}

const INCONCLUSIVE: &str = TIMEOUT;
const UNAVAILABLE: &str = UNSUPPORTED;

fn obligation(id: &str) -> Value {
    json!({"id": id, "statement": "s"})
}

/// Record with stated obligations over a store whose obligations are `known`.
fn discharge(
    dir: &Path,
    tool: &str,
    results: &str,
    stated: &[&str],
    known: &[&str],
) -> Result<Value, quoin_core::error::CoreError> {
    let mut body = request(dir, tool, results);
    body["discharges"] = json!(stated);
    body["obligations"] = known.iter().map(|id| obligation(id)).collect();
    dispatch("evidence.record", &body, &RuntimeSettings::default()).map(|r| r.payload)
}

/// The bindings the store holds, empty when none were written.
fn bindings(dir: &Path) -> Vec<Value> {
    let path = dir.join("spec/evidence/bindings.json");
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let document: Value = serde_json::from_str(&text).unwrap();
    document["bindings"].as_array().cloned().unwrap_or_default()
}

fn symbol_of(run: &Value) -> String {
    run["entries"][0]["symbol"].as_str().unwrap().to_owned()
}

/// `<analysisKind>::<requestDigest>`, built from the report's own fields.
fn expected_symbol(report: &str) -> String {
    let report: Value = serde_json::from_str(report).unwrap();
    format!(
        "{}::{}",
        report["analysisKind"].as_str().unwrap(),
        report["requestDigest"].as_str().unwrap()
    )
}

#[trace("FR-152-AC-1")]
#[test]
fn tc_002_a_stated_obligation_binds_a_satisfied_report() {
    let dir = repo();
    let payload = discharge(
        dir.path(),
        "quire-analyze 0.1.0",
        SATISFIED,
        &["FR-001-AC-1"],
        &["FR-001-AC-1"],
    )
    .unwrap();
    assert_eq!(payload["bound"], json!(["FR-001-AC-1"]), "{payload}");
    let run = the_run(dir.path());
    assert_eq!(run["entries"].as_array().unwrap().len(), 1, "{run}");
    assert_eq!(run["entries"][0]["traceIds"], json!(["FR-001-AC-1"]));
    let held = bindings(dir.path());
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0]["obligation"], "FR-001-AC-1");
    let expected = expected_symbol(SATISFIED);
    assert_eq!(symbol_of(&run), expected);
    assert_eq!(held[0]["symbols"], json!([expected]));
}

#[trace("FR-152-AC-2", "FR-152-AC-3")]
#[test]
fn tc_002_no_stated_obligation_leaves_the_entry_unbound() {
    // AC-3: ids unrelated to the symbol, analysisKind and requestDigest.
    // NFR-033: an id equal to the analysisKind still binds nothing.
    for ids in [
        ["FR-001-AC-1", "FR-002-AC-1"],
        ["FR-001-AC-1", "consistency"],
    ] {
        let dir = repo();
        let mut body = request(dir.path(), "quire-analyze 0.1.0", SATISFIED);
        body["obligations"] = ids.iter().map(|id| obligation(id)).collect();
        let payload = dispatch("evidence.record", &body, &RuntimeSettings::default())
            .unwrap()
            .payload;
        assert_eq!(payload["bound"], json!([]), "{payload}");
        assert!(the_run(dir.path())["entries"][0].get("traceIds").is_none());
        assert!(bindings(dir.path()).is_empty());
    }
}

#[trace("FR-152-AC-4")]
#[test]
fn tc_002_only_a_passing_entry_binds() {
    for report in [REFUTED, INCONCLUSIVE, UNAVAILABLE] {
        let dir = repo();
        let payload = discharge(
            dir.path(),
            "quire-analyze 0.1.0",
            report,
            &["FR-001-AC-1"],
            &["FR-001-AC-1"],
        )
        .unwrap();
        assert_eq!(payload["bound"], json!([]), "{payload}");
        let run = the_run(dir.path());
        assert_eq!(run["entries"][0]["traceIds"], json!(["FR-001-AC-1"]));
        assert!(bindings(dir.path()).is_empty());
    }
}

#[trace("FR-152-AC-5")]
#[test]
fn tc_002_a_disagreement_stores_no_entry_and_binds_nothing() {
    let dir = repo();
    let payload = discharge(
        dir.path(),
        "differential-report",
        DISAGREEMENT,
        &["FR-001-AC-1"],
        &["FR-001-AC-1"],
    )
    .unwrap();
    assert_eq!(payload["bound"], json!([]), "{payload}");
    assert_eq!(payload["unrepresented"][0]["state"], "disagreement");
    assert_eq!(the_run(dir.path())["entries"], json!([]));
    assert!(bindings(dir.path()).is_empty());
}

#[trace("FR-152-AC-6")]
#[test]
fn tc_002_stated_ids_are_sorted_deduplicated_and_unknown_ids_are_unmatched() {
    let dir = repo();
    let payload = discharge(
        dir.path(),
        "quire-analyze 0.1.0",
        SATISFIED,
        &["FR-002-AC-1", " FR-001-AC-1", "FR-002-AC-1", "NO-SUCH-1"],
        &["FR-001-AC-1", "FR-002-AC-1"],
    )
    .unwrap();
    assert_eq!(payload["bound"], json!(["FR-001-AC-1", "FR-002-AC-1"]));
    assert_eq!(payload["unmatched"], json!(["NO-SUCH-1"]));
    let run = the_run(dir.path());
    assert_eq!(
        run["entries"][0]["traceIds"],
        json!(["FR-001-AC-1", "FR-002-AC-1"])
    );
    let held = bindings(dir.path());
    assert_eq!(held.len(), 2, "{held:?}");
    for binding in &held {
        assert_eq!(binding["symbols"], json!([symbol_of(&run)]));
    }
}

#[trace("FR-152-AC-7")]
#[test]
fn tc_002_a_refused_report_with_stated_ids_writes_nothing() {
    for results in [
        "",
        &SATISFIED.replace("differential-report/v2", "differential-report/v1"),
        &serde_json::to_string_pretty(&serde_json::from_str::<Value>(SATISFIED).unwrap()).unwrap(),
    ] {
        let dir = repo();
        let response = discharge(
            dir.path(),
            "quire-analyze 0.1.0",
            results,
            &["FR-001-AC-1"],
            &["FR-001-AC-1"],
        );
        assert!(response.is_err(), "{results}");
        assert!(!dir.path().join("spec/evidence/runs").exists());
        assert!(bindings(dir.path()).is_empty());
    }
}

#[trace("FR-152-AC-8")]
#[test]
fn tc_002_discharges_is_still_refused_on_the_junit_adapter() {
    let dir = repo();
    let mut body = request(
        dir.path(),
        "vitest 1.0.0",
        r#"<testsuite name="unit"><testcase name="t"/></testsuite>"#,
    );
    body["adapter"] = json!("junit");
    body["discharges"] = json!(["FR-001-AC-1"]);
    body["obligations"] = json!([obligation("FR-001-AC-1")]);
    let error = dispatch("evidence.record", &body, &RuntimeSettings::default())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("--discharges") && error.contains("junit"),
        "{error}"
    );
    assert!(!dir.path().join("spec/evidence/runs").exists());
}
