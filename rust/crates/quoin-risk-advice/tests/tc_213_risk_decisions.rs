// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The overall-evaluation checks of `spec-objects-safety` FR-107 (FR-213) and
//! the git attribution of each decision (FR-214), over authored fixtures.

use std::collections::BTreeMap;
use std::process::Command;

use ix_trace_rs::trace;
use quoin_risk_advice::advise;
use serde_json::Value;

mod common;

use common::*;

fn base(c: &Corpus) {
    policy(c, "POL-1");
    estimate(
        c,
        "EST-1",
        "post_control",
        ("Serious", "Likely"),
        &[("SIT-1", "estimates")],
    );
}

fn residual_by(c: &Corpus, id: &str, decision: &str, by: &str, rationale: &str) {
    c.put(
        id,
        "residual_risk",
        &rels(&[("EST-1", "estimate"), ("POL-1", "references")]),
        &format!(
            "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| {decision} | {rationale} | {by} |\n"
        ),
    );
}

fn evaluation(c: &Corpus, id: &str, decision: &str, refs: &[&str]) {
    let by = if decision == "undecided" { "" } else { "Pat" };
    let edges: Vec<(&str, &str)> = refs.iter().map(|id| (*id, "references")).collect();
    c.put(
        id,
        "overall_residual_risk_evaluation",
        &rels(&edges),
        &format!(
            "## Determination\n\n| Product | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| Pump | {decision} | r | {by} |\n"
        ),
    );
}

fn json(c: &Corpus) -> Value {
    let advice = advise(c.dir.path(), &modules()).unwrap();
    serde_json::from_str(&advice.to_json().unwrap()).unwrap()
}

fn check_findings(json: &Value, evaluation: &str) -> Vec<(String, Vec<String>)> {
    json["overall_evaluation_checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["evaluation"] == evaluation)
        .unwrap()["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding["finding"].as_str().unwrap().to_owned(),
                finding["names"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|name| name.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect()
}

fn names(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[trace("FR-107-AC-3", "FR-213-AC-1")]
#[test]
fn a_residual_risk_missing_from_the_considered_list_is_named() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    residual_by(&c, "RR-2", "accepted", "Pat", "r");
    evaluation(&c, "ORE-1", "accepted", &["RR-1"]);
    assert_eq!(
        check_findings(&json(&c), "ORE-1"),
        vec![("residual-risk-not-considered".to_owned(), names(&["RR-2"]))]
    );
    evaluation(&c, "ORE-1", "accepted", &["RR-1", "RR-2"]);
    assert!(check_findings(&json(&c), "ORE-1").is_empty());
}

#[trace("FR-107-AC-4", "FR-213-AC-2")]
#[test]
fn an_accepted_evaluation_over_a_not_accepted_risk_is_named() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    residual_by(&c, "RR-2", "not_accepted", "Pat", "r");
    evaluation(&c, "ORE-1", "accepted", &["RR-1", "RR-2"]);
    assert_eq!(
        check_findings(&json(&c), "ORE-1"),
        vec![(
            "evaluation-accepted-with-open-risk".to_owned(),
            names(&["RR-2"])
        )]
    );
}

#[trace("FR-107-AC-7", "FR-213-AC-3")]
#[test]
fn an_accepted_evaluation_over_an_undecided_risk_is_named_and_over_accepted_risks_is_not() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    residual_by(&c, "RR-2", "undecided", "", "r");
    evaluation(&c, "ORE-1", "accepted", &["RR-1", "RR-2"]);
    assert_eq!(
        check_findings(&json(&c), "ORE-1"),
        vec![(
            "evaluation-accepted-with-open-risk".to_owned(),
            names(&["RR-2"])
        )]
    );
    residual_by(&c, "RR-2", "accepted", "Pat", "r");
    assert!(check_findings(&json(&c), "ORE-1").is_empty());
}

#[trace("FR-107-AC-5", "FR-213-AC-4")]
#[test]
fn accepted_risks_with_no_evaluation_create_none_and_state_no_accepted_product() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    let before = snapshot(c.dir.path());
    let report = json(&c);
    assert_eq!(
        report["overall_evaluation_checks"],
        Value::Array(Vec::new())
    );
    assert_eq!(snapshot(c.dir.path()), before);
    let markdown = advise(c.dir.path(), &modules()).unwrap().to_markdown();
    assert!(markdown.contains("## overall_evaluation_checks\n\nnone\n"));
    assert!(!markdown.contains("product"));
}

