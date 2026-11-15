// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed cause of `stale-evidence` findings: `missing-run` or `failed-run`.
//!
//! FR-032-AC-18..21 set the typed cause the auditor emits, and FR-032-AC-20
//! requires deterministic selection: at most one stale-evidence finding per
//! obligation, the first stale suite in bytewise order.
//!
//! Provenance: PLAT-1760

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_auditor::audit::{AuditInput, audit};
use quoin_evidence::ids::{ObligationId, SuiteId, SymbolId};
use quoin_evidence::types::{Binding, Outcome, RunEntry, RunRecord, STORE_SCHEMA_VERSION};
use quoin_finding_types::{AuditReport, Finding, FindingKind, Severity, StaleCause};
use quoin_quire_types::Obligation;

fn an_obligation(id: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        identity: None,
        statement: "the system SHALL do the thing".to_owned(),
        target_ids: None,
        method: None,
        criticality: None,
        parameters: None,
    }
}

fn a_binding(obligation: &str, suite: &str, symbol: &str) -> Binding {
    Binding {
        obligation: ObligationId::new(obligation),
        suite: SuiteId::new(suite),
        symbols: vec![SymbolId::new(symbol)],
        lineage: None,
    }
}

fn a_run(suite: &str, timestamp: &str, entries: Vec<(&str, Outcome)>) -> RunRecord {
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: "cargo-test".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        entries: entries
            .into_iter()
            .map(|(symbol, outcome)| RunEntry::new(symbol, outcome))
            .collect(),
    }
}

fn an_input(
    obligations: Vec<Obligation>,
    bindings: Vec<Binding>,
    runs: Vec<RunRecord>,
) -> AuditInput {
    AuditInput {
        obligations,
        bindings,
        runs,
        mock_inspection_suites: Some(vec!["unit".to_owned(), "integration".to_owned()]),
        ..AuditInput::default()
    }
}

fn stale_findings<'a>(report: &'a AuditReport, obligation: &str) -> Vec<&'a Finding> {
    report
        .findings
        .iter()
        .filter(|finding| {
            finding.kind == FindingKind::STALE_EVIDENCE && finding.obligation == obligation
        })
        .collect()
}

/// A binding whose suite has no recorded run yields a `stale-evidence` finding
/// with `cause` `missing-run` (FR-032-AC-18).
#[trace("FR-032-AC-18")]
#[test]
fn tc_1760_missing_run_yields_missing_run_cause() {
    let obligation = an_obligation("FR-1760-AC-1");
    let binding = a_binding("FR-1760-AC-1", "unit", "tests::tc_1760");
    let input = an_input(vec![obligation], vec![binding], vec![]);

    let report = audit(&input).expect("a well-formed input never refuses");

    let findings = stale_findings(&report, "FR-1760-AC-1");
    assert_eq!(findings.len(), 1, "exactly one stale-evidence finding");
    let finding = findings[0];
    assert_eq!(
        finding.cause,
        Some(StaleCause::MissingRun),
        "the cause must be missing-run, saw {:?}",
        finding.cause
    );
    assert_eq!(finding.severity, Some(Severity::high()));
}

/// A binding whose suite's newest run reports `fail` for a bound symbol yields
/// a `stale-evidence` finding with `cause` `failed-run` (FR-032-AC-19).
#[trace("FR-032-AC-19")]
#[test]
fn tc_1760_failed_run_yields_failed_run_cause() {
    let obligation = an_obligation("FR-1760-AC-2");
    let binding = a_binding("FR-1760-AC-2", "unit", "tests::tc_1760");
    let run = a_run(
        "unit",
        "2026-11-15T00:00:00Z",
        vec![("tests::tc_1760", Outcome::Fail)],
    );
    let input = an_input(vec![obligation], vec![binding], vec![run]);

    let report = audit(&input).expect("a well-formed input never refuses");

    let findings = stale_findings(&report, "FR-1760-AC-2");
    assert_eq!(findings.len(), 1, "exactly one stale-evidence finding");
    let finding = findings[0];
    assert_eq!(
        finding.cause,
        Some(StaleCause::FailedRun),
        "the cause must be failed-run, saw {:?}",
        finding.cause
    );
    assert_eq!(finding.severity, Some(Severity::high()));
}

