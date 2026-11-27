// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Profile-selected independence in the auditor (FR-094).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_auditor::audit::{AuditInput, audit};
use quoin_evidence::ids::{ObligationId, ProfileId, SuiteId, SymbolId};
use quoin_evidence::independence::assess_independence;
use quoin_evidence::types::{
    Binding, EvidenceLineage, IndependenceDimension, IndependencePolicy, IndependenceRequirement,
    Outcome, RunEntry, RunRecord, STORE_SCHEMA_VERSION,
};
use quoin_quire_types::Obligation;
use serde_json::Value;

const OBLIGATION_ID: &str = "FR-094-AC-6";

fn obligation() -> Obligation {
    Obligation {
        id: OBLIGATION_ID.to_owned(),
        identity: None,
        document: None,
        statement: "two qualifying evidence relationships SHALL differ by actor".to_owned(),
        target_ids: None,
        method: None,
        criticality: None,
        parameters: None,
    }
}

fn binding(suite: &str, actor: &str) -> Binding {
    Binding {
        obligation: ObligationId::new(OBLIGATION_ID),
        suite: SuiteId::new(suite),
        symbols: vec![SymbolId::new(format!("{suite}::evidence"))],
        lineage: Some(EvidenceLineage {
            actor: Some(actor.to_owned()),
            ..EvidenceLineage::default()
        }),
    }
}

fn run(binding: &Binding) -> RunRecord {
    let suite = binding.suite.clone();
    let symbol = binding.symbols[0].as_str().to_owned();
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite,
        tool: "test-runner".to_owned(),
        evidence_kind: Some("test-run".to_owned()),
        timestamp: "2026-11-27T00:00:00Z".to_owned(),
        entries: vec![RunEntry::new(symbol, Outcome::Pass)],
    }
}

fn selected_input(bindings: Vec<Binding>) -> AuditInput {
    let requirement = IndependenceRequirement {
        id: "independent-actor".to_owned(),
        obligation: ObligationId::new(OBLIGATION_ID),
        dimensions: vec![IndependenceDimension::Actor],
        rationale: "The profile requires independent actors.".to_owned(),
    };
    let assessment = assess_independence(
        &ProfileId::parse("AP-094").unwrap(),
        &requirement,
        &bindings,
    );
    let runs = bindings.iter().map(run).collect();
    let mock_inspection_suites = Some(
        bindings
            .iter()
            .map(|binding| binding.suite.as_str().to_owned())
            .collect(),
    );
    AuditInput {
        obligations: vec![obligation()],
        bindings,
        runs,
        mock_inspection_suites,
        independence: Some(vec![assessment]),
        independence_policy: Some(IndependencePolicy {
            schema_version: 1,
            profile: ProfileId::parse("AP-094").unwrap(),
            requirements: vec![requirement],
        }),
        ..AuditInput::default()
    }
}

fn report(input: &AuditInput) -> Value {
    serde_json::to_value(audit(input).expect("valid profile independence input"))
        .expect("audit report serializes")
}

#[trace("FR-094-AC-5")]
#[test]
fn no_independence_policy_preserves_the_existing_audit_output() {
    let bindings = vec![binding("suite-a", "alice"), binding("suite-b", "alice")];
    let without_profile_data = AuditInput {
        obligations: vec![obligation()],
        bindings: bindings.clone(),
        ..AuditInput::default()
    };
    let assessment = assess_independence(
        &ProfileId::parse("AP-094").unwrap(),
        &IndependenceRequirement {
            id: "unused-profile-requirement".to_owned(),
            obligation: ObligationId::new(OBLIGATION_ID),
            dimensions: vec![IndependenceDimension::Actor],
            rationale: "Not selected by a profile.".to_owned(),
        },
        &bindings,
    );
    let with_unselected_assessment = AuditInput {
        independence: Some(vec![assessment]),
        ..without_profile_data.clone()
    };

    let baseline = report(&without_profile_data);
    let with_ignored_assessment = report(&with_unselected_assessment);
    assert_eq!(with_ignored_assessment, baseline);
    assert!(baseline.get("independence").is_none(), "{baseline}");
    assert!(
        baseline["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["kind"] != "insufficient-independence")
    );
}

#[trace("FR-094-AC-6")]
#[test]
fn selected_obligation_fails_until_two_bound_actors_are_distinct() {
    let one_line = report(&selected_input(vec![binding("suite-a", "alice")]));
    let insufficient: Vec<_> = one_line["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["kind"] == "insufficient-independence")
        .collect();
    assert_eq!(insufficient.len(), 1, "{one_line}");
    assert_eq!(insufficient[0]["obligation"], OBLIGATION_ID);

    let two_shared_actors = report(&selected_input(vec![
        binding("suite-a", "alice"),
        binding("suite-b", "alice"),
    ]));
    assert!(
        two_shared_actors["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["kind"] == "insufficient-independence")
    );

    let separated = report(&selected_input(vec![
        binding("suite-a", "alice"),
        binding("suite-b", "bob"),
    ]));
    assert!(
        separated["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["kind"] != "insufficient-independence")
    );
    assert!(
        separated["healthy"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == OBLIGATION_ID)
    );
}
