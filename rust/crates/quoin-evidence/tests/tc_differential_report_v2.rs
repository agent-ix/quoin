// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! TC-001: the `differential-report` adapter transcribes every
//! `quire.differential-report/v2` disposition and refuses malformed reports
//! (FR-151, NFR-032).
//!
//! The accepted inputs are reports `quire-analyze run-package` wrote (see
//! `tests/fixtures/evidence/README.md`). Each refusal input is one of those
//! reports with one stated change.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_analyze::MAX_REPORT_BYTES;
use quoin_evidence::adapters::{
    ADAPTERS, AdapterResult, AdapterSelection, parse_differential_report, select_adapter,
};
use quoin_evidence::{Outcome, SymbolId};
use serde_json::{Value, json};

const SATISFIED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json"
);
const REFUTED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-refuted.json"
);
const DISAGREEMENT: &str =
    include_str!("../../../../tests/fixtures/evidence/differential-report-v2/disagreement.json");
const UNKNOWN: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/inconclusive-unknown.json"
);
const TIMEOUT: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/inconclusive-timeout.json"
);
const MISSING: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/unavailable-missing-executable.json"
);
const UNSUPPORTED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/unavailable-unsupported-platform.json"
);

const SATISFIED_DIGEST: &str = "3dbffb15f695159ab9331b9708144a86b3845beb04bcc31767445b914065b1ae";
const REFUTED_DIGEST: &str = "dc9233cb2a167e99c80d49b42e09f2de424d0a22989b9f2487acf1262b265da1";

/// Canonical compact JSON with every object's keys sorted, optionally with the
/// top level in reverse order. The report format is canonical, so a mutated
/// document is re-emitted this way and stays canonical.
fn emit(value: &Value, reverse_top: bool) -> String {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<&String, &Value> = map.iter().collect();
            let mut members: Vec<String> = sorted
                .into_iter()
                .map(|(key, member)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        emit(member, false)
                    )
                })
                .collect();
            if reverse_top {
                members.reverse();
            }
            format!("{{{}}}", members.join(","))
        }
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(|item| emit(item, false)).collect();
            format!("[{}]", items.join(","))
        }
        scalar => serde_json::to_string(scalar).unwrap(),
    }
}

fn derive(capture: &str, change: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(capture).unwrap();
    change(&mut value);
    emit(&value, false)
}

fn accepted(capture: &str) -> AdapterResult {
    parse_differential_report(capture).unwrap()
}

fn refusal(input: &str) -> String {
    parse_differential_report(input).unwrap_err().to_string()
}

#[trace("FR-151-AC-14")]
#[test]
fn tc_001_the_canonical_emitter_reproduces_every_capture_byte_for_byte() {
    for capture in [
        SATISFIED,
        REFUTED,
        DISAGREEMENT,
        UNKNOWN,
        TIMEOUT,
        MISSING,
        UNSUPPORTED,
    ] {
        assert_eq!(derive(capture, |_| {}), capture);
    }
}

#[trace("FR-151-AC-1")]
#[test]
fn tc_001_agreement_satisfied_is_one_pass_entry_keyed_on_kind_and_request_digest() {
    let result = accepted(SATISFIED);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(
        result.entries[0].symbol,
        SymbolId::new(format!("consistency::{SATISFIED_DIGEST}"))
    );
    assert_eq!(result.entries[0].outcome, Outcome::Pass);
    assert!(result.entries[0].trace_ids.is_none());
    assert!(result.unrepresented.is_none());
    assert!(result.evidence_kind.is_none());
}

#[trace("FR-151-AC-2")]
#[test]
fn tc_001_agreement_refuted_is_one_fail_entry() {
    let result = accepted(REFUTED);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(
        result.entries[0].symbol,
        SymbolId::new(format!("consistency::{REFUTED_DIGEST}"))
    );
    assert_eq!(result.entries[0].outcome, Outcome::Fail);
    assert!(result.unrepresented.is_none());
}

#[trace("FR-151-AC-3", "FR-069-AC-3")]
#[test]
fn tc_001_disagreement_is_no_entry_and_one_named_unrepresented_result() {
    let result = accepted(DISAGREEMENT);
    assert!(result.entries.is_empty());
    let unrepresented = result.unrepresented.unwrap();
    assert_eq!(unrepresented.len(), 1);
    assert_eq!(
        unrepresented[0].symbol,
        format!("consistency::{SATISFIED_DIGEST}")
    );
    assert_eq!(unrepresented[0].state, "disagreement");
    assert_eq!(
        unrepresented[0].reason,
        "the two engines both ran and concluded opposite results (z3 status satisfied outcome sat; cvc5 status refuted outcome unsat); that is neither a pass nor a fail nor an error, and no run-entry outcome carries it"
    );
}

