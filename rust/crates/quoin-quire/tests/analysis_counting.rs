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

use std::{collections::BTreeMap, fs, path::PathBuf};

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
    EvidenceSource, MemoryEvidence, Outcome, RunEntry, STORE_SCHEMA_VERSION, SuiteId,
    TrustDecisionId,
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

fn copy_tree(source: &std::path::Path, destination: &std::path::Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn obligations() -> Vec<quoin_quire_types::Obligation> {
    [(AC1, "Analysis"), (AC2, "Analysis"), (AC3, "Test")]
        .into_iter()
        .map(|(id, method)| quoin_quire_types::Obligation {
            id: id.to_owned(),
            identity: None,
            document: None,
            statement: "stated".to_owned(),
            target_ids: None,
            method: Some(method.to_owned()),
            criticality: None,
            parameters: None,
        })
        .collect()
}

/// Synthetic FR-093 decision fixture for `producer`, permitting the positive
/// code path. Its corpus/environment labels are test-only, and no decision file
/// exists in any store or represents an owner assessment.
fn decision(producer: (&str, &str), limitations: &[&str]) -> TrustDecision {
    let context = ProducerContext {
        name: producer.0.to_owned(),
        version: producer.1.to_owned(),
        configuration: "run-package".to_owned(),
        validation_corpus: "tc-001-captures".to_owned(),
        input_contract: "filament.differential-report/v2".to_owned(),
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
        "selection": {"selected": ["claim-0"], "excluded": ["claim-1"]},
        "run_kind": "analysis",
        "producer": report["producer"],
        "engines": engines,
    })
}

/// The host facts that belong to no one record, every premise admitted.
fn admitted() -> HostObservations {
    HostObservations {
        reliance: Some(decision(("filament-analyze", "0.1.0"), &[])),
        selected_head: Some(HEAD.to_owned()),
        selected_head_conflict: false,
    }
}

#[trace("FR-153-AC-14", "FR-116-AC-13")]
#[test]
fn conflicting_selected_heads_are_a_record_conflict() {
    let store = satisfied_store(&[AC1]);
    let host = HostObservations {
        selected_head: None,
        selected_head_conflict: true,
        ..admitted()
    };
    let outcome = store.count(&[AC1], host);
    assert_uncounted_naming(
        &outcome,
        CauseCode::RecordConflict,
        "performed revision premise",
    );
    let refusal = evidence_result(&outcome, CauseCode::RecordConflict);
    assert_eq!(refusal.requested_target, AC1);
    let mut expected = expected_locator(&store.runs()[0]);
    expected.run = format!(
        "{}@{}",
        store.runs()[0].record.suite,
        store.runs()[0].record.timestamp
    );
    assert_eq!(refusal.locator.as_ref(), Some(&expected));
}

/// What the host observed of the one act that produced `file`: its own CI
/// invocation, its own checkout SHA and its own admitted signer.
fn admitted_run(file: &str, run_kind: Option<&str>) -> RunObservation {
    RunObservation {
        run_kind: run_kind.map(ToOwned::to_owned),
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
        actor_rejection: None,
        current_formal_rule_status: None,
        selected_target: None,
    }
}

/// An in-memory store and the report each run was transcribed from.
struct Recorded {
    source: MemoryEvidence,
    reports: Vec<(String, Value)>,
    current_formal_rule_status: Option<Value>,
}

impl Recorded {
    fn new() -> Self {
        Self {
            source: MemoryEvidence::new(),
            reports: Vec::new(),
            current_formal_rule_status: None,
        }
    }

    /// `quoin evidence record --adapter differential-report --discharges`.
    fn record(&mut self, suite: &str, timestamp: &str, report: &str, discharges: &[&str]) {
        self.record_with_selection(suite, timestamp, report, discharges, true);
    }

    /// Retain a report without a claim selection to check fail-closed qualification.
    fn record_without_selection(
        &mut self,
        suite: &str,
        timestamp: &str,
        report: &str,
        discharges: &[&str],
    ) {
        self.record_with_selection(suite, timestamp, report, discharges, false);
    }

