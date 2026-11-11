// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `evidence.record` for a cargo-deny scan (FR-034) and for an SBOM that
//! discharges an obligation (FR-041), through the production runtime.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;

use ix_trace_rs::trace;
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

fn record(request: &Value) -> Value {
    let response = dispatch("evidence.record", request, &RuntimeSettings::default()).unwrap();
    response.payload
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("spec/evidence")).unwrap();
    dir
}

fn obligation(id: &str) -> Value {
    json!({"id": id, "statement": "s"})
}

/// A cargo-deny scan is recorded under `--tool cargo-deny <version>` as a
/// finding record in `scans/`, not refused as another tool's output and not
/// written to `runs/`.
#[trace("FR-034-AC-24", "TC-2703")]
#[test]
fn tc_2703_a_cargo_deny_scan_records_under_its_own_tool() {
    let dir = repo();
    let payload = record(&json!({
        "repo": dir.path(),
        "suite": "SUITE-DENY",
        "tool": "cargo-deny 0.19.8",
        "timestamp": "2026-01-01T00:00:00Z",
        "results": r#"{"type":"summary","fields":{}}"#,
    }));
    assert_eq!(payload["kind"], "scan", "{payload}");
    assert_eq!(payload["findings"], json!([]));
    assert!(!dir.path().join("spec/evidence/runs").exists());
}

/// An SBOM carries no trace ids, so the obligation it discharges is stated
/// with `--discharges`; every component is a passing entry, so the obligation
/// binds, and an id no obligation states is reported rather than dropped.
#[trace("FR-041-AC-7", "FR-041-AC-8", "TC-2704", "TC-2705")]
#[test]
fn tc_2704_an_sbom_discharges_the_obligation_it_is_stated_to_check() {
    let dir = repo();
    let sbom = json!({
        "bomFormat": "CycloneDX",
        "components": [{"name": "left", "version": "1.0.0", "purl": "pkg:npm/left@1.0.0"}],
    })
    .to_string();
    let payload = record(&json!({
        "repo": dir.path(),
        "suite": "SUITE-SBOM",
        "tool": "syft 1.0.0",
        "timestamp": "2026-01-01T00:00:00Z",
        "adapter": "sbom",
        "results": sbom,
        "discharges": ["SCA-1", "NO-SUCH-1"],
        "obligations": [obligation("SCA-1")],
    }));
    assert_eq!(payload["kind"], "run", "{payload}");
    assert_eq!(payload["bound"], json!(["SCA-1"]));
    assert_eq!(payload["unmatched"], json!(["NO-SUCH-1"]));
    let bindings: Value = serde_json::from_str(
        &fs::read_to_string(dir.path().join("spec/evidence/bindings.json")).unwrap(),
    )
    .unwrap();
    assert!(
        bindings.to_string().contains("pkg:npm/left@1.0.0"),
        "{bindings}"
    );
}

/// Without `--discharges` an SBOM binds nothing: it names no obligation.
#[trace("FR-041-AC-7", "TC-2704")]
#[test]
fn tc_2704_an_sbom_with_no_stated_with_no_stated_obligation_binds_nothing() {
    let dir = repo();
    let payload = record(&json!({
        "repo": dir.path(),
        "suite": "SUITE-SBOM",
        "tool": "syft 1.0.0",
        "timestamp": "2026-01-01T00:00:00Z",
        "adapter": "sbom",
        "results": r#"{"bomFormat":"CycloneDX","components":[{"name":"a"}]}"#,
        "obligations": [obligation("SCA-1")],
    }));
    assert_eq!(payload["bound"], json!([]));
}