#[trace("FR-213-AC-5")]
#[test]
fn an_evaluation_not_decided_as_accepted_raises_no_open_risk_finding() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "undecided", "", "r");
    residual_by(&c, "RR-2", "not_accepted", "Pat", "r");
    evaluation(&c, "ORE-1", "undecided", &["RR-1", "RR-2"]);
    evaluation(&c, "ORE-2", "not_accepted", &["RR-1", "RR-2"]);
    let report = json(&c);
    assert!(check_findings(&report, "ORE-1").is_empty());
    assert!(check_findings(&report, "ORE-2").is_empty());
}

#[trace("FR-107-AC-3", "FR-213-AC-6")]
#[test]
fn entry_keys_are_exact_and_carry_no_verdict_key() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    evaluation(&c, "ORE-1", "accepted", &[]);
    let report = json(&c);
    let entry = &report["overall_evaluation_checks"][0];
    assert_eq!(
        keys(entry),
        ["decision", "evaluation", "findings", "product"]
    );
    assert_eq!(entry["product"], "Pump");
    for finding in entry["findings"].as_array().unwrap() {
        assert_eq!(keys(finding), ["finding", "names"]);
    }
}

fn git(c: &Corpus, author: &str, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(c.dir.path())
        .args(args)
        .env("GIT_AUTHOR_NAME", author)
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", author)
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn repo(c: &Corpus) {
    git(c, "Pat", &["init", "-q", "-b", "main"]);
    git(c, "Pat", &["config", "commit.gpgsign", "false"]);
}

fn commit(c: &Corpus, author: &str) -> String {
    git(c, author, &["add", "-A"]);
    git(c, author, &["commit", "-q", "-m", "step"]);
    git(c, author, &["rev-parse", "HEAD"])
}

fn attribution<'a>(json: &'a Value, record: &str) -> &'a Value {
    json["decision_attribution"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["record"] == record)
        .unwrap_or_else(|| panic!("no attribution entry for {record}"))
}

