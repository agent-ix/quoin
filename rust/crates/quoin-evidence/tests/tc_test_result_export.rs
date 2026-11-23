// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Test the pure in-toto Test Result mapping against FR-185.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "the fixture is valid and assertions should identify malformed mapped fields"
)]

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quoin_evidence::export::test_result::{
    ExportError, ExportScope, TestResult, TestResultPredicate, TestResultStatement, build_statement,
};
use quoin_evidence::types::{Binding, Outcome, RunEntry, RunRecord};
use quoin_evidence::{ObligationId, SuiteId, SymbolId};
use serde_json::{Value, json};

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn subject() -> Value {
    json!([{ "name": "artifact", "digest": { "sha256": SHA }, "annotations": { "revision": REVISION } }])
}

/// `build_statement` over the population the bindings name (the shape every
/// older test here already assumed), at the subject's revision.
fn stmt(
    subject: Value,
    bindings: &[Binding],
    runs: &[RunRecord],
) -> Result<TestResultStatement, ExportError> {
    let mut required: BTreeSet<String> = bindings
        .iter()
        .map(|binding| binding.obligation.as_str().to_owned())
        .collect();
    if required.is_empty() {
        required.insert("FR-0-AC-1".to_owned());
    }
    build_statement(
        subject,
        bindings,
        runs,
        ExportScope {
            source_revision: REVISION,
            required: &required,
        },
    )
}

fn required(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
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

fn traced_entry(symbol: &str, outcome: Outcome, obligation: &str) -> RunEntry {
    let mut entry = RunEntry::new(symbol, outcome);
    entry.trace_ids = Some(vec![obligation.to_owned()]);
    entry
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
            vec![traced_entry("test_pass", Outcome::Pass, "FR-1-AC-1")],
        ),
        run(
            "suite-fail",
            vec![traced_entry("test_fail", Outcome::Fail, "FR-1-AC-2")],
        ),
    ];
    let statement = stmt(subject(), &bindings, &runs).expect("valid subject");
    assert_eq!(statement.predicate.result, TestResult::Failed);
    assert_eq!(statement.predicate.passed_tests, ["FR-1-AC-1"]);
    assert_eq!(statement.predicate.failed_tests, ["FR-1-AC-2", "FR-1-AC-3"]);
    assert!(statement.predicate.warned_tests.is_empty());
}