    fn record_with_selection(
        &mut self,
        suite: &str,
        timestamp: &str,
        report: &str,
        discharges: &[&str],
        selected: bool,
    ) {
        let mut result = parse_differential_report(report).unwrap();
        let mut projection = result.analysis_projection.take().unwrap();
        projection["run_kind"] =
            serde_json::json!(if selected { "analysis" } else { "formal_rule" });
        if selected {
            projection["selection"] = serde_json::json!({
                "selected": ["claim-0"],
                "excluded": ["claim-1"]
            });
        }
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
                tool: "filament-analyze 0.1.0".to_owned(),
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
                tool: "filament-analyze 0.1.0".to_owned(),
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
                observation: {
                    let mut observation =
                        admitted_run(path, report.get("run_kind").and_then(Value::as_str));
                    observation
                        .current_formal_rule_status
                        .clone_from(&self.current_formal_rule_status);
                    observation
                },
            })
            .collect()
    }

    fn bindings(&self) -> Vec<Binding> {
        read_bindings(&self.source).unwrap().bindings
    }

    fn store_bytes(&self) -> BTreeMap<String, String> {
        self.source
            .paths()
            .into_iter()
            .map(|path| {
                (
                    path.to_owned(),
                    self.source
                        .read(path)
                        .unwrap()
                        .expect("listed record exists"),
                )
            })
            .collect()
    }

    fn mark_formal_rule_selection(&mut self, first_state: &str, second_state: &str) {
        let (_, projection) = self.reports.last_mut().unwrap();
        let source = [
            serde_json::json!({"document":"qsl/rule.native","revision":1,"start":{"byte":0,"column":1,"line":1},"end":{"byte":8,"column":9,"line":1}}),
            serde_json::json!({"document":"qsl/rule.native","revision":1,"start":{"byte":8,"column":9,"line":1},"end":{"byte":16,"column":17,"line":1}}),
        ];
        projection["run_kind"] = serde_json::json!("formal_rule");
        projection["selection"] = serde_json::json!({
            "selected": ["claim-0", "claim-1"],
            "excluded": []
        });
        projection["formal_rule_selection"] = serde_json::json!({
            "claims": [
                {
                    "claim_id": "claim-0",
                    "source": source[0],
                    "symbol": "qsl:qsl/rule.native::Rule0",
                    "state": first_state,
                },
                {
                    "claim_id": "claim-1",
                    "source": source[1],
                    "symbol": "qsl:qsl/rule.native::Rule1",
                    "state": second_state,
                }
            ]
        });
        self.current_formal_rule_status =
            Some(Self::current_formal_rule_status(first_state, second_state));
    }

    fn current_formal_rule_status(first_state: &str, second_state: &str) -> Value {
        serde_json::json!({
            "protocol": "quoin.formal-rule-status/v1",
            "rules": [
                {"symbol": "qsl:qsl/rule.native::Rule0", "path":"qsl/rule.native", "declaration_range":{"start":0,"end":8}, "state": first_state},
                {"symbol": "qsl:qsl/rule.native::Rule1", "path":"qsl/rule.native", "declaration_range":{"start":8,"end":16}, "state": second_state}
            ]
        })
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
        requested_targets: vec![None; requested.len()],
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

#[trace(
    "FR-153-AC-1",
    "FR-184-AC-2",
    "FR-184-AC-13",
    "FR-184-AC-15",
    "FR-116-AC-1"
)]
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
    assert_eq!(binder.locator.entry, "consistency");
    assert!(
        binder
            .provenance
            .contains(&"credit:consistency-evidence-only".to_owned())
    );
    let premises = [
        "tool:filament-analyze:0.1.0".to_owned(),
        "reliance:ETD-1:accepted".to_owned(),
        "actor-authority:signer-1:admission-7".to_owned(),
        format!("performed-revision:{HEAD}"),
        format!("selected-head:{HEAD}"),
        "execution:ci:github/agent-ix/quoin/100/1/analysis/run-package:".to_owned(),
    ];
    for premise in &premises {
        assert!(
            binder
                .provenance
                .iter()
                .any(|item| item.starts_with(premise)),
            "the admitted premise is retained: {premise}; {:?}",
            binder.provenance
        );
    }
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

#[trace("FR-184-AC-18")]
#[test]
fn a_passing_report_without_selection_is_rejected_as_draft_rule_in_judgement() {
    let mut store = Recorded::new();
    store.record_without_selection("SUITE-A", "2026-11-13T10:00:00Z", SATISFIED, &[AC1]);

    let outcome = store.count(&[AC1], admitted());
    assert_uncounted_naming(
        &outcome,
        CauseCode::QualificationRejected,
        "draft_rule_in_judgement",
    );
    assert_ne!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
}

#[trace("FR-184-AC-17")]
#[test]
fn a_formal_rule_report_qualifies_only_when_every_selected_rule_is_approved() {
    let mut store = satisfied_store(&[AC1]);
    store.mark_formal_rule_selection("approved", "approved");

    let outcome = store.count(&[AC1], admitted());
    assert_eq!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
}

#[trace("FR-184-AC-18")]
#[test]
fn a_formal_rule_report_with_one_draft_rule_is_rejected() {
    let mut store = satisfied_store(&[AC1]);
    store.mark_formal_rule_selection("approved", "draft");

    let outcome = store.count(&[AC1], admitted());
    assert_uncounted_naming(
        &outcome,
        CauseCode::QualificationRejected,
        "draft_rule_in_judgement",
    );
}

#[trace("FR-184-AC-25")]
#[test]
fn a_preparation_approval_does_not_override_a_current_changed_since_approval_result() {
    let mut store = satisfied_store(&[AC1]);
    store.mark_formal_rule_selection("approved", "approved");
    store.current_formal_rule_status =
        Some(Recorded::current_formal_rule_status("approved", "draft"));
    store.current_formal_rule_status.as_mut().unwrap()["rules"][1]["reason"] =
        serde_json::json!("changed_since_approval");

    let stale = store.count(&[AC1], admitted());
    assert_uncounted_naming(
        &stale,
        CauseCode::QualificationRejected,
        "draft_rule_in_judgement",
    );
    assert!(
        causes(&stale)
            .iter()
            .any(|(_, detail)| detail.contains("changed_since_approval"))
    );

    store.current_formal_rule_status = None;
    let unavailable = store.count(&[AC1], admitted());
    assert_uncounted_naming(
        &unavailable,
        CauseCode::Unevaluated,
        "approved formal-rule selection",
    );

    store.mark_formal_rule_selection("approved", "approved");
    let approved = store.count(&[AC1], admitted());
    assert_eq!(
        status(&approved, AC1),
        CoverageMatrixStatus::QualifiedRecord
    );
}

#[trace("FR-184-AC-17")]
#[test]
fn a_current_formal_rule_with_changed_source_span_is_a_selection_mismatch() {
    let mut store = satisfied_store(&[AC1]);
    store.mark_formal_rule_selection("approved", "approved");
    store.current_formal_rule_status.as_mut().unwrap()["rules"][0]["path"] =
        serde_json::json!("qsl/moved.native");

    let outcome = store.count(&[AC1], admitted());
    assert_uncounted_naming(
        &outcome,
        CauseCode::SelectionMismatch,
        "current FR-188 declaration path or span",
    );
}

#[test]
fn a_formal_rule_selection_absent_from_current_status_is_a_selection_mismatch() {
    let mut store = satisfied_store(&[AC1]);
    store.mark_formal_rule_selection("approved", "approved");
    store.current_formal_rule_status = Some(serde_json::json!({
        "protocol": "quoin.formal-rule-status/v1",
        "rules": [
            {"symbol": "qsl:qsl/rule.native::Rule0", "path":"qsl/rule.native", "declaration_range":{"start":0,"end":8}, "state": "approved"},
            {"symbol": "qsl:qsl/other.native::Rule1", "path":"qsl/rule.native", "declaration_range":{"start":8,"end":16}, "state": "approved"}
        ]
    }));

    let outcome = store.count(&[AC1], admitted());
    assert_uncounted_naming(
        &outcome,
        CauseCode::SelectionMismatch,
        "formal-rule selection premise",
    );
}

