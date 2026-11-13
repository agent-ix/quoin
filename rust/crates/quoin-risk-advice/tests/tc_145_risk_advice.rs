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
//! from `spec-objects-safety`. The record types come from that module, read in
//! place from `SPEC_OBJECTS_SAFETY_DIR` or the checkout next to this
//! repository, at or after the commit that declares `risk_estimate` and
//! `residual_risk` (PLAT-1563, PLAT-1565).

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_quire::{ModuleRoot, ModuleSelection};
use quoin_risk_advice::{ErrorCode, advise};
use serde_json::Value;

mod common;

use common::*;

impl Corpus {
    fn json(&self) -> Value {
        let advice = advise(self.dir.path(), &modules()).unwrap();
        serde_json::from_str(&advice.to_json().unwrap()).unwrap()
    }

    fn markdown(&self) -> String {
        advise(self.dir.path(), &modules()).unwrap().to_markdown()
    }
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
    residual(c, "RR-1", "EST-B", "POL-1", "undecided");
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

fn ids(entries: &Value, key: &str) -> Vec<String> {
    entries
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry[key].as_str().unwrap().to_owned())
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
    let a = advise(c.dir.path(), &modules()).unwrap();
    let b = advise(c.dir.path(), &modules()).unwrap();
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

#[trace("FR-145-AC-7")]
#[test]
fn a_malformed_decision_record_stops_the_report_and_names_the_record() {
    let c = corpus();
    chain(&c);
    let broken = [
        (
            "RR-1",
            "residual_risk",
            rels(&[("EST-B", "estimate"), ("POL-1", "references")]),
            "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| maybe | r | |\n",
        ),
        (
            "BR-1",
            "benefit_risk",
            rels(&[("RR-1", "references")]),
            "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| undecided | r | |\n",
        ),
        (
            "ORE-1",
            "overall_residual_risk_evaluation",
            rels(&[("RR-1", "references")]),
            "## Determination\n\n| Product | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| p | undecided | <why> | |\n",
        ),
    ];
    for (id, kind, front, body) in broken {
        c.put(id, kind, &front, body);
        let error = advise(c.dir.path(), &modules()).err().unwrap();
        assert_eq!(error.code, ErrorCode::Record, "{id}");
        assert!(error.message.contains(id), "{}", error.message);
        std::fs::remove_file(c.dir.path().join(format!("{id}.md"))).ok();
        if id == "RR-1" {
            residual(&c, "RR-1", "EST-B", "POL-1", "undecided");
        }
    }
    let ok = c.json();
    assert_eq!(ids(&ok["decision_queue"]["queue"], "record"), ["RR-1"]);
}

#[trace("FR-145-AC-7")]
#[test]
fn a_policy_with_a_placeholder_level_stops_the_report() {
    let c = corpus();
    chain(&c);
    let text = std::fs::read_to_string(c.dir.path().join("POL-1.md")).unwrap();
    std::fs::write(
        c.dir.path().join("POL-1.md"),
        text.replace("| Minor |\n| Serious |", "| <level> |\n| Serious |"),
    )
    .unwrap();
    let error = advise(c.dir.path(), &modules()).err().unwrap();
    assert_eq!(error.code, ErrorCode::Record);
    assert!(error.message.contains("POL-1"), "{}", error.message);
}

#[trace("FR-145-AC-7")]
#[test]
fn an_estimate_that_breaks_its_contract_is_skipped() {
    let c = corpus();
    chain(&c);
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
    let json = c.json();
    assert_eq!(ids(&json["policy_cells"], "estimate"), ["EST-A", "EST-B"]);
    assert_eq!(ids(&json["evidence_gaps"], "estimate"), ["EST-B"]);
    assert!(
        json["decision_queue"]["no_record"]
            .as_array()
            .unwrap()
            .is_empty()
    );
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
    // Two pre-control and two post-control estimates, interleaved by id.
    estimate(
        &c,
        "EST-C",
        "pre_control",
        ("Serious", "Rare"),
        &[("SIT-1", "estimates")],
    );
    estimate(
        &c,
        "EST-D",
        "post_control",
        ("Minor", "Likely"),
        &[("SIT-1", "estimates")],
    );
    let json = c.json();
    let entries = json["policy_cells"].as_array().unwrap();
    assert_eq!(
        ids(&json["policy_cells"], "estimate"),
        ["EST-A", "EST-B", "EST-C", "EST-D"]
    );
    let phases: Vec<&str> = entries
        .iter()
        .map(|e| e["phase"].as_str().unwrap())
        .collect();
    assert_eq!(
        phases,
        ["pre_control", "post_control", "pre_control", "post_control"]
    );
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
    assert_eq!(cell(&json, "EST-C")["region"], "Mid");
    let md = c.markdown();
    assert!(md.contains("the policy marks this cell as High. The policy flags this region as outside its acceptable region."));
    assert!(md.contains("the policy marks this cell as Low.\n"));
    assert!(md.contains("the policy marks this cell as Mid.\n"));
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

#[trace("FR-146-AC-4", "FR-146-AC-5")]
#[test]
fn an_undeclared_level_or_a_marker_gives_a_finding_and_no_region() {
    let c = corpus();
    chain(&c);
    estimate(
        &c,
        "EST-B",
        "post_control",
        ("Catastrophic", "Rare"),
        &[("SIT-1", "estimates")],
    );
    let json = c.json();
    let entry = cell(&json, "EST-B");
    assert_eq!(tokens(entry), ["level-not-in-policy"]);
    assert_eq!(
        entry["findings"][0]["names"],
        serde_json::json!(["Catastrophic", "POL-1"])
    );
    assert_eq!(entry["region"], Value::Null);
    for marker in ["unknown", "not_assessed", "not_applicable"] {
        estimate(
            &c,
            "EST-B",
            "post_control",
            ("Minor", marker),
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

#[trace("FR-146-AC-11")]
#[test]
fn a_pair_the_policy_lists_no_region_for_is_a_cell_finding_not_a_level_finding() {
    let c = corpus();
    chain(&c);
    let text = std::fs::read_to_string(c.dir.path().join("POL-1.md")).unwrap();
    std::fs::write(
        c.dir.path().join("POL-1.md"),
        text.replace("| Minor | Rare | Low |\n", ""),
    )
    .unwrap();
    let json = c.json();
    let entry = cell(&json, "EST-B");
    assert_eq!(tokens(entry), ["cell-not-in-policy"]);
    assert_eq!(
        entry["findings"][0]["names"],
        serde_json::json!(["Minor", "Rare", "POL-1"])
    );
    assert_eq!(entry["region"], Value::Null);
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
        ("Minor", "Rare"),
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

#[trace("FR-147-AC-10")]
#[test]
fn entries_with_different_gap_counts_are_ordered_by_estimate_id() {
    let c = corpus();
    policy(&c, "POL-1");
    c.put("HAZ-1", "hazard", "", "");
    c.put(
        "SIT-1",
        "hazardous_situation",
        &rels(&[("HAZ-1", "arises_from")]),
        "",
    );
    c.put("CTRL-1", "control", "", "");
    c.put("CTRL-2", "control", "", "");
    c.put(
        "FR-9",
        "FR",
        &rels(&[("CTRL-1", "satisfies"), ("CTRL-2", "satisfies")]),
        "",
    );
    c.put(
        "TC-9",
        "TC",
        &rels(&[("CTRL-1", "verifies"), ("CTRL-2", "verifies")]),
        "",
    );
    // EST-A: no control and no situation (two gaps). EST-B: a fully linked control
    // (no gap). EST-C: a control with no requirement and no verification, plus
    // another with no verification (three gaps).
    estimate(&c, "EST-A", "post_control", ("Minor", "Rare"), &[]);
    estimate(
        &c,
        "EST-B",
        "post_control",
        ("Minor", "Rare"),
        &[("SIT-1", "estimates"), ("CTRL-1", "assumes")],
    );
    c.put("CTRL-3", "control", "", "");
    c.put("CTRL-4", "control", "", "");
    c.put("FR-8", "FR", &rels(&[("CTRL-4", "satisfies")]), "");
    estimate(
        &c,
        "EST-C",
        "post_control",
        ("Minor", "Rare"),
        &[
            ("SIT-1", "estimates"),
            ("CTRL-3", "assumes"),
            ("CTRL-4", "assumes"),
        ],
    );
    let json = c.json();
    assert_eq!(
        ids(&json["evidence_gaps"], "estimate"),
        ["EST-A", "EST-B", "EST-C"]
    );
    let count = |i: usize| {
        let entry = &json["evidence_gaps"][i];
        tokens(entry).len()
            + entry["controls"]
                .as_array()
                .unwrap()
                .iter()
                .map(|control| tokens(control).len())
                .sum::<usize>()
    };
    assert_eq!((count(0), count(1), count(2)), (2, 0, 3));
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
        ("Minor", "Rare"),
        &[("SIT-1", "estimates")],
    );
    estimate(&c, "EST-2", "post_control", ("Minor", "Rare"), &[]);
    estimate(&c, "EST-3", "pre_control", ("Minor", "Rare"), &[]);
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

#[trace("FR-147-AC-7")]
#[test]
fn several_situations_give_the_first_by_id_the_union_of_hazards_and_a_finding_per_bare_situation() {
    let c = corpus();
    chain(&c);
    c.put("HAZ-2", "hazard", "", "");
    c.put(
        "SIT-2",
        "hazardous_situation",
        &rels(&[("HAZ-2", "arises_from")]),
        "",
    );
    c.put("SIT-3", "hazardous_situation", "", "");
    estimate(
        &c,
        "EST-B",
        "post_control",
        ("Minor", "Rare"),
        &[
            ("SIT-3", "estimates"),
            ("SIT-2", "estimates"),
            ("SIT-1", "estimates"),
            ("CTRL-1", "assumes"),
        ],
    );
    let json = c.json();
    let entry = gaps(&json);
    assert_eq!(entry["situation"], "SIT-1");
    assert_eq!(entry["hazards"], serde_json::json!(["HAZ-1", "HAZ-2"]));
    assert_eq!(entry["findings"].as_array().unwrap().len(), 1);
    assert_eq!(entry["findings"][0]["finding"], "situation-without-hazard");
    assert_eq!(entry["findings"][0]["names"], serde_json::json!(["SIT-3"]));
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
        let by = if decision == "undecided" { "" } else { "Pat" };
        c.put(
            id,
            kind,
            &rels(&[("RR-1", "references"), ("RR-0", "references")]),
            &format!("## Determination\n\n| {head} | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| b | {decision} | r | {by} |\n"),
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

#[trace("FR-148-AC-3")]
#[test]
fn the_region_list_of_the_policy_does_not_order_the_queue() {
    let c = corpus();
    policy(&c, "POL-1");
    c.put("SIT-1", "hazardous_situation", "", "");
    // Region order Low, Mid, High is the reverse of the order the ids put the
    // residual risks in: RR-A lands in High, RR-B in Mid, RR-C in Low.
    estimate(&c, "EST-A", "post_control", ("Serious", "Likely"), &[]);
    estimate(&c, "EST-B", "post_control", ("Serious", "Rare"), &[]);
    estimate(&c, "EST-C", "post_control", ("Minor", "Rare"), &[]);
    for (rr, est) in [("RR-A", "EST-A"), ("RR-B", "EST-B"), ("RR-C", "EST-C")] {
        residual(&c, rr, est, "POL-1", "undecided");
    }
    let before = c.json();
    let regions = |json: &Value| -> Vec<String> {
        json["decision_queue"]["queue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["region"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(
        ids(&before["decision_queue"]["queue"], "record"),
        ["RR-A", "RR-B", "RR-C"]
    );
    assert_eq!(regions(&before), ["High", "Mid", "Low"]);
    policy_with(
        &c,
        "POL-1",
        true,
        &[("High", true), ("Mid", false), ("Low", false)],
    );
    let after = c.json();
    assert_eq!(after["decision_queue"], before["decision_queue"]);
}

#[trace("FR-148-AC-5")]
#[test]
fn two_residual_risks_on_different_policies_each_show_their_own_region() {
    let c = corpus();
    policy_with(&c, "POL-1", true, &REGIONS);
    // The same cell is flagged outside the acceptable region in POL-1 only.
    policy_with(&c, "POL-2", false, &REGIONS);
    estimate(&c, "EST-A", "post_control", ("Serious", "Likely"), &[]);
    estimate(&c, "EST-B", "post_control", ("Serious", "Likely"), &[]);
    residual(&c, "RR-A", "EST-A", "POL-1", "undecided");
    residual(&c, "RR-B", "EST-B", "POL-2", "undecided");
    let json = c.json();
    let queue = json["decision_queue"]["queue"].as_array().unwrap();
    assert_eq!(
        (
            queue[0]["policy"].as_str(),
            queue[0]["outside_acceptable"].as_bool()
        ),
        (Some("POL-1"), Some(true))
    );
    assert_eq!(
        (
            queue[1]["policy"].as_str(),
            queue[1]["outside_acceptable"].as_bool()
        ),
        (Some("POL-2"), Some(false))
    );
    assert_eq!(queue[0]["region"], "High");
    assert_eq!(queue[1]["region"], "High");
}

#[trace("FR-148-AC-4")]
#[test]
fn a_residual_risk_with_a_marker_level_carries_the_marker_finding_and_no_region() {
    let c = corpus();
    chain(&c);
    estimate(
        &c,
        "EST-B",
        "post_control",
        ("Minor", "not_assessed"),
        &[("SIT-1", "estimates")],
    );
    let json = c.json();
    let entry = &json["decision_queue"]["queue"][0];
    assert_eq!(tokens(entry), ["level-not-assessed"]);
    assert_eq!(entry["region"], Value::Null);
    assert_eq!(entry["probability"], "not_assessed");
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
        ("Minor", "Rare"),
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

#[trace("NFR-031-AC-4")]
#[test]
fn with_every_optional_input_absent_each_value_is_null_a_marker_a_token_or_copied() {
    let c = corpus();
    // One post-control estimate and nothing else: no situation, control,
    // policy, residual risk, requirement or anomaly.
    estimate(&c, "EST-1", "post_control", ("Minor", "unknown"), &[]);
    let json = c.json();
    let entry = cell(&json, "EST-1");
    for key in ["policy", "cell", "region", "outside_acceptable"] {
        assert_eq!(entry[key], Value::Null, "{key}");
    }
    assert_eq!(entry["probability"], "unknown");
    assert_eq!(tokens(entry), ["policy-not-found"]);
    let gap = gaps(&json);
    assert_eq!(gap["situation"], Value::Null);
    for key in ["hazards", "controls", "anomalies"] {
        assert_eq!(gap[key], serde_json::json!([]), "{key}");
    }
    assert_eq!(
        tokens(gap),
        ["post-control-without-control", "estimate-without-situation"]
    );
    assert_eq!(json["decision_queue"]["queue"], serde_json::json!([]));
    assert_eq!(
        tokens(&json["decision_queue"]["no_record"][0]),
        ["post-control-estimate-without-decision"]
    );
}

#[trace("NFR-031-AC-2", "FR-148-AC-9")]
#[test]
fn the_markdown_never_says_the_tool_accepts() {
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

#[trace("NFR-031-AC-5")]
#[test]
fn an_all_acceptable_policy_and_an_undecided_risk_state_nothing_accepted() {
    let c = corpus();
    policy_with(&c, "POL-1", false, &REGIONS);
    estimate(&c, "EST-1", "post_control", ("Serious", "Likely"), &[]);
    residual(&c, "RR-1", "EST-1", "POL-1", "undecided");
    let json = c.json();
    let entry = &json["decision_queue"]["queue"][0];
    assert_eq!(
        (
            entry["region"].as_str(),
            entry["outside_acceptable"].as_bool()
        ),
        (Some("High"), Some(false))
    );
    let md = c.markdown();
    assert!(md.contains("waiting for a decision by a named person"));
    for forbidden in [
        "accepted",
        "is acceptable",
        "acceptable risk",
        "outside its acceptable region",
    ] {
        assert!(!md.contains(forbidden), "{forbidden} in {md}");
    }
}

fn without_the_safety_types() -> ModuleSelection {
    let checkout = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    ModuleSelection::Closed(vec![
        ModuleRoot::open(checkout.join("spec_artifacts_process")).unwrap(),
    ])
}

#[trace("FR-145-AC-7")]
#[test]
fn a_type_no_loaded_module_declares_stops_the_report_naming_the_flag_and_the_types() {
    let c = corpus();
    chain(&c);
    let error = advise(c.dir.path(), &without_the_safety_types())
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::TypeUndeclared);
    for needle in [
        "--module DIR",
        "residual_risk",
        "benefit_risk",
        "acceptability_policy",
        "unknown type",
    ] {
        assert!(
            error.message.contains(needle),
            "{needle} in {}",
            error.message
        );
    }
}

#[trace("FR-145-AC-7")]
#[test]
fn a_link_in_a_table_cell_is_no_edge_and_both_sections_name_one_policy() {
    let c = corpus();
    chain(&c);
    c.put(
        "RR-1",
        "residual_risk",
        &rels(&[("EST-B", "estimate"), ("POL-1", "references")]),
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n\
         | undecided | see [the hazard](ix://agent-ix/demo/HAZ-1) | |\n",
    );
    c.put(
        "BR-1",
        "benefit_risk",
        &rels(&[("RR-1", "references")]),
        "## Determination\n\n| Benefit | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n\
         | meets [FR-9](ix://agent-ix/demo/FR-9) | undecided | r | |\n",
    );
    let json = c.json();
    assert_eq!(cell(&json, "EST-B")["policy"], "POL-1");
    assert_eq!(tokens(cell(&json, "EST-B")), Vec::<String>::new());
    let queue = json["decision_queue"]["queue"].as_array().unwrap();
    assert_eq!(queue[0]["record"], "RR-1");
    assert_eq!(queue[0]["policy"], "POL-1");
    assert_eq!(queue[0]["region"], "Low");
    assert_eq!(queue[1]["record"], "BR-1");
    assert_eq!(queue[1]["references"], serde_json::json!(["RR-1"]));
}

#[trace("FR-146-AC-6", "FR-148-AC-11")]
#[test]
fn one_residual_risk_naming_two_policies_is_ambiguous_in_both_sections() {
    let c = corpus();
    chain(&c);
    policy_with(&c, "POL-0", false, &REGIONS);
    c.put(
        "RR-1",
        "residual_risk",
        &rels(&[("EST-B", "estimate"), ("POL-1", "references"), ("POL-0", "references")]),
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| undecided | r | |\n",
    );
    let json = c.json();
    let entry = cell(&json, "EST-B");
    assert_eq!(tokens(entry), ["policy-ambiguous"]);
    assert_eq!(
        entry["findings"][0]["names"],
        serde_json::json!(["POL-0", "POL-1"])
    );
    assert_eq!(
        (&entry["policy"], &entry["region"]),
        (&Value::Null, &Value::Null)
    );
    let queued = &json["decision_queue"]["queue"][0];
    assert_eq!(tokens(queued), ["policy-ambiguous"]);
    assert_eq!(
        queued["findings"][0]["names"],
        serde_json::json!(["POL-0", "POL-1"])
    );
    assert_eq!(
        (
            &queued["policy"],
            &queued["region"],
            &queued["outside_acceptable"]
        ),
        (&Value::Null, &Value::Null, &Value::Null)
    );
}

#[trace("FR-145-AC-7")]
#[test]
fn a_file_the_reader_cannot_load_is_listed_not_dropped() {
    let c = corpus();
    chain(&c);
    std::fs::write(c.dir.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    let advice = advise(c.dir.path(), &modules()).unwrap();
    assert_eq!(advice.unreadable.len(), 1);
    assert!(
        advice.unreadable[0].contains("RR-9.md"),
        "{:?}",
        advice.unreadable
    );
}
