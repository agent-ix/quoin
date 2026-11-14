// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! TC-002 steps 3 to 6: an analysis result counts toward an obligation only as
//! a qualified passing record (FR-153, FR-184), through the engine's retained
//! evidence reconciliation.
//!
//! Runs are recorded into an in-memory store exactly as `quoin evidence record
//! --discharges` stores them: the `differential-report` adapter's entry, with
//! the stated obligation on its `traceIds`, bound by `record_run`. The FR-093
//! assessment is built in the test through the trust-decision mechanism; no
//! real decision is recorded and none exists, so the shipped default is
//! `unevaluated` (FR-184-AC-1).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use std::path::PathBuf;

use ix_trace_rs::trace;
use quire_rs::coverage::CoverageMatrixStatus;
use quire_rs::evidence::{CauseCode, strict_decision};
use quoin_evidence::adapters::parse_differential_report;
use quoin_evidence::qualify::{
    ActorAdmission, CiAssociation, ExecutionAssociation, HostObservations, POSITIVE_USE_DECISION,
    RetainedRun, RunObservation,
};
use quoin_evidence::record::{RecordRequest, record_run_with_analysis_projection};
use quoin_evidence::store::read_bindings;
use quoin_evidence::types::{
    Binding, ProducerContext, RunRecord, TrustDecision, TrustDecisionKind, TrustTrigger, TrustUse,
};
use quoin_evidence::{
    MemoryEvidence, Outcome, RunEntry, STORE_SCHEMA_VERSION, SuiteId, TrustDecisionId,
};
use quoin_quire::coverage::Request as ScopeRequest;
use quoin_quire::evidence::{Outcome as Counted, Query, Request, compute};
use quoin_quire::{ModuleSelection, ids};
use serde_json::Value;

const SATISFIED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json"
);
const REFUTED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-refuted.json"
);
const DISAGREEMENT: &str =
    include_str!("../../../../tests/fixtures/evidence/differential-report-v2/disagreement.json");
const INCONCLUSIVE: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/inconclusive-timeout.json"
);
const UNAVAILABLE: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/unavailable-missing-executable.json"
);

const AC1: &str = "FR-001-AC-1";
const AC2: &str = "FR-001-AC-2";
const AC3: &str = "FR-001-AC-3";
const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("analysis")
}

fn scope() -> ScopeRequest {
    ScopeRequest {
        scope: ids::ScopeRoot::open(fixture()).unwrap(),
        modules: ModuleSelection::Closed(vec![ids::ModuleRoot::open(fixture()).unwrap()]),
    }
}

fn obligations() -> Vec<quoin_quire_types::Obligation> {
    [(AC1, "Analysis"), (AC2, "Analysis"), (AC3, "Test")]
        .into_iter()
        .map(|(id, method)| quoin_quire_types::Obligation {
            id: id.to_owned(),
            identity: None,
            statement: "stated".to_owned(),
            target_ids: None,
            method: Some(method.to_owned()),
            criticality: None,
            parameters: None,
        })
        .collect()
}

/// An FR-093 decision from the trust-decision mechanism for `producer`,
/// permitting the positive use. Built here; no decision file exists in any
/// store.
fn decision(producer: (&str, &str), limitations: &[&str]) -> TrustDecision {
    let context = ProducerContext {
        name: producer.0.to_owned(),
        version: producer.1.to_owned(),
        configuration: "run-package".to_owned(),
        validation_corpus: "tc-001-captures".to_owned(),
        input_contract: "quire.differential-report/v2".to_owned(),
        environment: "ci".to_owned(),
        adapter: None,
    };
    TrustDecision {
        schema_version: STORE_SCHEMA_VERSION,
        id: TrustDecisionId::parse("ETD-1").unwrap(),
        r#use: TrustUse {
            id: "U-analysis".to_owned(),
            intended_function: "consistency analysis".to_owned(),
            permitted_decisions: vec![POSITIVE_USE_DECISION.to_owned()],
        },
        decision: TrustDecisionKind::ReliedUpon,
        accepted_context: context.clone(),
        observed_context: Some(context),
        revalidate_on: vec![
            TrustTrigger::ProducerVersion,
            TrustTrigger::Configuration,
            TrustTrigger::ValidationCorpus,
            TrustTrigger::InputContract,
            TrustTrigger::Environment,
        ],
        validation_evidence: vec![quoin_evidence::types::TrustEvidenceReference {
            id: "V-1".to_owned(),
            reference: Some("evidence/v-1".to_owned()),
        }],
        limitations: limitations.iter().map(|text| (*text).to_owned()).collect(),
        owner: "owner".to_owned(),
        decided_at: "2026-11-13T00:00:00Z".to_owned(),
    }
}

