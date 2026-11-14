// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Test the pure in-toto Test Result mapping against FR-185.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the fixture is valid and assertions should identify malformed mapped fields"
)]

use ix_trace_rs::trace;
use quoin_evidence::export::test_result::{
    TestResult, TestResultPredicate, TestResultStatement, build_statement,
};
use quoin_evidence::types::{Binding, Outcome, RunEntry, RunRecord};
use quoin_evidence::{ObligationId, SuiteId, SymbolId};
use serde_json::{Value, json};

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn subject() -> Value {
    json!([{ "name": "artifact", "digest": { "sha256": SHA } }])
}

fn binding(obligation: &str, suite: &str, symbols: &[&str]) -> Binding {
    Binding {
        obligation: ObligationId::new(obligation),
        suite: SuiteId::new(suite),
        symbols: symbols
            .iter()
            .map(|symbol| SymbolId::new(*symbol))
            .collect(),
        lineage: None,
    }
}

fn run(suite: &str, entries: Vec<RunEntry>) -> RunRecord {
    RunRecord {
        schema_version: 1,
        suite: SuiteId::new(suite),
        tool: "cargo test".to_owned(),
        evidence_kind: Some("test".to_owned()),
        timestamp: "2026-11-13T12:00:00Z".to_owned(),
        entries,
    }
}

#[trace("FR-185-AC-1", "FR-185-AC-8")]
#[test]
fn tc_185_failures_and_missing_runs_never_pass() {
    let bindings = vec![
        binding("FR-1-AC-1", "suite-pass", &["test_pass"]),
        binding("FR-1-AC-2", "suite-fail", &["test_fail"]),
        binding("FR-1-AC-3", "suite-missing", &["test_missing"]),
    ];
    let runs = vec![
        run(
            "suite-pass",
            vec![RunEntry::new("test_pass", Outcome::Pass)],
        ),
        run(
            "suite-fail",
            vec![RunEntry::new("test_fail", Outcome::Fail)],
        ),
    ];
    let statement = build_statement(subject(), &bindings, &runs).expect("valid subject");
    assert_eq!(statement.predicate.result, TestResult::Failed);
    assert_eq!(statement.predicate.passed_tests, ["FR-1-AC-1"]);
    assert_eq!(statement.predicate.failed_tests, ["FR-1-AC-2", "FR-1-AC-3"]);
    assert!(statement.predicate.warned_tests.is_empty());
}

#[trace("FR-185-AC-1", "FR-185-AC-8")]
#[test]
fn tc_185_a_later_duplicate_failure_cannot_be_hidden_by_a_pass() {
    let statement = build_statement(
        subject(),
        &[binding("FR-1-AC-1", "suite", &["test"])],
        &[run(
            "suite",
            vec![
                RunEntry::new("test", Outcome::Pass),
                RunEntry::new("test", Outcome::Fail),
            ],
        )],
    )
    .expect("valid subject");

    assert_eq!(statement.predicate.result, TestResult::Failed);
    assert!(statement.predicate.passed_tests.is_empty());
    assert_eq!(statement.predicate.failed_tests, ["FR-1-AC-1"]);
}

#[trace("FR-185-AC-2")]
#[test]
fn tc_185_skip_warns_and_all_pass_passes() {
    let skip = build_statement(
        subject(),
        &[binding("FR-2-AC-1", "suite", &["skip"])],
        &[run("suite", vec![RunEntry::new("skip", Outcome::Skip)])],
    )
    .expect("valid subject");
    assert_eq!(skip.predicate.result, TestResult::Warned);
    assert_eq!(skip.predicate.warned_tests, ["FR-2-AC-1"]);

    let pass = build_statement(
        subject(),
        &[binding("FR-2-AC-2", "suite", &["pass"])],
        &[run("suite", vec![RunEntry::new("pass", Outcome::Pass)])],
    )
    .expect("valid subject");
    assert_eq!(pass.predicate.result, TestResult::Passed);
    assert!(pass.predicate.warned_tests.is_empty());
    assert!(pass.predicate.failed_tests.is_empty());
}

