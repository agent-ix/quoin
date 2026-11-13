// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The risk advice report over authored fixture records (FR-145 to FR-148,
//! NFR-031). Fixtures are written into a throwaway directory; nothing is copied
//! from `spec-objects-safety`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_risk_advice::advise;
use serde_json::Value;

struct Corpus {
    dir: tempfile::TempDir,
}

impl Corpus {
    fn put(&self, id: &str, kind: &str, front: &str, body: &str) {
        let text =
            format!("---\nid: {id}\ntitle: \"{id}\"\ntype: {kind}\n{front}---\n# [{id}]\n\n{body}");
        std::fs::write(self.dir.path().join(format!("{id}.md")), text).unwrap();
    }

    fn json(&self) -> Value {
        serde_json::from_str(&advise(self.dir.path()).unwrap().to_json().unwrap()).unwrap()
    }

    fn markdown(&self) -> String {
        advise(self.dir.path()).unwrap().to_markdown()
    }
}

fn rels(edges: &[(&str, &str)]) -> String {
    let mut out = String::from("relationships:\n");
    for (target, verb) in edges {
        let _ = writeln!(out, "  - target: {target}\n    type: {verb}");
    }
    out
}

fn policy(c: &Corpus, id: &str) {
    c.put(
        id,
        "acceptability_policy",
        "",
        "## Severity levels\n\n| Severity level |\n| --- |\n| Minor |\n| Serious |\n\n\
         ## Probability levels\n\n| Probability level |\n| --- |\n| Rare |\n| Likely |\n\n\
         ## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| Low | false |\n| High | true |\n\n\
         ## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n\
         | Minor | Rare | Low |\n| Minor | Likely | Low |\n| Serious | Rare | Low |\n| Serious | Likely | High |\n",
    );
}

fn estimate(
    c: &Corpus,
    id: &str,
    phase: &str,
    severity: &str,
    probability: &str,
    edges: &[(&str, &str)],
) {
    c.put(
        id,
        "risk_estimate",
        &format!("phase: {phase}\n{}", rels(edges)),
        &format!("## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| {severity} | {probability} | why |\n"),
    );
}

fn residual(c: &Corpus, id: &str, est: &str, pol: &str, decision: &str) {
    let by = if decision == "undecided" { "" } else { "Pat" };
    c.put(
        id,
        "residual_risk",
        &rels(&[(est, "estimate"), (pol, "references")]),
        &format!("## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| {decision} | r | {by} |\n"),
    );
}

fn chain(c: &Corpus) {
    policy(c, "POL-1");
    c.put("HAZ-1", "hazard", "status: accepted\n", "");
    c.put(
        "SIT-1",
        "hazardous_situation",
        &rels(&[("HAZ-1", "arises_from")]),
        "",
    );
    c.put("CTRL-1", "control", "", "");
    estimate(
        c,
        "EST-A",
        "pre_control",
        "Serious",
        "Likely",
        &[("SIT-1", "estimates")],
    );
    estimate(
        c,
        "EST-B",
        "post_control",
        "Minor",
        "Rare",
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    residual(c, "RR-1", "EST-B", "POL-1", "undecided");
}

fn corpus() -> Corpus {
    Corpus {
        dir: tempfile::tempdir().unwrap(),
    }
}

fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(&path).unwrap().flatten() {
            if entry.file_type().unwrap().is_dir() {
                stack.push(entry.path());
            } else {
                out.insert(entry.path(), std::fs::read(entry.path()).unwrap());
            }
        }
    }
    out
}

fn cell<'a>(json: &'a Value, estimate: &str) -> &'a Value {
    json["policy_cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["estimate"] == estimate)
        .unwrap()
}

