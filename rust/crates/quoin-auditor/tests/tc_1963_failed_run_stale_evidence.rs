// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! A run that failed the tagged test is stale evidence, not a clean bill.
//!
//! FR-032 line 28 named a failed run beside a missing run and a run behind
//! HEAD as one of the three ways evidence rots, but the ladder's only
//! outcome-aware rung was `ladder.rs`'s vacuity check, which asks only
//! `Outcome::Skip`. A suite that ran the tagged test and reported `fail` or
//! `error` at HEAD read as healthy on a red build, and the upcoming computed
//! matrix maps `healthy` to "bound by a passing run" (PLAT-1086).
//!
//! Trace: FR-032-AC-17
//! Provenance: PLAT-1086

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_auditor::audit::{AuditInput, audit};
use quoin_evidence::ids::{Commit, ObligationId, StatementHash, SuiteId, SymbolId};
use quoin_evidence::source::MemoryEvidence;
use quoin_evidence::store::{latest_runs, write_run};
use quoin_evidence::types::{Binding, Outcome, RunEntry, RunRecord, STORE_SCHEMA_VERSION};
use quoin_finding_types::{AuditReport, Finding, Severity};
use quoin_quire_types::Obligation;

const COMMIT: &str = "deadbeefdead";
const OLDER_COMMIT: &str = "0000feedface";
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

/// One run of one suite at a given commit and timestamp, with the given
/// outcomes.
fn a_run(suite: &str, commit: &str, timestamp: &str, entries: Vec<(&str, Outcome)>) -> RunRecord {
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        commit: Commit::new(commit),
        tool: "cargo-test".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        entries: entries
            .into_iter()
            .map(|(symbol, outcome)| RunEntry::new(symbol, outcome))
            .collect(),
    }
}

/// One run at HEAD (`COMMIT`), for the single-run fixtures.
fn a_run_at_head(suite: &str, entries: Vec<(&str, Outcome)>) -> RunRecord {
    a_run(suite, COMMIT, "2026-09-27T00:00:00Z", entries)
}

/// An input with one suite inspected (so the mocked-confirmation rung does
/// not report `unevaluated` and mask what this test is about) and `COMMIT`
/// as HEAD.
fn an_input(
    obligations: Vec<Obligation>,
    bindings: Vec<Binding>,
    runs: Vec<RunRecord>,
) -> AuditInput {
    AuditInput {
        obligations,
        bindings,
        runs,
        head_commit: Some(COMMIT.to_owned()),
        mock_inspection_suites: Some(vec!["unit".to_owned()]),
        ..AuditInput::default()
    }
}

/// The `stale-evidence` findings naming this obligation.
fn stale_findings<'a>(report: &'a AuditReport, obligation: &str) -> Vec<&'a Finding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.kind == "stale-evidence" && finding.obligation == obligation)
        .collect()
}

/// The one `stale-evidence` finding naming this obligation, or a failure
/// naming why there was not exactly one.
fn the_stale_finding<'a>(report: &'a AuditReport, obligation: &str) -> &'a Finding {
    let findings = stale_findings(report, obligation);
    let [only] = findings.as_slice() else {
        panic!(
            "exactly one stale-evidence finding must name {obligation}, saw {} ({:?})",
            findings.len(),
            findings
                .iter()
                .map(|f| f.summary.as_str())
                .collect::<Vec<_>>()
        );
    };
    only
}

/// A pass-then-fail run pair: the criterion leaves `healthy` and a
/// `stale-evidence` finding names the failing suite, symbol and commit at
/// `high` severity.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_a_pass_then_fail_run_raises_stale_evidence() {
    let obligation = an_obligation("FR-900-AC-1");
    let binding = a_binding("FR-900-AC-1", "unit", "tests::tc_900");
    let run = a_run_at_head("unit", vec![("tests::tc_900", Outcome::Fail)]);
    let input = an_input(vec![obligation], vec![binding], vec![run]);

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        !report.healthy.contains(&"FR-900-AC-1".to_owned()),
        "a failed tagged test must not read as healthy: {:?}",
        report.healthy
    );
    let finding = the_stale_finding(&report, "FR-900-AC-1");
    assert_eq!(
        finding.severity,
        Some(Severity::high()),
        "AC-17 requires high severity, saw {:?}",
        finding.severity
    );
    assert!(
        finding.summary.contains("unit:tests::tc_900"),
        "the reason must name the failing suite and symbol: {}",
        finding.summary
    );
    assert!(
        finding.summary.contains(&COMMIT[..12]),
        "the reason must name the failing commit: {}",
        finding.summary
    );
    assert!(
        finding.summary.contains("fail"),
        "the reason must name the outcome: {}",
        finding.summary
    );
}