#[trace("FR-185-AC-2")]
#[test]
fn tc_185_a_later_duplicate_skip_is_not_hidden_by_a_pass() {
    let statement = build_statement(
        subject(),
        &[binding("FR-2-AC-1", "suite", &["test"])],
        &[run(
            "suite",
            vec![
                RunEntry::new("test", Outcome::Pass),
                RunEntry::new("test", Outcome::Skip),
            ],
        )],
    )
    .expect("valid subject");

    assert_eq!(statement.predicate.result, TestResult::Warned);
    assert!(statement.predicate.passed_tests.is_empty());
    assert_eq!(statement.predicate.warned_tests, ["FR-2-AC-1"]);
    assert!(statement.predicate.failed_tests.is_empty());
}

#[trace("FR-185-AC-3")]
#[test]
fn tc_185_subject_is_preserved_and_bad_subjects_are_refused() {
    let supplied = json!([{ "name": "artifact", "digest": { "sha256": SHA }, "extra": [1, "x"] }]);
    let statement = build_statement(supplied.clone(), &[], &[]).expect("valid subject");
    assert_eq!(json!(statement.subject), supplied);
    assert!(build_statement(json!([]), &[], &[]).is_err());
    assert!(build_statement(json!({}), &[], &[]).is_err());
    assert!(build_statement(json!([{ "digest": {} }]), &[], &[]).is_err());
    assert!(
        build_statement(
            json!([{ "digest": { "sha256": "A".repeat(64) } }]),
            &[],
            &[]
        )
        .is_err()
    );
}

#[trace("FR-185-AC-4")]
#[test]
fn tc_185_lists_and_configuration_are_deterministic() {
    let bindings = vec![
        binding("FR-4-AC-2", "suite-b", &["b"]),
        binding("FR-4-AC-1", "suite-a", &["a"]),
        binding("FR-4-AC-1", "suite-a", &["a"]),
    ];
    let runs = vec![
        run("suite-b", vec![RunEntry::new("b", Outcome::Pass)]),
        run("suite-a", vec![RunEntry::new("a", Outcome::Pass)]),
    ];
    let first = build_statement(subject(), &bindings, &runs).expect("valid");
    let second = build_statement(
        subject(),
        &[
            bindings[2].clone(),
            bindings[1].clone(),
            bindings[0].clone(),
        ],
        &[runs[1].clone(), runs[0].clone()],
    )
    .expect("valid");
    assert_eq!(first, second);
    assert_eq!(first.predicate.passed_tests, ["FR-4-AC-1", "FR-4-AC-2"]);
    assert_eq!(
        first
            .predicate
            .configuration
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["suite-a", "suite-b"]
    );
}

#[trace("FR-185-AC-5")]
#[test]
fn tc_185_predicate_type_is_closed_and_has_exact_members() {
    let statement = build_statement(subject(), &[], &[]).expect("valid");
    let value = serde_json::to_value(&statement).expect("serializes");
    let mut members = value
        .get("predicate")
        .and_then(Value::as_object)
        .expect("predicate object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    members.sort_unstable();
    assert_eq!(
        members,
        [
            "configuration",
            "failedTests",
            "passedTests",
            "result",
            "warnedTests"
        ]
    );
    serde_json::from_value::<TestResultStatement>(value.clone()).expect("closed statement reads");
    let mut invalid = value;
    invalid["predicate"]["unexpected"] = json!(true);
    assert!(serde_json::from_value::<TestResultStatement>(invalid).is_err());
    serde_json::from_value::<TestResultPredicate>(json!({
        "result": "PASSED",
        "configuration": [],
        "passedTests": [],
        "warnedTests": [],
        "failedTests": [],
        "unexpected": true
    }))
    .expect_err("predicate rejects unknown members");
}