fn tokens(entry: &Value) -> Vec<String> {
    entry["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["finding"].as_str().unwrap().to_owned())
        .collect()
}

#[trace("FR-145-AC-1", "FR-145-AC-6", "NFR-031-AC-3")]
#[test]
fn the_json_has_exactly_the_four_keys_and_no_chain_rows() {
    let c = corpus();
    chain(&c);
    let json = c.json();
    assert_eq!(
        keys(&json),
        [
            "attribution",
            "decision_queue",
            "evidence_gaps",
            "policy_cells"
        ]
    );
    let text = json.to_string();
    for forbidden in [
        "\"harms\"",
        "\"hazard\":",
        "\"situation\":null,\"hazard",
        "score",
        "rank",
        "verdict",
        "recommend",
        "proposal",
        "priority",
        "weight",
        "total",
    ] {
        assert!(!text.contains(forbidden), "{forbidden} in {text}");
    }
}

#[trace("FR-145-AC-2", "NFR-031-AC-1")]
#[test]
fn a_run_changes_no_file() {
    let c = corpus();
    chain(&c);
    let before = snapshot(c.dir.path());
    c.json();
    c.markdown();
    assert_eq!(snapshot(c.dir.path()), before);
}

#[trace("FR-145-AC-3", "NFR-031-AC-6")]
#[test]
fn two_runs_are_byte_identical() {
    let c = corpus();
    chain(&c);
    let a = advise(c.dir.path()).unwrap();
    let b = advise(c.dir.path()).unwrap();
    assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    assert_eq!(a.to_markdown(), b.to_markdown());
}

#[trace("FR-145-AC-4", "FR-145-AC-5")]
#[test]
fn an_empty_scope_prints_three_empty_sections_and_the_attribution() {
    let c = corpus();
    let json = c.json();
    assert_eq!(json["policy_cells"], serde_json::json!([]));
    assert_eq!(json["evidence_gaps"], serde_json::json!([]));
    assert_eq!(
        json["decision_queue"],
        serde_json::json!({"queue": [], "no_record": []})
    );
    let attribution = json["attribution"].as_str().unwrap();
    assert!(attribution.contains("policy") && attribution.contains("named person"));
}

#[trace(
    "FR-146-AC-1",
    "FR-146-AC-2",
    "FR-146-AC-3",
    "FR-146-AC-8",
    "FR-146-AC-10"
)]
#[test]
fn each_estimate_shows_the_region_its_policy_cell_names() {
    let c = corpus();
    chain(&c);
    let json = c.json();
    let entries = json["policy_cells"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["estimate"], "EST-A");
    assert_eq!(entries[1]["estimate"], "EST-B");
    for entry in entries {
        assert_eq!(
            keys(entry),
            [
                "cell",
                "estimate",
                "findings",
                "outside_acceptable",
                "phase",
                "policy",
                "probability",
                "region",
                "severity"
            ]
        );
    }
    assert_eq!(cell(&json, "EST-A")["region"], "High");
    assert_eq!(cell(&json, "EST-A")["outside_acceptable"], true);
    assert_eq!(cell(&json, "EST-B")["region"], "Low");
    assert_eq!(cell(&json, "EST-B")["outside_acceptable"], false);
    let md = c.markdown();
    assert!(md.contains("the policy marks this cell as High. The policy flags this region as outside its acceptable region."));
    assert!(md.contains("the policy marks this cell as Low."));
    assert!(!md.contains("the policy marks this cell as Low. The policy flags"));
}

#[trace("FR-146-AC-7", "FR-146-AC-9")]
#[test]
fn a_pre_control_estimate_takes_the_policy_of_its_situation_and_is_not_compared() {
    let c = corpus();
    chain(&c);
    let json = c.json();
    assert_eq!(cell(&json, "EST-A")["policy"], "POL-1");
    assert_eq!(
        cell(&json, "EST-A")["cell"],
        serde_json::json!({"severity": "Serious", "probability": "Likely"})
    );
    assert_eq!(
        cell(&json, "EST-B")["cell"],
        serde_json::json!({"severity": "Minor", "probability": "Rare"})
    );
}

