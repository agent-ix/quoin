// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Verifier refusals must preserve child output when the verifier exits before
//! consuming its request (PLAT-1822).

#![cfg(unix)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_core::runtime::engineering_assurance_call;
use serde_json::json;

fn executable(directory: &Path, name: &str, body: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write verifier stub");
    let mut permissions = fs::metadata(&path)
        .expect("verifier metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("verifier permissions");
    path
}

/// Provenance: PLAT-1822
#[trace("FR-188-AC-20")]
#[test]
fn tc_1822_early_verifier_refusals_preserve_child_output_across_repeated_calls() {
    let directory = tempfile::tempdir().expect("temporary verifier directory");
    let refusal = executable(
        directory.path(),
        "ea-allowed-signers-refusal",
        "printf '%s\\n' '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_allowed_signers_invalid\",\"message\":\"bad allowed signers\"}'; printf '%s\\n' diagnostic >&2; exit 2",
    );
    let request = json!({"padding": "x".repeat(1024 * 1024)});

    for iteration in 0..50 {
        let output = engineering_assurance_call(&refusal, "probe", &request)
            .expect("verifier refusal must return its child output");
        assert_eq!(output.status.code(), Some(2), "iteration {iteration}");
        assert_eq!(
            output.stdout,
            b"{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"signature_verify_allowed_signers_invalid\",\"message\":\"bad allowed signers\"}\n",
            "iteration {iteration}"
        );
        assert_eq!(output.stderr, b"diagnostic\n", "iteration {iteration}");
    }
}
