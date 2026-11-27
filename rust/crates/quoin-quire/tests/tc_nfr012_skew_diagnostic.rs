// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Missing fields from a skewed Quire engine fail as named diagnostics.

#![allow(
    clippy::expect_used,
    reason = "test fixtures use expect as the failure report"
)]

use ix_trace_rs::trace;
use quoin_quire::{CoveragePayload, ModuleSelection, PayloadLimit, ScopeRoot, ids, payload};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repo")
}

#[trace("NFR-012-M-1")]
#[test]
fn an_engine_payload_missing_a_required_capability_is_not_an_empty_report() {
    let scope = ScopeRoot::open(root()).expect("fixture repository exists");
    let modules = ModuleSelection::Closed(vec![
        ids::ModuleRoot::open(root()).expect("fixture module exists"),
    ]);
    let report = quoin_quire::coverage::compute(&quoin_quire::coverage::Request { scope, modules })
        .expect("the current engine computes its complete report");
    let mut stale = serde_json::to_value(CoveragePayload::from_report(report.report))
        .expect("current engine report serializes");
    let removed = stale
        .as_object_mut()
        .expect("flattened object")
        .remove("unmatched_tags");
    assert!(
        removed.is_some(),
        "fixture report exercises the capability field"
    );
    let error = payload::from_slice::<CoveragePayload>(
        "coverage",
        "CoverageReport",
        &serde_json::to_vec(&stale).expect("stale report serializes"),
        PayloadLimit::DEFAULT,
    )
    .expect_err("a skewed engine must not masquerade as a clean empty report");
    assert_eq!(error.code(), quoin_quire::ErrorCode::PayloadShape);
    assert!(
        error.to_string().contains("unmatched_tags"),
        "named skew diagnostic: {error}"
    );
}
