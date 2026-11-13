// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin risk advise` command surface (FR-145).
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
         place and keep no copy: check out agent-ix/spec-objects-safety (at or after \
         PLAT-1565) next to this repository, or set SPEC_OBJECTS_SAFETY_DIR to its checkout",
        root.display()
    );
    root
}

fn advise(scope: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["risk", "advise"])
        .arg(scope)
        .arg("--module")
        .arg(module_root())
        .args(extra)
        .output()
        .unwrap()
}

#[trace("FR-145-AC-1", "FR-145-AC-4")]
#[test]
fn json_over_an_empty_scope_exits_zero_with_the_four_keys() {
    let dir = tempfile::tempdir().unwrap();
    let out = advise(dir.path(), &["--format", "json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut keys: Vec<&str> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "attribution",
            "decision_queue",
            "evidence_gaps",
            "policy_cells"
        ]
    );
}

#[trace("FR-145-AC-4")]
#[test]
fn markdown_is_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let out = advise(dir.path(), &[]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("policy_cells"));
}

#[trace("FR-145-AC-7")]
#[test]
fn a_missing_scope_and_a_malformed_record_each_exit_2_naming_what_is_wrong() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("absent");
    let out = advise(&missing, &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("absent"));

    std::fs::write(
        dir.path().join("RR-1.md"),
        "---\nid: RR-1\ntitle: \"RR-1\"\ntype: residual_risk\nobject: residual_risk\n---\n# [RR-1] RR-1\n\n\
         ## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| maybe | r | |\n\n\
         ## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| id | UUID | 1..1 | identity |\n",
    )
    .unwrap();
    let out = advise(dir.path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("RR-1"));
}
