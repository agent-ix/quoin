// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The reconcile path re-judges what is already installed (quoin#446).
//!
//! `tests/semantic-manifest.test.ts` carried this criterion in TypeScript:
//! `ensureDefaultModules` ran `reconcile(...)` and then
//! `validateInstalledSemantics(home)`, so a module whose `manifest.yaml`
//! changed AFTER it was installed was caught the next time an author touched
//! the catalog. A `lazy` reconcile deliberately skips a settled entry without
//! re-reading it — `quoin_modules::reconcile` `continue`s past a materialised,
//! pin-matching entry — so nothing on the reconcile path would have re-read
//! that manifest, and the criterion would have been dropped by the port rather
//! than carried.
//!
//! Driven through the production runtime rather than a fake host on purpose:
//! the whole condition is a file on disk that no request can describe, so a
//! test that wrote its own in-memory module would be a check over a population
//! it invented. The module here is installed BY the runtime, through
//! `modules.install`, and then tampered with on disk. This preserves the real
//! capability boundary without retaining the temporary `quoin-core` executable.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, request: &serde_json::Value, home: &Path, semantic_root: &Path) -> Run {
    let settings = RuntimeSettings {
        ix_home: Some(home.to_path_buf()),
        semantic_root: Some(semantic_root.to_path_buf()),
    };
    let response = dispatch(op, request, &settings).unwrap_or_else(|error| Response {
        payload: serde_json::Value::Null,
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

/// The real semantic contract `QUOIN_SEMANTIC_ROOT` publishes in production.
/// Materialized once per process from the embedded bytes `quoin-semantic`'s
/// `build.rs` compiles in (PLAT-887 de-vendoring), not read from a
/// source-tree `src/semantic`, which no longer holds a schema tree.
fn semantic_root() -> PathBuf {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let root = std::env::temp_dir().join(format!(
            "quoin-core-tc-446-reconcile-semantic-{}",
            std::process::id()
        ));
        quoin_semantic::materialize_embedded_contract(&root).unwrap();
        root
    })
    .clone()
}

/// The `module-ok` fixture `tests/semantic-manifest.test.ts` installs.
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

/// A home with `module-ok` installed through the boundary itself.
struct Fixture {
    _root: tempfile::TempDir,
    home: PathBuf,
    manifest: PathBuf,
}

fn install_fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    copy_tree(&fixture(), &source);
    let home = root.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let result = run(
        "modules.install",
        &serde_json::json!({ "source": format!("path:{}", source.display()) }),
        &home,
        &semantic_root(),
    );
    assert_eq!(result.status, 0, "install failed: {}", result.stderr);

    let manifest = home
        .join("filament")
        .join("modules")
        .join("spec-objects-fixture")
        .join("manifest.yaml");
    assert!(manifest.is_file(), "module was not materialised");
    Fixture {
        _root: root,
        home,
        manifest,
    }
}

/// An empty default set, so the reconcile itself has nothing to do and the
/// verdict can only come from the re-validation.
const EMPTY_MANIFEST: &str = "schemaVersion: 1\nentries: []\n";

fn ensure_defaults(home: &Path) -> Run {
    run(
        "modules.ensure_defaults",
        &serde_json::json!({ "manifest": EMPTY_MANIFEST, "mode": "lazy" }),
        home,
        &semantic_root(),
    )
}

/// Trace: FR-070-AC-1, NFR-017-AC-1
/// Provenance: quoin#446
#[test]
fn tc_446_040_a_clean_home_reconciles_without_complaint() {
    // The positive control. Without it, the refusal below would be consistent
    // with `ensure_defaults` refusing every home it is ever shown.
    let fixture = install_fixture();
    let result = ensure_defaults(&fixture.home);
    assert_eq!(result.status, 0, "{}", result.stderr);
    let payload: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(payload["installed"], serde_json::json!([]));
}

/// Trace: FR-070-AC-1, NFR-017-AC-1
/// Provenance: quoin#446
#[test]
fn tc_446_041_a_tampered_installed_module_is_refused_on_the_reconcile_path() {
    let fixture = install_fixture();
    let manifest = fs::read_to_string(&fixture.manifest).unwrap();
    let tampered = manifest.replace("targets: [json-schema, markdown]", "targets: [go]");
    assert_ne!(tampered, manifest, "the fixture's targets line moved");
    fs::write(&fixture.manifest, &tampered).unwrap();

    let result = ensure_defaults(&fixture.home);
    assert_eq!(
        result.status, 2,
        "stdout={} err={}",
        result.stdout, result.stderr
    );
    assert_eq!(result.stdout, "", "a refused run writes no payload");
    assert!(
        result.stderr.contains("QM020_SEMANTIC_CONTRACT_VIOLATION"),
        "{}",
        result.stderr
    );
    assert!(
        result.stderr.contains("semantic.unknown-target"),
        "the diagnostic naming the offending rule is not carried: {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("spec-objects-fixture"),
        "the offending module is not named: {}",
        result.stderr
    );
}

/// Trace: FR-070-AC-1, NFR-017-AC-1
/// Provenance: quoin#446
#[test]
fn tc_446_042_an_unavailable_contract_refuses_rather_than_reporting_clean() {
    // Unjudged is not clean. `ContractGate` refuses an install it cannot judge;
    // the re-validation refuses for the same reason, under its own code, so a
    // caller can tell "your module is wrong" from "I could not look".
    let fixture = install_fixture();
    let result = run(
        "modules.ensure_defaults",
        &serde_json::json!({ "manifest": EMPTY_MANIFEST, "mode": "lazy" }),
        &fixture.home,
        Path::new("/nonexistent/semantic/root"),
    );
    assert_eq!(result.status, 2, "{}", result.stderr);
    assert!(
        result
            .stderr
            .contains("QM022_SEMANTIC_CONTRACT_UNAVAILABLE"),
        "{}",
        result.stderr
    );
}
