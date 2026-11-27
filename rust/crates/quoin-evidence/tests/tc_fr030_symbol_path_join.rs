// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run-entry paths disambiguate repeated qualified symbols without suppressing explicit traces (FR-030).

#![allow(
    clippy::expect_used,
    reason = "test fixtures deliberately panic to report a failed store contract"
)]

use ix_trace_rs::trace;
use quoin_evidence::record::{RecordRequest, record_run_with_coverage_matrix};
use quoin_evidence::types::{Outcome, RunEntry};
use quoin_evidence::{MemoryEvidence, SuiteId};
use quoin_quire_types::{
    CoverageMatrixBinder, CoverageMatrixCriterion, CoverageMatrixRequirement, Obligation,
};

fn obligation(id: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        identity: None,
        document: None,
        statement: format!("the service retains {id}"),
        target_ids: None,
        method: Some("Test".to_owned()),
        criticality: None,
        parameters: None,
    }
}

fn criterion(id: &str, path: &str) -> CoverageMatrixCriterion {
    CoverageMatrixCriterion {
        id: id.to_owned(),
        method: Some("Test".to_owned()),
        binders: vec![CoverageMatrixBinder {
            path: path.to_owned(),
            line: 1,
            column: 1,
            qualified_name: "tests::duplicated".to_owned(),
            kind: "test_function".to_owned(),
            ignored: false,
        }],
        identity: None,
        unbacked_claims: Vec::new(),
        status: "tagged".to_owned(),
    }
}

fn matrix() -> Vec<CoverageMatrixRequirement> {
    vec![CoverageMatrixRequirement {
        document: "spec/FR-001.md".to_owned(),
        carried: None,
        criteria: vec![
            criterion("FR-001-AC-1", "tests/one.rs"),
            criterion("FR-001-AC-2", "tests/two.rs"),
        ],
    }]
}

fn request(suite: &str, entry: RunEntry) -> RecordRequest {
    RecordRequest {
        suite: SuiteId::new(suite),
        tool: "vitest 3.2.4".to_owned(),
        evidence_kind: None,
        lineage: None,
        timestamp: "2026-11-01T00:00:00Z".to_owned(),
        entries: vec![entry],
    }
}

#[trace("FR-030-AC-18")]
#[test]
fn tc_fr030_ambiguous_symbol_is_reported_but_explicit_trace_id_still_binds() {
    let mut source = MemoryEvidence::new();
    let mut entry = RunEntry::new("tests::duplicated", Outcome::Pass);
    entry.trace_ids = Some(vec!["FR-001-AC-1".to_owned()]);
    let result = record_run_with_coverage_matrix(
        &mut source,
        &request("SUITE-AMBIGUOUS", entry),
        &[obligation("FR-001-AC-1"), obligation("FR-001-AC-2")],
        &matrix(),
    )
    .expect("ambiguous symbol is retained and reported");

    assert_eq!(result.ambiguous, ["tests::duplicated"]);
    assert_eq!(
        result.bound,
        [quoin_evidence::ObligationId::new("FR-001-AC-1")]
    );
}

#[trace("FR-030-AC-18")]
#[test]
fn tc_fr030_entry_path_joins_only_the_symbol_at_that_path() {
    let mut source = MemoryEvidence::new();
    let mut entry = RunEntry::new("tests::duplicated", Outcome::Pass);
    entry.path = Some("tests/two.rs".to_owned());
    let result = record_run_with_coverage_matrix(
        &mut source,
        &request("SUITE-PATHED", entry),
        &[obligation("FR-001-AC-1"), obligation("FR-001-AC-2")],
        &matrix(),
    )
    .expect("the path resolves the duplicate symbol");

    assert!(result.ambiguous.is_empty());
    assert_eq!(
        result.bound,
        [quoin_evidence::ObligationId::new("FR-001-AC-2")]
    );
}