/// A binding whose suite's newest run reports `error` for a bound symbol also
/// yields `failed-run` (FR-032-AC-19 treats Fail and Error identically).
#[trace("FR-032-AC-19")]
#[test]
fn tc_1760_errored_run_yields_failed_run_cause() {
    let obligation = an_obligation("FR-1760-AC-3");
    let binding = a_binding("FR-1760-AC-3", "unit", "tests::tc_1760");
    let run = a_run(
        "unit",
        "2026-11-15T00:00:00Z",
        vec![("tests::tc_1760", Outcome::Error)],
    );
    let input = an_input(vec![obligation], vec![binding], vec![run]);

    let report = audit(&input).expect("a well-formed input never refuses");

    let findings = stale_findings(&report, "FR-1760-AC-3");
    assert_eq!(findings.len(), 1, "exactly one stale-evidence finding");
    let finding = findings[0];
    assert_eq!(
        finding.cause,
        Some(StaleCause::FailedRun),
        "the cause must be failed-run for an errored run, saw {:?}",
        finding.cause
    );
}

/// An obligation bound to one suite with no run and another whose newest run
/// failed yields exactly one `stale-evidence` finding: the first stale suite
/// in bytewise suite-name order determines the finding and its cause
/// (FR-032-AC-20).
#[trace("FR-032-AC-20")]
#[test]
fn tc_1760_deterministic_stale_suite_selection_bytewise() {
    let obligation = an_obligation("FR-1760-AC-4");
    // "integration" comes before "unit" bytewise
    let binding_missing = a_binding("FR-1760-AC-4", "integration", "tests::tc_integration");
    let binding_failed = a_binding("FR-1760-AC-4", "unit", "tests::tc_unit");
    let run_failed = a_run(
        "unit",
        "2026-11-15T00:00:00Z",
        vec![("tests::tc_unit", Outcome::Fail)],
    );
    let input = an_input(
        vec![obligation],
        vec![binding_missing, binding_failed],
        vec![run_failed],
    );

    let report = audit(&input).expect("a well-formed input never refuses");

    let findings = stale_findings(&report, "FR-1760-AC-4");
    assert_eq!(
        findings.len(),
        1,
        "at most one stale-evidence finding per obligation, saw {} ({:?})",
        findings.len(),
        findings
            .iter()
            .map(|f| f.summary.as_str())
            .collect::<Vec<_>>()
    );
    let finding = findings[0];
    assert_eq!(
        finding.cause,
        Some(StaleCause::MissingRun),
        "the first stale suite bytewise (integration) has cause missing-run, saw {:?}",
        finding.cause
    );
    assert!(
        finding.summary.contains("integration"),
        "the finding must name the first stale suite: {}",
        finding.summary
    );
}

/// The reciprocal: when "unit" comes before "integration" bytewise and "unit"
/// has the failed run while "integration" is missing, "unit" is selected.
#[trace("FR-032-AC-20")]
#[test]
fn tc_1760_deterministic_stale_suite_selection_failed_first() {
    let obligation = an_obligation("FR-1760-AC-5");
    let binding_missing = a_binding("FR-1760-AC-5", "z-suite", "tests::tc_z");
    let binding_failed = a_binding("FR-1760-AC-5", "a-suite", "tests::tc_a");
    let run_failed = a_run(
        "a-suite",
        "2026-11-15T00:00:00Z",
        vec![("tests::tc_a", Outcome::Fail)],
    );
    let input = an_input(
        vec![obligation],
        vec![binding_missing, binding_failed],
        vec![run_failed],
    );

    let report = audit(&input).expect("a well-formed input never refuses");

    let findings = stale_findings(&report, "FR-1760-AC-5");
    assert_eq!(findings.len(), 1, "at most one stale-evidence finding");
    let finding = findings[0];
    assert_eq!(
        finding.cause,
        Some(StaleCause::FailedRun),
        "the first stale suite bytewise (a-suite) has cause failed-run, saw {:?}",
        finding.cause
    );
    assert!(
        finding.summary.contains("a-suite"),
        "the finding must name the first stale suite: {}",
        finding.summary
    );
}

/// A finding of any kind other than `stale-evidence` serializes with no `cause`
/// key, and `cause` changes no `summary`, severity, finding order or healthy set
/// (FR-032-AC-21).
#[trace("FR-032-AC-21")]
#[test]
fn tc_1760_non_stale_findings_have_no_cause() {
    let obligation = an_obligation("FR-1760-AC-6");
    let input = an_input(vec![obligation.clone()], vec![], vec![]);

    let report = audit(&input).expect("a well-formed input never refuses");

    let undischarged = report
        .findings
        .iter()
        .find(|finding| {
            finding.kind == FindingKind::UNDISCHARGED && finding.obligation == "FR-1760-AC-6"
        })
        .expect("an unbound obligation yields undischarged");
    assert_eq!(
        undischarged.cause, None,
        "an undischarged finding must have no cause, saw {:?}",
        undischarged.cause
    );
    // Serialize and verify the JSON has no `cause` key
    let json = serde_json::to_value(undischarged).expect("serializes");
    assert!(
        json.get("cause").is_none(),
        "the cause key must not appear in JSON for non-stale findings: {json}"
    );
}

