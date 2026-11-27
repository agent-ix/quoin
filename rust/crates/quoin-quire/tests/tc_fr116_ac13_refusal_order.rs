// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Invalid selected targets retain their distinct refusal cause and order.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixtures use expect and indexing as the failure report"
)]

use ix_trace_rs::trace;
use quire_rs::evidence::CauseCode;
use quoin_evidence::qualify::HostObservations;
use quoin_quire::coverage::Request as ScopeRequest;
use quoin_quire::evidence::{Request, compute};
use quoin_quire::{ModuleSelection, ids};
use std::path::PathBuf;

fn scope() -> ScopeRequest {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repo");
    ScopeRequest {
        scope: ids::ScopeRoot::open(&root).expect("fixture scope exists"),
        modules: ModuleSelection::Closed(vec![
            ids::ModuleRoot::open(&root).expect("fixture module exists"),
        ]),
    }
}

#[trace("FR-116-AC-13")]
#[test]
fn nonexistent_targets_keep_invalid_target_cause_and_requested_order() {
    let requested = ["FR-999-AC-2", "FR-999-AC-1"];
    let outcome = compute(&Request {
        scope: scope(),
        requested: requested.iter().map(|id| (*id).to_owned()).collect(),
        requested_targets: vec![None, None],
        runs: vec![],
        bindings: vec![],
        injections: None,
        mock_inspection_suites: None,
        host: HostObservations::default(),
    })
    .expect("coverage and refusal reconciliation complete");
    let results = outcome
        .report
        .evidence_results
        .expect("unresolved requests are reported");
    assert_eq!(results.len(), requested.len());
    for (result, id) in results.iter().zip(requested) {
        assert_eq!(result.cause, CauseCode::InvalidTarget);
        assert_eq!(result.requested_target, id);
        assert!(
            result.locator.is_none(),
            "no available record locator is invented"
        );
    }
}