#[trace("FR-116-AC-19", "FR-116-AC-13")]
#[test]
fn same_rendered_id_with_a_different_selected_document_is_a_selection_mismatch() {
    let source = fixture();
    let temporary = tempfile::tempdir().unwrap();
    copy_tree(&source, temporary.path());
    fs::write(
        temporary.path().join("spec/FR-002-other.md"),
        "---\nid: FR-002\ntype: FR\ntitle: Duplicate rendered target\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | The same rendered criterion appears in another source document. | Analysis |\n",
    )
    .unwrap();
    let selected_scope = ScopeRequest {
        scope: ids::ScopeRoot::open(temporary.path()).unwrap(),
        modules: ModuleSelection::Closed(vec![ids::ModuleRoot::open(temporary.path()).unwrap()]),
    };
    let inventory = compute(&Request {
        scope: selected_scope.clone(),
        requested: vec![AC1.to_owned()],
        requested_targets: vec![None],
        runs: Vec::new(),
        bindings: Vec::new(),
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(Vec::new()),
        host: HostObservations::default(),
    })
    .unwrap();
    let mut identities = inventory
        .report
        .coverage_matrix
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .filter(|criterion| criterion.id == AC1)
        .filter_map(|criterion| criterion.identity.clone());
    let first: quoin_quire_types::TargetLocus = serde_json::from_value(
        serde_json::to_value(identities.next().expect("first FR-078 target identity")).unwrap(),
    )
    .unwrap();
    let second: quoin_quire_types::TargetLocus = serde_json::from_value(
        serde_json::to_value(identities.next().expect("second FR-078 target identity")).unwrap(),
    )
    .unwrap();
    assert_ne!(first.document, second.document);

    let store = satisfied_store(&[AC1]);
    let mut runs = store.runs();
    runs[0].observation.selected_target = Some(second.clone());
    let mut selected_run = runs.clone();
    selected_run[0].observation.selected_target = Some(second.clone());
    let mut expected_selection = expected_locator(&selected_run[0]);
    expected_selection.run = format!(
        "{}/{}",
        selected_run[0].record.suite, selected_run[0].record.timestamp
    );
    let mismatch = compute(&Request {
        scope: selected_scope.clone(),
        requested: vec![AC1.to_owned()],
        requested_targets: vec![Some(first.clone())],
        runs: selected_run,
        bindings: store.bindings(),
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(vec!["SUITE-A".to_owned()]),
        host: admitted(),
    })
    .unwrap();
    assert!(
        causes(&mismatch)
            .iter()
            .any(|(cause, _)| *cause == CauseCode::SelectionMismatch)
    );
    let refusal = evidence_result(&mismatch, CauseCode::SelectionMismatch);
    assert_eq!(refusal.requested_target, AC1);
    assert_eq!(refusal.locator.as_ref(), Some(&expected_selection));

    runs[0].observation.selected_target = Some(first.clone());
    let matching = compute(&Request {
        scope: selected_scope,
        requested: vec![AC1.to_owned()],
        requested_targets: vec![Some(first)],
        runs,
        bindings: store.bindings(),
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(vec!["SUITE-A".to_owned()]),
        host: admitted(),
    })
    .unwrap();
    assert!(
        !causes(&matching)
            .iter()
            .any(|(cause, _)| *cause == CauseCode::SelectionMismatch)
    );
}

