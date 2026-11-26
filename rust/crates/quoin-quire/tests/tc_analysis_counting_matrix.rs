// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additional independent evidence-counting matrix checks.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use std::{collections::BTreeSet, fs, path::PathBuf};

use ix_trace_rs::trace;
use quire_rs::coverage::CoverageMatrixStatus;
use quire_rs::evidence::CauseCode;
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
use quoin_evidence::{MemoryEvidence, STORE_SCHEMA_VERSION, SuiteId, TrustDecisionId};
use quoin_quire::coverage::Request as ScopeRequest;
use quoin_quire::evidence::{Outcome as Counted, Request, compute};
use quoin_quire::{ModuleSelection, ids};
use serde_json::Value;

const REPORT: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json"
);
const AC1: &str = "FR-001-AC-1";
const AC2: &str = "FR-001-AC-2";
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
    [AC1, AC2]
        .into_iter()
        .map(|id| quoin_quire_types::Obligation {
            id: id.to_owned(),
            identity: None,
            document: None,
            statement: "stated".to_owned(),
            target_ids: None,
            method: Some("Analysis".to_owned()),
            criticality: None,
            parameters: None,
        })
        .collect()
}

fn decision() -> TrustDecision {
    let context = ProducerContext {
        name: "filament-analyze".to_owned(),
        version: "0.1.0".to_owned(),
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
        limitations: Vec::new(),
        owner: "owner".to_owned(),
        decided_at: "2026-11-13T00:00:00Z".to_owned(),
    }
}

fn admitted() -> HostObservations {
    HostObservations {
        reliance: Some(decision()),
        selected_head: Some(HEAD.to_owned()),
        selected_head_conflict: false,
    }
}

fn admitted_run(file: &str) -> RunObservation {
    RunObservation {
        run_kind: Some("analysis".to_owned()),
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
            authorized: vec![AC1.to_owned(), AC2.to_owned()],
        }),
        ..RunObservation::default()
    }
}

struct Recorded {
    source: MemoryEvidence,
    report: Value,
    path: String,
}

impl Recorded {
    fn new(discharges: &[&str], suite: &str, timestamp: &str) -> Self {
        let mut source = MemoryEvidence::new();
        let mut result = parse_differential_report(REPORT).unwrap();
        let report = result.analysis_projection.take().unwrap();
        for entry in &mut result.entries {
            entry.trace_ids = Some(discharges.iter().map(|id| (*id).to_owned()).collect());
        }
        let recorded = record_run_with_analysis_projection(
            &mut source,
            &RecordRequest {
                suite: SuiteId::new(suite),
                tool: "filament-analyze 0.1.0".to_owned(),
                evidence_kind: None,
                lineage: None,
                timestamp: timestamp.to_owned(),
                entries: result.entries,
            },
            &obligations(),
            &report,
        )
        .unwrap();
        Self {
            source,
            report,
            path: recorded.run_path,
        }
    }

    fn runs(&self) -> Vec<RetainedRun> {
        use quoin_evidence::EvidenceSource;
        vec![RetainedRun {
            file: self.path.clone(),
            record: serde_json::from_str::<RunRecord>(
                &self.source.read(&self.path).unwrap().unwrap(),
            )
            .unwrap(),
            report: Some(self.report.clone()),
            observation: admitted_run(&self.path),
        }]
    }

    fn bindings(&self) -> Vec<Binding> {
        read_bindings(&self.source).unwrap().bindings
    }
}

fn count(
    runs: Vec<RetainedRun>,
    bindings: Vec<Binding>,
    requested: &[&str],
    host: HostObservations,
) -> Counted {
    let suites: BTreeSet<String> = runs
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

#[trace("FR-153-CON-2")]
#[test]
fn counting_uses_supplied_record_projection_and_leaves_inputs_untouched() {
    let recorded = Recorded::new(&[AC1], "analysis-suite", "2026-11-13T00:00:00Z");
    let before_record = recorded.runs()[0].record.clone();
    let before_bindings = recorded.bindings();
    let fixture_files_before = fs::read_dir(fixture())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<BTreeSet<_>>();

    let outcome = count(recorded.runs(), recorded.bindings(), &[AC1], admitted());

    assert_eq!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert_eq!(recorded.runs()[0].record, before_record);
    assert_eq!(recorded.bindings(), before_bindings);
    let fixture_files_after = fs::read_dir(fixture())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<BTreeSet<_>>();
    assert_eq!(fixture_files_after, fixture_files_before);
}

#[trace("FR-184-AC-19")]
#[test]
fn ordinary_analysis_projection_without_formal_rule_selection_is_eligible() {
    let recorded = Recorded::new(&[AC1], "analysis-suite", "2026-11-13T00:00:00Z");
    assert!(recorded.report.get("formal_rule_selection").is_none());

    let outcome = count(recorded.runs(), recorded.bindings(), &[AC1], admitted());

    assert_eq!(status(&outcome, AC1), CoverageMatrixStatus::QualifiedRecord);
    assert!(
        outcome
            .report
            .evidence_results
            .as_ref()
            .unwrap()
            .iter()
            .all(|result| result.cause != CauseCode::QualificationRejected)
    );
}

