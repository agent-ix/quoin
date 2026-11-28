// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The installed binary's self-test command and retained result contract.

use ix_trace_rs::trace;
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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

#[cfg(unix)]
#[trace("FR-235-AC-13")]
#[test]
fn selftest_run_does_not_access_sibling_checkouts() {
    let temp_result = tempfile::tempdir();
    assert!(
        temp_result.is_ok(),
        "temporary working directory is created"
    );
    let Some(temp) = temp_result.ok() else {
        return;
    };
    let bin = temp.path().join("bin");
    assert!(
        std::fs::create_dir(&bin).is_ok(),
        "fake command directory is created"
    );
    let marker = temp.path().join("git-was-invoked");
    let fake_git = bin.join("git");
    let script = format!(
        "#!/bin/sh\nprintf called > '{}'\nexit 1\n",
        marker.display()
    );
    assert!(
        std::fs::write(&fake_git, script).is_ok(),
        "fake git is written"
    );
    let mut permissions = match std::fs::metadata(&fake_git) {
        Ok(metadata) => metadata.permissions(),
        Err(error) => panic!("read fake git permissions: {error}"),
    };
    permissions.set_mode(0o755);
    assert!(std::fs::set_permissions(&fake_git, permissions).is_ok());

    let original_path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(bin.clone()).chain(std::env::split_paths(&original_path)),
    );
    assert!(path.is_ok(), "fake command path is valid");
    let record_path = temp.path().join("selftest-result.json");
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .current_dir(temp.path())
        .env("PATH", path.unwrap_or_default())
        .env("SPEC_OBJECTS_SAFETY_DIR", temp.path().join("absent-safety"))
        .env(
            "SPEC_OBJECTS_SECURITY_DIR",
            temp.path().join("absent-security"),
        )
        .args(["selftest", "--run", "--case", "selftest-list", "--out"])
        .arg(&record_path)
        .output();
    assert!(output.is_ok(), "installed self-test command runs");
    let Some(output) = output.ok() else {
        return;
    };
    assert!(
        output.status.success(),
        "selftest failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !marker.exists(),
        "selftest does not invoke git for sibling checkouts"
    );
    let bytes = std::fs::read(&record_path).unwrap_or_default();
    let record: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
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