fn projection(report: &Value) -> Value {
    let engines = report["engines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|engine| {
            let mut projected = serde_json::Map::from_iter([
                ("engine".to_owned(), engine["engine"].clone()),
                (
                    "configured_executable".to_owned(),
                    engine["configuredExecutable"].clone(),
                ),
                ("status".to_owned(), engine["status"].clone()),
                ("outcome".to_owned(), engine["outcome"].clone()),
            ]);
            if let Some(identity) = engine.get("identity") {
                if let Some(path) = identity.get("path") {
                    projected.insert("identity_path".to_owned(), path.clone());
                }
                if let Some(version) = identity.get("version") {
                    projected.insert("identity_version".to_owned(), version.clone());
                }
            }
            Value::Object(projected)
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "protocol": "quoin.analysis-report-projection/v1",
        "disposition": report["differential"]["disposition"],
        "agreed_status": report["differential"]["agreedStatus"],
        "producer": report["producer"],
        "engines": engines,
    })
}

/// The host facts that belong to no one record, every premise admitted.
fn admitted() -> HostObservations {
    HostObservations {
        reliance: Some(decision(("quire-analyze", "0.1.0"), &[])),
        selected_head: Some(HEAD.to_owned()),
    }
}

/// What the host observed of the one act that produced `file`: its own CI
/// invocation, its own checkout SHA and its own admitted signer.
fn admitted_run(file: &str) -> RunObservation {
    RunObservation {
        performed_revision: Some(HEAD.to_owned()),
        execution: Some(ExecutionAssociation::Ci(CiAssociation {
            system: "github".to_owned(),
            project: "agent-ix/quoin".to_owned(),
            run: "100".to_owned(),
            attempt: "1".to_owned(),
            job: "analysis".to_owned(),
            invocation: format!("run-package:{file}"),
        })),
        actor: Some(ActorAdmission {
            actor: "signer-1".to_owned(),
            admission_ref: "admission-7".to_owned(),
            authorized: vec![AC1.to_owned(), AC2.to_owned(), AC3.to_owned()],
        }),
    }
}

/// An in-memory store and the report each run was transcribed from.
struct Recorded {
    source: MemoryEvidence,
    reports: Vec<(String, Value)>,
}

impl Recorded {
    fn new() -> Self {
        Self {
            source: MemoryEvidence::new(),
            reports: Vec::new(),
        }
    }

    /// `quoin evidence record --adapter differential-report --discharges`.
    fn record(&mut self, suite: &str, timestamp: &str, report: &str, discharges: &[&str]) {
        let mut result = parse_differential_report(report).unwrap();
        let projection = result.analysis_projection.take().unwrap();
        let mut entries = result.entries;
        for entry in &mut entries {
            if !discharges.is_empty() {
                entry.trace_ids = Some(discharges.iter().map(|id| (*id).to_owned()).collect());
            }
        }
        let outcome = record_run_with_analysis_projection(
            &mut self.source,
            &RecordRequest {
                suite: SuiteId::new(suite),
                tool: "quire-analyze 0.1.0".to_owned(),
                evidence_kind: None,
                lineage: None,
                timestamp: timestamp.to_owned(),
                entries,
            },
            &obligations(),
            &projection,
        )
        .unwrap();
        self.reports.push((outcome.run_path, projection));
    }

    /// One hand-built entry on a symbol, the way a later run of the same
    /// request reports it.
    fn record_entry(
        &mut self,
        suite: &str,
        timestamp: &str,
        (symbol, outcome): (&str, Outcome),
        discharges: &[&str],
        report: &Value,
    ) {
        let mut entry = RunEntry::new(symbol, outcome);
        entry.trace_ids = Some(discharges.iter().map(|id| (*id).to_owned()).collect());
        let projection = projection(report);
        let outcome = record_run_with_analysis_projection(
            &mut self.source,
            &RecordRequest {
                suite: SuiteId::new(suite),
                tool: "quire-analyze 0.1.0".to_owned(),
                evidence_kind: None,
                lineage: None,
                timestamp: timestamp.to_owned(),
                entries: vec![entry],
            },
            &obligations(),
            &projection,
        )
        .unwrap();
        self.reports.push((outcome.run_path, projection));
    }