#[trace("FR-146-AC-4", "FR-146-AC-5", "NFR-031-AC-4")]
#[test]
fn an_undeclared_level_or_a_marker_gives_a_finding_and_no_region() {
    let c = corpus();
    chain(&c);
    estimate(
        &c,
        "EST-B",
        "post_control",
        "Catastrophic",
        "Rare",
        &[("SIT-1", "estimates")],
    );
    let json = c.json();
    let entry = cell(&json, "EST-B");
    assert_eq!(tokens(entry), ["level-not-in-policy"]);
    assert_eq!(entry["findings"][0]["names"][0], "Catastrophic");
    assert_eq!(entry["region"], Value::Null);
    for marker in ["unknown", "not_assessed", "not_applicable"] {
        estimate(
            &c,
            "EST-B",
            "post_control",
            "Minor",
            marker,
            &[("SIT-1", "estimates")],
        );
        let json = c.json();
        let entry = cell(&json, "EST-B");
        assert_eq!(tokens(entry), ["level-not-assessed"]);
        assert_eq!(entry["findings"][0]["names"][0], marker);
        assert_eq!(entry["region"], Value::Null);
        assert_eq!(entry["probability"], marker);
    }
}

#[trace("FR-146-AC-6")]
#[test]
fn no_policy_and_two_policies_show_no_region() {
    let c = corpus();
    chain(&c);
    estimate(
        &c,
        "EST-C",
        "post_control",
        "Minor",
        "Rare",
        &[("SIT-1", "estimates")],
    );
    let json = c.json();
    assert_eq!(tokens(cell(&json, "EST-C")), ["policy-not-found"]);
    policy(&c, "POL-2");
    residual(&c, "RR-2", "EST-B", "POL-2", "undecided");
    let json = c.json();
    let entry = cell(&json, "EST-B");
    assert_eq!(tokens(entry), ["policy-ambiguous"]);
    assert_eq!(
        entry["findings"][0]["names"],
        serde_json::json!(["POL-1", "POL-2"])
    );
    assert_eq!(entry["region"], Value::Null);
}

fn requirement(c: &Corpus) {
    c.put(
        "FR-9",
        "FR",
        &rels(&[("CTRL-1", "satisfies")]),
        "| ID | Criteria |\n|---|---|\n| FR-9-AC-1 | x |\n",
    );
}

fn anomaly(c: &Corpus, id: &str, status: &str, target: &str, kind: &str) {
    c.put(
        id,
        "Anomaly",
        &format!("status: {status}\nsafety_impact: yes\n"),
        &format!("## Safety Impact Assessment\n\n| Kind | Target | Impact | Rationale |\n| --- | --- | --- | --- |\n| {kind} | {target} | Minor | because |\n"),
    );
}

fn gaps(json: &Value) -> &Value {
    &json["evidence_gaps"][0]
}

#[trace(
    "FR-147-AC-1",
    "FR-147-AC-2",
    "FR-147-AC-3",
    "FR-147-AC-4",
    "FR-147-AC-10",
    "FR-147-AC-11"
)]
#[test]
fn control_links_are_tested_by_their_edges_alone() {
    let c = corpus();
    chain(&c);
    let json = c.json();
    assert_eq!(json["evidence_gaps"].as_array().unwrap().len(), 1);
    assert_eq!(
        tokens(&gaps(&json)["controls"][0]),
        [
            "control-without-requirement",
            "control-without-verification"
        ]
    );
    requirement(&c);
    assert_eq!(
        tokens(&gaps(&c.json())["controls"][0]),
        ["control-without-verification"]
    );
    c.put(
        "TC-9",
        "TC",
        &rels(&[("CTRL-1", "verifies")]),
        "result: failed\n",
    );
    let json = c.json();
    assert_eq!(tokens(&gaps(&json)["controls"][0]), Vec::<String>::new());
    assert_eq!(
        gaps(&json)["controls"][0]["verifications"],
        serde_json::json!(["TC-9"])
    );
    assert_eq!(
        keys(&gaps(&json)["controls"][0]),
        ["control", "findings", "requirements", "verifications"]
    );
    assert_eq!(
        keys(gaps(&json)),
        [
            "anomalies",
            "controls",
            "estimate",
            "findings",
            "hazards",
            "situation"
        ]
    );
    assert_eq!(tokens(gaps(&json)), Vec::<String>::new());
    assert_eq!(gaps(&json)["hazards"], serde_json::json!(["HAZ-1"]));
    let md = c.markdown();
    assert!(
        !md.contains("sound")
            && !md.contains("ineffective")
            && !md.contains("unsafe")
            && !md.contains("fail")
    );
}

