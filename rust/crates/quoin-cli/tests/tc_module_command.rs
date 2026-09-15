// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shipped-command coverage for module installation and removal.

#![allow(
    clippy::expect_used,
    reason = "test assertions report command failures"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the CLI crate is nested below the repository root")
        .to_path_buf()
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let destination = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn invoke(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home)
        .env(
            "QUOIN_SEMANTIC_ROOT",
            repository_root().join("src/semantic"),
        )
        .output()
        .expect("the native quoin binary runs")
}

fn output_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("the native command emits UTF-8")
}

/// Install, list, and remove must operate on one real temporary IX home through
/// the shipped command—not the retired protocol executable.
///
/// Trace: FR-070, FR-101, FR-102
/// Provenance: quoin#449, quoin#521
#[test]
fn tc_521_module_install_list_and_remove_use_the_shipped_binary() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    let source = scratch.path().join("module-source");
    copy_tree(
        &repository_root().join("tests/fixtures/semantic-module/module-ok"),
        &source,
    )
    .expect("module fixture is copied");

    let source_argument = format!("path:{}", source.display());
    let installed = invoke(&home, &["module", "install", &source_argument]);
    assert!(installed.status.success(), "{installed:?}");
    assert!(output_text(&installed).contains("spec-objects-fixture"));

    let listed = invoke(&home, &["module", "list"]);
    assert!(listed.status.success(), "{listed:?}");
    assert!(output_text(&listed).contains("spec-objects-fixture"));

    let removed = invoke(&home, &["module", "remove", "spec-objects-fixture"]);
    assert!(removed.status.success(), "{removed:?}");
    assert_eq!(output_text(&removed), "removed spec-objects-fixture\n");

    let empty = invoke(&home, &["module", "list"]);
    assert!(empty.status.success(), "{empty:?}");
    assert!(
        output_text(&empty).contains("\"plugins\": []"),
        "the registry removal must be observed through a subsequent list"
    );
}
