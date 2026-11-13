// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin anomaly` command surface (FR-136).
//!
//! Anomaly documents are the fixtures `spec-artifacts-process` ships, read in
//! place from `SPEC_ARTIFACTS_PROCESS_DIR` or the checkout next to this one.
//! That checkout must hold `tests/fixtures/anomaly` (spec-artifacts-process
//! PLAT-1415, #140); nothing is copied into this repository.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn fixtures() -> PathBuf {
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    // Canonicalised: `quire` rejects `..` in `--module`.
    let root = std::fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "the spec-artifacts-process checkout is not at {}: {error}. These tests read \
             the documents that repository ships in place and keep no copy: check out \
             agent-ix/spec-artifacts-process next to this repository, or set \
             SPEC_ARTIFACTS_PROCESS_DIR to its checkout",
            root.display()
        )
    });
    let dir = root.join("tests/fixtures/anomaly");
    assert!(
        dir.is_dir(),
        "the spec-artifacts-process anomaly fixtures are not at {}. These tests read \
         the documents that repository ships in place and keep no copy: check out \
         agent-ix/spec-artifacts-process (at or after PLAT-1415, #140) next to this \
         repository, or set SPEC_ARTIFACTS_PROCESS_DIR to its checkout",
        dir.display()
    );
    dir
}

fn quoin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .output()
        .unwrap()
}

fn scope() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let corpus = fixtures().join("corpus/spec");
    for sub in ["functional", "hazard"] {
        std::fs::create_dir_all(dir.path().join(sub)).unwrap();
        for entry in std::fs::read_dir(corpus.join(sub)).unwrap().flatten() {
            std::fs::copy(entry.path(), dir.path().join(sub).join(entry.file_name())).unwrap();
        }
    }
    let text = std::fs::read_to_string(fixtures().join("valid/two-hazards.md")).unwrap();
    std::fs::write(dir.path().join("AN-001.md"), text).unwrap();
    dir
}

#[trace("FR-136-AC-7")]
#[test]
fn refs_prints_one_json_array_and_an_empty_array_for_an_unnamed_hazard() {
    let scope = scope();
    let scope_arg = scope.path().to_str().unwrap();
    let named = quoin(&["anomaly", "refs", "HAZ-001", "--scope", scope_arg, "--json"]);
    assert!(named.status.success());
    let entries: serde_json::Value = serde_json::from_slice(&named.stdout).unwrap();
    assert_eq!(entries.as_array().unwrap().len(), 1);
    assert_eq!(entries[0]["anomaly"], "AN-001");
    let unnamed = quoin(&["anomaly", "refs", "HAZ-003", "--scope", scope_arg, "--json"]);
    assert!(unnamed.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&unnamed.stdout).unwrap(),
        serde_json::json!([])
    );
}

fn module_with_hazard() -> tempfile::TempDir {
    let module = tempfile::tempdir().unwrap();
    std::fs::write(
        module.path().join("manifest.yaml"),
        "name: spec-objects-safety\nversion: 0.0.0\nobject_types:\n- name: hazard\n",
    )
    .unwrap();
    module
}

#[trace("FR-136-AC-10")]
#[test]
fn check_fails_with_one_missing_module_diagnostic_until_the_hazard_type_is_installed() {
    let scope = scope();
    let scope_arg = scope.path().to_str().unwrap();
    let empty = tempfile::tempdir().unwrap();
    let missing = quoin(&[
        "anomaly",
        "check",
        scope_arg,
        "--module",
        empty.path().to_str().unwrap(),
    ]);
    assert_eq!(missing.status.code(), Some(1));
    let out = String::from_utf8(missing.stdout).unwrap();
    let diagnostics = out
        .lines()
        .filter(|l| l.contains("error anomaly.missing-module (spec-objects-safety)"))
        .count();
    assert_eq!(diagnostics, 1, "{out}");

    let module = module_with_hazard();
    let installed = quoin(&[
        "anomaly",
        "check",
        scope_arg,
        "--module",
        module.path().to_str().unwrap(),
    ]);
    assert_eq!(installed.status.code(), Some(0));
}

#[trace("FR-136-AC-6")]
#[test]
fn check_says_status_history_is_not_applicable_for_a_file_outside_git() {
    let scope = scope();
    let module = module_with_hazard();
    let out = quoin(&[
        "anomaly",
        "check",
        scope.path().to_str().unwrap(),
        "--module",
        module.path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("statusHistory not applicable"), "{stdout}");
}

#[test]
fn a_missing_scope_or_module_is_refused_by_name_with_a_status_unlike_findings() {
    let scope = scope();
    let gone = scope.path().join("does-not-exist");
    let gone_arg = gone.to_str().unwrap();
    for args in [
        vec!["anomaly", "check", gone_arg],
        vec!["anomaly", "refs", "FR-001", "--scope", gone_arg],
        vec![
            "anomaly",
            "check",
            scope.path().to_str().unwrap(),
            "--module",
            gone_arg,
        ],
    ] {
        let out = quoin(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains(gone_arg), "{stderr}");
        assert!(stderr.contains("does not exist"), "{stderr}");
        assert!(!stderr.contains("missing-module"), "{stderr}");
    }
}

#[test]
fn git_that_cannot_run_is_an_error_not_a_silent_not_applicable() {
    let scope = scope();
    let module = module_with_hazard();
    let out = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "anomaly",
            "check",
            scope.path().to_str().unwrap(),
            "--module",
            module.path().to_str().unwrap(),
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("cannot run `git"), "{stderr}");
    assert!(
        !String::from_utf8(out.stdout)
            .unwrap()
            .contains("not applicable")
    );
}