#[trace("FR-185-AC-1", "FR-185-AC-8")]
#[test]
fn tc_185_a_later_duplicate_failure_cannot_be_hidden_by_a_pass() {
    let statement = stmt(
        subject(),
        &[binding("FR-1-AC-1", "suite", &["test"])],
        &[run(
            "suite",
            vec![
                traced_entry("test", Outcome::Pass, "FR-1-AC-1"),
                traced_entry("test", Outcome::Fail, "FR-1-AC-1"),
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
    let skip = stmt(
        subject(),
        &[binding("FR-2-AC-1", "suite", &["skip"])],
        &[run(
            "suite",
            vec![traced_entry("skip", Outcome::Skip, "FR-2-AC-1")],
        )],
    )
    .expect("valid subject");
    assert_eq!(skip.predicate.result, TestResult::Warned);
    assert_eq!(skip.predicate.warned_tests, ["FR-2-AC-1"]);

    let pass = stmt(
        subject(),
        &[binding("FR-2-AC-2", "suite", &["pass"])],
        &[run(
            "suite",
            vec![traced_entry("pass", Outcome::Pass, "FR-2-AC-2")],
        )],
    )
    .expect("valid subject");
    assert_eq!(pass.predicate.result, TestResult::Passed);
    assert!(pass.predicate.warned_tests.is_empty());
    assert!(pass.predicate.failed_tests.is_empty());
}

#[trace("FR-185-AC-2")]
#[test]
fn tc_185_absent_symbol_remains_warned() {
    let statement = stmt(
        subject(),
        &[binding("FR-2-AC-3", "suite", &["missing"])],
        &[run(
            "suite",
            vec![traced_entry("other", Outcome::Pass, "FR-2-AC-4")],
        )],
    )
    .expect("valid subject");

    assert_eq!(statement.predicate.result, TestResult::Warned);
    assert_eq!(statement.predicate.warned_tests, ["FR-2-AC-3"]);
    assert!(statement.predicate.passed_tests.is_empty());
    assert!(statement.predicate.failed_tests.is_empty());
}

#[trace("FR-185-AC-2")]
#[test]
fn tc_185_a_later_duplicate_skip_is_not_hidden_by_a_pass() {
    let statement = stmt(
        subject(),
        &[binding("FR-2-AC-1", "suite", &["test"])],
        &[run(
            "suite",
            vec![
                traced_entry("test", Outcome::Pass, "FR-2-AC-1"),
                traced_entry("test", Outcome::Skip, "FR-2-AC-1"),
            ],
        )],
    )
    .expect("valid subject");

    assert_eq!(statement.predicate.result, TestResult::Warned);
    assert!(statement.predicate.passed_tests.is_empty());
    assert_eq!(statement.predicate.warned_tests, ["FR-2-AC-1"]);
    assert!(statement.predicate.failed_tests.is_empty());
}

#[trace("FR-185-AC-9")]
#[test]
fn tc_185_untraced_passing_symbol_is_stale() {
    let statement = stmt(
        subject(),
        &[binding("FR-9-AC-1", "suite", &["test"])],
        &[run("suite", vec![RunEntry::new("test", Outcome::Pass)])],
    )
    .expect("valid subject");

    assert_eq!(statement.predicate.result, TestResult::Failed);
    assert!(statement.predicate.passed_tests.is_empty());
    assert!(statement.predicate.warned_tests.is_empty());
    assert_eq!(statement.predicate.failed_tests, ["FR-9-AC-1"]);
}

#[trace("FR-185-AC-9")]
#[test]
fn tc_185_passing_symbol_traced_to_another_obligation_is_stale() {
    let mut entry = RunEntry::new("test", Outcome::Pass);
    entry.trace_ids = Some(vec!["FR-9-AC-2".to_owned()]);
    let statement = stmt(
        subject(),
        &[binding("FR-9-AC-1", "suite", &["test"])],
        &[run("suite", vec![entry])],
    )
    .expect("valid subject");

    assert_eq!(statement.predicate.result, TestResult::Failed);
    assert!(statement.predicate.passed_tests.is_empty());
    assert!(statement.predicate.warned_tests.is_empty());
    assert_eq!(statement.predicate.failed_tests, ["FR-9-AC-1"]);
}

#[trace("FR-185-AC-9")]
#[test]
fn tc_185_matching_trace_id_allows_passing_symbol() {
    let statement = stmt(
        subject(),
        &[binding("FR-9-AC-1", "suite", &["test"])],
        &[run(
            "suite",
            vec![traced_entry("test", Outcome::Pass, "FR-9-AC-1")],
        )],
    )
    .expect("valid subject");

    assert_eq!(statement.predicate.result, TestResult::Passed);
    assert_eq!(statement.predicate.passed_tests, ["FR-9-AC-1"]);
    assert!(statement.predicate.warned_tests.is_empty());
    assert!(statement.predicate.failed_tests.is_empty());
}

#[trace("FR-185-AC-3")]
#[test]
fn tc_185_subject_is_preserved_and_bad_subjects_are_refused() {
    let supplied = json!([{ "name": "artifact", "digest": { "sha256": SHA }, "annotations": { "revision": REVISION }, "extra": [1, "x"] }]);
    let statement = stmt(supplied.clone(), &[], &[]).expect("valid subject");
    assert_eq!(json!(statement.subject), supplied);
    assert!(stmt(json!([]), &[], &[]).is_err());
    assert!(stmt(json!({}), &[], &[]).is_err());
    assert!(stmt(json!([{ "digest": {} }]), &[], &[]).is_err());
    // A valid digest without a revision, or with inconsistent revisions, is refused.
    let no_revision = json!([{ "digest": { "sha256": SHA } }]);
    assert!(stmt(no_revision, &[], &[]).is_err());
    let split = json!([
        { "digest": { "sha256": SHA }, "annotations": { "revision": "a" } },
        { "digest": { "sha256": SHA }, "annotations": { "revision": "b" } }
    ]);
    assert!(stmt(split, &[], &[]).is_err());
    assert!(
        stmt(
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
        run(
            "suite-b",
            vec![traced_entry("b", Outcome::Pass, "FR-4-AC-2")],
        ),
        run(
            "suite-a",
            vec![traced_entry("a", Outcome::Pass, "FR-4-AC-1")],
        ),
    ];
    let first = stmt(subject(), &bindings, &runs).expect("valid");
    let second = stmt(
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
    let statement = stmt(subject(), &[], &[]).expect("valid");
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

fn scope(required: &BTreeSet<String>) -> ExportScope<'_> {
    ExportScope {
        source_revision: REVISION,
        required,
    }
}

fn passing_pair() -> (Vec<Binding>, Vec<RunRecord>) {
    (
        vec![binding("FR-9-AC-1", "suite-ok", &["ok"])],
        vec![run(
            "suite-ok",
            vec![traced_entry("ok", Outcome::Pass, "FR-9-AC-1")],
        )],
    )
}

/// A complete PASSED needs the declared population: a genuine one passes, and
/// each way of shrinking, emptying or mis-scoping it cannot.
#[trace("FR-185-AC-10")]
#[test]
fn tc_1792_001_a_complete_pass_needs_the_declared_population() {
    let (bindings, runs) = passing_pair();
    let full = required(&["FR-9-AC-1"]);

    // Healthy control.
    let statement = build_statement(subject(), &bindings, &runs, scope(&full)).expect("valid");
    assert_eq!(statement.predicate.result, TestResult::Passed);
    assert_eq!(statement.predicate.passed_tests, ["FR-9-AC-1"]);

    // Empty graph: the population is declared, the graph proves none of it.
    let wider = required(&["FR-9-AC-1", "FR-9-AC-2"]);
    let omitted = build_statement(subject(), &bindings, &runs, scope(&wider)).unwrap();
    assert_eq!(omitted.predicate.result, TestResult::Failed);
    assert_eq!(omitted.predicate.passed_tests, ["FR-9-AC-1"]);
    assert_eq!(omitted.predicate.failed_tests, ["FR-9-AC-2"]);
    let empty_graph = build_statement(subject(), &[], &[], scope(&full)).unwrap();
    assert_eq!(empty_graph.predicate.result, TestResult::Failed);
    assert_eq!(empty_graph.predicate.failed_tests, ["FR-9-AC-1"]);

    // An empty declared population is not a complete one.
    let none = BTreeSet::new();
    assert_eq!(
        build_statement(subject(), &bindings, &runs, scope(&none)),
        Err(ExportError::EmptyPopulation)
    );
}

/// A binding with no symbols proves nothing; an unrelated binding adds nothing.
#[trace("FR-185-AC-10")]
#[test]
fn tc_1792_002_zero_symbol_and_unrelated_bindings() {
    let (mut bindings, runs) = passing_pair();
    let full = required(&["FR-9-AC-1"]);
    let scope = ExportScope {
        source_revision: REVISION,
        required: &full,
    };
    bindings.push(binding("FR-OTHER-AC-1", "suite-ok", &["ok"]));
    let unrelated = build_statement(subject(), &bindings, &runs, scope).unwrap();
    assert_eq!(unrelated.predicate.result, TestResult::Passed);
    assert_eq!(unrelated.predicate.passed_tests, ["FR-9-AC-1"]);

    bindings.push(binding("FR-9-AC-1", "suite-ok", &[]));
    let zero = build_statement(subject(), &bindings, &runs, scope).unwrap();
    assert_eq!(zero.predicate.result, TestResult::Failed);
    assert_eq!(zero.predicate.failed_tests, ["FR-9-AC-1"]);
}

/// A missing run and a duplicate failed entry cannot pass; the subject must be
/// the provenance's revision.
#[trace("FR-185-AC-8", "FR-185-AC-12")]
#[test]
fn tc_1792_003_missing_run_duplicate_failure_and_revision_mismatch() {
    let (bindings, runs) = passing_pair();
    let full = required(&["FR-9-AC-1"]);
    let scope = ExportScope {
        source_revision: REVISION,
        required: &full,
    };
    let missing = build_statement(subject(), &bindings, &[], scope).unwrap();
    assert_eq!(missing.predicate.failed_tests, ["FR-9-AC-1"]);

    let duplicate = vec![run(
        "suite-ok",
        vec![
            traced_entry("ok", Outcome::Pass, "FR-9-AC-1"),
            traced_entry("ok", Outcome::Fail, "FR-9-AC-1"),
        ],
    )];
    let failed = build_statement(subject(), &bindings, &duplicate, scope).unwrap();
    assert_eq!(failed.predicate.result, TestResult::Failed);

    // Unrelated subject / old run against a new subject.
    let other = ExportScope {
        source_revision: "fedcba9876543210fedcba9876543210fedcba98",
        required: &full,
    };
    assert!(matches!(
        build_statement(subject(), &bindings, &runs, other),
        Err(ExportError::RevisionMismatch { .. })
    ));
}