#[trace("FR-147-AC-9")]
#[test]
fn a_missing_verification_reads_no_verification_is_linked() {
    let c = corpus();
    chain(&c);
    let md = c.markdown();
    assert!(md.contains("no verification is linked"));
    assert!(!md.contains("ineffective") && !md.contains("unsafe"));
}

#[trace("FR-147-AC-5", "FR-147-AC-6")]
#[test]
fn only_open_anomalies_naming_the_hazard_or_a_requirement_are_listed() {
    let c = corpus();
    chain(&c);
    requirement(&c);
    anomaly(&c, "AN-1", "assessed", "HAZ-1", "Hazard");
    anomaly(&c, "AN-2", "closed", "HAZ-1", "Hazard");
    anomaly(&c, "AN-3", "identified", "FR-9", "Requirement");
    anomaly(&c, "AN-4", "assessed", "FR-9-AC-1", "Requirement");
    anomaly(&c, "AN-5", "assessed", "FR-77", "Requirement");
    let json = c.json();
    let listed: Vec<(&str, &str)> = gaps(&json)["anomalies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["anomaly"].as_str().unwrap(), a["names"].as_str().unwrap()))
        .collect();
    assert_eq!(
        listed,
        [("AN-1", "HAZ-1"), ("AN-3", "FR-9"), ("AN-4", "FR-9-AC-1")]
    );
    assert_eq!(
        keys(&gaps(&json)["anomalies"][0]),
        ["anomaly", "impact", "kind", "names"]
    );
}

#[trace("FR-147-AC-7", "FR-147-AC-8")]
#[test]
fn estimate_level_gaps_are_named_and_pre_control_estimates_have_no_entry() {
    let c = corpus();
    policy(&c, "POL-1");
    c.put("HAZ-1", "hazard", "", "");
    c.put("SIT-1", "hazardous_situation", "", "");
    estimate(
        &c,
        "EST-1",
        "post_control",
        "Minor",
        "Rare",
        &[("SIT-1", "estimates")],
    );
    estimate(&c, "EST-2", "post_control", "Minor", "Rare", &[]);
    estimate(&c, "EST-3", "pre_control", "Minor", "Rare", &[]);
    let json = c.json();
    let entries = json["evidence_gaps"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        tokens(&entries[0]),
        ["post-control-without-control", "situation-without-hazard"]
    );
    assert_eq!(
        tokens(&entries[1]),
        ["post-control-without-control", "estimate-without-situation"]
    );
}

fn queue(json: &Value) -> Vec<(&str, &str)> {
    json["decision_queue"]["queue"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["kind"].as_str().unwrap(), e["record"].as_str().unwrap()))
        .collect()
}

#[trace("FR-148-AC-1", "FR-148-AC-2", "FR-148-AC-7", "FR-148-AC-10")]
#[test]
fn only_undecided_records_are_queued_in_kind_then_id_order() {
    let c = corpus();
    chain(&c);
    residual(&c, "RR-0", "EST-B", "POL-1", "undecided");
    residual(&c, "RR-2", "EST-B", "POL-1", "accepted");
    residual(&c, "RR-3", "EST-B", "POL-1", "not_accepted");
    for (id, decision) in [
        ("ORE-1", "undecided"),
        ("BR-2", "undecided"),
        ("BR-1", "undecided"),
        ("BR-3", "accepted"),
    ] {
        let (kind, head) = if id.starts_with("BR") {
            ("benefit_risk", "Benefit")
        } else {
            ("overall_residual_risk_evaluation", "Product")
        };
        c.put(
            id,
            kind,
            &rels(&[("RR-1", "references"), ("RR-0", "references")]),
            &format!("## Determination\n\n| {head} | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| b | {decision} | r | |\n"),
        );
    }
    let json = c.json();
    assert_eq!(
        queue(&json),
        [
            ("residual_risk", "RR-0"),
            ("residual_risk", "RR-1"),
            ("benefit_risk", "BR-1"),
            ("benefit_risk", "BR-2"),
            ("overall_residual_risk_evaluation", "ORE-1"),
        ]
    );
    let entries = json["decision_queue"]["queue"].as_array().unwrap();
    assert_eq!(
        entries[2]["references"],
        serde_json::json!(["RR-0", "RR-1"])
    );
    assert_eq!(
        entries[4]["references"],
        serde_json::json!(["RR-0", "RR-1"])
    );
    assert_eq!(
        keys(&entries[0]),
        [
            "estimate",
            "findings",
            "kind",
            "outside_acceptable",
            "policy",
            "probability",
            "record",
            "references",
            "region",
            "severity"
        ]
    );
    let md = c.markdown();
    assert_eq!(
        md.matches("waiting for a decision by a named person")
            .count(),
        5
    );
    assert!(!md.contains("low priority"));
}

