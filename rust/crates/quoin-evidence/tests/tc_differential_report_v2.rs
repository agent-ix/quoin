// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! TC-001: the `differential-report` adapter transcribes every
//! `filament.differential-report/v2` disposition and refuses malformed reports
//! (FR-151, NFR-032).
//!
//! The accepted inputs are reports `filament-analyze run-package` wrote (see
//! `tests/fixtures/evidence/README.md`). Each refusal input is one of those
//! reports with one stated change.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use std::collections::BTreeMap;

use filament_analyze::MAX_REPORT_BYTES;
use ix_trace_rs::trace;
use quoin_evidence::adapters::{
    ADAPTERS, AdapterResult, AdapterSelection, build_approved_claim_selection,
    build_approved_formal_rule_selection, parse_differential_report,
    parse_differential_report_for_run, select_adapter,
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
fn tc_001_agreement_satisfied_is_one_pass_entry_keyed_on_analysis_kind() {
    let result = accepted(SATISFIED);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].symbol, SymbolId::new("consistency"));
    assert_eq!(result.entries[0].outcome, Outcome::Pass);
    assert!(result.entries[0].trace_ids.is_none());
    assert!(result.unrepresented.is_none());
    assert!(result.evidence_kind.is_none());
}

#[trace("FR-151-AC-15")]
#[test]
fn tc_001_projection_contains_only_the_qualified_allowlist() {
    let projection = accepted(SATISFIED).analysis_projection.unwrap();
    assert_eq!(
        projection["protocol"],
        "quoin.analysis-report-projection/v1"
    );
    assert_eq!(projection["disposition"], "agreement");
    assert_eq!(projection["agreed_status"], "satisfied");
    assert!(projection.get("selection").is_none());
    assert_eq!(
        projection["producer"],
        json!({"name":"filament-analyze","version":"0.1.0"})
    );
    assert_eq!(projection["engines"][0]["engine"], "z3");
    assert!(projection["engines"][0].get("identity_path").is_some());
    for forbidden in [
        "requestDigest",
        "query",
        "modelHex",
        "stdoutHex",
        "stderrHex",
        "proof",
    ] {
        assert!(projection.get(forbidden).is_none());
    }
    assert_eq!(projection.as_object().unwrap().len(), 5);
}

#[trace("FR-151-AC-16")]
#[test]
fn tc_001_projection_retains_selection_and_omits_it_when_source_report_has_none() {
    let unselected = accepted(SATISFIED).analysis_projection.unwrap();
    assert!(unselected.get("selection").is_none());

    let report = derive(SATISFIED, |report| {
        report["selection"] = json!({
            "selected": ["claim-0", "claim-1"],
            "excluded": []
        });
    });
    let projection = accepted(&report).analysis_projection.unwrap();
    assert_eq!(
        projection["selection"],
        json!({"selected":["claim-0", "claim-1"],"excluded":[]})
    );
    assert_eq!(projection.as_object().unwrap().len(), 6);
}

#[trace("FR-151-AC-17")]
#[test]
fn tc_001_formal_rule_projection_checks_and_retains_claim_mapping() {
    let report = derive(SATISFIED, |report| {
        report["selection"] = json!({"selected":["claim-0", "claim-1"],"excluded":[]});
    });
    let source: Value = serde_json::from_str(&report).unwrap();
    let mapping = json!({
        "claims": [
            {
                "claim_id": "claim-0",
                "source": source["assertions"][0]["source"],
                "symbol": "qsl:qsl/rule.native::Rule0",
                "state": "approved"
            },
            {
                "claim_id": "claim-1",
                "source": source["assertions"][1]["source"],
                "symbol": "qsl:qsl/rule.native::Rule1",
                "state": "draft"
            }
        ]
    });
    let projection = parse_differential_report_for_run(&report, "formal_rule", Some(&mapping))
        .unwrap()
        .analysis_projection
        .unwrap();
    assert_eq!(projection["run_kind"], "formal_rule");
    assert_eq!(projection["formal_rule_selection"], mapping);

    let mut incomplete = mapping.clone();
    incomplete["claims"].as_array_mut().unwrap().pop();
    let error =
        parse_differential_report_for_run(&report, "formal_rule", Some(&incomplete)).unwrap_err();
    assert!(error.to_string().contains("every selected claim"));
}

