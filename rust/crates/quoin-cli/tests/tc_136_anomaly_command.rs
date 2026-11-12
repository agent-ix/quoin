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

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn fixtures() -> PathBuf {
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    let dir = root.join("tests/fixtures/anomaly");
    assert!(
        dir.is_dir(),
        "the spec-artifacts-process fixtures are not at {}; set SPEC_ARTIFACTS_PROCESS_DIR",
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
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.contains("anomaly.missing-module"), "{out}");

    let module = tempfile::tempdir().unwrap();
    std::fs::write(
        module.path().join("manifest.yaml"),
        "object_types:\n- name: hazard\n",
    )
    .unwrap();
    let installed = quoin(&[
        "anomaly",
        "check",
        scope_arg,
        "--module",
        module.path().to_str().unwrap(),
    ]);
    assert_eq!(installed.status.code(), Some(0));
}