#[trace("FR-151-AC-4")]
#[test]
fn tc_001_inconclusive_reports_are_one_error_entry() {
    for capture in [UNKNOWN, TIMEOUT] {
        let result = accepted(capture);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result.entries[0].symbol,
            SymbolId::new(format!("consistency::{SATISFIED_DIGEST}"))
        );
        assert_eq!(result.entries[0].outcome, Outcome::Error);
        assert!(result.unrepresented.is_none());
    }
}

#[trace("FR-151-AC-5")]
#[test]
fn tc_001_unavailable_reports_are_one_error_entry() {
    for capture in [MISSING, UNSUPPORTED] {
        let result = accepted(capture);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result.entries[0].symbol,
            SymbolId::new(format!("consistency::{SATISFIED_DIGEST}"))
        );
        assert_eq!(result.entries[0].outcome, Outcome::Error);
    }
}

/// One refusal class: an input and a fragment its error must carry.
struct Row {
    class: &'static str,
    input: String,
    names: &'static str,
}

fn row(class: &'static str, input: String, names: &'static str) -> Row {
    Row {
        class,
        input,
        names,
    }
}

fn schema(value: Value) -> String {
    derive(SATISFIED, move |report| report["schema"] = value)
}

fn top(change: fn(&mut serde_json::Map<String, Value>)) -> String {
    derive(SATISFIED, move |report| {
        change(report.as_object_mut().unwrap());
    })
}

fn engine(change: fn(&mut serde_json::Map<String, Value>)) -> String {
    derive(SATISFIED, move |report| {
        change(report["engines"][0].as_object_mut().unwrap());
    })
}

/// Wrong schema values and members the schema does not allow.
fn schema_and_member_rows() -> Vec<Row> {
    vec![
        row(
            "schema v1",
            schema(json!("quire.differential-report/v1")),
            "\"quire.differential-report/v1\"",
        ),
        row(
            "retired differential-summary schema",
            schema(json!("tl-mltl.differential-summary/v1")),
            "\"tl-mltl.differential-summary/v1\"",
        ),
        row(
            "schema v3",
            schema(json!("quire.differential-report/v3")),
            "\"quire.differential-report/v3\"",
        ),
        row(
            "schema absent",
            top(|report| {
                report.remove("schema");
            }),
            "schema absent",
        ),
        row("schema not a string", schema(json!(2)), "schema 2"),
        row(
            "required member removed",
            top(|report| {
                report.remove("queryHex");
            }),
            "queryHex",
        ),
        row(
            "undeclared top-level member",
            top(|report| {
                report.insert("extra".to_owned(), json!(1));
            }),
            "extra",
        ),
        row(
            "undeclared engine member",
            engine(|engine| {
                engine.insert("extra".to_owned(), json!(1));
            }),
            "extra",
        ),
        row(
            "wrongly typed member",
            engine(|engine| {
                engine.insert("elapsedMs".to_owned(), json!("x"));
            }),
            "/engines/0/elapsedMs",
        ),
        row(
            "member outside its constant",
            top(|report| {
                report.insert("logic".to_owned(), json!(5));
            }),
            "/logic",
        ),
        row(
            "unknown disposition",
            derive(SATISFIED, |report| {
                report["differential"]["disposition"] = json!("maybe");
            }),
            "/differential/disposition",
        ),
        row(
            "unknown engine status",
            engine(|engine| {
                engine.insert("status".to_owned(), json!("bogus"));
            }),
            "/engines/0/status",
        ),
        row(
            "unknown engine outcome",
            engine(|engine| {
                engine.insert("outcome".to_owned(), json!("bogus"));
            }),
            "/engines/0/outcome",
        ),
    ]
}

