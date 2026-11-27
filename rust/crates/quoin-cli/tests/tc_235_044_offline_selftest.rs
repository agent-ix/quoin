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

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
use ix_trace_rs::trace;
#[cfg(target_os = "linux")]
use serde_json::Value;

#[cfg(target_os = "linux")]
fn assert_offline_selftest(binary: &Path) {
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
        .arg(binary)
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

#[trace("NFR-044-M-1")]
#[test]
#[cfg(target_os = "linux")]
fn selftest_runs_with_empty_home_and_no_network() {
    assert_offline_selftest(Path::new(env!("CARGO_BIN_EXE_quoin")));
}

#[trace("FR-235-AC-13")]
#[test]
#[cfg(target_os = "linux")]
#[cfg(feature = "e2e-published-binary")]
fn release_binary_selftest_runs_without_sibling_checkouts() {
    let binary = std::env::var_os("QUOIN_BINARY")
        .map(std::path::PathBuf::from)
        .expect("set QUOIN_BINARY to the matching published Quoin release asset");
    assert!(
        binary.is_file(),
        "QUOIN_BINARY is not a file: {}",
        binary.display()
    );
    let binary = fs::canonicalize(binary).expect("canonicalize QUOIN_BINARY");
    assert_offline_selftest(&binary);
}