    fn runs(&self) -> Vec<RetainedRun> {
        use quoin_evidence::EvidenceSource;
        self.reports
            .iter()
            .map(|(path, report)| RetainedRun {
                file: path.clone(),
                record: serde_json::from_str::<RunRecord>(
                    &self.source.read(path).unwrap().unwrap(),
                )
                .unwrap(),
                report: Some(report.clone()),
                observation: admitted_run(path),
            })
            .collect()
    }

    fn bindings(&self) -> Vec<Binding> {
        read_bindings(&self.source).unwrap().bindings
    }

    fn count(&self, requested: &[&str], host: HostObservations) -> Counted {
        count(self.runs(), self.bindings(), requested, host)
    }

    /// As [`Self::count`], with each run's own observation changed by `tweak`.
    fn count_with(
        &self,
        requested: &[&str],
        host: HostObservations,
        tweak: impl Fn(&mut RunObservation),
    ) -> Counted {
        let mut runs = self.runs();
        for run in &mut runs {
            tweak(&mut run.observation);
        }
        count(runs, self.bindings(), requested, host)
    }
}

fn count(
    runs: Vec<RetainedRun>,
    bindings: Vec<Binding>,
    requested: &[&str],
    host: HostObservations,
) -> Counted {
    // The host inspected every suite for mocks and found none.
    let suites: std::collections::BTreeSet<String> = runs
        .iter()
        .map(|run| run.record.suite.as_str().to_owned())
        .collect();
    compute(&Request {
        scope: scope(),
        requested: requested.iter().map(|id| (*id).to_owned()).collect(),
        runs,
        bindings,
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(suites.into_iter().collect()),
        host,
    })
    .unwrap()
}

fn status(outcome: &Counted, id: &str) -> CoverageMatrixStatus {
    outcome
        .report
        .coverage_matrix
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .find(|criterion| criterion.id == id)
        .unwrap()
        .status
}

fn causes(outcome: &Counted) -> Vec<(CauseCode, String)> {
    outcome
        .report
        .evidence_results
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|result| (result.cause, result.detail))
        .collect()
}

fn unbacked(outcome: &Counted) -> Vec<String> {
    outcome
        .report
        .unbacked_rows
        .iter()
        .filter_map(|row| row.row_id.clone())
        .collect()
}

fn counted(outcome: &Counted) -> usize {
    outcome.report.record_binders.as_ref().map_or(0, Vec::len)
}

/// A satisfied run in a fresh store, discharging `ids`.
fn satisfied_store(ids: &[&str]) -> Recorded {
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, ids);
    store
}

#[trace("FR-153-AC-1", "FR-184-AC-2", "FR-184-AC-13", "FR-184-AC-15")]
#[test]
fn an_admitted_passing_report_is_a_qualified_record_for_its_own_target_only() {
    let outcome = satisfied_store(&[AC1]).count(&[AC1], admitted());

    assert_eq!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert!(!unbacked(&outcome).contains(&AC1.to_owned()));
    assert_ne!(
        status(&outcome, AC2),
        CoverageMatrixStatus::QualifiedRecord,
        "a sibling is not credited"
    );
    assert!(unbacked(&outcome).contains(&AC2.to_owned()));

    let binders = outcome.report.record_binders.as_ref().unwrap();
    assert_eq!(binders.len(), 1);
    let binder = &binders[0];
    assert_eq!(binder.target.id, AC1);
    assert_eq!(binder.method, "Analysis");
    assert_eq!(binder.actor, "signer-1");
    assert!(binder.locator.record.starts_with("runs/SUITE-A/"));
    assert_eq!(binder.locator.run, "SUITE-A@2026-11-13T10:00:00Z");
    assert!(binder.locator.entry.starts_with("consistency::"));
    assert!(
        binder
            .provenance
            .contains(&"credit:consistency-evidence-only".to_owned())
    );
    assert!(
        binder
            .provenance
            .iter()
            .all(|reference| !reference.contains("relevan")),
        "no relevance claim is made"
    );
    // The siblings stay unbacked, so the strict decision still rejects on them.
    assert!(strict_decision(&outcome.report, &[]).is_err());
}

