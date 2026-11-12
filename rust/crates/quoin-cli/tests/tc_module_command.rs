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

use ix_trace_rs::trace;
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

/// Copy the `quire-fixtures` `module-ok` fixture to `to`.
fn copy_module_ok(to: &Path) -> std::io::Result<()> {
    copy_tree(&quire_fixtures::module_ok_dir(), to)
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
/// Provenance: quoin#449, quoin#521
#[trace("FR-070", "FR-102")]
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
/// Provenance: quoin#527, quoin#531, quoin#533
#[trace("FR-070")]
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
#[trace("FR-124-AC-1", "FR-124-AC-2")]
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
    assert_eq!(compiled["semantic"]["contract_version"], "1.0.0");
    assert_eq!(
        compiled["semantic"]["package"],
        "agent-ix/spec-objects-consumer"
    );
    assert_eq!(compiled["semantic"]["targets"][1], "rust");
    assert_eq!(
        compiled["semantic"]["imports"]["agent-ix/spec-objects-fixture"],
        "0.1.0"
    );
    assert!(errors(compiled).is_empty());
    assert!(errors(dynamic).is_empty());
}

/// A module with no `semantic` block is still listed, as such, and is not an
/// error: the block is optional.
#[trace("FR-124-AC-3")]
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
#[trace("FR-124-AC-4")]
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

const FIXTURE_BASE: &str = "https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/";

/// A consumer of the fixture module whose entity schema `$ref`s `type_name` of
/// it, importing `entity` through `import_types`.
fn typed_consumer(scratch: &Path, refs: &[&str]) -> PathBuf {
    let source = module_source(scratch, "typed-consumer", |text| {
        consumer(&text).replace(
            "    agent-ix/spec-objects-fixture: 0.1.0",
            "    agent-ix/spec-objects-fixture: 0.1.0\n  import_types:\n    agent-ix/spec-objects-fixture: [entity]",
        )
    });
    let file = source.join("schemas/Entity.json");
    let mut schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    for (i, type_name) in refs.iter().enumerate() {
        schema["properties"][format!("ref{i}")] =
            serde_json::json!({ "$ref": format!("{FIXTURE_BASE}{type_name}.json") });
    }
    std::fs::write(&file, schema.to_string()).unwrap();
    source
}

/// An installed importer that lists or `$ref`s a type its installed provider
/// does not export is reported with `semantic.unknown-import-type` for each,
/// and the command exits non-zero.
#[trace("FR-124-AC-9")]
#[test]
fn tc_212_001_inspect_reports_unknown_import_types_for_list_and_ref() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    install(&home, &typed_consumer(scratch.path(), &["entity"]));
    let (output, _) = inspect(&home);
    assert!(output.status.success(), "{output:?}");

    // The importer moves on to list and `$ref` types the provider does not export.
    let installed = home.join("filament/modules/spec-objects-consumer");
    let manifest = installed.join("manifest.yaml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, text.replace("[entity]", "[entity, Nope]")).unwrap();
    let schema = installed.join("schemas/Entity.json");
    let text = std::fs::read_to_string(&schema).unwrap();
    std::fs::write(&schema, text.replace("entity.json", "Gone.json")).unwrap();

    let (output, report) = inspect(&home);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let found: Vec<(&str, &str)> = errors(entry(&report, "spec-objects-consumer"))
        .into_iter()
        .filter(|d| d["code"] == "semantic.unknown-import-type")
        .map(|d| (d["path"].as_str().unwrap(), d["message"].as_str().unwrap()))
        .collect();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().any(|(path, message)| {
        *path == "semantic.import_types.agent-ix/spec-objects-fixture" && message.contains("Nope")
    }));
    assert!(found.iter().any(|(path, message)| {
        *path == "object_types[entity].data_schema.schema" && message.contains("Gone")
    }));
}