/// Assert the obligation is uncounted and one cause names `premise`.
fn assert_uncounted_naming(outcome: &Counted, cause: CauseCode, premise: &str) {
    assert_eq!(counted(outcome), 0);
    assert_ne!(status(outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert!(unbacked(outcome).contains(&AC1.to_owned()));
    assert!(
        causes(outcome)
            .iter()
            .any(|(found, detail)| *found == cause && detail.contains(premise)),
        "{premise}: {:?}",
        causes(outcome)
    );
    assert!(strict_decision(&outcome.report, &[]).is_err());
    assert!(
        outcome
            .report
            .evidence_results
            .as_ref()
            .expect("unbacked causes remain in the report")
            .iter()
            .all(|result| result.requested_target == AC1),
        "every refusal keeps the original requested target"
    );
    if cause == CauseCode::Unevaluated {
        let refusal = evidence_result(outcome, cause);
        if !refusal.detail.starts_with("FR-032 audit premise") {
            assert_eq!(
                refusal.locator.as_ref(),
                Some(&quire_rs::evidence::RetainedLocator {
                    record: "runs/SUITE-A/2026-11-13T10~3A00~3A00Z.json".to_owned(),
                    run: "SUITE-A@2026-11-13T10:00:00Z".to_owned(),
                    entry: SATISFIED_SYMBOL.to_owned(),
                }),
                "a run-specific unevaluated refusal identifies its retained entry"
            );
        }
    }
}

fn evidence_result(outcome: &Counted, cause: CauseCode) -> &quire_rs::evidence::EvidenceResult {
    outcome
        .report
        .evidence_results
        .as_ref()
        .expect("refusals remain in the evidence report")
        .iter()
        .find(|result| result.cause == cause)
        .expect("the requested refusal cause remains distinct")
}

fn expected_locator(run: &RetainedRun) -> quire_rs::evidence::RetainedLocator {
    quire_rs::evidence::RetainedLocator {
        record: run.file.clone(),
        run: format!("{}@{}", run.record.suite, run.record.timestamp),
        entry: run
            .record
            .entries
            .first()
            .map_or_else(String::new, |entry| entry.symbol.to_string()),
    }
}

#[trace("FR-153-AC-2", "FR-184-AC-1", "FR-184-AC-14", "FR-116-AC-13")]
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
    assert_eq!(
        evidence_result(&outcome, CauseCode::Unevaluated)
            .locator
            .as_ref(),
        Some(&expected_locator(&store.runs()[0])),
        "an unevaluated retained attempt keeps its exact source locator"
    );

    // What ships today: no assessment exists and the host supplies nothing.
    let outcome = compute(&Request {
        scope: scope(),
        requested: vec![AC1.to_owned()],
        requested_targets: vec![None],
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
        requested_targets: vec![None],
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

#[trace("FR-116-AC-7", "FR-116-AC-8", "FR-153-AC-13", "FR-116-AC-13")]
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
    assert_eq!(wrong[0].requested_target, AC1);
    let expected_wrong = expected_locator(&runs[1]);
    assert_eq!(wrong[0].locator.as_ref(), Some(&expected_wrong));

    // No observation of its own: unevaluated, whatever the other record has.
    runs[1].observation = RunObservation::default();
    let expected_unobserved = expected_locator(&runs[1]);
    let outcome = count(runs, store.bindings(), &[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    let unobserved = outcome
        .report
        .evidence_results
        .as_ref()
        .unwrap()
        .iter()
        .find(|result| {
            result.cause == CauseCode::Unevaluated
                && result.detail.starts_with("execution association premise")
        })
        .expect("the missing host observation has a named refusal");
    assert_eq!(unobserved.requested_target, AC1);
    assert_eq!(unobserved.locator.as_ref(), Some(&expected_unobserved));
}

#[trace("FR-030-AC-12", "FR-030-AC-30", "FR-116-AC-7", "FR-153-AC-2")]
#[test]
fn a_same_suite_newer_stale_revision_supersedes_an_older_current_pass() {
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-22T10:00:00Z", SATISFIED, &[AC1]);
    store.record("SUITE-A", "2026-11-22T10:00:00.001Z", SATISFIED, &[AC1]);

    let mut runs = store.runs();
    assert_eq!(runs.len(), 2);
    runs[0].observation.performed_revision = Some(HEAD.to_owned());
    runs[1].observation.performed_revision = Some("stale-sha".to_owned());
    let outcome = count(runs.clone(), store.bindings(), &[AC1], admitted());
    assert_eq!(counted(&outcome), 0, "the older pass was reused");
    let wrong_revision = outcome
        .report
        .evidence_results
        .clone()
        .unwrap()
        .into_iter()
        .find(|result| result.cause == CauseCode::WrongRevision)
        .expect("newer stale revision must be reported");
    assert_eq!(wrong_revision.locator.unwrap().record, runs[1].file);

    runs[0].observation.performed_revision = Some("stale-sha".to_owned());
    runs[1].observation.performed_revision = Some(HEAD.to_owned());
    let reverse = count(runs, store.bindings(), &[AC1], admitted());
    assert_eq!(
        counted(&reverse),
        1,
        "the newer current pass was not selected"
    );
}

#[trace("FR-030-AC-30", "FR-153-AC-2")]
#[test]
fn an_empty_run_and_empty_disagreement_never_qualify_a_candidate() {
    let mut store = Recorded::new();
    store.record("SUITE-A", "2026-11-22T10:00:00Z", DISAGREEMENT, &[AC1]);
    assert_eq!(
        store.runs()[0].record.entries,
        [] as [quoin_evidence::RunEntry; 0]
    );

    let outcome = store.count(&[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    assert_ne!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
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

#[trace("FR-116-AC-15", "FR-116-AC-13")]
#[test]
fn the_reliance_decision_must_be_for_the_reports_producer() {
    let store = satisfied_store(&[AC1]);
    for producer in [("other-tool", "0.1.0"), ("filament-analyze", "9.9.9")] {
        let host = HostObservations {
            reliance: Some(decision(producer, &[])),
            ..admitted()
        };
        let outcome = store.count(&[AC1], host);
        assert_uncounted_naming(
            &outcome,
            CauseCode::QualificationRejected,
            "bounded reliance premise",
        );
        let refusal = evidence_result(&outcome, CauseCode::QualificationRejected);
        assert_eq!(refusal.requested_target, AC1);
        assert_eq!(
            refusal.locator.as_ref(),
            Some(&expected_locator(&store.runs()[0]))
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
            reliance: Some(decision(("filament-analyze", "0.1.0"), &[limitation])),
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

#[trace("FR-184-AC-10", "FR-184-AC-11", "FR-184-AC-12", "FR-153-AC-15")]
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
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.execution = Some(ExecutionAssociation::Ci(CiAssociation {
            system: String::new(),
            project: "agent-ix/quoin".to_owned(),
            run: "run-17".to_owned(),
            attempt: "1".to_owned(),
            job: "analysis".to_owned(),
            invocation: "step-9".to_owned(),
        }));
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

#[trace(
    "FR-153-AC-3",
    "FR-184-AC-3",
    "FR-184-AC-4",
    "FR-184-AC-5",
    "FR-116-AC-13"
)]
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
        let refusal = results
            .iter()
            .find(|result| result.cause == cause)
            .expect("failure and error causes remain distinct");
        assert_eq!(refusal.requested_target, AC1);
        assert_eq!(
            refusal.locator.as_ref(),
            Some(&expected_locator(&store.runs()[0])),
            "{cause:?}: {results:?}"
        );
        assert!(strict_decision(&outcome.report, &[]).is_err());
    }

    let mut skipped = Recorded::new();
    skipped.record_entry(
        "SUITE-A",
        "2026-11-13T10:00:00Z",
        (SATISFIED_SYMBOL, Outcome::Skip),
        &[AC1],
        &satisfied_report(),
    );
    let outcome = skipped.count(&[AC1], admitted());
    let refusal = outcome
        .report
        .evidence_results
        .as_ref()
        .unwrap()
        .iter()
        .find(|result| result.cause == CauseCode::Skipped)
        .expect("a skipped attempt keeps its distinct cause");
    assert!(
        refusal.locator.is_some(),
        "a retained skipped run keeps its locator"
    );
    assert_eq!(refusal.requested_target, AC1);
    assert_eq!(
        refusal.locator.as_ref(),
        Some(&expected_locator(&skipped.runs()[0]))
    );
}

#[trace("FR-153-AC-4", "FR-184-AC-6", "FR-116-AC-13")]
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
    for outcome in [&disagreed, &never] {
        assert!(
            outcome
                .report
                .evidence_results
                .as_ref()
                .unwrap()
                .iter()
                .all(|result| result.requested_target == AC1)
        );
    }
}

/// The satisfied capture's own entry symbol.
const SATISFIED_SYMBOL: &str = "consistency";

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

#[trace("FR-153-AC-5", "FR-184-AC-7", "FR-184-AC-8", "FR-116-AC-13")]
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
            assert!(
                outcome
                    .report
                    .evidence_results
                    .as_ref()
                    .unwrap()
                    .iter()
                    .filter(|result| result.cause != CauseCode::Absent)
                    .all(|result| result.locator.is_some()),
                "available stale/failure records retain their locator"
            );
        }
        assert_eq!(causes(&forward), causes(&reversed));
        assert_eq!(
            forward.report.evidence_results, reversed.report.evidence_results,
            "requested targets, locators, causes and ordering survive run reordering"
        );
        let available: Vec<_> = store.runs().iter().map(expected_locator).collect();
        let results = forward.report.evidence_results.as_ref().unwrap();
        assert!(results.iter().all(|result| result.requested_target == AC1));
        assert!(
            results
                .iter()
                .filter_map(|result| result.locator.as_ref())
                .all(|locator| available.contains(locator)),
            "retained result locators resolve to original run records: {results:?}"
        );
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
    let other = "contradiction";
    let mut second = satisfied_report();
    second["analysisKind"] = Value::from("contradiction");
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

#[trace("FR-153-AC-9", "FR-116-AC-16")]
#[test]
fn trace_returns_the_record_claim_and_never_a_source_symbol() {
    let store = satisfied_store(&[AC1]);
    let bytes_before = store.store_bytes();
    let outcome = store.count(&[AC1], admitted());
    assert_eq!(
        store.store_bytes(),
        bytes_before,
        "qualification only reads the canonical stores"
    );
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
    assert_eq!(
        outcome.trace(Query::Obligation(AC2)),
        [] as [quire_rs::evidence::RecordBinder; 0]
    );
    assert_ne!(
        status(&outcome, AC2),
        CoverageMatrixStatus::QualifiedRecord,
        "legacy no-evidence matrix behavior stays unqualified"
    );

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
    assert_eq!(store.bindings(), [] as [quoin_evidence::types::Binding; 0]);

    // Even where the entry's symbol and the report's analysis kind equal an
    // obligation id: the text is not a binding source.
    let mut colliding = Recorded::new();
    let mut report = satisfied_report();
    report["analysisKind"] = Value::from("FR-001-AC-1");
    colliding.record_entry(
        "SUITE-A",
        "2026-11-13T10:00:00Z",
        ("FR-001-AC-1", Outcome::Pass),
        &[],
        &report,
    );
    assert_eq!(
        colliding.bindings(),
        [] as [quoin_evidence::types::Binding; 0]
    );
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

// ---------------------------------------------------------------------------
// Swap-and-refuse (PLAT-1790). Each case starts from a record that qualifies,
// changes exactly one input, and expects the owner qualifier to refuse with the
// exact cause category and premise the FR-116 contract names for that change.
// The `(cause, premise)` pairs come from the FR-116 and FR-184 text. The one
// exception is `RELIANCE_CASCADE`, which is the implementation's evaluation
// order, not contract text; it is named separately below. Every case is
// bracketed by the healthy control so the refusal is shown to come from the
// swapped input.
//
// Confirmed implementation gap (PLAT-1790): FR-116 names `selection-mismatch`,
// `invalid-target` and `ambiguous-target`, but `quoin_evidence::qualify::Cause`
// has no such variants, so no test here can assert them. Nothing below claims
// them.
// ---------------------------------------------------------------------------

/// Not FR-116 contract text: `qualify::evaluate` still evaluates reliance after
/// the report premises fail, and with no producer to compare it reports reliance
/// as unevaluated. Cases whose report premises fail include this pair so the
/// exact-set comparison stays honest; it is the code's cascade, not a rule.
const RELIANCE_CASCADE: (CauseCode, &str) = RELIANCE_PREMISE;

/// FR-116: missing reliance is `unevaluated` on the bounded-reliance premise.
const RELIANCE_PREMISE: (CauseCode, &str) = (CauseCode::Unevaluated, "bounded reliance");

/// The premise a refusal names: the text before " premise".
fn premise_of(detail: &str) -> &str {
    detail
        .split_once(" premise")
        .map_or(detail, |(name, _)| name)
}

/// The sorted multiset of `(cause, premise)` pairs the outcome retains.
fn refusal_set(outcome: &Counted) -> Vec<(CauseCode, String)> {
    let mut found: Vec<(CauseCode, String)> = causes(outcome)
        .into_iter()
        .map(|(cause, detail)| (cause, premise_of(&detail).to_owned()))
        .collect();
    // A multiset: a duplicated refusal for one premise is a difference.
    found.sort();
    found
}

/// The obligation `id` is not owner-qualified and the refusal is exactly
/// `expected`: no record binder, no qualified status, strict decision refuses.
fn assert_refused_exactly(outcome: &Counted, id: &str, expected: &[(CauseCode, &str)]) {
    assert_eq!(counted(outcome), 0, "{:?}", causes(outcome));
    assert_ne!(status(outcome, id), CoverageMatrixStatus::QualifiedRecord);
    assert!(unbacked(outcome).contains(&id.to_owned()));
    assert!(strict_decision(&outcome.report, &[]).is_err());
    let mut want: Vec<(CauseCode, String)> = expected
        .iter()
        .map(|(cause, premise)| (*cause, (*premise).to_owned()))
        .collect();
    want.sort();
    assert_eq!(refusal_set(outcome), want);
}

/// The healthy control: the untouched store still qualifies AC1 and only AC1.
fn assert_healthy_control(store: &Recorded) {
    let outcome = store.count(&[AC1], admitted());
    assert_eq!(counted(&outcome), 1);
    assert_eq!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
}

/// Count with the retained runs changed by `tweak`.
fn count_runs(
    store: &Recorded,
    host: HostObservations,
    tweak: impl Fn(&mut RetainedRun),
) -> Counted {
    let mut runs = store.runs();
    for run in &mut runs {
        tweak(run);
    }
    count(runs, store.bindings(), &[AC1], host)
}

/// One edit of a retained projection.
type ReportEdit = fn(&mut Value);

/// A premise removal: label, host facts, run-observation edit, premise named.
type PremiseCase<'a> = (&'a str, HostObservations, fn(&mut RunObservation), &'a str);

const OTHER_HEAD: &str = "fedcba9876543210fedcba9876543210fedcba98";

#[trace("FR-116-AC-6", "FR-116-AC-18")]
#[test]
fn swapping_the_actor_refuses_with_the_actor_authority_cause() {
    let store = satisfied_store(&[AC1]);
    assert_healthy_control(&store);
    let authority = [(CauseCode::QualificationRejected, "actor authority")];
    let unevaluated = [(CauseCode::Unevaluated, "actor authority")];

    // A rejected signer result keeps its own reason and is a rejection.
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.actor = None;
        observation.actor_rejection = Some("signer is not in the allowed signers".to_owned());
    });
    assert_refused_exactly(&outcome, AC1, &authority);
    assert!(
        causes(&outcome)
            .iter()
            .any(|(_, detail)| detail.contains("signer is not in the allowed signers")),
        "{:?}",
        causes(&outcome)
    );
    // An admitted actor with no authority over this obligation.
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.actor = Some(ActorAdmission {
            actor: "signer-2".to_owned(),
            admission_ref: "admission-9".to_owned(),
            authorized: vec![AC2.to_owned()],
        });
    });
    assert_refused_exactly(&outcome, AC1, &authority);
    // No actor, an unnamed actor, and an actor without an admission reference.
    for actor in [None, Some(("", "admission-7")), Some(("signer-1", ""))] {
        let outcome = store.count_with(&[AC1], admitted(), |observation| {
            observation.actor = actor.map(|(name, reference)| ActorAdmission {
                actor: name.to_owned(),
                admission_ref: reference.to_owned(),
                authorized: vec![AC1.to_owned()],
            });
        });
        assert_refused_exactly(&outcome, AC1, &unevaluated);
    }
    // A different actor who is admitted for the target is the asserting actor:
    // the qualified record names the swapped identity, never the original one.
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.actor = Some(ActorAdmission {
            actor: "signer-2".to_owned(),
            admission_ref: "admission-9".to_owned(),
            authorized: vec![AC1.to_owned()],
        });
    });
    let binder = &outcome.report.record_binders.as_ref().unwrap()[0];
    assert_eq!(binder.actor, "signer-2");
    assert!(
        binder
            .provenance
            .contains(&"actor-authority:signer-2:admission-9".to_owned())
    );
    assert_healthy_control(&store);
}

