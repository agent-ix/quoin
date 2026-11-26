// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The installed binary's self-test command and retained result contract.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions inspect the fixed result schema; a missing field is a failed test"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::process::Command;

#[trace("FR-235-AC-1")]
#[test]
fn installed_binary_runs_selected_embedded_case_and_retains_its_result() {
    let temp = tempfile::tempdir().expect("temporary working directory");
    let record_path = temp.path().join("selftest-result.json");
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .current_dir(temp.path())
        .args(["selftest", "--run", "--case", "selftest-list", "--out"])
        .arg(&record_path)
        .output()
        .expect("run installed Quoin binary");
    assert!(
        output.status.success(),
        "selftest command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let record: Value = serde_json::from_slice(
        &std::fs::read(&record_path).expect("self-test run record is retained"),
    )
    .expect("run record is JSON");
    let result = record["results"]
        .as_array()
        .expect("run record contains case results")
        .iter()
        .find(|result| result["id"] == "selftest-list")
        .expect("selected case has a result");
    assert_eq!(result["status"], "pass");
}
