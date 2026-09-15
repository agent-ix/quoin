// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native command-path coverage for the retained catalog fixture.

#![allow(
    clippy::expect_used,
    reason = "test assertions deliberately panic to report a failed command contract"
)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn retained_home() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-difftest/fixtures/catalog/ix-home")
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the CLI crate is nested below the repository root")
}

fn invoke(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home)
        .env("QUOIN_SEMANTIC_ROOT", repository_root().join("dist"))
        .output()
        .expect("the native quoin binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

/// The frozen catalog fixture is copied before invocation because catalog access
/// reconciles defaults. The native command must retain the old shell's JSON
/// and human listing shapes without mutating migration evidence in place. This
/// fixture deliberately has no resolved module roots: its contract is the
/// empty JSON module list and an entirely empty human response.
///
/// Trace: FR-099, FR-101, FR-102, TC-1650
#[test]
fn tc_1650_catalog_list_replays_the_retained_fixture_in_json_and_human_forms() {
    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let home = scratch.path().join("ix-home");
    copy_tree(&retained_home(), &home).expect("retained fixture is copied");

    let json = invoke(&home, &["catalog", "list", "--json"]);
    assert!(json.status.success(), "stderr: {}", text(&json.stderr));
    assert!(json.stderr.is_empty());
    let catalog: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("catalog JSON is emitted");
    let modules = catalog
        .get("modules")
        .and_then(serde_json::Value::as_array)
        .expect("catalog JSON names its modules");
    assert!(
        modules.is_empty(),
        "fixture deliberately resolves no modules"
    );

    let human = invoke(&home, &["catalog", "list"]);
    assert!(human.status.success(), "stderr: {}", text(&human.stderr));
    assert!(human.stderr.is_empty());
    assert!(human.stdout.is_empty());
}
