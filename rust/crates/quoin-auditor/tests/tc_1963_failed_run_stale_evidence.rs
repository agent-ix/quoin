// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! A run that failed the tagged test is stale evidence, not a clean bill.
//!
//! FR-032 line 28 named a failed run beside a missing run and a run behind
//! HEAD as one of the three ways evidence rots, but
//! `rust/crates/quoin-auditor/src/audit/ladder.rs`'s only outcome-aware rung
//! was vacuity, which asks only `Outcome::Skip`. A suite that ran the tagged
//! test and reported `fail` or `error` at HEAD read as healthy on a red
//! build, and the upcoming computed matrix maps `healthy` to "bound by a
//! passing run" (PLAT-1086).
//!
//! Trace: FR-032-AC-17
//! Provenance: PLAT-1086

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_auditor::audit::{AuditInput, audit};
use quoin_evidence::ids::{Commit, ObligationId, StatementHash, SuiteId, SymbolId};
use quoin_evidence::types::{Binding, Outcome, RunEntry, RunRecord, STORE_SCHEMA_VERSION};
use quoin_quire_types::Obligation;

const COMMIT: &str = "deadbeefdead";
const STATEMENT_HASH: &str = "hash-of-the-statement";

/// One obligation, statement hash matching, so nothing suspect-links.
fn an_obligation(id: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        statement: "the system SHALL do the thing".to_owned(),
        statement_hash: STATEMENT_HASH.to_owned(),
        target_ids: None,
        method: None,
        criticality: None,
        parameters: None,
    }
}

/// A binding naming one suite and one symbol, unaged (bound at the current
/// statement hash).
fn a_binding(obligation: &str, suite: &str, symbol: &str) -> Binding {
    Binding {
        obligation: ObligationId::new(obligation),
        statement_hash_at_binding: StatementHash::new(STATEMENT_HASH),
        suite: SuiteId::new(suite),
        commit: Commit::new(COMMIT),
        symbols: vec![SymbolId::new(symbol)],
        lineage: None,
        affirmations: None,
    }
}

/// One run of one suite at HEAD, with the given outcomes.
fn a_run(suite: &str, entries: Vec<(&str, Outcome)>) -> RunRecord {
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        commit: Commit::new(COMMIT),
        tool: "cargo-test".to_owned(),
        evidence_kind: None,
        timestamp: "2026-09-27T00:00:00Z".to_owned(),
        entries: entries
            .into_iter()
            .map(|(symbol, outcome)| RunEntry::new(symbol, outcome))
            .collect(),
    }
}

/// The stale-evidence findings, if any, naming this obligation.
fn stale_findings<'a>(
    report: &'a quoin_finding_types::AuditReport,
    obligation: &str,
) -> Vec<&'a str> {
    report
        .findings
        .iter()
        .filter(|finding| finding.kind == "stale-evidence" && finding.obligation == obligation)
        .map(|finding| finding.summary.as_str())
        .collect()
}

/// A pass-then-fail run pair: the criterion leaves `healthy` and a
/// `stale-evidence` finding names the failing run.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_a_pass_then_fail_run_raises_stale_evidence() {
    let obligation = an_obligation("FR-900-AC-1");
    let binding = a_binding("FR-900-AC-1", "unit", "tests::tc_900");
    let run = a_run("unit", vec![("tests::tc_900", Outcome::Fail)]);
    let input = AuditInput {
        obligations: vec![obligation],
        bindings: vec![binding],
        runs: vec![run],
        head_commit: Some(COMMIT.to_owned()),
        ..AuditInput::default()
    };

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        !report.healthy.contains(&"FR-900-AC-1".to_owned()),
        "a failed tagged test must not read as healthy: {:?}",
        report.healthy
    );
    let stale = stale_findings(&report, "FR-900-AC-1");
    assert_eq!(
        stale.len(),
        1,
        "exactly one stale-evidence finding must name the failing run, saw {stale:?}"
    );
    assert!(
        stale[0].contains("unit") && stale[0].contains("fail"),
        "the reason must name the failing suite and outcome: {}",
        stale[0]
    );
}