#[trace("FR-116-AC-8", "FR-116-AC-10")]
#[test]
fn swapping_the_report_bytes_refuses_as_a_record_conflict() {
    let store = satisfied_store(&[AC1]);
    assert_healthy_control(&store);
    let conflict = [
        (CauseCode::RecordConflict, "tool and configuration"),
        RELIANCE_CASCADE,
    ];
    let edits: [(&str, ReportEdit); 5] = [
        ("verdict flipped", |report| {
            report["agreed_status"] = Value::from("refuted");
        }),
        ("disposition flipped", |report| {
            report["disposition"] = Value::from("disagreement");
        }),
        ("member added", |report| {
            report["note"] = Value::from("added after the run");
        }),
        ("protocol changed", |report| {
            report["protocol"] = Value::from("quoin.analysis-report-projection/v2");
        }),
        ("engine reordered", |report| {
            report["engines"].as_array_mut().unwrap().reverse();
        }),
    ];
    for (label, edit) in edits {
        let outcome = count_runs(&store, admitted(), |run| {
            edit(run.report.as_mut().unwrap());
        });
        assert_refused_exactly(&outcome, AC1, &conflict);
        assert!(
            causes(&outcome).iter().any(|(_, detail)| detail
                .contains("the supplied projection does not match the retained run")),
            "{label}: {:?}",
            causes(&outcome)
        );
    }
    // Retained bytes changed under the same projection: the entry now fails.
    let outcome = count_runs(&store, admitted(), |run| {
        run.record.entries[0].outcome = Outcome::Fail;
    });
    assert_eq!(counted(&outcome), 0);
    assert_ne!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert_eq!(
        refusal_set(&outcome),
        vec![(
            CauseCode::Failed,
            "consistency in suite SUITE-A reported failure".to_owned()
        )]
    );
    // The report names another producer version than the relied-upon one.
    let outcome = count_runs(&store, admitted(), |run| {
        run.report.as_mut().unwrap()["producer"]["version"] = Value::from("9.9.9");
    });
    assert_refused_exactly(
        &outcome,
        AC1,
        &[(CauseCode::QualificationRejected, "bounded reliance")],
    );
    // No report observation at all is unevaluated, not a pass by omission.
    let outcome = count_runs(&store, admitted(), |run| run.report = None);
    assert_refused_exactly(
        &outcome,
        AC1,
        &[
            (CauseCode::Unevaluated, "tool and configuration"),
            RELIANCE_CASCADE,
        ],
    );
    assert_healthy_control(&store);
}

