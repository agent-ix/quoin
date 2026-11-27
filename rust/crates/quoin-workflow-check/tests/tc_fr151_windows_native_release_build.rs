// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The native Windows release job builds the Quoin CLI with its filament-analyze dependency (FR-151).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is the intended failure signal"
)]

use std::fs;
use std::path::PathBuf;

use ix_trace_rs::trace;
use serde_yaml_ng::Value;

fn native_release_workflow() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.github/workflows/native-release-build.yml");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read native release workflow {}: {error}", path.display()));
    serde_yaml_ng::from_str(&source)
        .unwrap_or_else(|error| panic!("parse native release workflow {}: {error}", path.display()))
}

#[trace("FR-151-CON-4")]
#[test]
fn windows_release_workflow_selects_a_native_quoin_build() {
    let workflow = native_release_workflow();
    let include = workflow["jobs"]["package"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("native release target matrix is a list");
    let mut configured = include
        .iter()
        .map(|entry| {
            (
                entry["target"]
                    .as_str()
                    .expect("native release matrix target is a string")
                    .to_owned(),
                entry["runner"]
                    .as_str()
                    .expect("native release matrix runner is a string")
                    .to_owned(),
                entry["binary"]
                    .as_str()
                    .expect("native release matrix binary is a string")
                    .to_owned(),
                entry["extension"]
                    .as_str()
                    .expect("native release matrix extension is a string")
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();
    configured.sort();
    assert_eq!(
        configured,
        [
            (
                "aarch64-apple-darwin".to_owned(),
                "macos-15".to_owned(),
                "quoin".to_owned(),
                "tar.gz".to_owned(),
            ),
            (
                "aarch64-unknown-linux-gnu".to_owned(),
                "ubuntu-22.04-arm".to_owned(),
                "quoin".to_owned(),
                "tar.gz".to_owned(),
            ),
            (
                "x86_64-pc-windows-msvc".to_owned(),
                "windows-2025".to_owned(),
                "quoin.exe".to_owned(),
                "zip".to_owned(),
            ),
            (
                "x86_64-unknown-linux-gnu".to_owned(),
                "ubuntu-22.04".to_owned(),
                "quoin".to_owned(),
                "tar.gz".to_owned(),
            ),
        ],
        "native release matrix targets and packaging must remain explicit",
    );
    let windows = include
        .iter()
        .find(|entry| entry["target"].as_str() == Some("x86_64-pc-windows-msvc"))
        .expect("native release matrix includes x86_64-pc-windows-msvc");

    assert_eq!(windows["runner"].as_str(), Some("windows-2025"));
    assert_eq!(windows["binary"].as_str(), Some("quoin.exe"));
    assert_eq!(windows["extension"].as_str(), Some("zip"));

    let steps = workflow["jobs"]["package"]["steps"]
        .as_sequence()
        .expect("native release job has steps");
    let build = steps
        .iter()
        .find(|step| step["name"].as_str() == Some("Build the native quoin artifact"))
        .expect("native release job builds the Quoin artifact");
    assert_eq!(
        build["run"].as_str(),
        Some("cd rust && cargo build --locked --release -p quoin-cli --bin quoin")
    );
}
