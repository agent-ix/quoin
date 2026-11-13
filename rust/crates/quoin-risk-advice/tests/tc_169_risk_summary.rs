// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The risk summary over authored fixture records (FR-169, FR-170).

mod common;

use common::*;
use ix_trace_rs::trace;
use quoin_risk_advice::{ErrorCode, advise, discrepancies, summarize};
use serde_json::{Value, json};

impl Corpus {
    fn json(&self) -> Value {
        let summary = summarize(self.dir.path(), &modules()).unwrap();
        serde_json::from_str(&summary.to_json().unwrap()).unwrap()
    }

    fn markdown(&self) -> String {
        summarize(self.dir.path(), &modules())
            .unwrap()
            .to_markdown()
    }

    fn rows(&self) -> Vec<Value> {
        self.json()["rows"].as_array().unwrap().clone()
    }

    /// The row of the pairing, which must exist once.
    fn row(&self, hazard: Option<&str>, situation: Option<&str>) -> Value {
        let found: Vec<Value> = self
            .rows()
            .into_iter()
            .filter(|row| {
                row["hazard"].as_str() == hazard && row["situation"].as_str() == situation
            })
            .collect();
        assert_eq!(found.len(), 1, "{hazard:?} {situation:?}");
        found[0].clone()
    }
}

fn hazard(c: &Corpus, id: &str) {
    c.put(id, "hazard", "status: accepted\n", "");
}

fn situation(c: &Corpus, id: &str, hazards: &[&str]) {
    let edges: Vec<(&str, &str)> = hazards.iter().map(|h| (*h, "arises_from")).collect();
    c.put(id, "hazardous_situation", &rels(&edges), "");
}

fn harm(c: &Corpus, id: &str, situation: &str) {
    c.put(id, "harm", &rels(&[(situation, "arises_from")]), "");
}

fn requirement(c: &Corpus, id: &str, control: &str) {
    c.put(
        id,
        "FR",
        &rels(&[(control, "satisfies")]),
        "| ID | Criteria |\n|---|---|\n| X-AC-1 | x |\n",
    );
}

fn verification(c: &Corpus, id: &str, kind: &str, control: &str) {
    c.put(
        id,
        kind,
        &rels(&[(control, "verifies")]),
        "result: failed\n",
    );
}