/// Assert the obligation is uncounted and one cause names `premise`.
fn assert_uncounted_naming(outcome: &Counted, cause: CauseCode, premise: &str) {
    assert_eq!(counted(outcome), 0);
    assert_ne!(status(outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert!(unbacked(outcome).contains(&AC1.to_owned()));
    assert!(
        causes(outcome)
            .iter()
            .any(|(found, detail)| *found == cause && detail.starts_with(premise)),
        "{premise}: {:?}",
        causes(outcome)
    );
    assert!(strict_decision(&outcome.report, &[]).is_err());
}

#[trace("FR-153-AC-2", "FR-184-AC-1", "FR-184-AC-14")]
#[test]
fn the_shipped_default_and_a_missing_checkout_sha_leave_the_obligation_unevaluated() {
    let store = satisfied_store(&[AC1]);
    let no_assessment = HostObservations {
        reliance: None,
        ..admitted()
    };
    let outcome = store.count(&[AC1], no_assessment);
    assert_uncounted_naming(&outcome, CauseCode::Unevaluated, "bounded reliance premise");

    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.performed_revision = None;
    });
    assert_uncounted_naming(
        &outcome,
        CauseCode::Unevaluated,
        "performed revision premise",
    );

    // What ships today: no assessment exists and the host supplies nothing.
    let outcome = compute(&Request {
        scope: scope(),
        requested: vec![AC1.to_owned()],
        runs: store
            .runs()
            .into_iter()
            .map(|run| RetainedRun {
                observation: RunObservation::default(),
                ..run
            })
            .collect(),
        bindings: store.bindings(),
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(vec!["SUITE-A".to_owned()]),
        host: HostObservations::default(),
    })
    .unwrap();
    assert_uncounted_naming(&outcome, CauseCode::Unevaluated, "bounded reliance premise");
}

#[trace("FR-153-AC-8")]
#[test]
fn a_passing_entry_with_a_healthy_audit_and_no_decision_is_not_counted() {
    let store = satisfied_store(&[AC1]);
    let latest: Vec<RunRecord> = store.runs().iter().map(|run| run.record.clone()).collect();
    // The audit this entry survives: nothing found, and healthy.
    let audit = quoin_auditor::audit(&quoin_auditor::AuditInput {
        obligations: obligations()
            .into_iter()
            .filter(|obligation| obligation.id == AC1)
            .collect(),
        bindings: store.bindings(),
        runs: latest,
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(vec!["SUITE-A".to_owned()]),
        ..quoin_auditor::AuditInput::default()
    })
    .unwrap();
    assert!(audit.findings.is_empty(), "{:?}", audit.findings);
    assert!(audit.healthy.contains(&AC1.to_owned()), "{audit:?}");

    // Healthy, bound and passing, but no FR-116 decision: not counted.
    let outcome = store.count(&[AC1], HostObservations::default());
    assert_eq!(counted(&outcome), 0);
    assert_ne!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert!(unbacked(&outcome).contains(&AC1.to_owned()));
}

#[trace("FR-153-AC-1", "FR-153-CON-3")]
#[test]
fn the_bridge_runs_the_fr_032_audit_and_a_finding_leaves_the_entry_uncounted() {
    let store = satisfied_store(&[AC1]);
    // A binding whose suite has no recorded run: the audit calls it stale.
    let mut bindings = store.bindings();
    bindings.push(Binding {
        obligation: quoin_evidence::ObligationId::new(AC1),
        suite: SuiteId::new("SUITE-GHOST"),
        symbols: vec![quoin_evidence::SymbolId::new("ghost")],
        lineage: None,
    });
    let outcome = count(store.runs(), bindings, &[AC1], admitted());
    assert_uncounted_naming(&outcome, CauseCode::Unevaluated, "FR-032 audit premise");
}

