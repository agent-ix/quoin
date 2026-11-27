// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-116 tests for the implemented Analysis-act owner qualification path.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a failed contract assertion is the test's failure report"
)]

use std::path::PathBuf;

use ix_trace_rs::trace;
use quire_rs::coverage::CoverageMatrixStatus;
use quoin_evidence::adapters::parse_differential_report;
use quoin_evidence::qualify::{
    ActorAdmission, CiAssociation, ExecutionAssociation, HostObservations, RetainedRun,
    RunObservation,
};
use quoin_evidence::record::{RecordRequest, record_run_with_analysis_projection};
use quoin_evidence::store::read_bindings;
use quoin_evidence::types::{
    Binding, ProducerContext, RunRecord, TrustDecision, TrustDecisionKind, TrustTrigger, TrustUse,
};
use quoin_evidence::{
    EvidenceSource, MemoryEvidence, STORE_SCHEMA_VERSION, SuiteId, TrustDecisionId,
};
use quoin_quire::coverage::Request as ScopeRequest;
use quoin_quire::evidence::{Outcome as Counted, Request as EvidenceRequest, compute};
use quoin_quire::{ModuleSelection, ids};
use serde_json::Value;

const AGREEMENT: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json"
);
const OBLIGATION: &str = "FR-001-AC-1";
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
            permitted_decisions: vec!["qualify-analysis-act".to_owned()],
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

fn host() -> HostObservations {
    HostObservations {
        reliance: Some(decision()),
        selected_head: Some(HEAD.to_owned()),
        selected_head_conflict: false,
    }
}