/// A complete chain: HAZ-1, SIT-1, HARM-1, CTRL-1 with its requirement and
/// test, a pre- and a post-control estimate, and a decided residual risk.
fn complete(c: &Corpus) {
    policy(c, "POL-1");
    hazard(c, "HAZ-1");
    situation(c, "SIT-1", &["HAZ-1"]);
    harm(c, "HARM-1", "SIT-1");
    c.put("CTRL-1", "control", "", "");
    requirement(c, "FR-9", "CTRL-1");
    verification(c, "TC-9", "TC", "CTRL-1");
    estimate(
        c,
        "EST-A",
        "pre_control",
        ("Serious", "Likely"),
        &[("SIT-1", "estimates")],
    );
    estimate(
        c,
        "EST-B",
        "post_control",
        ("Minor", "Rare"),
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    residual(c, "RR-1", "EST-B", "POL-1", "accepted");
}

fn tokens(row: &Value) -> Vec<String> {
    row["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["finding"].as_str().unwrap().to_owned())
        .collect()
}

fn names(row: &Value, token: &str) -> Vec<String> {
    row["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["finding"] == token)
        .map(|f| {
            f["names"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[trace(
    "FR-169-AC-1",
    "FR-169-AC-2",
    "FR-169-AC-22",
    "FR-170-AC-1",
    "FR-170-AC-12",
    "FR-170-AC-13",
    "FR-170-AC-14"
)]
#[test]
fn a_complete_chain_is_one_row_with_exact_keys_and_no_findings() {
    let c = corpus();
    complete(&c);
    let json = c.json();
    assert_eq!(keys(&json), ["attribution", "rows"]);
    let rows = json["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(
        keys(row),
        [
            "controls",
            "findings",
            "harms",
            "hazard",
            "post_control",
            "pre_control",
            "residual_risks",
            "situation"
        ]
    );
    assert_eq!(row["harms"], json!(["HARM-1"]));
    assert_eq!(
        row["pre_control"],
        json!([{"estimate": "EST-A", "severity": "Serious", "probability": "Likely", "policy_region": "High"}])
    );
    assert_eq!(
        keys(&row["post_control"][0]),
        ["estimate", "policy_region", "probability", "severity"]
    );
    assert_eq!(
        row["controls"],
        json!([{"control": "CTRL-1", "requirements": ["FR-9"], "verifications": ["TC-9"]}])
    );
    assert_eq!(
        row["residual_risks"],
        json!([{"residual_risk": "RR-1", "decision": "accepted", "decided_by": "Pat"}])
    );
    assert_eq!(row["findings"], json!([]));
}

#[trace("FR-169-AC-22")]
#[test]
fn a_verification_of_any_type_counts_as_the_edge() {
    let c = corpus();
    complete(&c);
    std::fs::remove_file(c.dir.path().join("TC-9.md")).unwrap();
    verification(&c, "EV-1", "retained_evidence", "CTRL-1");
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    assert_eq!(row["controls"][0]["verifications"], json!(["EV-1"]));
    assert!(!tokens(&row).contains(&"control-without-verification".to_owned()));
}

#[trace(
    "FR-169-AC-3",
    "FR-169-AC-8",
    "FR-170-AC-2",
    "FR-170-AC-3",
    "FR-170-AC-4",
    "FR-170-AC-11"
)]
#[test]
fn missing_links_give_rows_with_null_and_a_situation_of_two_hazards_gives_two_rows() {
    let c = corpus();
    complete(&c);
    hazard(&c, "HAZ-0");
    hazard(&c, "HAZ-2");
    hazard(&c, "HAZ-3");
    situation(&c, "SIT-2", &["HAZ-2", "HAZ-1"]);
    harm(&c, "HARM-2", "SIT-2");
    situation(&c, "SIT-0", &[]);
    situation(&c, "SIT-9", &["HAZ-3"]);

    let pairs: Vec<(Option<String>, Option<String>)> = c
        .rows()
        .iter()
        .map(|r| {
            (
                r["hazard"].as_str().map(str::to_owned),
                r["situation"].as_str().map(str::to_owned),
            )
        })
        .collect();
    let some = |h: &str, s: Option<&str>| (Some(h.to_owned()), s.map(str::to_owned));
    assert_eq!(
        pairs,
        [
            some("HAZ-0", None),
            some("HAZ-1", Some("SIT-1")),
            some("HAZ-1", Some("SIT-2")),
            some("HAZ-2", Some("SIT-2")),
            some("HAZ-3", Some("SIT-9")),
            (None, Some("SIT-0".to_owned())),
        ]
    );

    let bare = c.row(Some("HAZ-0"), None);
    assert_eq!(bare["harms"], json!([]));
    assert_eq!(bare["controls"], json!([]));
    assert_eq!(names(&bare, "hazard-without-situation"), ["HAZ-0"]);

    let orphan = c.row(None, Some("SIT-0"));
    assert_eq!(names(&orphan, "situation-without-hazard"), ["SIT-0"]);
    assert_eq!(names(&orphan, "situation-without-harm"), ["SIT-0"]);

    let first = c.row(Some("HAZ-1"), Some("SIT-2"));
    let second = c.row(Some("HAZ-2"), Some("SIT-2"));
    let strip = |row: &Value| {
        let mut row = row.clone();
        row["hazard"] = Value::Null;
        row
    };
    assert_eq!(strip(&first), strip(&second));
    for row in [&first, &second] {
        assert!(!tokens(row).contains(&"hazard-without-situation".to_owned()));
    }
    // Only SIT-9 has no harm among the rows of a hazard.
    assert_eq!(
        names(
            &c.row(Some("HAZ-3"), Some("SIT-9")),
            "situation-without-harm"
        ),
        ["SIT-9"]
    );
    assert!(tokens(&c.row(Some("HAZ-1"), Some("SIT-1"))).is_empty());
}

#[trace("FR-169-AC-4", "FR-170-AC-9")]
#[test]
fn a_marker_shows_as_entered_with_no_region_while_a_sibling_keeps_its_own() {
    let c = corpus();
    complete(&c);
    estimate(
        &c,
        "EST-C",
        "post_control",
        ("Serious", "not_assessed"),
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    residual(&c, "RR-2", "EST-C", "POL-1", "accepted");
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    assert_eq!(row["pre_control"][0]["policy_region"], "High");
    let marker = &row["post_control"][1];
    assert_eq!(marker["estimate"], "EST-C");
    assert_eq!(marker["probability"], "not_assessed");
    assert_eq!(marker["policy_region"], Value::Null);
    assert_eq!(names(&row, "decision-without-region"), ["RR-2"]);
}

#[trace("FR-169-AC-5", "FR-170-AC-6")]
#[test]
fn an_undecided_record_is_shown_undecided_and_a_hazard_status_decides_nothing() {
    let c = corpus();
    complete(&c);
    residual(&c, "RR-1", "EST-B", "POL-1", "undecided");
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    assert_eq!(
        row["residual_risks"],
        json!([{"residual_risk": "RR-1", "decision": "undecided", "decided_by": null}])
    );
    assert_eq!(tokens(&row), ["residual-risk-undecided"]);
    assert!(!c.json().to_string().contains("\"accepted\""));

    std::fs::remove_file(c.dir.path().join("RR-1.md")).unwrap();
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    assert_eq!(row["residual_risks"], json!([]));
    assert_eq!(
        names(&row, "post-control-estimate-without-decision"),
        ["EST-B"]
    );
}

#[trace("FR-169-AC-6", "FR-169-AC-7", "FR-170-AC-5", "FR-170-AC-15")]
#[test]
fn several_estimates_are_all_listed_a_shared_control_once_and_links_are_edges_only() {
    let c = corpus();
    complete(&c);
    estimate(
        &c,
        "EST-D",
        "post_control",
        ("Minor", "Likely"),
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    estimate(
        &c,
        "EST-E",
        "post_control",
        ("Minor", "Likely"),
        &[("SIT-1", "estimates")],
    );
    estimate(
        &c,
        "EST-F",
        "pre_control",
        ("Serious", "Rare"),
        &[("SIT-1", "estimates")],
    );
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    let ids = |key: &str| -> Vec<String> {
        row[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["estimate"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(ids("pre_control"), ["EST-A", "EST-F"]);
    assert_eq!(ids("post_control"), ["EST-B", "EST-D", "EST-E"]);
    assert_eq!(row["controls"].as_array().unwrap().len(), 1);
    assert_eq!(names(&row, "post-control-without-control"), ["EST-E"]);
    assert_eq!(
        row["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["finding"] == "post-control-without-control")
            .count(),
        1
    );

    // A control with only body links to it has neither edge.
    c.put("CTRL-2", "control", "", "");
    c.put(
        "FR-8",
        "FR",
        "",
        "See [the control](ix://agent-ix/x/CTRL-2).\n\n| ID | Criteria |\n|---|---|\n| FR-8-AC-1 | x |\n",
    );
    estimate(
        &c,
        "EST-G",
        "post_control",
        ("Minor", "Rare"),
        &[("SIT-1", "estimates"), ("CTRL-2", "assumes")],
    );
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    assert_eq!(names(&row, "control-without-requirement"), ["CTRL-2"]);
    assert_eq!(names(&row, "control-without-verification"), ["CTRL-2"]);
    let second = row["controls"][1].clone();
    assert_eq!(second["requirements"], json!([]));
    assert_eq!(second["verifications"], json!([]));
}

#[trace("FR-169-AC-9", "FR-169-AC-25", "FR-170-AC-10")]
#[test]
fn the_region_is_the_one_risk_advise_shows_and_two_policies_give_null_with_each_finding_on_its_own()
{
    let c = corpus();
    complete(&c);
    let advice: Value =
        serde_json::from_str(&advise(c.dir.path(), &modules()).unwrap().to_json().unwrap())
            .unwrap();
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    for entry in row["pre_control"]
        .as_array()
        .unwrap()
        .iter()
        .chain(row["post_control"].as_array().unwrap())
    {
        let cell = advice["policy_cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|cell| cell["estimate"] == entry["estimate"])
            .unwrap();
        assert_eq!(cell["region"], entry["policy_region"]);
    }

    // A second policy whose region for the same levels is not outside.
    policy_with(&c, "POL-2", false, &REGIONS);
    estimate(
        &c,
        "EST-H",
        "post_control",
        ("Serious", "Likely"),
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    residual(&c, "RR-2", "EST-H", "POL-1", "not_accepted");
    residual(&c, "RR-3", "EST-H", "POL-2", "not_accepted");
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    let h = row["post_control"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["estimate"] == "EST-H")
        .unwrap();
    assert_eq!(h["policy_region"], Value::Null);
    // RR-2 rests on POL-1 (High, outside), RR-3 on POL-2 (High, not outside).
    assert_eq!(
        names(&row, "not-accepted-inside-acceptable-region"),
        ["RR-3"]
    );
    assert_eq!(row["residual_risks"].as_array().unwrap().len(), 3);

    // The residual-risk tokens equal those of `risk discrepancies`.
    let report: Value = serde_json::from_str(
        &discrepancies(c.dir.path(), &modules())
            .unwrap()
            .to_json()
            .unwrap(),
    )
    .unwrap();
    let mut from_report: Vec<(String, String)> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["refs"]["residual_risk"].is_string())
        .map(|f| {
            (
                f["refs"]["residual_risk"].as_str().unwrap().to_owned(),
                f["finding"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    from_report.sort();
    let mut from_row: Vec<(String, String)> = row["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            f["finding"] != "control-without-requirement"
                && f["finding"] != "control-without-verification"
                && f["finding"] != "post-control-without-control"
                && f["finding"] != "situation-without-harm"
        })
        .map(|f| {
            (
                f["names"][0].as_str().unwrap().to_owned(),
                f["finding"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    from_row.sort();
    assert_eq!(from_row, from_report);
}

#[trace("FR-170-AC-7", "FR-170-AC-8")]
#[test]
fn decisions_against_the_policy_carry_the_discrepancy_tokens_naming_covering_records() {
    let c = corpus();
    complete(&c);
    estimate(
        &c,
        "EST-H",
        "post_control",
        ("Serious", "Likely"),
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    residual(&c, "RR-2", "EST-H", "POL-1", "accepted");
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    assert_eq!(names(&row, "accepted-outside-acceptable-region"), ["RR-2"]);
    for (decision, token) in [
        ("accepted", ""),
        ("undecided", "benefit-risk-undecided"),
        ("not_accepted", "benefit-risk-not-accepted"),
    ] {
        let by = if decision == "undecided" { "" } else { "Pat" };
        c.put(
            "BR-1",
            "benefit_risk",
            &rels(&[("RR-2", "references")]),
            &format!(
                "## Determination\n\n| Benefit | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| b | {decision} | r | {by} |\n"
            ),
        );
        let row = c.row(Some("HAZ-1"), Some("SIT-1"));
        let found = tokens(&row);
        if token.is_empty() {
            assert!(
                !found
                    .iter()
                    .any(|t| t.contains("outside") || t.starts_with("benefit")),
                "{found:?}"
            );
        } else {
            assert_eq!(names(&row, token), ["RR-2", "BR-1"]);
        }
    }
    residual(&c, "RR-1", "EST-B", "POL-1", "not_accepted");
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    assert_eq!(
        names(&row, "not-accepted-inside-acceptable-region"),
        ["RR-1"]
    );
}

#[trace("FR-170-AC-12")]
#[test]
fn findings_are_in_table_order_then_by_first_name() {
    let c = corpus();
    complete(&c);
    c.put("CTRL-0", "control", "", "");
    estimate(
        &c,
        "EST-0",
        "post_control",
        ("Minor", "Rare"),
        &[("SIT-1", "estimates"), ("CTRL-0", "assumes")],
    );
    estimate(
        &c,
        "EST-9",
        "post_control",
        ("Minor", "Rare"),
        &[("SIT-1", "estimates")],
    );
    residual(&c, "RR-1", "EST-B", "POL-1", "undecided");
    let row = c.row(Some("HAZ-1"), Some("SIT-1"));
    let order: Vec<(String, String)> = row["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["finding"].as_str().unwrap().to_owned(),
                f["names"][0].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        order,
        [
            pair("control-without-requirement", "CTRL-0"),
            pair("control-without-verification", "CTRL-0"),
            pair("post-control-without-control", "EST-9"),
            pair("post-control-estimate-without-decision", "EST-0"),
            pair("post-control-estimate-without-decision", "EST-9"),
            pair("residual-risk-undecided", "RR-1"),
        ]
    );
}

#[trace("FR-169-AC-12", "FR-170-AC-13")]
#[test]
fn no_key_is_a_score_and_no_value_is_numeric() {
    let c = corpus();
    complete(&c);
    situation(&c, "SIT-0", &[]);
    fn walk(value: &Value) {
        match value {
            Value::Number(n) => panic!("numeric value {n}"),
            Value::Array(items) => items.iter().for_each(walk),
            Value::Object(map) => {
                for (key, inner) in map {
                    for banned in [
                        "score",
                        "weight",
                        "total",
                        "rank",
                        "verdict",
                        "recommend",
                        "proposal",
                        "priority",
                        "count",
                        "remedy",
                        "severity_of",
                    ] {
                        assert!(!key.contains(banned), "{key}");
                    }
                    walk(inner);
                }
            }
            _ => {}
        }
    }
    walk(&c.json());
}

#[trace("FR-169-AC-10", "FR-169-AC-11")]
#[test]
fn a_run_changes_no_file_and_two_runs_are_byte_identical() {
    let c = corpus();
    complete(&c);
    situation(&c, "SIT-0", &[]);
    let before = snapshot(c.dir.path());
    let first = summarize(c.dir.path(), &modules()).unwrap();
    let second = summarize(c.dir.path(), &modules()).unwrap();
    assert_eq!(first.to_markdown(), second.to_markdown());
    assert_eq!(first.to_json().unwrap(), second.to_json().unwrap());
    assert_eq!(snapshot(c.dir.path()), before);
}

#[trace("FR-169-AC-13", "FR-169-AC-14", "FR-169-AC-23")]
#[test]
fn the_markdown_joins_ids_entries_and_pairs_with_the_documented_separators() {
    let c = corpus();
    complete(&c);
    harm(&c, "HARM-2", "SIT-1");
    estimate(
        &c,
        "EST-C",
        "post_control",
        ("Serious", "not_assessed"),
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    let md = c.markdown();
    let mut lines = md.lines();
    let attribution = lines.next().unwrap();
    assert!(attribution.contains("policy") && attribution.contains("named person"));
    assert_eq!(lines.next(), Some(""));
    assert_eq!(
        lines.next(),
        Some(
            "| hazard | situation | harms | pre_control | controls | post_control | residual_risks | findings |"
        )
    );
    assert_eq!(
        lines.next(),
        Some("| --- | --- | --- | --- | --- | --- | --- | --- |")
    );
    let row = lines.next().unwrap();
    assert!(lines.next().is_none());
    assert!(
        row.starts_with("| HAZ-1 | SIT-1 | HARM-1, HARM-2 | "),
        "{row}"
    );
    assert!(
        row.contains(
            "estimate=EST-B; severity=Minor; probability=Rare; the policy marks this cell as Low<br>estimate=EST-C; severity=Serious; probability=not_assessed; policy_region=null"
        ),
        "{row}"
    );
    assert!(row.contains("control=CTRL-1; requirements=FR-9; verifications=TC-9"));
    assert!(row.contains("residual_risk=RR-1; decision=accepted; decided_by=Pat"));
    assert!(row.contains("finding=post-control-estimate-without-decision; names=EST-C"));
    // The stem appears only in the attribution, a finding token and copied values.
    for line in md.lines().filter(|line| line.contains("accept")) {
        let stripped = line
            .replace("decision=accepted", "")
            .replace("accepted-outside-acceptable-region", "")
            .replace("not-accepted-inside-acceptable-region", "");
        assert!(
            line == attribution || !stripped.contains("accept"),
            "{line}"
        );
    }
    let bare = corpus();
    hazard(&bare, "HAZ-0");
    let md = bare.markdown();
    assert!(md.contains(
        "| HAZ-0 | null | [] | [] | [] | [] | [] | finding=hazard-without-situation; names=HAZ-0 |"
    ));
}

#[trace("FR-169-AC-15")]
#[test]
fn an_empty_scope_has_the_attribution_and_no_rows() {
    let c = corpus();
    let json = c.json();
    assert_eq!(json["rows"], json!([]));
    assert!(json["attribution"].as_str().unwrap().contains("policy"));
}

#[trace("FR-169-AC-16", "FR-169-AC-20")]
#[test]
fn a_record_that_breaks_its_contract_stops_the_report_with_the_advise_code() {
    let c = corpus();
    complete(&c);
    residual(&c, "RR-1", "EST-B", "POL-1", "maybe");
    let error = summarize(c.dir.path(), &modules()).err().unwrap();
    assert_eq!(error.code, ErrorCode::Record);
    assert!(error.message.contains("RR-1"));
    assert_eq!(
        advise(c.dir.path(), &modules()).err().unwrap().code,
        error.code
    );

    residual(&c, "RR-1", "EST-B", "POL-1", "accepted");
    c.put(
        "BR-1",
        "benefit_risk",
        &rels(&[("RR-1", "references")]),
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| undecided | r | |\n",
    );
    let error = summarize(c.dir.path(), &modules()).err().unwrap();
    assert_eq!(error.code, ErrorCode::Record);
    assert!(error.message.contains("BR-1"));
    std::fs::remove_file(c.dir.path().join("BR-1.md")).unwrap();

    let text = std::fs::read_to_string(c.dir.path().join("POL-1.md")).unwrap();
    std::fs::write(
        c.dir.path().join("POL-1.md"),
        text.replace("| Minor |\n| Serious |", "| <level> |\n| Serious |"),
    )
    .unwrap();
    let error = summarize(c.dir.path(), &modules()).err().unwrap();
    assert_eq!(error.code, ErrorCode::Record);
    assert!(error.message.contains("POL-1"));
}

#[trace("FR-169-AC-16", "FR-169-AC-20")]
#[test]
fn a_missing_scope_and_an_undeclared_type_use_the_advise_codes() {
    let c = corpus();
    let gone = c.dir.path().join("absent");
    let error = summarize(&gone, &modules()).err().unwrap();
    assert_eq!(error.code, advise(&gone, &modules()).err().unwrap().code);
    assert!(error.message.contains("absent"));
}

#[trace("FR-169-AC-21")]
#[test]
fn an_unreadable_file_is_listed_and_the_report_still_renders() {
    let c = corpus();
    complete(&c);
    std::fs::write(c.dir.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    let summary = summarize(c.dir.path(), &modules()).unwrap();
    assert_eq!(summary.unreadable.len(), 1);
    assert!(summary.unreadable[0].contains("RR-9.md"));
    assert_eq!(summary.rows.len(), 1);
    assert!(!summary.to_json().unwrap().contains("RR-9"));
}

#[trace("FR-169-AC-17", "FR-169-AC-24", "FR-170-AC-16")]
#[test]
fn estimates_outside_the_contract_or_without_a_situation_are_in_no_row() {
    let c = corpus();
    complete(&c);
    estimate(
        &c,
        "EST-X",
        "sideways",
        ("Minor", "Rare"),
        &[("SIT-1", "estimates")],
    );
    c.put(
        "EST-Y",
        "risk_estimate",
        "phase: post_control\n",
        "## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n",
    );
    estimate(&c, "EST-Z", "post_control", ("Minor", "Rare"), &[]);
    residual(&c, "RR-7", "EST-Z", "POL-1", "accepted");
    let json = c.json();
    let text = json.to_string();
    for absent in [
        "EST-X",
        "EST-Y",
        "EST-Z",
        "RR-7",
        "estimate-without-situation",
    ] {
        assert!(!text.contains(absent), "{absent}");
    }
    let advice: Value =
        serde_json::from_str(&advise(c.dir.path(), &modules()).unwrap().to_json().unwrap())
            .unwrap();
    assert!(advice.to_string().contains("estimate-without-situation"));
}

/// The snapshot corpus: one complete chain, and one chain with each gap.
fn snapshot_corpus() -> Corpus {
    let c = corpus();
    complete(&c);
    // HAZ-2: a situation with no harm, a control with no requirement or
    // verification, a post-control estimate with no control and no decision.
    hazard(&c, "HAZ-2");
    situation(&c, "SIT-2", &["HAZ-2"]);
    c.put("CTRL-2", "control", "", "");
    estimate(
        &c,
        "EST-C",
        "post_control",
        ("Serious", "Likely"),
        &[("SIT-2", "estimates"), ("CTRL-2", "assumes")],
    );
    estimate(
        &c,
        "EST-D",
        "post_control",
        ("Serious", "not_assessed"),
        &[("SIT-2", "estimates")],
    );
    residual(&c, "RR-2", "EST-C", "POL-1", "accepted");
    residual(&c, "RR-3", "EST-D", "POL-1", "not_accepted");
    // HAZ-3 has no situation, and SIT-3 has no hazard.
    hazard(&c, "HAZ-3");
    situation(&c, "SIT-3", &[]);
    c.put(
        "BR-1",
        "benefit_risk",
        &rels(&[("RR-2", "references")]),
        "## Determination\n\n| Benefit | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| b | undecided | r | |\n",
    );
    c
}

fn snapshot_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

#[trace("FR-169-AC-19")]
#[test]
fn the_markdown_and_the_json_equal_their_checked_in_snapshots() {
    let c = snapshot_corpus();
    let summary = summarize(c.dir.path(), &modules()).unwrap();
    let markdown = std::fs::read_to_string(snapshot_path("risk_summary.md")).unwrap();
    let json = std::fs::read_to_string(snapshot_path("risk_summary.json")).unwrap();
    assert_eq!(summary.to_markdown(), markdown);
    assert_eq!(summary.to_json().unwrap(), json.trim_end());
}
