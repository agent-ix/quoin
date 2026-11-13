// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The discrepancy report over authored fixture records (FR-154, FR-155).

mod common;

use common::*;
use ix_trace_rs::trace;
use quoin_risk_advice::{advise, discrepancies};
use serde_json::{Value, json};

impl Corpus {
    fn report(&self) -> Value {
        let report = discrepancies(self.dir.path(), &modules()).unwrap();
        serde_json::from_str(&report.to_json().unwrap()).unwrap()
    }

    /// The Markdown, checked against the voice rules of FR-154-AC-7 on every
    /// fixture it is read from.
    fn markdown(&self) -> String {
        let md = discrepancies(self.dir.path(), &modules())
            .unwrap()
            .to_markdown();
        assert_no_acceptance_voice(&md);
        md
    }

    fn findings(&self) -> Vec<(String, Option<String>)> {
        self.report()["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|finding| {
                (
                    finding["finding"].as_str().unwrap().to_owned(),
                    finding["refs"]["residual_risk"].as_str().map(str::to_owned),
                )
            })
            .collect()
    }

    fn tokens(&self) -> Vec<String> {
        self.findings()
            .into_iter()
            .map(|(token, _)| token)
            .collect()
    }
}

/// The stem "accept" appears only in the attribution, in a finding token and in
/// a decision value copied from a record; no risk is called compliant or passing.
fn assert_no_acceptance_voice(md: &str) {
    for line in md.lines().filter(|line| line.contains("accept")) {
        if let Some(entry) = line.strip_prefix("- ") {
            let (token, rest) = entry.split_once(':').unwrap();
            assert!(!token.contains(' '), "{line}");
            let copied = rest
                .split(", ")
                .find_map(|part| part.strip_prefix("recorded decision "));
            let remainder = rest.replace(copied.unwrap_or("\u{0}"), "");
            assert!(!remainder.contains("accept"), "{line}");
        } else {
            assert!(line.contains("Decisions come from"), "{line}");
        }
    }
    for forbidden in ["compliant", "passing", "is acceptable", "acceptable risk"] {
        assert!(!md.contains(forbidden), "{forbidden} in {md}");
    }
}

/// A benefit-risk record covering `RR-1` with the given decision.
fn benefit(c: &Corpus, id: &str, decision: &str) {
    let by = if decision == "undecided" { "" } else { "Pat" };
    c.put(
        id,
        "benefit_risk",
        &rels(&[("RR-1", "references")]),
        &format!(
            "## Determination\n\n| Benefit | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| b | {decision} | r | {by} |\n"
        ),
    );
}

/// `RR-1` on `EST-1` (Serious/Likely, region High, outside acceptable).
fn outside(decision: &str) -> Corpus {
    let c = corpus();
    policy(&c, "POL-1");
    estimate(&c, "EST-1", "post_control", ("Serious", "Likely"), &[]);
    residual(&c, "RR-1", "EST-1", "POL-1", decision);
    c
}