#[trace("FR-153-AC-1", "FR-116-AC-12")]
#[test]
fn without_the_hosts_mock_inspection_the_audit_cannot_call_the_obligation_healthy() {
    let store = satisfied_store(&[AC1]);
    let outcome = compute(&Request {
        scope: scope(),
        requested: vec![AC1.to_owned()],
        runs: store.runs(),
        bindings: store.bindings(),
        injections: None,
        mock_inspection_suites: None,
        host: admitted(),
    })
    .unwrap();
    assert_uncounted_naming(&outcome, CauseCode::Unevaluated, "FR-032 audit premise");
    assert!(
        causes(&outcome)
            .iter()
            .any(|(_, detail)| detail.contains("mocked-confirmation")),
        "the audit's unevaluated check is kept: {:?}",
        causes(&outcome)
    );
}

#[trace("FR-116-AC-7", "FR-116-AC-8")]
#[test]
fn an_observation_of_one_record_never_qualifies_another() {
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    store.record("SUITE-B", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    assert_eq!(counted(&store.count(&[AC1], admitted())), 2);

    let mut runs = store.runs();
    runs[1].observation.performed_revision = Some("old-sha".to_owned());
    let outcome = count(runs.clone(), store.bindings(), &[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    let wrong: Vec<_> = outcome
        .report
        .evidence_results
        .clone()
        .unwrap()
        .into_iter()
        .filter(|result| result.cause == CauseCode::WrongRevision)
        .collect();
    assert_eq!(wrong.len(), 1);
    assert_eq!(
        wrong[0].locator.as_ref().unwrap().record,
        runs[1].file,
        "the refusal names the record whose own revision differs"
    );

    // No observation of its own: unevaluated, whatever the other record has.
    runs[1].observation = RunObservation::default();
    let outcome = count(runs, store.bindings(), &[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    assert!(causes(&outcome).iter().any(|(cause, detail)| {
        *cause == CauseCode::Unevaluated && detail.starts_with("execution association premise")
    }));
}

#[trace("FR-116-AC-6", "FR-116-AC-18")]
#[test]
fn the_actor_needs_an_admission_reference_and_authority_over_the_target() {
    let store = satisfied_store(&[AC1]);
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.actor = None;
    });
    assert_uncounted_naming(&outcome, CauseCode::Unevaluated, "actor authority premise");
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        if let Some(actor) = &mut observation.actor {
            actor.admission_ref = String::new();
        }
    });
    assert_uncounted_naming(&outcome, CauseCode::Unevaluated, "actor authority premise");
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        if let Some(actor) = &mut observation.actor {
            actor.authorized = vec![AC2.to_owned()];
        }
    });
    assert_uncounted_naming(
        &outcome,
        CauseCode::QualificationRejected,
        "actor authority premise",
    );
    let counted_run = store.count(&[AC1], admitted());
    let binder = &counted_run.report.record_binders.as_ref().unwrap()[0];
    assert!(
        binder
            .provenance
            .contains(&"actor-authority:signer-1:admission-7".to_owned())
    );
}

#[trace("FR-116-AC-15")]
#[test]
fn the_reliance_decision_must_be_for_the_reports_producer() {
    let store = satisfied_store(&[AC1]);
    for producer in [("other-tool", "0.1.0"), ("quire-analyze", "9.9.9")] {
        let host = HostObservations {
            reliance: Some(decision(producer, &[])),
            ..admitted()
        };
        assert_uncounted_naming(
            &store.count(&[AC1], host),
            CauseCode::QualificationRejected,
            "bounded reliance premise",
        );
    }
    // A decision with no limitations is the positive case.
    assert_eq!(counted(&store.count(&[AC1], admitted())), 1);
}

#[trace("FR-116-AC-6")]
#[test]
fn a_reliance_decision_with_any_limitation_grants_no_positive_decision() {
    let store = satisfied_store(&[AC1]);
    // An excluding limitation, and one that looks harmless: both are free text
    // that cannot be shown not to exclude this use.
    for limitation in ["does not cover FR-001-AC-1", "Linux x86-64 only"] {
        let host = HostObservations {
            reliance: Some(decision(("quire-analyze", "0.1.0"), &[limitation])),
            ..admitted()
        };
        let outcome = store.count(&[AC1], host);
        assert_uncounted_naming(&outcome, CauseCode::Unevaluated, "bounded reliance premise");
        assert!(
            causes(&outcome)
                .iter()
                .any(|(_, detail)| detail.contains(limitation)),
            "the limitation text is kept: {:?}",
            causes(&outcome)
        );
    }
}