#[trace("FR-116-AC-8", "FR-116-AC-14")]
#[test]
fn swapping_the_candidate_record_refuses_and_names_no_other_record() {
    let store = satisfied_store(&[AC1]);
    assert_healthy_control(&store);
    let conflict = [
        (CauseCode::RecordConflict, "tool and configuration"),
        RELIANCE_CASCADE,
    ];
    // The record file is another record's path than the run it is supplied as.
    let outcome = count_runs(&store, admitted(), |run| {
        run.file = "runs/SUITE-B/2026-11-13T10~3A00~3A00Z.json".to_owned();
    });
    assert_refused_exactly(&outcome, AC1, &conflict);
    // The run's own suite is not the one its path names.
    let outcome = count_runs(&store, admitted(), |run| {
        run.record.suite = SuiteId::new("SUITE-B");
    });
    // The FR-032 audit pair is the implementation's cascade (the audit finds no
    // run under the swapped suite), not FR-116 text.
    assert_refused_exactly(
        &outcome,
        AC1,
        &[
            (CauseCode::RecordConflict, "tool and configuration"),
            (CauseCode::Unevaluated, "FR-032 audit"),
            RELIANCE_CASCADE,
        ],
    );
    // A second entry stating a second obligation qualifies neither.
    let outcome = count_runs(&store, admitted(), |run| {
        run.record.entries[0].trace_ids = Some(vec![AC1.to_owned(), AC2.to_owned()]);
    });
    assert_eq!(counted(&outcome), 0);
    assert!(
        causes(&outcome)
            .iter()
            .any(|(cause, detail)| *cause == CauseCode::Unevaluated
                && detail.starts_with("act scope premise"))
    );
    assert_healthy_control(&store);
}

