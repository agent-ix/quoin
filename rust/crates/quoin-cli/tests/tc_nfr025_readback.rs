// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! NFR-025 evidence-store inventory and byte read-back command.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test fixtures may panic"
)]

use ix_trace_rs::trace;
use std::process::{Command, Output};

fn read_back(repo: &std::path::Path, extra: &[&std::ffi::OsStr]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command.args(["evidence", "read-back", "--repo"]).arg(repo);
    command.args(extra);
    command.output().unwrap()
}

fn fixture() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(repo.path().join("spec/evidence/runs")).unwrap();
    std::fs::write(
        repo.path().join("spec/evidence-store-inventory.json"),
        br#"{"schema_version":1,"roots":["spec/evidence"]}"#,
    )
    .unwrap();
    std::fs::write(
        repo.path().join("spec/evidence/runs/record.json"),
        br#"{"result":"pass"}"#,
    )
    .unwrap();
    repo
}

#[trace("NFR-025-AC-1")]
#[trace("NFR-025-AC-2")]
#[test]
fn full_population_snapshot_is_clean_then_a_single_byte_change_fails() {
    let repo = fixture();
    let snapshot = repo.path().join("baseline.json");
    let captured = read_back(repo.path(), &[]);
    assert_eq!(
        captured.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&captured.stderr)
    );
    let captured_report: serde_json::Value = serde_json::from_slice(&captured.stdout).unwrap();
    assert_eq!(captured_report["status"], "captured");
    assert_eq!(captured_report["files"].as_array().unwrap().len(), 1);
    std::fs::write(
        &snapshot,
        serde_json::to_vec(&captured_report["snapshot"]).unwrap(),
    )
    .unwrap();

    let clean = read_back(repo.path(), &["--baseline".as_ref(), snapshot.as_os_str()]);
    assert_eq!(
        clean.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );
    let clean_report: serde_json::Value = serde_json::from_slice(&clean.stdout).unwrap();
    assert_eq!(clean_report["status"], "clean");
    assert_eq!(clean_report["changed_paths"], serde_json::json!([]));

    std::fs::write(
        repo.path().join("spec/evidence/runs/record.json"),
        br#"{"result":"fail"}"#,
    )
    .unwrap();
    let changed = read_back(repo.path(), &["--baseline".as_ref(), snapshot.as_os_str()]);
    assert_eq!(changed.status.code(), Some(1));
    let changed_report: serde_json::Value = serde_json::from_slice(&changed.stdout).unwrap();
    assert_eq!(changed_report["status"], "changed");
    assert_eq!(
        changed_report["changed_paths"],
        serde_json::json!(["spec/evidence/runs/record.json"])
    );
}

#[trace("NFR-025-AC-5")]
#[test]
fn empty_population_and_missing_inventory_exit_inconclusive() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(repo.path().join("spec/evidence")).unwrap();
    std::fs::write(
        repo.path().join("spec/evidence-store-inventory.json"),
        br#"{"schema_version":1,"roots":["spec/evidence"]}"#,
    )
    .unwrap();
    let empty = read_back(repo.path(), &[]);
    assert_eq!(empty.status.code(), Some(1));
    let empty_report: serde_json::Value = serde_json::from_slice(&empty.stdout).unwrap();
    assert_eq!(empty_report["status"], "inconclusive");

    std::fs::remove_file(repo.path().join("spec/evidence-store-inventory.json")).unwrap();
    let missing = read_back(repo.path(), &[]);
    assert_eq!(missing.status.code(), Some(1));
    let missing_report: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(missing_report["status"], "inconclusive");
}
