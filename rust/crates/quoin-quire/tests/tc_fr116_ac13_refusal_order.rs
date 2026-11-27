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
use std::fs;
use std::path::{Path, PathBuf};

fn scope() -> ScopeRequest {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/analysis");
    ScopeRequest {
        scope: ids::ScopeRoot::open(&root).expect("fixture scope exists"),
        modules: ModuleSelection::Closed(vec![
            ids::ModuleRoot::open(&root).expect("fixture module exists"),
        ]),
    }
}

fn scope_at(root: &Path) -> ScopeRequest {
    ScopeRequest {
        scope: ids::ScopeRoot::open(root).expect("fixture scope exists"),
        modules: ModuleSelection::Closed(vec![
            ids::ModuleRoot::open(root).expect("fixture module exists"),
        ]),
    }
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("temporary fixture root is created");
    for item in fs::read_dir(source).expect("fixture directory reads") {
        let item = item.expect("fixture entry reads");
        let target = destination.join(item.file_name());
        if item.file_type().expect("fixture type reads").is_dir() {
            copy_tree(&item.path(), &target);
        } else {
            fs::copy(item.path(), target).expect("fixture file copies");
        }
    }
}

#[trace("FR-116-AC-13")]
#[test]
fn nonexistent_targets_keep_invalid_target_cause_and_stable_order() {
    let requested = ["FR-999-AC-2", "FR-999-AC-1"];
    let run = || {
        compute(&Request {
            scope: scope(),
            requested: requested.iter().map(|id| (*id).to_owned()).collect(),
            requested_targets: vec![None, None],
            runs: vec![],
            bindings: vec![],
            injections: None,
            mock_inspection_suites: None,
            host: HostObservations::default(),
        })
        .expect("coverage and refusal reconciliation complete")
        .report
        .evidence_results
        .expect("unresolved requests are reported")
    };
    let results = run();
    assert_eq!(results, run(), "refusal order is deterministic");
    assert_eq!(results.len(), requested.len(), "{results:#?}");
    assert_eq!(results[0].requested_target, "FR-999-AC-1");
    assert_eq!(results[1].requested_target, "FR-999-AC-2");
    for result in &results {
        assert_eq!(result.cause, CauseCode::InvalidTarget);
        assert!(
            result.locator.is_none(),
            "no available record locator is invented"
        );
    }
}

#[trace("FR-116-AC-13")]
#[test]
fn duplicate_selected_rows_keep_ambiguous_cause_locator_absence_and_stable_order() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/analysis");
    let temporary = tempfile::tempdir().expect("isolated duplicate-row fixture exists");
    copy_tree(&source, temporary.path());
    fs::write(
        temporary.path().join("spec/FR-002-other.md"),
        "---\nid: FR-002\ntype: FR\ntitle: Duplicate rendered target\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | The same rendered criterion appears in another source document. | Analysis |\n",
    )
    .expect("second selected row is authored");
    let selected_scope = scope_at(temporary.path());
    let requested = ["FR-001-AC-2", "FR-001-AC-1"];
    let run = || {
        compute(&Request {
            scope: selected_scope.clone(),
            requested: requested.iter().map(|id| (*id).to_owned()).collect(),
            requested_targets: vec![None, None],
            runs: vec![],
            bindings: vec![],
            injections: Some(vec![]),
            mock_inspection_suites: Some(vec![]),
            host: HostObservations::default(),
        })
        .expect("duplicate target refusal reports successfully")
        .report
        .evidence_results
        .expect("unresolved decisions appear in the report")
    };
    let first = run();
    let second = run();
    assert_eq!(first, second, "the same request has stable result order");
    assert_eq!(first.len(), requested.len(), "{first:#?}");
    for (result, target) in first.iter().zip(requested) {
        assert_eq!(result.requested_target, target);
        let expected = if target == "FR-001-AC-1" {
            CauseCode::AmbiguousTarget
        } else {
            CauseCode::Absent
        };
        assert_eq!(result.cause, expected);
        assert!(
            result.locator.is_none(),
            "no available record locator is invented"
        );
    }
}