#[trace("FR-148-AC-3", "FR-148-AC-4", "FR-148-AC-5")]
#[test]
fn a_residual_risk_shows_its_own_policy_region_and_its_marker_finding() {
    let c = corpus();
    chain(&c);
    let json = c.json();
    let entry = &json["decision_queue"]["queue"][0];
    assert_eq!(entry["policy"], "POL-1");
    assert_eq!(entry["region"], "Low");
    assert_eq!(entry["outside_acceptable"], false);
    estimate(
        &c,
        "EST-B",
        "post_control",
        "Minor",
        "not_assessed",
        &[("SIT-1", "estimates")],
    );
    let json = c.json();
    let entry = &json["decision_queue"]["queue"][0];
    assert_eq!(tokens(entry), ["level-not-assessed"]);
    assert_eq!(entry["region"], Value::Null);
    estimate(
        &c,
        "EST-B",
        "post_control",
        "Serious",
        "Likely",
        &[("SIT-1", "estimates")],
    );
    let before = c.json()["decision_queue"].clone();
    let text = std::fs::read_to_string(c.dir.path().join("POL-1.md")).unwrap();
    std::fs::write(
        c.dir.path().join("POL-1.md"),
        text.replace(
            "| Low | false |\n| High | true |",
            "| High | true |\n| Low | false |",
        ),
    )
    .unwrap();
    assert_eq!(c.json()["decision_queue"], before);
    assert_eq!(before["queue"][0]["region"], "High");
}

#[trace("FR-148-AC-6", "FR-148-AC-8")]
#[test]
fn an_estimate_with_no_residual_risk_is_listed_apart() {
    let c = corpus();
    chain(&c);
    estimate(
        &c,
        "EST-C",
        "post_control",
        "Minor",
        "Rare",
        &[("SIT-1", "estimates")],
    );
    let json = c.json();
    let none = json["decision_queue"]["no_record"].as_array().unwrap();
    assert_eq!(none.len(), 1);
    assert_eq!(none[0]["estimate"], "EST-C");
    assert_eq!(keys(&none[0]), ["estimate", "findings"]);
    assert_eq!(tokens(&none[0]), ["post-control-estimate-without-decision"]);
    residual(&c, "RR-5", "EST-C", "POL-1", "undecided");
    let json = c.json();
    assert!(
        json["decision_queue"]["no_record"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(queue(&json).len(), 2);
}

#[trace("NFR-031-AC-2", "NFR-031-AC-5", "FR-148-AC-9")]
#[test]
fn the_markdown_never_says_the_tool_accepts_or_finds_a_risk_accepted() {
    let c = corpus();
    chain(&c);
    let md = c.markdown();
    assert!(md.contains("Regions come from the project's policy."));
    for line in md.lines() {
        if line.contains("accept") {
            assert!(
                line.contains("acceptable region") || line.contains("Regions come from"),
                "{line}"
            );
        }
        if line.starts_with("- EST-") {
            assert!(
                line.contains("the policy marks this cell as") || line.contains("no region"),
                "{line}"
            );
        }
    }
    assert!(!md.contains("accepted"));
}