#[trace("FR-184-AC-17")]
#[test]
fn formal_rule_selection_builder_maps_only_approved_selected_symbols() {
    let mut report: Value = serde_json::from_str(SATISFIED).unwrap();
    report["selection"] = json!({"selected":["claim-0", "claim-1"],"excluded":[]});
    let symbols = BTreeMap::from([
        (
            "claim-0".to_owned(),
            "qsl:qsl/rule.native::Rule0".to_owned(),
        ),
        (
            "claim-1".to_owned(),
            "qsl:qsl/rule.native::Rule1".to_owned(),
        ),
    ]);
    let status = json!({
        "protocol":"quoin.formal-rule-status/v1",
        "rules":[
            {"symbol":"qsl:qsl/rule.native::Rule0","state":"approved"},
            {"symbol":"qsl:qsl/rule.native::Rule1","state":"approved"}
        ]
    });
    let mapping = build_approved_formal_rule_selection(&report, &symbols, &status).unwrap();
    assert_eq!(mapping["claims"].as_array().unwrap().len(), 2);
    assert_eq!(mapping["claims"][0]["claim_id"], "claim-0");
    assert_eq!(
        mapping["claims"][0]["source"],
        report["assertions"][0]["source"]
    );
    assert_eq!(mapping["claims"][0]["state"], "approved");

    let draft_status = json!({
        "protocol":"quoin.formal-rule-status/v1",
        "rules":[
            {"symbol":"qsl:qsl/rule.native::Rule0","state":"approved"},
            {"symbol":"qsl:qsl/rule.native::Rule1","state":"draft"}
        ]
    });
    let draft_error =
        build_approved_formal_rule_selection(&report, &symbols, &draft_status).unwrap_err();
    assert!(draft_error.to_string().contains("not approved"));
    assert_eq!(draft_error.code().as_str(), "QE-E001");
}

#[trace("FR-190-AC-1")]
#[test]
fn approved_claim_selection_builder_excludes_draft_spans() {
    let status = json!({
        "protocol":"quoin.formal-rule-status/v1",
        "rules":[
            {"symbol":"qsl:rules/a.qsl::Approved","path":"rules/a.qsl","declaration_range":{"start":12,"end":34},"state":"approved"},
            {"symbol":"qsl:rules/d.qsl::Draft","path":"rules/d.qsl","declaration_range":{"start":40,"end":68},"state":"draft"},
            {"symbol":"qsl:rules/a.qsl::ApprovedAgain","path":"rules/a.qsl","declaration_range":{"start":50,"end":81},"state":"approved"}
        ]
    });
    let selection = build_approved_claim_selection(&status, "quoin").unwrap();
    assert_eq!(selection["protocol"], "filament-analyze.claim-selection/v1");
    assert_eq!(selection["spans"].as_array().unwrap().len(), 2);
    assert_eq!(
        selection["spans"][0],
        json!({
            "authority":"quoin", "identity":"rules/a.qsl", "start":12, "end":34
        })
    );
    assert_eq!(selection["spans"][1]["start"], 50);
    assert!(selection.to_string().find("Draft").is_none());

    let invalid_authority = build_approved_claim_selection(&status, "").unwrap_err();
    assert_eq!(invalid_authority.code().as_str(), "QE-E001");
}

#[trace("FR-151-AC-2")]
#[test]
fn tc_001_agreement_refuted_is_one_fail_entry() {
    let result = accepted(REFUTED);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].symbol, SymbolId::new("consistency"));
    assert_eq!(result.entries[0].outcome, Outcome::Fail);
    assert!(result.unrepresented.is_none());
}

