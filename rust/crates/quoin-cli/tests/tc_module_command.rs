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

/// A real, on-disk semantic contract root for `QUOIN_SEMANTIC_ROOT`, so
/// `invoke` exercises the override path deliberately rather than always
/// falling through to the binary's own per-`IX_HOME` auto-materialization
/// that [`tc_527_001_native_module_install_bootstraps_without_a_semantic_root_env`]
/// covers separately. Built once per test process from the same embedded
/// bytes `quoin-semantic`'s `build.rs` compiles in (PLAT-887 de-vendoring) --
/// not read from a source-tree `src/semantic`, which no longer exists.
fn semantic_root() -> &'static Path {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let root = std::env::temp_dir().join(format!(
            "quoin-cli-tc-module-semantic-{}",
            std::process::id()
        ));
        quoin_semantic::materialize_embedded_contract(&root)
            .expect("the embedded semantic contract materializes");
        root
    })
}

fn invoke(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home)
        .env("QUOIN_SEMANTIC_ROOT", semantic_root())
        .output()
        .expect("the native quoin binary runs")
}

fn invoke_without_semantic_root(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home)
        .env_remove("QUOIN_SEMANTIC_ROOT")
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

/// A release executable owns its semantic contract; it must not rely on the
/// source checkout that happened to build it.
///
/// Trace: FR-070, FR-101
/// Provenance: quoin#527, quoin#531, quoin#533
#[test]
fn tc_527_001_native_module_install_bootstraps_without_a_semantic_root_env() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    let source = scratch.path().join("module-source");
    copy_tree(
        &repository_root().join("tests/fixtures/semantic-module/module-ok"),
        &source,
    )
    .expect("module fixture is copied");

    let source_argument = format!("path:{}", source.display());
    let output = invoke_without_semantic_root(&home, &["module", "install", &source_argument]);
    assert!(output.status.success(), "{output:?}");
    assert!(
        home.join("cache/quoin-semantic/v1/schemas/module-manifest.schema.json")
            .is_file(),
        "the shipped contract is materialized under the selected IX home"
    );
}
