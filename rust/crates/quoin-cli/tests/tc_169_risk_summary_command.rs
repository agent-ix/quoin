// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin risk-summary` command surface (FR-169, FR-170).
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

fn run(command: &str, scope: &Path, extra: &[&str]) -> Output {
    let mut args: Vec<&str> = if command == "advise" {
        vec!["risk", "advise"]
    } else {
        vec!["risk-summary"]
    };
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .arg(scope)
        .arg("--module")
        .arg(module_root())
        .output()
        .unwrap()
}

fn summary(scope: &Path, extra: &[&str]) -> Output {
    run("summary", scope, extra)
}

fn put(dir: &Path, id: &str, kind: &str, front: &str, body: &str) {
    let text = format!(
        "---\nid: {id}\ntitle: \"{id}\"\ntype: {kind}\nobject: {kind}\n{front}---\n# [{id}] {id}\n\n{body}\
         \n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| record_id | UUID | 1..1 | identity |\n"
    );
    std::fs::write(dir.join(format!("{id}.md")), text).unwrap();
}

fn chain(dir: &Path) {
    put(dir, "HAZ-1", "hazard", "", "");
    put(
        dir,
        "SIT-1",
        "hazardous_situation",
        "relationships:\n  - target: HAZ-1\n    type: arises_from\n",
        "",
    );
    put(
        dir,
        "EST-1",
        "risk_estimate",
        "phase: post_control\nrelationships:\n  - target: SIT-1\n    type: estimates\n",
        "## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| Serious | Likely | why |\n\n\
         ## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n",
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

#[trace("FR-169-AC-15", "FR-169-AC-26")]
#[test]
fn an_empty_scope_exits_zero_and_markdown_is_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let out = summary(dir.path(), &["--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["rows"], serde_json::json!([]));

    chain(dir.path());
    let plain = summary(dir.path(), &[]);
    let markdown = summary(dir.path(), &["--format", "markdown"]);
    assert_eq!(plain.status.code(), Some(0));
    assert_eq!(plain.stdout, markdown.stdout);
    assert!(String::from_utf8_lossy(&plain.stdout).contains("| HAZ-1 | SIT-1 |"));
}

#[trace("FR-169-AC-18")]
#[test]
fn an_unknown_format_exits_2_with_no_report() {
    let dir = tempfile::tempdir().unwrap();
    chain(dir.path());
    let out = summary(dir.path(), &["--format", "xml"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[trace("FR-169-AC-10")]
#[test]
fn neither_format_changes_a_file() {
    let dir = tempfile::tempdir().unwrap();
    chain(dir.path());
    let before = tree(dir.path());
    for format in ["markdown", "json"] {
        assert_eq!(
            summary(dir.path(), &["--format", format]).status.code(),
            Some(0)
        );
    }
    assert_eq!(tree(dir.path()), before);
}

#[trace("FR-169-AC-16", "FR-169-AC-20", "FR-169-AC-21")]
#[test]
fn each_failure_exits_2_with_the_diagnostic_code_of_risk_advise_and_an_unreadable_file_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let stderr = |out: &Output| String::from_utf8_lossy(&out.stderr).into_owned();

    let absent = dir.path().join("absent");
    let (a, s) = (run("advise", &absent, &[]), summary(&absent, &[]));
    assert_eq!(s.status.code(), Some(2));
    assert!(s.stdout.is_empty());
    assert!(stderr(&s).contains("risk.scope_missing"), "{}", stderr(&s));
    assert!(stderr(&a).contains("risk.scope_missing"));

    let out = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("risk-summary")
        .arg(dir.path())
        .args(["--module", "/nonexistent-module-root"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("risk.module_missing"),
        "{}",
        stderr(&out)
    );

    let process = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    std::fs::write(
        dir.path().join("RR-1.md"),
        "---\nid: RR-1\ntitle: x\ntype: residual_risk\nobject: residual_risk\n---\n# x\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("risk-summary")
        .arg(dir.path())
        .arg("--module")
        .arg(process.join("spec_artifacts_process"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("risk.type_undeclared"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("--module DIR"));

    let out = summary(dir.path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        stderr(&out).contains("risk.record_invalid"),
        "{}",
        stderr(&out)
    );

    std::fs::remove_file(dir.path().join("RR-1.md")).unwrap();
    chain(dir.path());
    std::fs::write(dir.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    let out = summary(dir.path(), &["--format", "json"]);
    assert!(out.status.success());
    assert!(
        stderr(&out).contains("risk.document_unreadable"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("RR-9.md"));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("RR-9"));
}
