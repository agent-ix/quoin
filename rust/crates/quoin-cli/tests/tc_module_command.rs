// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shipped-command coverage for module installation and removal.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

/// `semantic.semantic_core` as the module-manifest schema pinned through
/// `@agent-ix/semantic-schema` requires it. The `quire-fixtures` `module-ok`
/// manifest omits the key, so `copy_module_ok` adds it; drop this injection
/// when the pinned `@agent-ix/semantic-schema` no longer requires
/// `semantic.semantic_core`. Quoin reads nothing from the value.
const SCHEMA_REQUIRED_SEMANTIC_CORE: &str = "0.0.0";

/// Copy the `quire-fixtures` `module-ok` fixture to `to` with the
/// `semantic_core` key the pinned schema requires.
fn copy_module_ok(to: &Path) -> std::io::Result<()> {
    copy_tree(&quire_fixtures::module_ok_dir(), to)?;
    let manifest = to.join("manifest.yaml");
    let text = std::fs::read_to_string(&manifest)?;
    std::fs::write(
        &manifest,
        text.replace(
            "semantic:\n",
            &format!("semantic:\n  semantic_core: {SCHEMA_REQUIRED_SEMANTIC_CORE}\n"),
        ),
    )
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
/// Trace: FR-070, FR-102
/// Provenance: quoin#449, quoin#521
#[test]
fn tc_521_module_install_list_and_remove_use_the_shipped_binary() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    let source = scratch.path().join("module-source");
    copy_module_ok(&source).expect("module fixture is copied");

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
/// Trace: FR-070
/// Provenance: quoin#527, quoin#531, quoin#533
#[test]
fn tc_527_001_native_module_install_bootstraps_without_a_semantic_root_env() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    let source = scratch.path().join("module-source");
    copy_module_ok(&source).expect("module fixture is copied");

    let source_argument = format!("path:{}", source.display());
    let output = invoke_without_semantic_root(&home, &["module", "install", &source_argument]);
    assert!(output.status.success(), "{output:?}");
    assert!(
        home.join("cache/quoin-semantic/v1/schemas/module-manifest.schema.json")
            .is_file(),
        "the shipped contract is materialized under the selected IX home"
    );
}

/// A module source under `scratch`, copied from the fixture with `edit`
/// applied to its manifest text.
fn module_source(scratch: &Path, dir: &str, edit: impl FnOnce(String) -> String) -> PathBuf {
    let source = scratch.join(dir);
    copy_module_ok(&source).expect("module fixture is copied");
    let manifest = source.join("manifest.yaml");
    let text = std::fs::read_to_string(&manifest).expect("manifest is readable");
    std::fs::write(&manifest, edit(text)).expect("manifest is written");
    source
}

fn install(home: &Path, source: &Path) {
    let argument = format!("path:{}", source.display());
    let output = invoke(home, &["module", "install", &argument]);
    assert!(output.status.success(), "{output:?}");
}

fn inspect(home: &Path) -> (Output, serde_json::Value) {
    let output = invoke(home, &["module", "inspect"]);
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("inspect writes a complete JSON report: {e}: {output:?}"));
    (output, report)
}

fn entry<'a>(report: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    report["modules"]
        .as_array()
        .expect("modules is a list")
        .iter()
        .find(|m| m["name"] == name)
        .unwrap_or_else(|| panic!("no module {name} in {report}"))
}

/// The error-severity diagnostics of one inspected module; the fixture's own
/// inline `data_schema` warning is not one.
fn errors(entry: &serde_json::Value) -> Vec<&serde_json::Value> {
    entry["diagnostics"]
        .as_array()
        .expect("diagnostics is a list")
        .iter()
        .filter(|d| d["severity"] == "error")
        .collect()
}

fn consumer(text: &str) -> String {
    text.replace("name: spec-objects-fixture", "name: spec-objects-consumer")
        .replace(
            "package: agent-ix/spec-objects-fixture",
            "package: agent-ix/spec-objects-consumer\n  imports:\n    agent-ix/spec-objects-fixture: 0.1.0",
        )
        .replace("targets: [json-schema, markdown]", "targets: [json-schema, rust]")
}