#[trace("FR-116-AC-8", "FR-116-AC-14")]
#[test]
fn swapping_the_criterion_never_credits_the_requested_one() {
    // The report discharges AC2; AC1 is asked for.
    let store = satisfied_store(&[AC2]);
    let outcome = store.count(&[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    assert_ne!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert!(unbacked(&outcome).contains(&AC1.to_owned()));
    let results = outcome.report.evidence_results.clone().unwrap();
    assert!(
        results
            .iter()
            .all(|result| result.cause == CauseCode::Absent && result.locator.is_none()),
        "{results:?}"
    );
    // The same retained run does qualify AC2 for its own criterion only.
    let own = store.count(&[AC1, AC2], admitted());
    assert_eq!(counted(&own), 1);
    assert_eq!(status(&own, AC2), CoverageMatrixStatus::QualifiedRecord);
    assert_ne!(status(&own, AC1), CoverageMatrixStatus::QualifiedRecord);
    // An actor admitted for the claimed criterion is not admitted for the other.
    let outcome = store.count_with(&[AC2], admitted(), |observation| {
        if let Some(actor) = &mut observation.actor {
            actor.authorized = vec![AC1.to_owned()];
        }
    });
    assert_refused_exactly(
        &outcome,
        AC2,
        &[(CauseCode::QualificationRejected, "actor authority")],
    );
}

#[trace("FR-116-AC-11")]
#[test]
fn swapping_the_method_refuses_on_the_method_pairing() {
    // AC3 declares Test; the analysis act discharges Analysis only.
    let store = satisfied_store(&[AC3]);
    let outcome = store.count(&[AC3], admitted());
    assert_refused_exactly(
        &outcome,
        AC3,
        &[(CauseCode::QualificationRejected, "method and act pairing")],
    );
    assert!(
        causes(&outcome).iter().any(|(_, detail)| detail
            .contains("the analysis act discharges Analysis; the obligation declares Test")),
        "{:?}",
        causes(&outcome)
    );
}

#[trace("FR-116-AC-7", "FR-116-AC-12")]
#[test]
fn swapping_the_snapshot_or_revision_refuses_with_the_revision_cause() {
    let store = satisfied_store(&[AC1]);
    assert_healthy_control(&store);
    let wrong = [(CauseCode::WrongRevision, "performed revision")];
    let unevaluated = [(CauseCode::Unevaluated, "performed revision")];
    // The selected HEAD is another snapshot than the one the run performed on.
    let other_head = HostObservations {
        selected_head: Some(OTHER_HEAD.to_owned()),
        ..admitted()
    };
    assert_refused_exactly(&store.count(&[AC1], other_head), AC1, &wrong);
    // The performed revision is another revision than the selected HEAD.
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.performed_revision = Some(OTHER_HEAD.to_owned());
    });
    assert_refused_exactly(&outcome, AC1, &wrong);
    // An unavailable revision is unevaluated, never matched by omission.
    for head in [None, Some(String::new())] {
        let host = HostObservations {
            selected_head: head,
            ..admitted()
        };
        assert_refused_exactly(&store.count(&[AC1], host), AC1, &unevaluated);
    }
    for performed in [None, Some(String::new())] {
        let outcome = store.count_with(&[AC1], admitted(), |observation| {
            observation.performed_revision = performed.clone();
        });
        assert_refused_exactly(&outcome, AC1, &unevaluated);
    }
    assert_healthy_control(&store);
}

