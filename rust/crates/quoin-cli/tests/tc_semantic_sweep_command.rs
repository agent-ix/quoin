// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shipped-command coverage for the FR-074 semantic sweep report.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic is the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Copy the five authoritative mapping fixtures into a scratch corpus.
fn fixture_corpus(root: &Path) {
    let directory = root.join("spec/functional");
    fs::create_dir_all(&directory).expect("the corpus directory is created");
    for name in [
        "config-version.table.md",
        "legacy-bullets.md",
        "legacy-mixed.md",
        "config-version.fence.md",
        "legacy-free-table.md",
    ] {
        fs::copy(
            quire_fixtures::mapping_dir().join(name),
            directory.join(name),
        )
        .expect("the authoritative mapping fixture is copied");
    }
}

/// The shipped command reports schema-valid counts for every legacy form.
///
/// Provenance: quoin#452
#[trace("FR-074-AC-5")]
#[test]
fn tc_452_656_semantic_sweep_command_reports_fixture_counts_and_valid_schema() {
    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let corpus = scratch.path().join("corpus");
    fixture_corpus(&corpus);
    let report_path = corpus.join("sweep-report.json");
    let home = scratch.path().join("ix-home");
    let semantic_root = scratch.path().join("semantic-contract");
    quoin_semantic::materialize_embedded_contract(&semantic_root)
        .expect("the embedded semantic contract materializes");

    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "semantic",
            "sweep",
            "--package",
            "agent-ix/spec-objects-fixture",
            "--module-version",
            "0.1.0",
            "--out",
        ])
        .arg(&report_path)
        .arg(&corpus)
        .env("IX_HOME", &home)
        .env("QUOIN_SEMANTIC_ROOT", &semantic_root)
        .output()
        .expect("the shipped quoin binary runs");
    assert!(
        output.status.success(),
        "semantic sweep exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("5 artifacts"),
        "the CLI summary names the five swept fixtures"
    );

    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&report_path).expect("the CLI writes its requested report"),
    )
    .expect("the report is JSON");
    assert_eq!(report["counts"]["artifacts"], 5);
    assert_eq!(report["counts"]["legacy"]["bullet-list"], 2);
    assert_eq!(report["counts"]["legacy"]["free-column-table"], 1);

    let validators = quoin_semantic::manifest::SemanticValidators::load(&semantic_root)
        .expect("the embedded report schema loads");
    assert!(
        validators.sweep_report().is_valid(&report),
        "the shipped command report validates against sweep-report.schema.json"
    );
}