/// Dynamic-only and compiled modules sit in one installed set, each reported
/// with its package, contract version and mode, and a clean set exits zero.
///
/// Trace: FR-124-AC-1
/// Trace: FR-124-AC-2
#[test]
fn tc_286_001_inspect_reports_dynamic_and_compiled_modules_together() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    install(
        &home,
        &module_source(scratch.path(), "compiled", |t| consumer(&t)),
    );

    let (output, report) = inspect(&home);
    assert!(output.status.success(), "{output:?}");

    let dynamic = entry(&report, "spec-objects-fixture");
    assert_eq!(dynamic["mode"], "dynamic");
    assert_eq!(dynamic["version"], "0.1.0");
    assert_eq!(dynamic["semantic"]["contract_version"], "1.0.0");
    assert_eq!(
        dynamic["semantic"]["package"],
        "agent-ix/spec-objects-fixture"
    );

    let compiled = entry(&report, "spec-objects-consumer");
    assert_eq!(compiled["mode"], "compiled");
    assert_eq!(compiled["semantic"]["targets"][1], "rust");
    assert_eq!(
        compiled["semantic"]["imports"]["agent-ix/spec-objects-fixture"],
        "0.1.0"
    );
    assert!(errors(compiled).is_empty());
}

/// A module with no `semantic` block is still listed, as such, and is not an
/// error: the block is optional.
///
/// Trace: FR-124-AC-3
#[test]
fn tc_286_002_a_module_without_a_semantic_block_is_reported_and_clean() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    let source = module_source(scratch.path(), "plain", |text| {
        text.split("semantic:").next().unwrap().to_owned()
    });
    install(&home, &source);

    let (output, report) = inspect(&home);
    assert!(output.status.success(), "{output:?}");
    let plain = entry(&report, "spec-objects-fixture");
    assert_eq!(plain["mode"], "no-semantic-block");
    assert!(plain.get("semantic").is_none());
}

/// An import locked at a version no installed module provides any more is
/// reported against the importing module, naming the installed versions, and
/// the command exits non-zero while still writing the whole report.
///
/// Trace: FR-124-AC-4
#[test]
fn tc_286_003_an_import_lock_the_installed_set_no_longer_satisfies_is_reported() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    install(
        &home,
        &module_source(scratch.path(), "compiled", |t| consumer(&t)),
    );
    // The imported module moves on after the importer was installed.
    install(
        &home,
        &module_source(scratch.path(), "dynamic-0-2", |t| {
            t.replace("version: 0.1.0", "version: 0.2.0")
        }),
    );

    let (output, report) = inspect(&home);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(errors(entry(&report, "spec-objects-fixture")).is_empty());
    let stale = errors(entry(&report, "spec-objects-consumer"));
    assert_eq!(stale.len(), 1, "{stale:?}");
    let message = stale[0]["message"].as_str().unwrap();
    assert!(
        message.contains("agent-ix/spec-objects-fixture@0.1.0"),
        "{message}"
    );
    assert!(message.contains("installed: 0.2.0"), "{message}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("spec-objects-consumer"), "{stderr}");
}

/// A block whose contract major this build does not know is reported with its
/// diagnostic and no semantic view, rather than as an empty model.
///
/// Trace: FR-124-AC-5
#[test]
fn tc_286_004_an_unknown_contract_major_is_reported_not_degraded_to_empty() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    let installed = home.join("filament/modules/spec-objects-fixture/manifest.yaml");
    let text = std::fs::read_to_string(&installed).expect("installed manifest is readable");
    std::fs::write(
        &installed,
        text.replace("contract_version: 1.0.0", "contract_version: 2.0.0"),
    )
    .expect("installed manifest is rewritten");

    let (output, report) = inspect(&home);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let refused = entry(&report, "spec-objects-fixture");
    assert_eq!(refused["mode"], "no-semantic-block");
    assert!(refused.get("semantic").is_none());
    let diagnostics = refused["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d["severity"] == "error" && d["message"].as_str().unwrap().contains("2.0.0")),
        "{diagnostics:?}"
    );
}