// FR-116-AC-10 also asks that a reused configuration name with different
// required arguments or features be rejected. The reliance context carries one
// `configuration` string and no argument or feature set, so that clause cannot
// be exercised here and is not claimed; the swapped name below is FR-093
// invalidation (FR-116-AC-12).
#[trace("FR-116-AC-12")]
#[test]
fn swapping_the_configuration_or_reliance_refuses_with_the_reliance_cause() {
    let store = satisfied_store(&[AC1]);
    assert_healthy_control(&store);
    let with = |change: &dyn Fn(&mut TrustDecision)| {
        let mut changed = decision(("filament-analyze", "0.1.0"), &[]);
        change(&mut changed);
        HostObservations {
            reliance: Some(changed),
            ..admitted()
        }
    };
    // The observed configuration moved away from the accepted one: stale.
    let host = with(&|d| {
        d.observed_context.as_mut().unwrap().configuration = "run-package-other".to_owned();
    });
    assert_refused_exactly(
        &store.count(&[AC1], host),
        AC1,
        &[(CauseCode::Stale, "bounded reliance")],
    );
    // The assessed use does not permit the positive decision.
    let host = with(&|d| d.r#use.permitted_decisions = vec!["other-use".to_owned()]);
    assert_refused_exactly(
        &store.count(&[AC1], host),
        AC1,
        &[(CauseCode::QualificationRejected, "bounded reliance")],
    );
    // The producer is not relied upon.
    let host = with(&|d| d.decision = TrustDecisionKind::NotReliedUpon);
    assert_refused_exactly(
        &store.count(&[AC1], host),
        AC1,
        &[(CauseCode::QualificationRejected, "bounded reliance")],
    );
    // Nothing was observed to compare with the accepted context.
    let host = with(&|d| d.observed_context = None);
    assert_refused_exactly(
        &store.count(&[AC1], host),
        AC1,
        &[(CauseCode::Unevaluated, "bounded reliance")],
    );
    assert_healthy_control(&store);
}

// What this asserts: a malformed claim selection, or a run kind that disagrees
// with the host's expectation, is refused as `record-conflict`; a formal-rule
// expectation with no approved selection is `qualification-rejected` with
// `draft_rule_in_judgement` (FR-184-AC-18). FR-116-AC-8's `selection-mismatch`
// for "a different selection" is NOT asserted: the qualifier has no such cause
// (confirmed gap, PLAT-1790). FR-184-AC-17's positive case is traced by
// `a_formal_rule_report_qualifies_only_when_every_selected_rule_is_approved`.
#[trace("FR-184-AC-18")]
#[test]
fn a_malformed_or_unapproved_selection_refuses_as_conflict_or_draft_rule() {
    let store = satisfied_store(&[AC1]);
    assert_healthy_control(&store);
    let conflict = [
        (CauseCode::RecordConflict, "tool and configuration"),
        RELIANCE_CASCADE,
    ];
    // An empty selection and a selection whose halves overlap are malformed.
    for selection in [
        serde_json::json!({"selected": [], "excluded": ["claim-1"]}),
        serde_json::json!({"selected": ["claim-0"], "excluded": ["claim-0"]}),
        serde_json::json!({"selected": ["claim-0"]}),
    ] {
        let outcome = count_runs(&store, admitted(), |run| {
            run.report.as_mut().unwrap()["selection"] = selection.clone();
        });
        assert_refused_exactly(&outcome, AC1, &conflict);
    }
    // The projection says formal_rule where the host expected an analysis run.
    let outcome = count_runs(&store, admitted(), |run| {
        run.report.as_mut().unwrap()["run_kind"] = Value::from("formal_rule");
    });
    assert_refused_exactly(&outcome, AC1, &conflict);
    // The host expected a formal-rule selection that the projection lacks.
    let outcome = store.count_with(&[AC1], admitted(), |observation| {
        observation.run_kind = Some("formal_rule".to_owned());
    });
    assert_refused_exactly(
        &outcome,
        AC1,
        &[
            (
                CauseCode::QualificationRejected,
                "approved formal-rule selection",
            ),
            RELIANCE_CASCADE,
        ],
    );
    assert!(
        causes(&outcome)
            .iter()
            .any(|(_, detail)| detail.contains("draft_rule_in_judgement")),
        "{:?}",
        causes(&outcome)
    );
    assert_healthy_control(&store);
}

// Each premise removed alone is named by its own unevaluated refusal.
#[trace("FR-116-AC-6", "FR-116-AC-8", "FR-116-AC-15")]
#[test]
fn removing_any_single_premise_refuses_naming_that_premise() {
    let store = satisfied_store(&[AC1]);
    assert_healthy_control(&store);

    let cases: [PremiseCase; 5] = [
        (
            "reliance",
            HostObservations {
                reliance: None,
                ..admitted()
            },
            |_| {},
            "bounded reliance",
        ),
        (
            "selected head",
            HostObservations {
                selected_head: None,
                ..admitted()
            },
            |_| {},
            "performed revision",
        ),
        (
            "performed revision",
            admitted(),
            |o| o.performed_revision = None,
            "performed revision",
        ),
        (
            "execution",
            admitted(),
            |o| o.execution = None,
            "execution association",
        ),
        ("actor", admitted(), |o| o.actor = None, "actor authority"),
    ];
    for (label, host, tweak, premise) in cases {
        let outcome = store.count_with(&[AC1], host, tweak);
        assert_refused_exactly(&outcome, AC1, &[(CauseCode::Unevaluated, premise)]);
        assert!(!causes(&outcome).is_empty(), "{label} left a named cause");
    }
    assert_healthy_control(&store);
}

#[trace("FR-116-AC-15", "FR-116-AC-18")]
#[test]
fn a_single_supplied_component_alone_never_qualifies() {
    let store = satisfied_store(&[AC1]);
    let only_host = |host: HostObservations| {
        count_runs(&store, host, |run| {
            run.observation = RunObservation::default();
        })
    };
    // With the run observations absent, each host-only supply leaves exactly
    // the premises it does not supply unevaluated.
    let run_side = [
        (CauseCode::Unevaluated, "performed revision"),
        (CauseCode::Unevaluated, "execution association"),
        (CauseCode::Unevaluated, "actor authority"),
    ];
    let reliance_only = HostObservations {
        reliance: admitted().reliance,
        ..HostObservations::default()
    };
    // Reliance alone supplies no revision, execution or actor.
    assert_refused_exactly(&only_host(reliance_only), AC1, &run_side);
    // The selected head alone also leaves reliance unevaluated.
    let head_only = HostObservations {
        selected_head: Some(HEAD.to_owned()),
        ..HostObservations::default()
    };
    let mut with_reliance = run_side.to_vec();
    with_reliance.push(RELIANCE_PREMISE);
    assert_refused_exactly(&only_host(head_only), AC1, &with_reliance);
    // Every host fact together still lacks the run's own observations.
    assert_refused_exactly(&only_host(admitted()), AC1, &run_side);
    // The run observations alone, with no host facts: no reliance, no head.
    let outcome = store.count(&[AC1], HostObservations::default());
    assert_refused_exactly(
        &outcome,
        AC1,
        &[
            RELIANCE_PREMISE,
            (CauseCode::Unevaluated, "performed revision"),
        ],
    );
    // A healthy audit and a retained passing run with neither is not counted
    // either: the observation without its retained run is absent.
    let outcome = Recorded::new().count(&[AC1], admitted());
    assert_eq!(counted(&outcome), 0);
    assert_all_absent(&outcome);
    assert_healthy_control(&store);
}

fn assert_all_absent(outcome: &Counted) {
    assert!(!causes(outcome).is_empty(), "absence must be reported");
    assert!(
        causes(outcome)
            .iter()
            .all(|(cause, _)| *cause == CauseCode::Absent),
        "{:?}",
        causes(outcome)
    );
}