/// An import no `$ref` references is listed as a warning, and a run whose only
/// finding is that warning exits zero.
#[trace("FR-124-AC-9")]
#[test]
fn tc_212_002_inspect_lists_an_unused_import_as_a_warning_and_exits_zero() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    install(&home, &typed_consumer(scratch.path(), &[]));

    let (output, report) = inspect(&home);
    assert!(output.status.success(), "{output:?}");
    let unused: Vec<&serde_json::Value> = entry(&report, "spec-objects-consumer")["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "semantic.unused-import")
        .collect();
    assert_eq!(unused.len(), 1, "{report}");
    assert_eq!(unused[0]["severity"], "warning");
    assert_eq!(
        unused[0]["path"],
        "semantic.imports.agent-ix/spec-objects-fixture"
    );
}

/// A block whose contract major this build does not know is reported with its
/// diagnostic and no semantic view, rather than as an empty model.
#[trace("FR-124-AC-5")]
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

/// An installed module whose manifest cannot be read is an entry with an error
/// diagnostic, not a failure of the whole report: the other modules are still
/// reported and the command exits 1.
#[trace("FR-124-AC-6")]
#[test]
fn tc_286_005_an_unreadable_installed_module_is_an_entry_not_a_failed_report() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    install(
        &home,
        &module_source(scratch.path(), "compiled", |t| consumer(&t)),
    );
    std::fs::remove_dir_all(home.join("filament/modules/spec-objects-consumer"))
        .expect("an installed module directory is removed");

    let (output, report) = inspect(&home);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(report["modules"].as_array().unwrap().len(), 2);
    assert!(errors(entry(&report, "spec-objects-fixture")).is_empty());
    let broken = entry(&report, "spec-objects-consumer");
    assert_eq!(broken["mode"], "unreadable");
    assert_eq!(errors(broken).len(), 1);
    assert_eq!(errors(broken)[0]["code"], "inspect.manifest-unreadable");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("QM023_INSTALLED_MODULE_INVALID"),
        "{stderr}"
    );
    assert!(!stderr.contains("QM020"), "{stderr}");
}

/// Two installed modules that import each other are reported as one cycle error
/// naming the path.
#[trace("FR-124-AC-7")]
#[test]
fn tc_286_006_an_import_cycle_is_reported() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    install(
        &home,
        &module_source(scratch.path(), "compiled", |t| consumer(&t)),
    );
    // The imported module is edited after install to import its importer.
    let installed = home.join("filament/modules/spec-objects-fixture/manifest.yaml");
    let text = std::fs::read_to_string(&installed).expect("installed manifest is readable");
    std::fs::write(
        &installed,
        text.replace(
            "  exports: [entity]",
            "  imports:\n    agent-ix/spec-objects-consumer: 0.1.0\n  exports: [entity]",
        ),
    )
    .expect("installed manifest is rewritten");

    let (output, report) = inspect(&home);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let all: Vec<String> = report["modules"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(errors)
        .map(|d| d["message"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        all.iter().any(|m| m.contains("import cycle:")
            && m.contains("agent-ix/spec-objects-fixture")
            && m.contains("agent-ix/spec-objects-consumer")),
        "{all:?}"
    );
}

/// `inspect` reads and reports; the registry and every installed module are
/// byte-identical afterwards, even when it finds errors.
#[trace("FR-124-AC-8")]
#[test]
fn tc_286_007_inspect_modifies_nothing() {
    let scratch = tempfile::tempdir().expect("scratch root is created");
    let home = scratch.path().join("ix-home");
    install(&home, &module_source(scratch.path(), "dynamic", |t| t));
    install(
        &home,
        &module_source(scratch.path(), "compiled", |t| consumer(&t)),
    );
    install(
        &home,
        &module_source(scratch.path(), "dynamic-0-2", |t| {
            t.replace("version: 0.1.0", "version: 0.2.0")
        }),
    );
    let snapshot = |root: &Path| -> Vec<(PathBuf, Vec<u8>)> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("directory is readable") {
                let path = entry.expect("entry is readable").path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let bytes = std::fs::read(&path).expect("file is readable");
                    out.push((path, bytes));
                }
            }
        }
        out.sort();
        out
    };
    let before = snapshot(&home.join("filament"));
    let (output, _) = inspect(&home);
    assert_eq!(
        output.status.code(),
        Some(1),
        "inspect found the stale lock"
    );
    assert_eq!(before, snapshot(&home.join("filament")));
}
