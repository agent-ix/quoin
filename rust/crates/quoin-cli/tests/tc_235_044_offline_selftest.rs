// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The installed self-test runner works without a source checkout or network.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test failures should report the violated assertion"
)]

use std::fs;
use std::process::Command;

use ix_trace_rs::trace;
use serde_json::Value;

#[trace("FR-235-AC-13", "NFR-044-M-1")]
#[test]
#[cfg(target_os = "linux")]
fn installed_selftest_runs_with_empty_home_and_no_network() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("empty-home");
    fs::create_dir(&home).unwrap();
    let record = temp.path().join("selftest-result.json");
    let trace = temp.path().join("network.trace");

    // The scratch cwd contains neither sibling checkout. The embedded case
    // runner executes the installed binary with this empty HOME. strace
    // injects ENETDOWN if any process attempts a network syscall and records
    // every attempted syscall for the zero-request assertion.
    let output = Command::new("strace")
        .args([
            "-f",
            "-qq",
            "-e",
            "trace=%network",
            "-e",
            "signal=none",
            "-e",
            "inject=%network:error=ENETDOWN",
            "-o",
        ])
        .arg(&trace)
        .arg(env!("CARGO_BIN_EXE_quoin"))
        .args(["selftest", "--run", "--case", "selftest-list", "--out"])
        .arg(&record)
        .current_dir(temp.path())
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("strace and installed Quoin start");

    assert!(
        output.status.success(),
        "offline selftest failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let attempted_network = fs::read_to_string(&trace).unwrap();
    assert!(
        attempted_network.trim().is_empty(),
        "selftest attempted network syscalls: {attempted_network}"
    );
    let retained: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    let result = retained["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["id"] == "selftest-list")
        .unwrap();
    assert_eq!(result["status"], "pass");
}