/// A pass-then-error run pair: `Error` is treated the same as `Fail`.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_a_pass_then_error_run_raises_stale_evidence() {
    let obligation = an_obligation("FR-901-AC-1");
    let binding = a_binding("FR-901-AC-1", "unit", "tests::tc_901");
    let run = a_run_at_head("unit", vec![("tests::tc_901", Outcome::Error)]);
    let input = an_input(vec![obligation], vec![binding], vec![run]);

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        !report.healthy.contains(&"FR-901-AC-1".to_owned()),
        "an errored tagged test must not read as healthy: {:?}",
        report.healthy
    );
    let finding = the_stale_finding(&report, "FR-901-AC-1");
    assert_eq!(finding.severity, Some(Severity::high()));
    assert!(
        finding.summary.contains("unit:tests::tc_901"),
        "the reason must name the erroring suite and symbol: {}",
        finding.summary
    );
    assert!(
        finding.summary.contains(&COMMIT[..12]),
        "the reason must name the erroring commit: {}",
        finding.summary
    );
    assert!(
        finding.summary.contains("error"),
        "the reason must name the error outcome: {}",
        finding.summary
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
    let run = a_run_at_head("unit", vec![("tests::tc_902", Outcome::Pass)]);
    let input = an_input(vec![obligation], vec![binding], vec![run]);

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
    let run = a_run_at_head(
        "unit",
        vec![
            ("tests::tc_903_watched", Outcome::Pass),
            ("tests::tc_903_other", Outcome::Fail),
        ],
    );
    let input = an_input(
        vec![watched, other],
        vec![watched_binding, other_binding],
        vec![run],
    );

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

/// Two runs of the same suite, resolved through the store's own `latest_runs`
/// — the path production uses, not `Indexed::of`'s insertion order — with the
/// newer one failing: the criterion is stale, proving the auditor reads the
/// newest run, not whichever run sorts last in the input `Vec`.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_two_runs_pass_then_newer_fail_flags_stale() {
    let obligation = an_obligation("FR-904-AC-1");
    let binding = a_binding("FR-904-AC-1", "unit", "tests::tc_904");

    let mut store = MemoryEvidence::new();
    let older = a_run(
        "unit",
        OLDER_COMMIT,
        "2026-09-01T00:00:00Z",
        vec![("tests::tc_904", Outcome::Pass)],
    );
    let newer = a_run(
        "unit",
        COMMIT,
        "2026-09-27T00:00:00Z",
        vec![("tests::tc_904", Outcome::Fail)],
    );
    write_run(&mut store, &older).expect("the store accepts a well-formed run");
    write_run(&mut store, &newer).expect("the store accepts a well-formed run");

    let mut skipped = Vec::new();
    let runs = latest_runs(&store, &mut skipped).expect("the store reads back what it holds");
    assert!(
        skipped.is_empty(),
        "no run should be unreadable: {skipped:?}"
    );
    assert_eq!(runs.len(), 1, "one suite has one latest run");

    let input = an_input(vec![obligation], vec![binding], runs);
    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        !report.healthy.contains(&"FR-904-AC-1".to_owned()),
        "the newer, failing run must decide, not the older passing one: {:?}",
        report.healthy
    );
    let finding = the_stale_finding(&report, "FR-904-AC-1");
    assert_eq!(finding.severity, Some(Severity::high()));
    assert!(finding.summary.contains(&COMMIT[..12]));
}

/// The reciprocal: an older failing run superseded by a newer passing run of
/// the same suite must stay healthy. Kills the "oldest run wins" mutant from
/// the other side.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_two_runs_fail_then_newer_pass_stays_healthy() {
    let obligation = an_obligation("FR-905-AC-1");
    let binding = a_binding("FR-905-AC-1", "unit", "tests::tc_905");

    let mut store = MemoryEvidence::new();
    let older = a_run(
        "unit",
        OLDER_COMMIT,
        "2026-09-01T00:00:00Z",
        vec![("tests::tc_905", Outcome::Fail)],
    );
    let newer = a_run(
        "unit",
        COMMIT,
        "2026-09-27T00:00:00Z",
        vec![("tests::tc_905", Outcome::Pass)],
    );
    write_run(&mut store, &older).expect("the store accepts a well-formed run");
    write_run(&mut store, &newer).expect("the store accepts a well-formed run");

    let mut skipped = Vec::new();
    let runs = latest_runs(&store, &mut skipped).expect("the store reads back what it holds");
    assert!(skipped.is_empty());
    assert_eq!(runs.len(), 1);

    let input = an_input(vec![obligation], vec![binding], runs);
    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        report.healthy.contains(&"FR-905-AC-1".to_owned()),
        "the newer, passing run must decide, not the older failing one: {:?}",
        report.healthy
    );
    assert!(stale_findings(&report, "FR-905-AC-1").is_empty());
}