#[trace("FR-151-AC-3", "FR-069-AC-3")]
#[test]
fn tc_001_disagreement_is_no_entry_and_one_named_unrepresented_result() {
    let result = accepted(DISAGREEMENT);
    assert_eq!(result.entries, [] as [quoin_evidence::RunEntry; 0]);
    let unrepresented = result.unrepresented.unwrap();
    assert_eq!(unrepresented.len(), 1);
    assert_eq!(unrepresented[0].symbol, "consistency");
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
        assert_eq!(result.entries[0].symbol, SymbolId::new("consistency"));
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
        assert_eq!(result.entries[0].symbol, SymbolId::new("consistency"));
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
            schema(json!("filament.differential-report/v1")),
            "\"filament.differential-report/v1\"",
        ),
        row(
            "retired quire schema",
            schema(json!("quire.differential-report/v2")),
            "\"quire.differential-report/v2\"",
        ),
        row(
            "retired differential-summary schema",
            schema(json!("tl-mltl.differential-summary/v1")),
            "\"tl-mltl.differential-summary/v1\"",
        ),
        row(
            "schema v3",
            schema(json!("filament.differential-report/v3")),
            "\"filament.differential-report/v3\"",
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
    "FR-151-AC-14",
    "NFR-032-M-3"
)]
#[test]
fn tc_001_every_refusal_class_is_refused_with_a_reason_and_no_panic() {
    let table = refusal_table();
    // 24 classes here; non-UTF-8 bytes cannot reach a `&str` adapter and is tested at
    // the command (tc_ver80_differential_report_command), and MAX+1 bytes has its own test.
    assert_eq!(table.len(), 25);
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

#[trace("FR-151-AC-10", "NFR-032-M-1")]
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

#[trace("FR-151-AC-11", "NFR-032-M-2")]
#[test]
fn tc_001_the_same_bytes_give_the_same_result_wherever_they_are_read() {
    const INPUT_PATH: &str = "QUIRE_TEST_DIFFERENTIAL_REPORT_PATH";
    if let Some(path) = std::env::var_os(INPUT_PATH) {
        let bytes = std::fs::read_to_string(path).unwrap();
        println!("PARSE_RESULT: {:?}", parse_differential_report(&bytes));
        return;
    }

    let root = tempfile::tempdir().unwrap();
    let first_dir = root.path().join("first-cwd");
    let second_dir = root.path().join("second-cwd");
    std::fs::create_dir_all(&first_dir).unwrap();
    std::fs::create_dir_all(&second_dir).unwrap();
    let first_path = first_dir.join("report.json");
    let second_path = second_dir.join("report.json");
    std::fs::write(&first_path, SATISFIED).unwrap();
    std::fs::write(&second_path, SATISFIED).unwrap();

    let executable = std::env::current_exe().unwrap();
    let run = |path: &std::path::Path, cwd: &std::path::Path, zone: &str| {
        let output = std::process::Command::new(&executable)
            .args([
                "--exact",
                "tc_001_the_same_bytes_give_the_same_result_wherever_they_are_read",
                "--nocapture",
                "--test-threads=1",
            ])
            .current_dir(cwd)
            .env(INPUT_PATH, path)
            .env("TZ", zone)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let marker = stdout
            .find("PARSE_RESULT: ")
            .expect("probe reports the adapter result");
        stdout[marker + "PARSE_RESULT: ".len()..]
            .lines()
            .next()
            .unwrap()
            .to_owned()
    };

    let first = run(&first_path, &first_dir, "UTC");
    let second = run(&second_path, &second_dir, "Pacific/Auckland");
    assert_eq!(first, second, "result changed across path, cwd, or TZ");
}

fn chosen(adapter: Option<&str>, tool: Option<&str>) -> &'static str {
    select_adapter(AdapterSelection { adapter, tool })
        .unwrap()
        .name
}

#[trace("FR-151-AC-12", "FR-069-AC-5")]
#[test]
fn tc_001_the_adapter_is_selected_by_name_and_by_filament_analyze_tools_only() {
    assert_eq!(
        chosen(Some("differential-report"), None),
        "differential-report"
    );
    assert_eq!(
        chosen(None, Some("differential-report")),
        "differential-report"
    );
    assert_eq!(
        chosen(None, Some("filament-analyze 0.1.0")),
        "differential-report"
    );
    assert_eq!(chosen(None, Some("quire-analyze 0.1.0")), "entries");
    for retired in ["tl-mltl", "r2u2", "c2po"] {
        assert_eq!(chosen(None, Some(retired)), "entries", "{retired}");
    }
    let listing = ADAPTERS
        .iter()
        .find(|adapter| adapter.name == "differential-report")
        .unwrap();
    assert!(listing.summary.contains("filament.differential-report/v2"));
    assert!(!listing.summary.contains("differential-summary"));
}