fn tokens(entry: &Value) -> Vec<&str> {
    entry["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["finding"].as_str().unwrap())
        .collect()
}

#[trace("FR-214-AC-1", "FR-214-AC-3")]
#[test]
fn the_commit_that_set_the_decision_is_reported_and_unsigned_is_flagged() {
    let c = corpus();
    base(&c);
    repo(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    let hash = commit(&c, "Pat");
    let report = json(&c);
    let entry = attribution(&report, "RR-1");
    let change = &entry["decision_change"];
    assert_eq!(change["state"], "present");
    assert_eq!(change["commit"], hash.as_str());
    assert_eq!(change["author"], "Pat");
    assert_eq!(change["signature"], "unsigned");
    assert_eq!(change["signatureCode"], "N");
    assert_eq!(tokens(entry), ["decision-commit-not-signed"]);
    assert_eq!(entry["findings"][0]["names"][0], hash.as_str());
}

#[trace("FR-214-AC-2")]
#[test]
fn a_decider_that_differs_from_the_commit_author_is_flagged_ignoring_case_and_space() {
    let c = corpus();
    base(&c);
    repo(&c);
    residual_by(&c, "RR-1", "accepted", "Lee", "r");
    commit(&c, "Pat");
    let report = json(&c);
    let entry = attribution(&report, "RR-1");
    let mismatch = &entry["findings"][0];
    assert_eq!(mismatch["finding"], "decided-by-differs-from-commit-author");
    assert_eq!(mismatch["names"], serde_json::json!(["Lee", "Pat"]));

    residual_by(&c, "RR-2", "accepted", "  pAT ", "r");
    commit(&c, "Pat");
    let report = json(&c);
    assert!(
        !tokens(attribution(&report, "RR-2")).contains(&"decided-by-differs-from-commit-author")
    );
}

#[trace("FR-214-AC-4")]
#[test]
fn a_rationale_edit_leaves_the_decision_commit_and_a_decision_change_moves_it() {
    let c = corpus();
    base(&c);
    repo(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "first");
    let set = commit(&c, "Pat");
    residual_by(&c, "RR-1", "accepted", "Pat", "second");
    commit(&c, "Pat");
    let report = json(&c);
    assert_eq!(
        attribution(&report, "RR-1")["decision_change"]["commit"],
        set.as_str()
    );
    residual_by(&c, "RR-1", "not_accepted", "Pat", "second");
    let moved = commit(&c, "Pat");
    let report = json(&c);
    assert_eq!(
        attribution(&report, "RR-1")["decision_change"]["commit"],
        moved.as_str()
    );
}

#[trace("FR-214-AC-5")]
#[test]
fn an_uncommitted_decision_is_pending_and_a_file_outside_git_is_not_applicable() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    let report = json(&c);
    let outside = attribution(&report, "RR-1");
    assert_eq!(outside["decision_change"]["state"], "not-applicable");
    assert!(tokens(outside).is_empty());

    repo(&c);
    commit(&c, "Pat");
    residual_by(&c, "RR-1", "not_accepted", "Lee", "r");
    let report = json(&c);
    let pending = attribution(&report, "RR-1");
    assert_eq!(pending["decision_change"]["state"], "pending");
    assert!(tokens(pending).is_empty());
}

#[trace("FR-214-AC-6", "FR-214-AC-7")]
#[test]
fn every_decided_kind_is_attributed_and_undecided_records_are_absent() {
    let c = corpus();
    base(&c);
    residual_by(&c, "RR-1", "accepted", "Pat", "r");
    residual_by(&c, "RR-2", "undecided", "", "r");
    c.put(
        "BR-1",
        "benefit_risk",
        &rels(&[("RR-1", "references")]),
        "## Determination\n\n| Benefit | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| b | not_accepted | r | Pat |\n",
    );
    evaluation(&c, "ORE-1", "accepted", &["RR-1"]);
    evaluation(&c, "ORE-2", "undecided", &["RR-1"]);
    let report = json(&c);
    let order: Vec<&str> = report["decision_attribution"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["record"].as_str().unwrap())
        .collect();
    assert_eq!(order, ["RR-1", "BR-1", "ORE-1"]);
    for entry in report["decision_attribution"].as_array().unwrap() {
        assert_eq!(
            keys(entry),
            [
                "decided_by",
                "decision",
                "decision_change",
                "findings",
                "kind",
                "record",
                "signer_principal"
            ]
        );
        assert!(entry["signer_principal"].is_null());
    }
}

#[trace("FR-214-AC-8")]
#[test]
fn verified_principal_replaces_author_comparison_and_unmapped_signer_has_no_fallback() {
    let c = corpus();
    base(&c);
    repo(&c);
    residual_by(&c, "RR-1", "accepted", "Lee", "r");
    let commit_id = commit(&c, "Pat");

    let mut advice = advise(c.dir.path(), &modules()).unwrap();
    let mapped = BTreeMap::from([(commit_id.clone(), Some("Lee".to_owned()))]);
    quoin_risk_advice::apply_verified_signers(&mut advice, &mapped);
    let entry = &advice.decision_attribution[0];
    assert_eq!(entry.signer_principal.as_deref(), Some("Lee"));
    assert!(!entry.findings.iter().any(
        |finding| finding.finding == quoin_risk_advice::Token::DecidedByDiffersFromCommitAuthor
    ));
    assert!(
        !entry.findings.iter().any(|finding| finding.finding
            == quoin_risk_advice::Token::DecidedByDiffersFromVerifiedSigner)
    );

    let mut advice = advise(c.dir.path(), &modules()).unwrap();
    let different = BTreeMap::from([(commit_id.clone(), Some("Morgan".to_owned()))]);
    quoin_risk_advice::apply_verified_signers(&mut advice, &different);
    let entry = &advice.decision_attribution[0];
    let finding = entry
        .findings
        .iter()
        .find(|finding| {
            finding.finding == quoin_risk_advice::Token::DecidedByDiffersFromVerifiedSigner
        })
        .unwrap();
    assert_eq!(finding.names, ["Lee", "Morgan"]);
    assert!(!entry.findings.iter().any(
        |finding| finding.finding == quoin_risk_advice::Token::DecidedByDiffersFromCommitAuthor
    ));

    let mut advice = advise(c.dir.path(), &modules()).unwrap();
    let unmapped = BTreeMap::from([(commit_id, None)]);
    quoin_risk_advice::apply_verified_signers(&mut advice, &unmapped);
    let entry = &advice.decision_attribution[0];
    assert!(entry.signer_principal.is_none());
    assert!(!entry.findings.iter().any(
        |finding| finding.finding == quoin_risk_advice::Token::DecidedByDiffersFromCommitAuthor
    ));
}
