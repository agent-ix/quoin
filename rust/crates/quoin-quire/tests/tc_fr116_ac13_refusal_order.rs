// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-116 unresolved target causes retain their request identity and order.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report"
)]

use std::{fs, path::Path};

use ix_trace_rs::trace;
use quire_rs::evidence::{CauseCode, EvidenceResult};
use quoin_evidence::qualify::HostObservations;
use quoin_quire::coverage::Request as ScopeRequest;
use quoin_quire::evidence::{Request, compute};
use quoin_quire::{ModuleSelection, ids};

const EXISTING_ID: &str = "FR-001-AC-1";
const UNKNOWN_ID: &str = "FR-999-AC-1";

fn fixture() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("analysis")
}

fn copy_tree(source: &Path, destination: &Path) {
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

fn count(scope: ScopeRequest, requested: &[&str]) -> Vec<EvidenceResult> {
    compute(&Request {
        scope,
        requested: requested.iter().map(|id| (*id).to_owned()).collect(),
        requested_targets: vec![None; requested.len()],
        runs: Vec::new(),
        bindings: Vec::new(),
        injections: Some(Vec::new()),
        mock_inspection_suites: Some(Vec::new()),
        host: HostObservations::default(),
    })
    .unwrap()
    .report
    .evidence_results
    .unwrap_or_default()
}

fn scope_at(root: &Path) -> ScopeRequest {
    ScopeRequest {
        scope: ids::ScopeRoot::open(root).unwrap(),
        modules: ModuleSelection::Closed(vec![ids::ModuleRoot::open(root).unwrap()]),
    }
}

#[trace("FR-116-AC-13")]
#[test]
fn invalid_and_ambiguous_targets_keep_distinct_causes_and_stable_order() {
    let source = fixture();
    let temporary = tempfile::tempdir().unwrap();
    copy_tree(&source, temporary.path());
    fs::write(
        temporary.path().join("spec/FR-002-other.md"),
        "---\nid: FR-002\ntype: FR\ntitle: Duplicate rendered target\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | The same rendered criterion appears in another source document. | Analysis |\n",
    )
    .unwrap();

    let scope = scope_at(temporary.path());
    let forward = count(scope.clone(), &[UNKNOWN_ID, EXISTING_ID]);
    let reverse = count(scope, &[EXISTING_ID, UNKNOWN_ID]);

    assert_eq!(
        forward, reverse,
        "requested order must not affect stable refusal ordering"
    );
    assert_eq!(forward.len(), 2);
    let invalid = forward
        .iter()
        .find(|result| result.requested_target == UNKNOWN_ID)
        .unwrap();
    assert_eq!(invalid.cause, CauseCode::InvalidTarget);
    assert_eq!(invalid.detail, "no selected obligation has this id");
    assert!(invalid.target.is_none());
    assert!(invalid.locator.is_none());

    let ambiguous = forward
        .iter()
        .find(|result| result.requested_target == EXISTING_ID)
        .unwrap();
    assert_eq!(ambiguous.cause, CauseCode::AmbiguousTarget);
    assert_eq!(ambiguous.detail, "more than one selected row has this id");
    assert!(ambiguous.target.is_none());
    assert!(ambiguous.locator.is_none());
    assert_ne!(invalid.cause, ambiguous.cause);
}