#[trace("FR-155-AC-1", "FR-155-AC-2", "FR-155-AC-3", "FR-155-AC-4")]
#[test]
fn an_accepted_risk_outside_the_region_is_reported_until_a_covering_record_is_accepted() {
    let c = outside("accepted");
    let report = c.report();
    assert_eq!(report["findings"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["findings"][0],
        json!({
            "finding": "accepted-outside-acceptable-region",
            "refs": {
                "residual_risk": "RR-1",
                "estimate": "EST-1",
                "decision": "accepted",
                "region": "High",
                "policy": "POL-1",
                "benefit_risk": [],
                "region_missing": null
            }
        })
    );

    benefit(&c, "BR-1", "accepted");
    assert!(c.findings().is_empty());

    benefit(&c, "BR-1", "undecided");
    assert_eq!(c.tokens(), ["benefit-risk-undecided"]);
    c.markdown();
    assert_eq!(
        c.report()["findings"][0]["refs"]["benefit_risk"],
        json!(["BR-1"])
    );

    benefit(&c, "BR-1", "not_accepted");
    assert_eq!(c.tokens(), ["benefit-risk-not-accepted"]);
    c.markdown();

    benefit(&c, "BR-2", "accepted");
    assert!(c.findings().is_empty());
}

#[trace("FR-155-AC-5", "FR-155-AC-6", "FR-155-AC-7")]
#[test]
fn the_other_decisions_are_compared_with_the_flag_of_their_region() {
    // Outside the acceptable region: not_accepted agrees; accepted needs cover.
    assert!(outside("not_accepted").findings().is_empty());

    // Inside it: not_accepted disagrees, accepted agrees.
    let c = corpus();
    policy(&c, "POL-1");
    estimate(&c, "EST-1", "post_control", ("Minor", "Rare"), &[]);
    residual(&c, "RR-1", "EST-1", "POL-1", "not_accepted");
    assert_eq!(c.tokens(), ["not-accepted-inside-acceptable-region"]);
    c.markdown();
    residual(&c, "RR-1", "EST-1", "POL-1", "accepted");
    assert!(c.findings().is_empty());

    // Undecided is reported whatever the flag, and nothing is said accepted.
    for levels in [("Minor", "Rare"), ("Serious", "Likely")] {
        let c = corpus();
        policy_with(&c, "POL-1", false, &REGIONS);
        estimate(&c, "EST-1", "post_control", levels, &[]);
        residual(&c, "RR-1", "EST-1", "POL-1", "undecided");
        assert_eq!(c.tokens(), ["residual-risk-undecided"]);
        let md = c.markdown();
        for forbidden in ["is acceptable", "acceptable risk", "outside its acceptable"] {
            assert!(!md.contains(forbidden), "{forbidden} in {md}");
        }
    }
}

#[trace("FR-155-AC-8")]
#[test]
fn a_level_the_policy_cannot_place_gives_a_decision_without_region() {
    for decision in ["accepted", "not_accepted"] {
        let c = corpus();
        policy(&c, "POL-1");
        estimate(
            &c,
            "EST-1",
            "post_control",
            ("Serious", "not_assessed"),
            &[],
        );
        estimate(&c, "EST-2", "post_control", ("Catastrophic", "Rare"), &[]);
        residual(&c, "RR-1", "EST-1", "POL-1", decision);
        residual(&c, "RR-2", "EST-2", "POL-1", decision);
        c.markdown();
        let report = c.report();
        let findings = report["findings"].as_array().unwrap();
        assert_eq!(findings.len(), 2);
        for (finding, reason) in findings
            .iter()
            .zip(["level-not-assessed", "level-not-in-policy"])
        {
            assert_eq!(finding["finding"], "decision-without-region");
            assert_eq!(finding["refs"]["region"], Value::Null);
            assert_eq!(finding["refs"]["region_missing"], reason);
            assert_eq!(finding["refs"]["decision"], decision);
        }
    }
}

#[trace("FR-155-AC-9")]
#[test]
fn a_post_control_estimate_with_no_residual_risk_is_reported_until_one_references_it() {
    let c = corpus();
    policy(&c, "POL-1");
    estimate(&c, "EST-1", "post_control", ("Serious", "Likely"), &[]);
    estimate(&c, "EST-0", "pre_control", ("Serious", "Likely"), &[]);
    let report = c.report();
    assert_eq!(
        report["findings"],
        json!([{
            "finding": "post-control-estimate-without-decision",
            "refs": {
                "residual_risk": null,
                "estimate": "EST-1",
                "decision": null,
                "region": null,
                "policy": null,
                "benefit_risk": [],
                "region_missing": null
            }
        }])
    );
    assert!(c.markdown().contains("EST-1"));
    residual(&c, "RR-1", "EST-1", "POL-1", "undecided");
    assert_eq!(c.tokens(), ["residual-risk-undecided"]);
}

#[trace("FR-155-AC-10")]
#[test]
fn a_hazard_status_is_not_a_decision() {
    let c = corpus();
    policy(&c, "POL-1");
    c.put("HAZ-1", "hazard", "status: accepted\n", "");
    c.put(
        "SIT-1",
        "hazardous_situation",
        &rels(&[("HAZ-1", "arises_from")]),
        "",
    );
    estimate(
        &c,
        "EST-1",
        "post_control",
        ("Serious", "Likely"),
        &[("SIT-1", "estimates")],
    );
    residual(&c, "RR-1", "EST-1", "POL-1", "undecided");
    assert_eq!(c.tokens(), ["residual-risk-undecided"]);
    c.markdown();
}

#[trace("FR-155-AC-11", "FR-155-AC-13", "FR-154-AC-10")]
#[test]
fn findings_have_the_fixed_shape_in_order_and_each_risk_uses_its_own_policy() {
    let c = corpus();
    policy(&c, "POL-1");
    policy_with(&c, "POL-2", false, &REGIONS);
    estimate(&c, "EST-B", "post_control", ("Serious", "Likely"), &[]);
    estimate(&c, "EST-A", "post_control", ("Serious", "Likely"), &[]);
    estimate(&c, "EST-C", "post_control", ("Minor", "Rare"), &[]);
    residual(&c, "RR-2", "EST-B", "POL-2", "undecided");
    residual(&c, "RR-1", "EST-B", "POL-1", "accepted");
    residual(&c, "RR-3", "EST-A", "POL-1", "undecided");
    c.markdown();
    let report = c.report();
    let findings = report["findings"].as_array().unwrap();
    let order: Vec<(&str, &str)> = findings
        .iter()
        .map(|f| {
            (
                f["refs"]["estimate"].as_str().unwrap(),
                f["refs"]["residual_risk"].as_str().unwrap_or("-"),
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            ("EST-A", "RR-3"),
            ("EST-B", "RR-1"),
            ("EST-B", "RR-2"),
            ("EST-C", "-")
        ]
    );
    for finding in findings {
        assert_eq!(keys(finding), ["finding", "refs"]);
        assert_eq!(
            keys(&finding["refs"]),
            [
                "benefit_risk",
                "decision",
                "estimate",
                "policy",
                "region",
                "region_missing",
                "residual_risk"
            ]
        );
    }
    let own = |risk: &str| {
        let finding = findings
            .iter()
            .find(|f| f["refs"]["residual_risk"] == risk)
            .unwrap();
        (
            finding["refs"]["policy"].clone(),
            finding["refs"]["region"].clone(),
        )
    };
    assert_eq!(own("RR-1"), (json!("POL-1"), json!("High")));
    assert_eq!(own("RR-2"), (json!("POL-2"), json!("High")));

    // FR-146 shows no region for the shared estimate; this report still does.
    let advice = advise(c.dir.path(), &modules()).unwrap();
    let advice: Value = serde_json::from_str(&advice.to_json().unwrap()).unwrap();
    let shared = advice["policy_cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["estimate"] == "EST-B")
        .unwrap();
    assert_eq!(shared["region"], Value::Null);
    assert_eq!(shared["findings"][0]["finding"], "policy-ambiguous");
}

#[trace("FR-154-AC-8")]
#[test]
fn the_region_equals_the_one_risk_advise_shows_for_the_estimate() {
    let c = outside("accepted");
    let advice = advise(c.dir.path(), &modules()).unwrap();
    let advice: Value = serde_json::from_str(&advice.to_json().unwrap()).unwrap();
    let cell = &advice["policy_cells"][0];
    let report = c.report();
    assert_eq!(report["findings"][0]["refs"]["region"], cell["region"]);
    assert_eq!(report["findings"][0]["refs"]["policy"], cell["policy"]);
}

#[trace("FR-154-AC-1", "FR-154-AC-4")]
#[test]
fn the_json_has_the_two_keys_and_an_empty_scope_has_no_findings() {
    let report = outside("accepted").report();
    assert_eq!(keys(&report), ["attribution", "findings"]);
    let empty = corpus().report();
    assert_eq!(keys(&empty), ["attribution", "findings"]);
    assert_eq!(empty["findings"], json!([]));
}

#[trace("FR-154-AC-2", "FR-154-AC-3")]
#[test]
fn a_run_changes_no_file_and_two_runs_are_byte_identical() {
    let c = outside("accepted");
    let before = snapshot(c.dir.path());
    let first = (c.markdown(), c.report().to_string());
    let second = (c.markdown(), c.report().to_string());
    assert_eq!(first, second);
    assert_eq!(snapshot(c.dir.path()), before);
}

#[trace("FR-154-AC-6", "FR-154-AC-7")]
#[test]
fn the_markdown_names_the_policy_and_the_person_and_never_says_the_tool_accepts() {
    let c = outside("accepted");
    policy(&c, "POL-1");
    estimate(&c, "EST-2", "post_control", ("Minor", "Rare"), &[]);
    residual(&c, "RR-2", "EST-2", "POL-1", "accepted");
    let md = c.markdown();
    assert!(md.contains("Regions come from the project's policy."));
    assert!(md.contains("Decisions come from the named person"));
    assert!(md.contains("the policy marks this cell as High"));
    assert!(
        !md.contains("RR-2"),
        "a risk without a finding has an entry: {md}"
    );
    for line in md.lines() {
        if line.contains("accept") {
            assert!(
                line.starts_with("- accepted-outside-acceptable-region")
                    || line.contains("Decisions come from"),
                "{line}"
            );
        }
    }
    for forbidden in ["compliant", "passing", "is acceptable"] {
        assert!(!md.contains(forbidden), "{forbidden} in {md}");
    }
    let report = c.report().to_string();
    for key in [
        "score",
        "weight",
        "total",
        "rank",
        "verdict",
        "recommend",
        "proposal",
        "priority",
    ] {
        assert!(!report.contains(key), "{key} in {report}");
    }
}

#[trace("FR-154-AC-9")]
#[test]
fn a_malformed_residual_risk_is_an_error_naming_the_record() {
    let c = corpus();
    c.put(
        "RR-1",
        "residual_risk",
        "",
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| maybe | r | |\n",
    );
    let error = discrepancies(c.dir.path(), &modules()).unwrap_err();
    assert!(error.message.contains("RR-1"), "{}", error.message);
}

#[trace("FR-155-AC-14")]
#[test]
fn each_other_reason_no_region_resolves_is_named_in_region_missing() {
    let reasons = |c: &Corpus| -> Vec<Value> {
        c.report()["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["refs"]["region_missing"].clone())
            .collect()
    };
    for decision in ["accepted", "not_accepted"] {
        let c = corpus();
        policy(&c, "POL-1");
        policy(&c, "POL-2");
        estimate(&c, "EST-1", "post_control", ("Serious", "Likely"), &[]);
        estimate(&c, "EST-2", "post_control", ("Serious", "Likely"), &[]);
        residual(&c, "RR-1", "EST-1", "POL-MISSING", decision);
        c.put(
            "RR-2",
            "residual_risk",
            &rels(&[("EST-2", "estimate"), ("POL-1", "references"), ("POL-2", "references")]),
            &format!("## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| {decision} | r | Pat |\n"),
        );
        residual(&c, "RR-3", "EST-GONE", "POL-1", decision);
        c.markdown();
        assert_eq!(
            reasons(&c),
            [
                json!("policy-not-found"),
                json!("policy-ambiguous"),
                json!("estimate-not-found")
            ]
        );
        let report = c.report();
        assert_eq!(report["findings"][0]["refs"]["policy"], "POL-MISSING");
        assert_eq!(report["findings"][1]["refs"]["policy"], Value::Null);
        assert!(c.markdown().contains("policy-not-found"));
    }
    let c = corpus();
    policy(&c, "POL-1");
    let text = std::fs::read_to_string(c.dir.path().join("POL-1.md")).unwrap();
    std::fs::write(
        c.dir.path().join("POL-1.md"),
        text.replace("| Minor | Rare | Low |\n", ""),
    )
    .unwrap();
    estimate(&c, "EST-1", "post_control", ("Minor", "Rare"), &[]);
    residual(&c, "RR-1", "EST-1", "POL-1", "accepted");
    assert_eq!(reasons(&c), [json!("cell-not-in-policy")]);
}

#[trace("FR-154-AC-9")]
#[test]
fn an_unreadable_file_or_unparseable_frontmatter_is_an_error_naming_the_path() {
    let c = outside("accepted");
    std::fs::write(c.dir.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    let error = discrepancies(c.dir.path(), &modules()).unwrap_err();
    assert!(error.message.contains("RR-9.md"), "{}", error.message);
    std::fs::remove_file(c.dir.path().join("RR-9.md")).unwrap();

    std::fs::write(
        c.dir.path().join("RR-8.md"),
        "---\nid: RR-8\ntype: residual_risk\nrelationships: [unclosed\n  - : :\n---\n# RR-8\n",
    )
    .unwrap();
    let error = discrepancies(c.dir.path(), &modules()).unwrap_err();
    assert!(error.message.contains("RR-8.md"), "{}", error.message);

    std::fs::remove_file(c.dir.path().join("RR-8.md")).unwrap();

    // A fence that is never closed is as unparseable; a plain README is not a record.
    std::fs::write(
        c.dir.path().join("RR-7.md"),
        "---\nid: RR-7\ntype: residual_risk\n# RR-7\n",
    )
    .unwrap();
    let error = discrepancies(c.dir.path(), &modules()).unwrap_err();
    assert!(error.message.contains("RR-7.md"), "{}", error.message);
    assert!(!error.message.contains("cannot be read or parsed"));
    assert!(advise(c.dir.path(), &modules()).is_ok());
    std::fs::remove_file(c.dir.path().join("RR-7.md")).unwrap();
    std::fs::write(c.dir.path().join("README.md"), "# Notes\n\nno fence here\n").unwrap();
    assert_eq!(c.tokens(), ["accepted-outside-acceptable-region"]);
    std::fs::remove_file(c.dir.path().join("README.md")).unwrap();

    // `risk advise` keeps listing such a file instead of refusing.
    std::fs::write(c.dir.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    assert_eq!(
        advise(c.dir.path(), &modules()).unwrap().unreadable.len(),
        1
    );
}

#[trace("FR-154-AC-11")]
#[test]
fn records_the_command_does_not_read_cannot_stop_it() {
    let c = outside("accepted");
    c.put(
        "ORE-1",
        "overall_residual_risk_evaluation",
        &rels(&[("RR-1", "references")]),
        "## Determination\n\n| Product | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| p | maybe | <why> | |\n",
    );
    c.put("ANOM-1", "Anomaly", "status: bogus\n", "");
    assert_eq!(c.tokens(), ["accepted-outside-acceptable-region"]);
}