/// A pass-then-error run pair: `Error` is treated the same as `Fail`.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_a_pass_then_error_run_raises_stale_evidence() {
    let obligation = an_obligation("FR-901-AC-1");
    let binding = a_binding("FR-901-AC-1", "unit", "tests::tc_901");
    let run = a_run("unit", vec![("tests::tc_901", Outcome::Error)]);
    let input = AuditInput {
        obligations: vec![obligation],
        bindings: vec![binding],
        runs: vec![run],
        head_commit: Some(COMMIT.to_owned()),
        ..AuditInput::default()
    };

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        !report.healthy.contains(&"FR-901-AC-1".to_owned()),
        "an errored tagged test must not read as healthy: {:?}",
        report.healthy
    );
    let stale = stale_findings(&report, "FR-901-AC-1");
    assert_eq!(
        stale.len(),
        1,
        "exactly one stale-evidence finding must name the erroring run, saw {stale:?}"
    );
    assert!(
        stale[0].contains("error"),
        "the reason must name the error outcome: {}",
        stale[0]
    );
}

/// A pass-then-pass control: an obligation whose tagged test still passes
/// stays healthy.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_a_pass_then_pass_run_stays_healthy() {
    let obligation = an_obligation("FR-902-AC-1");
    let binding = a_binding("FR-902-AC-1", "unit", "tests::tc_902");
    let run = a_run("unit", vec![("tests::tc_902", Outcome::Pass)]);
    let input = AuditInput {
        obligations: vec![obligation],
        bindings: vec![binding],
        runs: vec![run],
        head_commit: Some(COMMIT.to_owned()),
        // A suite the mocked-confirmation rung has not seen is reported
        // `unevaluated`, not healthy (#204) — orthogonal to this test, which
        // is only about the failed-run rung, so mark it inspected.
        mock_inspection_suites: Some(vec!["unit".to_owned()]),
        ..AuditInput::default()
    };

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        report.healthy.contains(&"FR-902-AC-1".to_owned()),
        "an obligation whose test still passes must stay healthy: {:?}",
        report.healthy
    );
    assert!(
        stale_findings(&report, "FR-902-AC-1").is_empty(),
        "a passing run must raise no stale-evidence finding"
    );
}

/// A fail on a DIFFERENT obligation's test, in the same run, must not spill
/// over: this obligation's own bound symbol still passed.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_a_different_obligations_failure_leaves_this_one_healthy() {
    let watched = an_obligation("FR-903-AC-1");
    let other = an_obligation("FR-903-AC-2");
    let watched_binding = a_binding("FR-903-AC-1", "unit", "tests::tc_903_watched");
    let other_binding = a_binding("FR-903-AC-2", "unit", "tests::tc_903_other");
    // One run records both symbols: the watched one passed, the other failed.
    let run = a_run(
        "unit",
        vec![
            ("tests::tc_903_watched", Outcome::Pass),
            ("tests::tc_903_other", Outcome::Fail),
        ],
    );
    let input = AuditInput {
        obligations: vec![watched, other],
        bindings: vec![watched_binding, other_binding],
        runs: vec![run],
        head_commit: Some(COMMIT.to_owned()),
        mock_inspection_suites: Some(vec!["unit".to_owned()]),
        ..AuditInput::default()
    };

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        report.healthy.contains(&"FR-903-AC-1".to_owned()),
        "a sibling test's failure must not fail this obligation's own passing evidence: {:?}",
        report.healthy
    );
    assert!(
        stale_findings(&report, "FR-903-AC-1").is_empty(),
        "the watched obligation must carry no stale-evidence finding"
    );
    assert!(
        !stale_findings(&report, "FR-903-AC-2").is_empty(),
        "the OTHER obligation, whose own symbol failed, must be reported stale"
    );
}