/// Stripping `cause` from an audit report leaves exactly the report the auditor
/// produced before the field existed: the same summaries, severities, finding
/// order and healthy set, written out here as literals (FR-032-AC-21).
#[trace("FR-032-AC-21")]
#[test]
fn tc_1760_cause_leaves_summaries_severity_order_and_healthy_set_unchanged() {
    let input = an_input(
        vec![
            an_obligation("FR-1760-AC-7a"),
            an_obligation("FR-1760-AC-7b"),
            an_obligation("FR-1760-AC-7c"),
        ],
        vec![
            a_binding("FR-1760-AC-7a", "gone", "tests::a"),
            a_binding("FR-1760-AC-7b", "broken", "tests::b"),
            a_binding("FR-1760-AC-7c", "unit", "tests::c"),
        ],
        vec![
            a_run(
                "broken",
                "2026-11-15T00:00:00Z",
                vec![("tests::b", Outcome::Fail)],
            ),
            a_run(
                "unit",
                "2026-11-15T00:00:00Z",
                vec![("tests::c", Outcome::Pass)],
            ),
        ],
    );

    let report = audit(&input).expect("a well-formed input never refuses");

    // The causes are present, so the comparison below is not vacuous.
    let causes: Vec<Option<StaleCause>> = report.findings.iter().map(|f| f.cause).collect();
    assert!(causes.contains(&Some(StaleCause::MissingRun)), "{causes:?}");
    assert!(causes.contains(&Some(StaleCause::FailedRun)), "{causes:?}");

    let mut baseline = report.clone();
    for finding in &mut baseline.findings {
        finding.cause = None;
    }
    let baseline_json = serde_json::to_value(&baseline).expect("serializes");
    let with_cause_json = serde_json::to_value(&report).expect("serializes");

    let project = |value: &serde_json::Value| -> Vec<(String, String, String, String)> {
        value["findings"]
            .as_array()
            .expect("findings is an array")
            .iter()
            .map(|f| {
                (
                    f["obligation"].as_str().unwrap_or_default().to_owned(),
                    f["kind"].as_str().unwrap_or_default().to_owned(),
                    f["severity"].as_str().unwrap_or_default().to_owned(),
                    f["summary"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    };
    assert_eq!(project(&with_cause_json), project(&baseline_json));
    assert_eq!(with_cause_json["healthy"], baseline_json["healthy"]);

    // Literals: the summaries and severity, in the order the auditor lists them.
    assert_eq!(
        project(&baseline_json),
        [
            (
                "FR-1760-AC-7a".to_owned(),
                "stale-evidence".to_owned(),
                "high".to_owned(),
                "FR-1760-AC-7a is bound to gone, which has no recorded run. The binding claims \
                 evidence that is not in the store."
                    .to_owned()
            ),
            (
                "FR-1760-AC-7b".to_owned(),
                "stale-evidence".to_owned(),
                "high".to_owned(),
                "FR-1760-AC-7b is bound to broken:tests::b, which reported failed in the recorded run. \
                 The binding claims evidence that does not hold."
                    .to_owned()
            ),
        ]
    );
    assert_eq!(
        baseline_json["healthy"],
        serde_json::json!(["FR-1760-AC-7c"])
    );
}

/// Within a failing suite, the first failing symbol in bytewise order determines
/// the finding (FR-032-AC-20).
#[trace("FR-032-AC-20")]
#[test]
fn tc_1760_deterministic_failing_symbol_selection() {
    let obligation = an_obligation("FR-1760-AC-7");
    let binding = Binding {
        obligation: ObligationId::new("FR-1760-AC-7"),
        suite: SuiteId::new("unit"),
        symbols: vec![SymbolId::new("tests::tc_z"), SymbolId::new("tests::tc_a")],
        lineage: None,
    };
    let run = a_run(
        "unit",
        "2026-11-15T00:00:00Z",
        vec![
            ("tests::tc_z", Outcome::Fail),
            ("tests::tc_a", Outcome::Fail),
        ],
    );
    let input = an_input(vec![obligation], vec![binding], vec![run]);

    let report = audit(&input).expect("a well-formed input never refuses");

    let findings = stale_findings(&report, "FR-1760-AC-7");
    assert_eq!(findings.len(), 1, "exactly one stale-evidence finding");
    let finding = findings[0];
    assert_eq!(finding.cause, Some(StaleCause::FailedRun));
    assert!(
        finding.summary.contains("tests::tc_a"),
        "the finding must name the first failing symbol bytewise: {}",
        finding.summary
    );
    assert!(
        !finding.summary.contains("tests::tc_z"),
        "the finding must not name the second failing symbol: {}",
        finding.summary
    );
}
