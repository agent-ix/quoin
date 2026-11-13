// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin risk advise` command surface (FR-145).

use std::process::{Command, Output};

use ix_trace_rs::trace;

fn quoin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .output()
        .unwrap()
}

#[trace("FR-145-AC-1", "FR-145-AC-4")]
#[test]
fn json_over_an_empty_scope_exits_zero_with_the_four_keys() {
    let dir = tempfile::tempdir().unwrap();
    let out = quoin(&[
        "risk",
        "advise",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let report: serde_json::Value =
        serde_json::from_str(payload["rendered"].as_str().unwrap()).unwrap();
    let mut keys: Vec<&str> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "attribution",
            "decision_queue",
            "evidence_gaps",
            "policy_cells"
        ]
    );
}

#[trace("FR-145-AC-4")]
#[test]
fn markdown_is_the_default_and_a_missing_scope_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let out = quoin(&["risk", "advise", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("policy_cells"));
    let missing = dir.path().join("absent");
    let out = quoin(&["risk", "advise", missing.to_str().unwrap()]);
    assert!(!out.status.success());
}
