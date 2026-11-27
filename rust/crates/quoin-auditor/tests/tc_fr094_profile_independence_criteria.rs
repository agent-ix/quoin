// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Profile-selected independence without changing class-policy behavior.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quoin_auditor::audit::{AuditInput, audit};
use quoin_auditor::catalog::{MethodCatalog, VerificationMethod};
use quoin_evidence::ids::{ObligationId, ProfileId, SuiteId, SymbolId};
use quoin_evidence::independence::assess_independence;
use quoin_evidence::types::{
    Binding, EvidenceLineage, IndependenceDimension, IndependencePolicy, IndependenceRequirement,
    Outcome, RunEntry, RunRecord, STORE_SCHEMA_VERSION,
};
use quoin_quire_types::Obligation;
use serde_json::Value;

const OBLIGATION: &str = "FR-001-AC-1";

fn lineage(actor: &str, technique: &str) -> EvidenceLineage {
    EvidenceLineage {
        actor: Some(actor.to_owned()),
        technique: Some(technique.to_owned()),
        ..EvidenceLineage::default()
    }
}

fn input(actors: [&str; 2], profile_selected: bool) -> AuditInput {
    let suites = ["SUITE-A", "SUITE-B"];
    let bindings = suites
        .iter()
        .zip(actors)
        .map(|(suite, actor)| Binding {
            obligation: ObligationId::new(OBLIGATION),
            suite: SuiteId::new(*suite),
            symbols: vec![SymbolId::new(format!("{suite}::test"))],
            lineage: Some(lineage(actor, "fuzz")),
        })
        .collect::<Vec<_>>();
    let runs = suites
        .iter()
        .map(|suite| RunRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: SuiteId::new(*suite),
            tool: "test runner".to_owned(),
            evidence_kind: Some("test-run".to_owned()),
            timestamp: "2026-11-26T00:00:00Z".to_owned(),
            entries: vec![RunEntry::new(format!("{suite}::test"), Outcome::Pass)],
        })
        .collect::<Vec<_>>();
    let mut result = AuditInput {
        obligations: vec![Obligation {
            id: OBLIGATION.to_owned(),
            identity: None,
            document: None,
            statement: "the system SHALL satisfy the criterion".to_owned(),
            target_ids: None,
            method: Some("unit-testing".to_owned()),
            criticality: None,
            parameters: None,
        }],
        bindings,
        runs,
        mock_inspection_suites: Some(suites.iter().map(|suite| (*suite).to_owned()).collect()),
        catalog: Some(MethodCatalog {
            methods: vec![VerificationMethod {
                id: "unit-testing".to_owned(),
                name: "Unit testing".to_owned(),
                class: "Test".to_owned(),
                definition: String::new(),
                evidence_kind: Some("test-run".to_owned()),
                applicability: BTreeMap::new(),
                unreachable_reason: None,
                tooling: Vec::new(),
                module_name: "test-module".to_owned(),
            }],
            duplicates: Vec::new(),
            unreadable: Vec::new(),
        }),
        ..AuditInput::default()
    };

    if profile_selected {
        let requirement = IndependenceRequirement {
            id: "IND-1".to_owned(),
            obligation: ObligationId::new(OBLIGATION),
            dimensions: vec![IndependenceDimension::Actor],
            rationale: "selected by the profile".to_owned(),
        };
        let policy = IndependencePolicy {
            schema_version: 1,
            profile: ProfileId::parse("AP-1").unwrap(),
            requirements: vec![requirement.clone()],
        };
        result.independence = Some(vec![assess_independence(
            &policy.profile,
            &requirement,
            &result.bindings,
        )]);
        result.independence_policy = Some(policy);
    }
    result
}

fn findings_of_kind<'a>(
    report: &'a quoin_finding_types::AuditReport,
    kind: &str,
) -> Vec<&'a BTreeMap<String, Value>> {
    report
        .findings
        .iter()
        .filter(|finding| finding.kind.as_str() == kind)
        .map(|finding| &finding.other)
        .collect()
}

#[trace("FR-094-AC-5", "FR-094-CON-2")]
#[test]
fn tc_fr094_no_profile_policy_preserves_the_existing_audit_behavior() {
    let report = audit(&input(["ann", "ann"], false)).unwrap();

    assert!(!report.other.contains_key("independence"));
    assert!(findings_of_kind(&report, "insufficient-independence").is_empty());
}

#[trace("FR-094-AC-6")]
#[test]
fn tc_fr094_selected_obligation_clears_after_two_qualifying_relationships() {
    let shared_actor = audit(&input(["ann", "ann"], true)).unwrap();
    assert_eq!(
        findings_of_kind(&shared_actor, "insufficient-independence").len(),
        1
    );

    let distinct_actors = audit(&input(["ann", "bob"], true)).unwrap();
    assert!(findings_of_kind(&distinct_actors, "insufficient-independence").is_empty());
}