fn obligations() -> Vec<quoin_quire_types::Obligation> {
    [(OBLIGATION, "Analysis")]
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

fn recorded(target: quoin_quire_types::TargetLocus) -> (MemoryEvidence, RetainedRun, Vec<Binding>) {
    let source_report: Value = serde_json::from_str(AGREEMENT).unwrap();
    let report = projection(&source_report);
    let mut parsed = parse_differential_report(AGREEMENT).unwrap();
    for entry in &mut parsed.entries {
        entry.trace_ids = Some(vec![OBLIGATION.to_owned()]);
    }
    let suite = SuiteId::new("SUITE-A");
    let timestamp = "2026-11-13T10:00:00Z";
    let mut source = MemoryEvidence::new();
    let stored = record_run_with_analysis_projection(
        &mut source,
        &RecordRequest {
            suite: suite.clone(),
            tool: "filament-analyze 0.1.0".to_owned(),
            evidence_kind: None,
            lineage: None,
            timestamp: timestamp.to_owned(),
            entries: parsed.entries,
        },
        &obligations(),
        &report,
    )
    .unwrap();
    let record: RunRecord = serde_json::from_str(
        &source
            .read(&stored.run_path)
            .unwrap()
            .expect("the run is retained"),
    )
    .unwrap();
    let retained = RetainedRun {
        file: stored.run_path.clone(),
        record,
        report: Some(report),
        observation: RunObservation {
            run_kind: Some("analysis".to_owned()),
            performed_revision: Some(HEAD.to_owned()),
            execution: Some(ExecutionAssociation::Ci(CiAssociation {
                system: "github".to_owned(),
                project: "agent-ix/quoin".to_owned(),
                run: "100".to_owned(),
                attempt: "1".to_owned(),
                job: "analysis".to_owned(),
                invocation: format!("run-package:{}", stored.run_path),
            })),
            actor: Some(ActorAdmission {
                actor: "signer-1".to_owned(),
                admission_ref: "admission-7".to_owned(),
                authorized: vec![OBLIGATION.to_owned()],
            }),
            actor_rejection: None,
            current_formal_rule_status: None,
            selected_target: Some(target),
        },
    };
    let bindings = read_bindings(&source).unwrap().bindings;
    (source, retained, bindings)
}

fn target_identity() -> quoin_quire_types::TargetLocus {
    let inventory = compute(&EvidenceRequest {
        scope: scope(),
        requested: vec![OBLIGATION.to_owned()],
        requested_targets: vec![None],
        runs: Vec::new(),
        bindings: Vec::new(),
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(Vec::new()),
        host: host(),
    })
    .unwrap();
    let inventory_identity = inventory
        .report
        .coverage_matrix
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .find(|criterion| criterion.id == OBLIGATION)
        .and_then(|criterion| criterion.identity.clone())
        .expect("the selected inventory supplies this exact target identity");
    serde_json::from_value(serde_json::to_value(inventory_identity).unwrap()).unwrap()
}

fn run(target: quoin_quire_types::TargetLocus) -> (Counted, MemoryEvidence, Vec<u8>) {
    let (source, retained, bindings) = recorded(target.clone());
    let record_before = source.read(&retained.file).unwrap().unwrap().into_bytes();
    let outcome = compute(&EvidenceRequest {
        scope: scope(),
        requested: vec![OBLIGATION.to_owned()],
        requested_targets: vec![Some(target)],
        runs: vec![retained],
        bindings,
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(vec!["SUITE-A".to_owned()]),
        host: host(),
    })
    .unwrap();
    (outcome, source, record_before)
}

fn store_snapshot(source: &MemoryEvidence) -> Vec<(String, String)> {
    source
        .paths()
        .into_iter()
        .map(|path| (path.to_owned(), source.read(path).unwrap().unwrap()))
        .collect()
}

#[trace("FR-116-AC-1")]
#[test]
fn the_admitted_analysis_qualifies_only_its_exact_selected_target_and_retains_each_premise() {
    let target = target_identity();
    let (outcome, source, record_before) = run(target.clone());
    assert_eq!(
        outcome
            .report
            .coverage_matrix
            .iter()
            .flat_map(|requirement| &requirement.criteria)
            .find(|criterion| criterion.id == OBLIGATION)
            .unwrap()
            .status,
        CoverageMatrixStatus::QualifiedRecord
    );
    let selected = outcome
        .report
        .coverage_matrix
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .find(|criterion| criterion.id == OBLIGATION)
        .unwrap();
    assert_eq!(
        serde_json::to_value(selected.identity.as_ref().unwrap()).unwrap(),
        serde_json::to_value(&target).unwrap(),
        "the matrix retains the complete selected FR-078 identity"
    );
    let binders = outcome.report.record_binders.as_ref().unwrap();
    assert_eq!(binders.len(), 1);
    let binder = &binders[0];
    assert_eq!(binder.target.id, target.id);
    assert_eq!(
        (
            binder.target.source.as_str(),
            binder.target.document.as_str(),
            binder.target.id.as_str(),
            binder.target.origin.as_str(),
        ),
        (
            target.source.as_str(),
            target.document.as_str(),
            target.id.as_str(),
            target.origin.as_str(),
        )
    );
    assert_eq!(binder.method, "Analysis");
    assert_eq!(binder.actor, "signer-1");
    assert_eq!(
        binder.locator.record,
        "runs/SUITE-A/2026-11-13T10~3A00~3A00Z.json"
    );
    assert_eq!(binder.locator.run, "SUITE-A@2026-11-13T10:00:00Z");
    assert_eq!(binder.locator.entry, "consistency");
    let provenance: std::collections::BTreeSet<_> =
        binder.provenance.iter().map(String::as_str).collect();
    assert!(provenance.contains("producer-definition:analysis-act/1"));
    assert!(provenance.contains("credit:consistency-evidence-only"));
    assert!(provenance.contains("basis:conditional-on-host-admitted-observations"));
    assert!(provenance.contains("reliance:ETD-1:accepted"));
    assert!(provenance.contains("actor-authority:signer-1:admission-7"));
    assert!(provenance.contains(&format!("performed-revision:{HEAD}").as_str()));
    assert!(provenance.contains(&format!("selected-head:{HEAD}").as_str()));
    assert!(
        provenance
            .iter()
            .any(|fact| fact.starts_with("tool:filament-analyze:0.1.0"))
    );
    assert!(provenance.iter().any(|fact| fact.starts_with("engine:z3:")));
    assert!(
        provenance
            .iter()
            .any(|fact| fact.starts_with("engine:cvc5:"))
    );
    assert!(
        provenance
            .iter()
            .any(|fact| fact.starts_with("execution:ci:github/agent-ix/quoin/100/1/analysis/"))
    );
    assert!(
        outcome
            .report
            .record_binders
            .as_ref()
            .unwrap()
            .iter()
            .all(|decision| decision.target.id == target.id),
        "no different selected target receives this act"
    );
    assert_eq!(
        source.read(&binder.locator.record).unwrap().unwrap(),
        String::from_utf8(record_before).unwrap(),
        "qualification leaves the canonical retained record bytes unchanged"
    );
}

#[trace("FR-116-AC-16")]
#[test]
fn accepted_claims_do_not_create_source_binders_or_copy_records_and_legacy_absence_stays_unqualified()
 {
    let target = target_identity();
    let (source, retained, bindings) = recorded(target.clone());
    let before = store_snapshot(&source);
    assert!(
        !before.is_empty(),
        "the retained store fixture is non-empty"
    );
    let run_bytes = source.read(&retained.file).unwrap().unwrap();
    let canonical_record: Value = serde_json::from_str(&run_bytes).unwrap();
    let mut record_keys: Vec<_> = canonical_record
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    record_keys.sort_unstable();
    assert_eq!(
        record_keys,
        ["entries", "schemaVersion", "suite", "timestamp", "tool"]
    );
    assert!(canonical_record.get("performedRevision").is_none());
    assert!(canonical_record.get("selectedTarget").is_none());
    assert!(canonical_record.get("actorAdmission").is_none());

    let accepted = compute(&EvidenceRequest {
        scope: scope(),
        requested: vec![OBLIGATION.to_owned()],
        requested_targets: vec![Some(target.clone())],
        runs: vec![retained],
        bindings: bindings.clone(),
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(vec!["SUITE-A".to_owned()]),
        host: host(),
    })
    .unwrap();
    let criterion = accepted
        .report
        .coverage_matrix
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .find(|criterion| criterion.id == OBLIGATION)
        .unwrap();
    assert_eq!(criterion.status, CoverageMatrixStatus::QualifiedRecord);
    assert!(
        criterion.binders.is_empty(),
        "a record does not mint a source symbol"
    );
    assert_eq!(accepted.report.record_binders.as_ref().unwrap().len(), 1);
    assert_eq!(
        store_snapshot(&source),
        before,
        "qualification did not copy or mutate store records"
    );

    let legacy_without_a_run = compute(&EvidenceRequest {
        scope: scope(),
        requested: vec![OBLIGATION.to_owned()],
        requested_targets: vec![Some(target)],
        runs: Vec::new(),
        bindings,
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(vec!["SUITE-A".to_owned()]),
        host: host(),
    })
    .unwrap();
    let unqualified = legacy_without_a_run
        .report
        .coverage_matrix
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .find(|criterion| criterion.id == OBLIGATION)
        .unwrap();
    assert_ne!(unqualified.status, CoverageMatrixStatus::QualifiedRecord);
    assert!(!legacy_without_a_run.report.unbacked_rows.is_empty());
    assert!(
        legacy_without_a_run
            .report
            .record_binders
            .as_ref()
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(
        store_snapshot(&source),
        before,
        "the no-evidence audit also leaves the legacy store untouched"
    );
}