#[trace("FR-184-AC-10", "FR-184-AC-11", "FR-184-AC-12")]
#[test]
fn association_and_engine_observations_are_premises_of_the_decision() {
    let store = satisfied_store(&[AC1]);
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.execution = Some(ExecutionAssociation::Manual {
            attester: "someone".to_owned(),
            reason: "no CI".to_owned(),
        });
    });
    assert_uncounted_naming(
        &outcome,
        CauseCode::QualificationRejected,
        "execution association premise",
    );
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.execution = None;
    });
    assert_uncounted_naming(
        &outcome,
        CauseCode::Unevaluated,
        "execution association premise",
    );

    // An engine record missing what producer-execute observes is no
    // observation of a solver it started.
    for (change, label) in [
        (
            (&["engines", "1", "identity_version"][..], Value::from("")),
            "identity_version",
        ),
        (
            (&["engines", "1", "identity_path"][..], Value::from("")),
            "identity_path",
        ),
        (
            (
                &["engines", "0", "configured_executable"][..],
                Value::from("z3"),
            ),
            "relative configured executable",
        ),
    ] {
        let mut incomplete = Recorded::new();
        incomplete.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
        let mut target = &mut incomplete.reports[0].1;
        let (path, value) = change;
        for key in path {
            target = match key.parse::<usize>() {
                Ok(index) => &mut target[index],
                Err(_) => &mut target[*key],
            };
        }
        *target = value;
        let outcome = incomplete.count(&[AC1], admitted());
        assert_uncounted_naming(
            &outcome,
            CauseCode::Unevaluated,
            "tool and configuration premise",
        );
        assert!(
            !causes(&outcome).is_empty(),
            "{label} must leave a named cause"
        );
    }
}

#[trace("FR-153-AC-3", "FR-184-AC-3", "FR-184-AC-4", "FR-184-AC-5")]
#[test]
fn refuted_inconclusive_and_unavailable_reports_are_failed_or_errored_with_locators() {
    for (report, cause) in [
        (REFUTED, CauseCode::Failed),
        (INCONCLUSIVE, CauseCode::Errored),
        (UNAVAILABLE, CauseCode::Errored),
    ] {
        let mut store = Recorded::new();
        store.record("SUITE-A", "2026-11-13T10:00:00Z", report, &[AC1]);
        let outcome = store.count(&[AC1], admitted());
        assert_eq!(counted(&outcome), 0);
        assert!(unbacked(&outcome).contains(&AC1.to_owned()));
        let results = outcome.report.evidence_results.clone().unwrap();
        assert!(
            results
                .iter()
                .any(|result| result.cause == cause && result.locator.is_some()),
            "{cause:?}: {results:?}"
        );
        assert!(strict_decision(&outcome.report, &[]).is_err());
    }
}

#[trace("FR-153-AC-4", "FR-184-AC-6")]
#[test]
fn a_disagreement_is_absent_like_an_obligation_never_recorded() {
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-13T10:00:00Z", DISAGREEMENT, &[AC1]);
    let disagreed = store.count(&[AC1], admitted());
    let never = Recorded::new().count(&[AC1], admitted());
    for outcome in [&disagreed, &never] {
        assert_eq!(counted(outcome), 0);
        let results = outcome.report.evidence_results.clone().unwrap();
        assert!(
            results
                .iter()
                .all(|result| result.cause == CauseCode::Absent && result.locator.is_none()),
            "{results:?}"
        );
    }
    assert_eq!(causes(&disagreed), causes(&never));
}

/// The satisfied capture's own entry symbol.
const SATISFIED_SYMBOL: &str =
    "consistency::3dbffb15f695159ab9331b9708144a86b3845beb04bcc31767445b914065b1ae";

fn satisfied_report() -> Value {
    serde_json::from_str(SATISFIED).unwrap()
}

/// A pass, then a later run of the same request that says `later`.
fn pass_then(later: Option<Outcome>) -> Recorded {
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    match later {
        Some(outcome) => store.record_entry(
            "SUITE-A",
            "2026-11-13T11:00:00Z",
            (SATISFIED_SYMBOL, outcome),
            &[AC1],
            &satisfied_report(),
        ),
        None => store.record("SUITE-A", "2026-11-13T11:00:00Z", DISAGREEMENT, &[AC1]),
    }
    store
}