/// A run that is both behind HEAD and failing reports only the high
/// failed-run finding, not a second medium behind-HEAD one: both are
/// `stale-evidence` and share one ratchet key, and the failed-run rung's
/// early return means the ladder never reaches the behind-HEAD rung for this
/// obligation.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_a_failed_run_behind_head_reports_only_the_high_finding() {
    let obligation = an_obligation("FR-906-AC-1");
    let binding = a_binding("FR-906-AC-1", "unit", "tests::tc_906");
    // Recorded at OLDER_COMMIT, not COMMIT (HEAD) — behind HEAD AND failing.
    let run = a_run(
        "unit",
        OLDER_COMMIT,
        "2026-09-01T00:00:00Z",
        vec![("tests::tc_906", Outcome::Fail)],
    );
    let input = an_input(vec![obligation], vec![binding], vec![run]);

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(!report.healthy.contains(&"FR-906-AC-1".to_owned()));
    let findings = stale_findings(&report, "FR-906-AC-1");
    assert_eq!(
        findings.len(),
        1,
        "a run both behind HEAD and failing must report exactly one \
         stale-evidence finding, saw {}: {:?}",
        findings.len(),
        findings
            .iter()
            .map(|f| f.summary.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        findings[0].severity,
        Some(Severity::high()),
        "the high failed-run finding must win, not the medium behind-HEAD one, saw {:?}",
        findings[0].severity
    );
    assert!(findings[0].summary.contains("fail"));
}

/// The auditor's own index (`Indexed::of`) keeps the LAST run supplied for a
/// suite, matching its own documented contract ("a later entry for the same
/// suite replaces an earlier one"). Unlike the two tests above, this
/// constructs `AuditInput.runs` directly with two entries for one suite,
/// bypassing the store, so it pins the index's own resolution rather than
/// the store's — a real caller could otherwise assemble bindings from a
/// pre-`latest_runs` list and get this wrong silently.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_the_index_keeps_the_last_run_supplied_for_a_suite() {
    let obligation = an_obligation("FR-907-AC-1");
    let binding = a_binding("FR-907-AC-1", "unit", "tests::tc_907");
    let older_pass = a_run(
        "unit",
        OLDER_COMMIT,
        "2026-09-01T00:00:00Z",
        vec![("tests::tc_907", Outcome::Pass)],
    );
    let newer_fail = a_run(
        "unit",
        COMMIT,
        "2026-09-27T00:00:00Z",
        vec![("tests::tc_907", Outcome::Fail)],
    );
    // Older first, newer last — the order the index is documented to resolve.
    let input = an_input(
        vec![obligation],
        vec![binding],
        vec![older_pass, newer_fail],
    );

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        !report.healthy.contains(&"FR-907-AC-1".to_owned()),
        "the LAST-supplied run must decide, not the first: {:?}",
        report.healthy
    );
    let finding = the_stale_finding(&report, "FR-907-AC-1");
    assert_eq!(finding.severity, Some(Severity::high()));
}

/// The reciprocal: a failing run followed, in supply order, by a passing one
/// for the same suite must stay healthy. Kills a mutant that made the index
/// keep the FIRST run instead of the last.
///
/// Trace: FR-032-AC-17
#[test]
fn tc_1963_the_index_keeps_the_last_run_supplied_for_a_suite_reciprocal() {
    let obligation = an_obligation("FR-908-AC-1");
    let binding = a_binding("FR-908-AC-1", "unit", "tests::tc_908");
    let older_fail = a_run(
        "unit",
        OLDER_COMMIT,
        "2026-09-01T00:00:00Z",
        vec![("tests::tc_908", Outcome::Fail)],
    );
    let newer_pass = a_run(
        "unit",
        COMMIT,
        "2026-09-27T00:00:00Z",
        vec![("tests::tc_908", Outcome::Pass)],
    );
    let input = an_input(
        vec![obligation],
        vec![binding],
        vec![older_fail, newer_pass],
    );

    let report = audit(&input).expect("a well-formed input never refuses");

    assert!(
        report.healthy.contains(&"FR-908-AC-1".to_owned()),
        "the LAST-supplied, passing run must decide: {:?}",
        report.healthy
    );
    assert!(stale_findings(&report, "FR-908-AC-1").is_empty());
}
