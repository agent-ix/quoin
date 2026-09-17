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
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/retained-catalog/ix-home")
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the CLI crate is nested below the repository root")
}

fn invoke(home: &Path, arguments: &[&str]) -> Output {
    invoke_with_modules(home, None, arguments)
}

fn invoke_with_modules(home: &Path, modules: Option<&Path>, arguments: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command.args(arguments).env("IX_HOME", home).env(
        "QUOIN_SEMANTIC_ROOT",
        repository_root().join("src/semantic"),
    );
    if let Some(modules) = modules {
        command.env("QUOIN_MODULE_PATHS", modules);
    }
    command.output().expect("the native quoin binary runs")
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

/// The shipped command, rather than the retired protocol executable, discovers
/// one nested module root and preserves its declared artifact projection.
///
/// Trace: FR-099, FR-101, FR-102
#[test]
fn tc_521_catalog_list_discovers_a_nested_module_through_the_shipped_binary() {
    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let home = scratch.path().join("ix-home");
    let candidates = scratch.path().join("candidates");
    fs::create_dir_all(candidates.join("00-not-a-module")).expect("empty sibling");
    let module = candidates.join("01-catalog-fixture");
    fs::create_dir_all(module.join("skeletons")).expect("module skeletons");
    fs::write(
        module.join("manifest.yaml"),
        "name: catalog-fixture\nversion: 1.0.0\nartifact_types:\n  - name: FR\n    frontmatter_schema_ref: schemas/fr.json\nobject_types:\n  - name: entity\n    data_schema: {type: object}\n",
    )
    .expect("manifest writes");
    fs::write(module.join("skeletons/FR.md"), "# FR\n").expect("skeleton writes");

    let output = invoke_with_modules(&home, Some(&candidates), &["catalog", "list", "--json"]);
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    let catalog: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("catalog JSON is emitted");
    assert_eq!(
        catalog.pointer("/modules/0/name"),
        Some(&serde_json::json!("catalog-fixture"))
    );
    assert_eq!(
        catalog.pointer("/entries/0/name"),
        Some(&serde_json::json!("FR"))
    );
    assert_eq!(
        catalog.pointer("/entries/0/skeletonPath"),
        Some(&serde_json::json!(
            module.join("skeletons/FR.md").to_string_lossy()
        ))
    );
}

/// Candidate discovery preserves the lexical symlink root the operator passed.
///
/// Trace: FR-099, FR-101, FR-102
#[cfg(unix)]
#[test]
fn tc_521_catalog_list_preserves_a_symlink_candidate_path() {
    use std::os::unix::fs::symlink;

    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let home = scratch.path().join("ix-home");
    let target = scratch.path().join("actual-module");
    fs::create_dir_all(&target).expect("target module directory");
    fs::write(target.join("manifest.yaml"), "name: linked\n").expect("manifest writes");
    let visible = scratch.path().join("visible-module");
    symlink(&target, &visible).expect("candidate symlink");

    let output = invoke_with_modules(&home, Some(&visible), &["catalog", "list", "--json"]);
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    let catalog: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("catalog JSON is emitted");
    assert_eq!(
        catalog.pointer("/modules/0/root"),
        Some(&serde_json::json!(visible.to_string_lossy()))
    );
}

/// The shipped `catalog methods` command retains first-wins merge semantics.
///
/// Trace: FR-099, FR-101, FR-102
#[test]
fn tc_521_catalog_methods_preserves_first_wins_data() {
    let scratch = tempfile::tempdir().expect("scratch directory is created");
    let home = scratch.path().join("ix-home");
    let candidates = scratch.path().join("candidates");
    let first = candidates.join("first");
    let second = candidates.join("second");
    fs::create_dir_all(&first).expect("first module directory");
    fs::create_dir_all(&second).expect("second module directory");
    fs::write(first.join("manifest.yaml"), "name: first\nverification_catalog:\n  analysis:\n    name: First analysis\n    class: Analysis\n    definition: inspect\n").expect("first manifest writes");
    fs::write(second.join("manifest.yaml"), "name: second\nverification_catalog:\n  analysis:\n    name: Second analysis\n    class: Test\n    definition: execute\n").expect("second manifest writes");

    let output = invoke_with_modules(&home, Some(&candidates), &["catalog", "methods", "--json"]);
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    let methods: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("method JSON is emitted");
    let methods = methods
        .get("methods")
        .and_then(serde_json::Value::as_array)
        .expect("method catalog contains methods");
    let analysis = methods
        .iter()
        .find(|method| method.get("id") == Some(&serde_json::json!("analysis")))
        .expect("the explicit analysis method is present");
    assert_eq!(
        analysis.get("name"),
        Some(&serde_json::json!("First analysis"))
    );
}