#[trace("FR-153-AC-5", "FR-184-AC-7", "FR-184-AC-8")]
#[test]
fn a_later_run_supersedes_a_pass_whatever_the_supply_order() {
    for (store, expected) in [
        (
            pass_then(Some(Outcome::Fail)),
            vec![CauseCode::Stale, CauseCode::Failed],
        ),
        (
            pass_then(Some(Outcome::Error)),
            vec![CauseCode::Stale, CauseCode::Errored],
        ),
        (pass_then(None), vec![CauseCode::Absent]),
    ] {
        let forward = store.count(&[AC1], admitted());
        let mut reversed_runs = store.runs();
        reversed_runs.reverse();
        let reversed = count(reversed_runs, store.bindings(), &[AC1], admitted());
        for outcome in [&forward, &reversed] {
            assert_eq!(counted(outcome), 0);
            let mut found: Vec<CauseCode> = causes(outcome).into_iter().map(|(c, _)| c).collect();
            found.sort();
            let mut want = expected.clone();
            want.sort();
            assert_eq!(found, want);
        }
        assert_eq!(causes(&forward), causes(&reversed));
    }
}

#[trace("FR-153-AC-5")]
#[test]
fn equal_timestamps_are_ordered_by_file_name() {
    let store = pass_then(Some(Outcome::Fail));
    let mut runs = store.runs();
    runs[1].record.timestamp = runs[0].record.timestamp.clone();
    runs[1].file = runs[0].file.clone();
    let conflict = count(runs.clone(), store.bindings(), &[AC1], admitted());
    assert_eq!(counted(&conflict), 0);
    assert!(
        causes(&conflict)
            .iter()
            .any(|(cause, _)| *cause == CauseCode::RecordConflict)
    );

    runs.reverse();
    let supplied_reversed = count(runs.clone(), store.bindings(), &[AC1], admitted());
    assert_eq!(causes(&conflict), causes(&supplied_reversed));
}

