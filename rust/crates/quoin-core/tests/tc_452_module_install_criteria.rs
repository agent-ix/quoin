// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The install-path halves of FR-070, FR-074 and FR-075 (quoin#452).
//!
//! `tests/semantic-manifest.test.ts` and `tests/semantic-package-manifest.test.ts`
//! carried these against `installModule`. The criteria are about *a module
//! joining a population* — a package another module already claims, a rejected
//! re-install, the manifest and the pin an accepted install leaves behind — so
//! they are restated where that composition actually happens: the production
//! runtime, a real temporary `~/.ix`, the real vendored contract.
//!
//! A unit test against `ContractGate` proves the rule and not the wiring, and
//! the wiring is what these criteria are about. The reconcile half of the same
//! family lives in `tc_446_reconcile_revalidation.rs`, which already restates
//! FR-070-AC-1 and NFR-017-AC-1 against this same path.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::{Path, PathBuf};

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

impl Run {
    /// The payload of a run that must have succeeded.
    fn ok(&self) -> Value {
        assert_eq!(self.status, 0, "{}", self.stderr);
        serde_json::from_str(&self.stdout).unwrap()
    }

    /// The stderr diagnostics of a run that must have been refused.
    fn refused(&self) -> String {
        assert_eq!(
            self.status, 2,
            "expected a refusal; stdout={} stderr={}",
            self.stdout, self.stderr
        );
        assert_eq!(self.stdout, "", "a refusal carries no payload");
        self.stderr.clone()
    }
}

fn run(op: &str, request: &Value) -> Run {
    let settings = RuntimeSettings {
        ix_home: None,
        semantic_root: Some(semantic_root()),
    };
    let response = dispatch(op, request, &settings).unwrap_or_else(|error| Response {
        payload: Value::Null,
        diagnostics: vec![Diagnostic::from(&error)],
        outcome: error.outcome(),
    });
    let stdout = if response.outcome.carries_payload() {
        canonical_json(&response.payload).unwrap()
    } else {
        String::new()
    };
    let stderr = if response.diagnostics.is_empty() {
        String::new()
    } else {
        canonical_json(&response.diagnostics).unwrap()
    };
    Run {
        stdout,
        stderr,
        status: i32::from(response.outcome.code()),
    }
}

/// The repository root: `rust/crates/quoin-core` up three.
fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        root.pop();
    }
    root
}

/// The vendored contract tree the npm package ships, which stays in
/// `src/semantic/` after the TypeScript beside it is deleted (quoin#452).
fn semantic_root() -> PathBuf {
    repo_root().join("src").join("semantic")
}

