// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin risk discrepancies` command surface (FR-154, FR-155).
//!
//! The record types come from `spec-objects-safety`, read in place from
//! `SPEC_OBJECTS_SAFETY_DIR` or the checkout next to this repository.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn module_root() -> PathBuf {
    let checkout = std::env::var_os("SPEC_OBJECTS_SAFETY_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
        PathBuf::from,
    );
    let root = checkout.join("spec_objects_safety");
    assert!(
        root.join("manifest.yaml").is_file(),
        "the spec-objects-safety module is not at {}. These tests read its record types in \
         place and keep no copy: check out agent-ix/spec-objects-safety next to this \
         repository, or set SPEC_OBJECTS_SAFETY_DIR to its checkout",
        root.display()
    );
    root
}

fn discrepancies(scope: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["risk", "discrepancies"])
        .arg(scope)
        .arg("--module")
        .arg(module_root())
        .args(extra)
        .output()
        .unwrap()
}

fn put(dir: &Path, id: &str, kind: &str, front: &str, body: &str) {
    let text = format!(
        "---\nid: {id}\ntitle: \"{id}\"\ntype: {kind}\nobject: {kind}\n{front}---\n# [{id}] {id}\n\n{body}\
         \n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| record_id | UUID | 1..1 | identity |\n"
    );
    std::fs::write(dir.join(format!("{id}.md")), text).unwrap();
}

/// One `accepted` residual risk in the High region, which is outside the
/// acceptable region, with no benefit-risk record.
fn with_a_finding(dir: &Path) {
    put(
        dir,
        "POL-1",
        "acceptability_policy",
        "",
        "## Severity levels\n\n| Severity level |\n| --- |\n| Serious |\n\n\
         ## Probability levels\n\n| Probability level |\n| --- |\n| Likely |\n\n\
         ## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| High | true |\n\n\
         ## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n| Serious | Likely | High |\n",
    );
    put(
        dir,
        "EST-1",
        "risk_estimate",
        "phase: post_control\n",
        "## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| Serious | Likely | why |\n\n\
         ## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n",
    );
    put(
        dir,
        "RR-1",
        "residual_risk",
        "relationships:\n  - target: EST-1\n    type: estimate\n  - target: POL-1\n    type: references\n",
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| accepted | r | Pat |\n",
    );
}

fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| (entry.path(), std::fs::read(entry.path()).unwrap()))
        .collect();
    files.sort();
    files
}

#[trace("FR-154-AC-1", "FR-154-AC-2", "FR-154-AC-5", "FR-155-AC-12")]
#[test]
fn strict_exits_1_only_with_findings_and_changes_neither_the_report_nor_a_file() {
    let dir = tempfile::tempdir().unwrap();
    with_a_finding(dir.path());
    let before = tree(dir.path());

    let plain = discrepancies(dir.path(), &["--format", "json"]);
    assert_eq!(plain.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&plain.stdout).unwrap();
    assert_eq!(
        report["findings"][0]["finding"],
        "accepted-outside-acceptable-region"
    );

    let strict = discrepancies(dir.path(), &["--format", "json", "--strict"]);
    assert_eq!(strict.status.code(), Some(1));
    assert_eq!(strict.stdout, plain.stdout);

    assert_eq!(tree(dir.path()), before);
}

#[trace("FR-154-AC-4", "FR-154-AC-5")]
#[test]
fn an_empty_scope_exits_zero_even_when_strict() {
    let dir = tempfile::tempdir().unwrap();
    let out = discrepancies(dir.path(), &["--strict", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["findings"], serde_json::json!([]));
}

#[trace("FR-154-AC-9")]
#[test]
fn a_missing_scope_and_a_malformed_record_exit_2_with_and_without_strict() {
    let dir = tempfile::tempdir().unwrap();
    for extra in [&[][..], &["--strict"][..]] {
        let out = discrepancies(&dir.path().join("absent"), extra);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("absent"));
    }
    put(
        dir.path(),
        "RR-1",
        "residual_risk",
        "",
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| maybe | r | |\n",
    );
    for extra in [&[][..], &["--strict"][..]] {
        let out = discrepancies(dir.path(), extra);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("RR-1"));
    }
}