#[trace("FR-153-AC-6")]
#[test]
fn an_obligation_bound_through_two_suites_needs_both_to_pass() {
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    store.record("SUITE-B", "2026-11-13T10:00:00Z", REFUTED, &[AC1]);
    let outcome = store.count(&[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    assert_ne!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    let failed: Vec<_> = outcome
        .report
        .evidence_results
        .clone()
        .unwrap()
        .into_iter()
        .filter(|result| result.cause == CauseCode::Failed)
        .collect();
    assert_eq!(failed.len(), 1);
    assert!(
        failed[0]
            .locator
            .as_ref()
            .unwrap()
            .record
            .starts_with("runs/SUITE-B/")
    );

    let mut both = Recorded::new();
    both.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    both.record("SUITE-B", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    assert_eq!(counted(&both.count(&[AC1], admitted())), 2);
}

#[trace("FR-153-AC-7")]
#[test]
fn a_second_report_in_the_same_suite_replaces_the_first() {
    let other = "consistency::other-request-digest";
    let mut second = satisfied_report();
    second["requestDigest"] = Value::from("other-request-digest");
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    store.record_entry(
        "SUITE-A",
        "2026-11-13T11:00:00Z",
        (other, Outcome::Pass),
        &[AC1],
        &second,
    );
    let outcome = store.count(&[AC1], admitted());
    let binders = outcome.report.record_binders.clone().unwrap();
    assert_eq!(binders.len(), 1, "only the second report counts");
    assert_eq!(binders[0].locator.entry, other);

    // Into a different suite, both are bound and both must pass.
    let mut apart = Recorded::new();
    apart.record("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);
    apart.record_entry(
        "SUITE-B",
        "2026-11-13T11:00:00Z",
        (other, Outcome::Fail),
        &[AC1],
        &satisfied_report(),
    );
    assert_eq!(counted(&apart.count(&[AC1], admitted())), 0);
}

#[trace("FR-153-AC-9")]
#[test]
fn trace_returns_the_record_claim_and_never_a_source_symbol() {
    let outcome = satisfied_store(&[AC1]).count(&[AC1], admitted());
    let criterion = outcome
        .report
        .coverage_matrix
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .find(|criterion| criterion.id == AC1)
        .unwrap();
    let binders = criterion.record_binders.clone().unwrap();
    assert_eq!(binders.len(), 1);
    assert!(criterion.binders.is_empty(), "no source symbol is minted");

    assert_eq!(outcome.trace(Query::Obligation(AC1)), binders);
    let locator = quoin_evidence::qualify::RunLocator {
        record: binders[0].locator.record.clone(),
        run: binders[0].locator.run.clone(),
        entry: binders[0].locator.entry.clone(),
    };
    assert_eq!(outcome.trace(Query::Record(&locator)), binders);
    assert_eq!(outcome.trace(Query::Run(&locator.run)), binders);
    assert!(outcome.trace(Query::Obligation(AC2)).is_empty());

    // A source-symbol query for the retained entry's own identity returns no
    // claim, and the obligation id in prose stays a citation (FR-077).
    let symbol = binders[0].locator.entry.clone();
    let registry = quire_rs::Registry::load_module(&fixture()).unwrap();
    let model = registry.traceability().unwrap();
    let extraction = quire_rs::symbols::extract_tree_scoped(
        &fixture(),
        &[std::path::Path::new("spec")],
        &model.source_exclude,
    );
    let graph = quire_rs::symbols::trace::bind(&extraction, model);
    let by_symbol = quire_rs::symbols::trace_search::search(
        &graph,
        &extraction,
        &quire_rs::symbols::trace_search::Query::SymbolName(symbol),
    )
    .unwrap();
    assert!(by_symbol.verifies.is_empty() && by_symbol.implements.is_empty());
    assert!(
        !by_symbol.resolved,
        "no source symbol carries that identity"
    );
    let by_id = quire_rs::symbols::trace_search::search(
        &graph,
        &extraction,
        &quire_rs::symbols::trace_search::Query::Id(AC1.to_owned()),
    )
    .unwrap();
    assert!(by_id.verifies.is_empty(), "prose makes no claim");
    assert!(
        by_id
            .citations
            .iter()
            .any(|mention| mention.path.ends_with("prose.rs")),
        "the id in prose stays a citation: {:?}",
        by_id.citations
    );
}

#[trace("FR-153-AC-10")]
#[test]
fn a_report_recorded_without_discharges_counts_for_no_obligation() {
    let store = satisfied_store(&[]);
    let outcome = store.count(&[AC1, AC2], admitted());
    assert_eq!(counted(&outcome), 0);
    assert!(store.bindings().is_empty());

    // Even where the entry's symbol and the report's digest text equal an
    // obligation id: the text is not a binding source.
    let mut colliding = Recorded::new();
    let mut report = satisfied_report();
    report["analysisKind"] = Value::from("FR-001-AC-1");
    report["requestDigest"] = Value::from("FR-001-AC-1");
    colliding.record_entry(
        "SUITE-A",
        "2026-11-13T10:00:00Z",
        ("FR-001-AC-1::FR-001-AC-1", Outcome::Pass),
        &[],
        &report,
    );
    assert!(colliding.bindings().is_empty());
    let outcome = colliding.count(&[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    assert_ne!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
}

#[trace("FR-153-AC-11", "FR-184-AC-9")]
#[test]
fn a_passing_report_for_a_test_obligation_is_rejected_on_the_method_pairing() {
    let outcome = satisfied_store(&[AC3]).count(&[AC3], admitted());
    assert_eq!(counted(&outcome), 0);
    assert!(
        causes(&outcome)
            .iter()
            .any(|(cause, detail)| *cause == CauseCode::QualificationRejected
                && detail.starts_with("method and act pairing premise")),
        "{:?}",
        causes(&outcome)
    );
}

#[trace("FR-184-AC-16")]
#[test]
fn one_report_stating_two_obligations_qualifies_neither() {
    let outcome = satisfied_store(&[AC1, AC2]).count(&[AC1, AC2], admitted());
    assert_eq!(counted(&outcome), 0);
    let scoped = causes(&outcome)
        .into_iter()
        .filter(|(cause, detail)| {
            *cause == CauseCode::Unevaluated && detail.starts_with("act scope premise")
        })
        .count();
    assert_eq!(scoped, 2, "each obligation is unevaluated on its own");
}