fn fixture() -> PathBuf {
    repo_root()
        .join("tests")
        .join("fixtures")
        .join("semantic-module")
        .join("module-ok")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A copy of the `module-ok` fixture under `parent`, renamed to `name`, with
/// `extra` appended to its `semantic` block.
///
/// Edited as TEXT, and the edit is asserted to have landed: the `semantic`
/// block is the last thing in the fixture's `manifest.yaml`, so an appended
/// key joins it. Re-emitting the document would put a YAML writer in the test
/// scaffolding for the sake of two lines.
fn module_copy(parent: &Path, name: &str, extra: &str) -> PathBuf {
    let root = parent.join(name);
    copy_tree(&fixture(), &root);
    let manifest_path = root.join("manifest.yaml");
    let text = fs::read_to_string(&manifest_path).unwrap();
    let renamed = text.replace(
        "\nname: spec-objects-fixture\n",
        &format!("\nname: {name}\n"),
    );
    assert_ne!(renamed, text, "the fixture's name line moved");
    assert!(
        renamed.trim_end().ends_with("markdown]"),
        "the semantic block is no longer last in the fixture: {renamed}"
    );
    fs::write(&manifest_path, format!("{renamed}{extra}")).unwrap();
    root
}

fn install(home: &Path, source: &Path) -> Run {
    run(
        "modules.install",
        &json!({
            "source": format!("path:{}", source.display()),
            "home": home.to_string_lossy(),
        }),
    )
}

fn installed(home: &Path) -> Vec<String> {
    run("modules.list", &json!({ "home": home.to_string_lossy() })).ok()["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|module| module["name"].as_str().unwrap().to_owned())
        .collect()
}

/// A scratch directory and an empty `~/.ix` inside it.
fn scratch() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();
    (dir, home)
}

/// A second module declaring a package another installed module already claims
/// is refused, and the refusal names BOTH modules.
///
/// The first install is the control: without it a gate that refused every
/// install would satisfy the refusal.
///
/// Trace: FR-070-AC-6
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_660_a_second_module_claiming_an_installed_package_is_refused_naming_both() {
    let (dir, home) = scratch();
    let alpha = module_copy(dir.path(), "alpha-module", "");
    let beta = module_copy(dir.path(), "beta-module", "");

    install(&home, &alpha).ok();
    let refusal = install(&home, &beta).refused();
    assert!(refusal.contains("semantic.duplicate-package"), "{refusal}");
    assert!(refusal.contains("alpha-module"), "{refusal}");
    assert!(refusal.contains("beta-module"), "{refusal}");
    assert_eq!(installed(&home), vec!["alpha-module".to_owned()]);
}

/// A re-install refused by the contract restores the version that was there.
///
/// Trace: FR-070-AC-3
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_661_a_rejected_reinstall_restores_the_installed_version() {
    let (dir, home) = scratch();
    let good = module_copy(dir.path(), "stable", "");
    install(&home, &good).ok();

    let bad_parent = dir.path().join("bad");
    fs::create_dir_all(&bad_parent).unwrap();
    let bad = module_copy(&bad_parent, "stable", "  foo: 1\n");
    let refusal = install(&home, &bad).refused();
    assert!(refusal.contains("semantic.unknown-key"), "{refusal}");

    assert_eq!(installed(&home), vec!["stable".to_owned()]);
    let restored = fs::read_to_string(
        home.join("filament")
            .join("modules")
            .join("stable")
            .join("manifest.yaml"),
    )
    .unwrap();
    assert!(
        !restored.contains("foo:"),
        "the rejected manifest is what is on disk: {restored}"
    );
}

/// Installing a semantic module writes the derived package manifest beside it
/// and records one digest per export in the registry.
///
/// Trace: FR-075-AC-1, FR-075-AC-2
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_662_installing_writes_the_derived_manifest_and_pins_every_export() {
    let (dir, home) = scratch();
    let root = module_copy(dir.path(), "derive", "");
    let record = install(&home, &root).ok()["module"].clone();

    let module_dir = home.join("filament").join("modules").join("derive");
    let derived: Value = serde_json::from_str(
        &fs::read_to_string(module_dir.join("semantic").join("package-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        derived["package"],
        json!({ "identity": "agent-ix/spec-objects-fixture", "version": "0.1.0" })
    );
    assert_eq!(
        derived["exports"][0]["typeIdentity"],
        "ix://agent-ix/spec-objects-fixture/type/entity"
    );

    let pin = &record["semantic"];
    assert_eq!(pin["package"], "agent-ix/spec-objects-fixture");
    assert_eq!(pin["semanticCore"], "0.1.0");
    let digest = pin["exports"]["entity"]
        .as_str()
        .expect("the export is pinned");
    // The digest is the one the installed manifest records for that export's
    // shipped file — the pin is over the bytes, not over the export's name.
    let manifest = fs::read_to_string(module_dir.join("manifest.yaml")).unwrap();
    assert!(
        manifest.contains(&format!("digest: {digest}")),
        "pin {digest} is not the digest the manifest records: {manifest}"
    );
}

/// `legacy_forms: error` without a usable sweep report refuses the install and
/// leaves nothing behind.
///
/// Trace: FR-074-AC-3
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_663_legacy_forms_error_without_a_sweep_report_refuses_the_install() {
    let (dir, home) = scratch();
    let root = module_copy(dir.path(), "unguarded", "  legacy_forms: error\n");
    let refusal = install(&home, &root).refused();
    assert!(
        refusal.contains("semantic.sweep-report-required"),
        "{refusal}"
    );
    assert_eq!(installed(&home), Vec::<String>::new());
    assert!(
        !home
            .join("filament")
            .join("modules")
            .join("unguarded")
            .exists(),
        "a refused first install leaves nothing behind"
    );
}