/// Inputs that are not a report, and reports that contradict themselves or are not
/// canonical.
fn input_and_contradiction_rows() -> Vec<Row> {
    let report: Value = serde_json::from_str(SATISFIED).unwrap();
    vec![
        row("empty input", String::new(), ""),
        row(
            "truncated report",
            SATISFIED[..SATISFIED.len() / 2].to_owned(),
            "",
        ),
        row("JSON array", "[]".to_owned(), ""),
        row("JSON scalar", "42".to_owned(), ""),
        row("non-JSON text", "differential report".to_owned(), ""),
        row(
            "agreement with null agreedStatus",
            derive(SATISFIED, |report| {
                report["differential"]["agreedStatus"] = Value::Null;
            }),
            "",
        ),
        row(
            "disagreement with non-null agreedStatus",
            derive(DISAGREEMENT, |report| {
                report["differential"]["agreedStatus"] = json!("satisfied");
            }),
            "",
        ),
        row(
            "agreement with an engine status differing from agreedStatus",
            derive(SATISFIED, |report| {
                report["engines"][1]["status"] = json!("refuted");
            }),
            "",
        ),
        row(
            "the same engine twice",
            derive(SATISFIED, |report| {
                report["engines"][1]["engine"] = json!("z3");
            }),
            "",
        ),
        row("members reordered", emit(&report, true), ""),
        row(
            "pretty-printed copy",
            serde_json::to_string_pretty(&report).unwrap(),
            "",
        ),
    ]
}

fn refusal_table() -> Vec<Row> {
    let mut rows = schema_and_member_rows();
    rows.extend(input_and_contradiction_rows());
    rows
}

#[trace(
    "FR-151-AC-6",
    "FR-151-AC-7",
    "FR-151-AC-8",
    "FR-151-AC-9",
    "FR-151-AC-14"
)]
#[test]
fn tc_001_every_refusal_class_is_refused_with_a_reason_and_no_panic() {
    let table = refusal_table();
    // 24 classes here; non-UTF-8 bytes cannot reach a `&str` adapter and is tested at
    // the command (tc_ver80_differential_report_command), and MAX+1 bytes has its own test.
    assert_eq!(table.len(), 24);
    for Row {
        class,
        input,
        names,
    } in table
    {
        let error = refusal(&input);
        assert!(
            error.starts_with("differential-report: "),
            "{class}: {error}"
        );
        assert!(error.contains(names), "{class}: {error}");
        assert_eq!(error, refusal(&input), "{class}: not deterministic");
    }
}

#[trace("FR-151-AC-10", "NFR-032")]
#[test]
fn tc_001_an_input_over_the_limit_is_refused_by_size_before_parsing() {
    // Not JSON at all: any parse would complain about that instead.
    let over = " ".repeat(MAX_REPORT_BYTES + 1);
    let error = refusal(&over);
    assert!(
        error.contains(&format!("over the {MAX_REPORT_BYTES}-byte limit")),
        "{error}"
    );
    assert!(!error.contains("JSON") && !error.contains("EOF"), "{error}");

    let at = " ".repeat(MAX_REPORT_BYTES);
    let error = refusal(&at);
    assert!(!error.contains("byte limit"), "{error}");
}

#[trace("FR-151-AC-11", "NFR-032")]
#[test]
fn tc_001_the_same_bytes_give_the_same_result_wherever_they_are_read() {
    // The adapter takes bytes, never a path, so its result cannot depend on the
    // working directory or the environment; repeated calls over the same bytes
    // are equal for an accepted and for a refused document.
    let first = accepted(DISAGREEMENT);
    let second = accepted(DISAGREEMENT);
    assert_eq!(first, second);
    let pretty =
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(SATISFIED).unwrap()).unwrap();
    assert_eq!(refusal(&pretty), refusal(&pretty));
}

fn chosen(adapter: Option<&str>, tool: Option<&str>) -> &'static str {
    select_adapter(AdapterSelection { adapter, tool })
        .unwrap()
        .name
}

#[trace("FR-151-AC-12", "FR-069-AC-5")]
#[test]
fn tc_001_the_adapter_is_selected_by_name_and_by_quire_analyze_tools_only() {
    assert_eq!(
        chosen(Some("differential-report"), None),
        "differential-report"
    );
    assert_eq!(
        chosen(None, Some("differential-report")),
        "differential-report"
    );
    assert_eq!(
        chosen(None, Some("quire-analyze 0.1.0")),
        "differential-report"
    );
    for retired in ["tl-mltl", "r2u2", "c2po"] {
        assert_eq!(chosen(None, Some(retired)), "entries", "{retired}");
    }
    let listing = ADAPTERS
        .iter()
        .find(|adapter| adapter.name == "differential-report")
        .unwrap();
    assert!(listing.summary.contains("quire.differential-report/v2"));
    assert!(!listing.summary.contains("differential-summary"));
}
