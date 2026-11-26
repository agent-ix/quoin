// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The installed binary's self-test command and retained result contract.

use ix_trace_rs::trace;
use serde_json::Value;
use std::process::Command;

#[trace("FR-235-AC-1")]
#[test]
fn installed_binary_runs_selected_embedded_case_and_retains_its_result() {
    let temp_result = tempfile::tempdir();
    assert!(
        temp_result.is_ok(),
        "temporary working directory is created"
    );
    let Some(temp) = temp_result.ok() else {
        return;
    };
    let record_path = temp.path().join("selftest-result.json");
    let output_result = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .current_dir(temp.path())
        .args(["selftest", "--run", "--case", "selftest-list", "--out"])
        .arg(&record_path)
        .output();
    assert!(output_result.is_ok(), "run installed Quoin binary");
    let Some(output) = output_result.ok() else {
        return;
    };
    assert!(
        output.status.success(),
        "selftest command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let record_bytes = std::fs::read(&record_path).unwrap_or_default();
    let record: Value = serde_json::from_slice(&record_bytes).unwrap_or(Value::Null);
    let result = record
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|result| result.get("id").and_then(Value::as_str) == Some("selftest-list"));
    assert_eq!(
        result
            .and_then(|result| result.get("status"))
            .and_then(Value::as_str),
        Some("pass")
    );
}
