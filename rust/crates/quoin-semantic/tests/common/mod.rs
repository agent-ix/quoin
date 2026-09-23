// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shared scaffolding for the quoin#452 criterion restatements.
//!
//! The TypeScript these tests replace built every case the same way: copy
//! `tests/fixtures/semantic-module/module-ok` into a scratch directory, edit
//! one thing in its `manifest.yaml`, and read the result. That shape is kept
//! deliberately — a criterion restated against a hand-written fixture of the
//! test's own invention proves the test agrees with itself, not that the
//! shipped fixture is judged the way the contract says.
//!
//! The mutated manifest is written back as **JSON**. JSON is a subset of YAML
//! 1.2, which is the dialect `quoin-yaml` reads, so `manifest.yaml` stays
//! readable by the same parser the production path uses; the alternative is a
//! YAML *emitter* in the test scaffolding, which would put a second
//! serialisation of the manifest shape in the tree.

#![allow(
    dead_code,
    unreachable_pub,
    reason = "one helper set shared by four test binaries; each uses a subset, and \
              `mod common` is private to each, so every item here reads as unreachable"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};

use quoin_semantic::manifest::{SemanticValidators, read_manifest_yaml};
use quoin_semantic::{SemanticDiagnostic, SemanticReadResult, read_module_semantic};
use serde_json::Value;

/// The repository root: `rust/crates/quoin-semantic` up three.
pub fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        root.pop();
    }
    root
}

/// The semantic contract's root: quoin's own `sweep-report.schema.json` plus
/// the module-manifest and semantic-core schemas quoin depends on from
/// `agent-ix/filament-core-data`'s published `@agent-ix/semantic-schema` and
/// `@agent-ix/semantic-core` packages, read out of `node_modules` (PLAT-887's
/// de-vendoring) -- not git submodules.
///
/// Materialized from [`quoin_semantic::materialize_embedded_contract`] --
/// the exact bytes `build.rs` embedded into this test binary -- rather than
/// read from a committed copy in the source tree. That is the same function
/// the production binary calls at run time, so a test judging against this
/// root is judging against the real embedded contract, not a directory a
/// test wrote for itself.
///
/// One directory per **process**, cached in a `OnceLock` rather than shared
/// `OUT_DIR` state: `OUT_DIR` is one physical directory reused across every
/// test binary this package builds, and cargo runs those binaries as
/// concurrent processes, so two of them materializing into it at once could
/// race on the same nested directories. A `std::process::id()`-suffixed
/// scratch directory gives each process its own, and the `OnceLock` keeps
/// concurrent test *threads* inside one process from re-materializing (and
/// racing each other) on every call.
pub fn semantic_root() -> PathBuf {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let root =
            std::env::temp_dir().join(format!("quoin-semantic-contract-{}", std::process::id()));
        quoin_semantic::materialize_embedded_contract(&root)
            .expect("the embedded semantic contract materializes");
        root
    })
    .clone()
}

/// `src/semantic/schemas`.
pub fn schema_dir() -> PathBuf {
    quoin_semantic::contract::schema_dir(&semantic_root())
}

/// The vendored semantic-core bundle directory.
pub fn semantic_core_dir() -> PathBuf {
    quoin_semantic::contract::semantic_core_dir(&semantic_root())
}

/// `tests/fixtures/semantic-module`.
pub fn fixtures_dir() -> PathBuf {
    repo_root()
        .join("tests")
        .join("fixtures")
        .join("semantic-module")
}

/// The `module-ok` fixture: a valid `semantic` block, one reference-form
/// export and one inline `data_schema`.
pub fn fixture() -> PathBuf {
    fixtures_dir().join("module-ok")
}

/// `tests/fixtures/semantic-module/mapping`.
pub fn mapping_dir() -> PathBuf {
    fixtures_dir().join("mapping")
}

/// The compiled validators, built from the live vendored tree.
pub fn validators() -> SemanticValidators {
    SemanticValidators::load(&semantic_root()).expect("the vendored contract compiles")
}

/// Copy a directory tree, recursively.
pub fn copy_tree(from: &Path, to: &Path) {
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

/// A scratch directory that lives as long as the test.
pub struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    /// A fresh empty directory, removed when the value drops.
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("a scratch directory"),
        }
    }

    /// The directory itself.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A copy of `module-ok` at `<scratch>/<name>`, renamed to `name`, with
    /// `mutate` applied to its manifest before it is written back.
    pub fn module_copy(&self, name: &str, mutate: impl FnOnce(&mut Value, &Path)) -> PathBuf {
        let root = self.path().join(name);
        copy_tree(&fixture(), &root);
        let manifest_path = root.join("manifest.yaml");
        let mut manifest = read_manifest_yaml(&manifest_path).expect("the fixture manifest parses");
        manifest["name"] = Value::String(name.to_owned());
        mutate(&mut manifest, &root);
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        root
    }

    /// A copy of `module-ok` with nothing changed but its name.
    pub fn plain_copy(&self, name: &str) -> PathBuf {
        self.module_copy(name, |_, _| ())
    }
}

impl Default for Scratch {
    fn default() -> Self {
        Self::new()
    }
}

/// The read of one module root, which must have been readable.
pub fn read(root: &Path, validators: &SemanticValidators) -> SemanticReadResult {
    read_module_semantic(root, validators).expect("the module manifest is readable")
}

/// Every diagnostic as `severity:code@path`, the spelling the deleted
/// TypeScript asserted on.
pub fn codes(root: &Path, validators: &SemanticValidators) -> Vec<String> {
    render(&read(root, validators).diagnostics)
}

/// The same rendering, over an explicit diagnostic list.
pub fn render(diagnostics: &[SemanticDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|d| format!("{}:{}@{}", d.severity.as_str(), d.code.as_str(), d.path))
        .collect()
}

/// Only the error-severity renderings.
pub fn errors(root: &Path, validators: &SemanticValidators) -> Vec<String> {
    codes(root, validators)
        .into_iter()
        .filter(|c| c.starts_with("error:"))
        .collect()
}

/// The message of the first diagnostic carrying `code`.
pub fn message_for(root: &Path, validators: &SemanticValidators, code: &str) -> String {
    read(root, validators)
        .diagnostics
        .into_iter()
        .find(|d| d.code.as_str() == code)
        .unwrap_or_else(|| panic!("no {code} diagnostic was reported"))
        .message
}

/// `sha256:<hex>` over a file's raw bytes, the spelling a `data_schema.digest`
/// carries.
pub fn digest_of(path: &Path) -> String {
    quoin_semantic::contract::file_sha256(path).expect("the schema file is readable")
}

/// Rewrite the fixture's `schemas/Entity.json` through `edit` and re-record
/// its digest in the manifest, so a case about one rule is not failed by the
/// digest rule it did not mean to trip.
pub fn rewrite_entity_schema(manifest: &mut Value, root: &Path, edit: impl FnOnce(&mut Value)) {
    let file = root.join("schemas").join("Entity.json");
    let mut schema: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    edit(&mut schema);
    fs::write(&file, serde_json::to_string(&schema).unwrap()).unwrap();
    manifest["object_types"][0]["data_schema"]["digest"] = Value::String(digest_of(&file));
}

/// One fixture file's text, from `tests/fixtures/semantic-module/mapping`.
pub fn mapping_fixture(name: &str) -> String {
    fs::read_to_string(mapping_dir().join(name))
        .unwrap_or_else(|e| panic!("mapping fixture {name}: {e}"))
}

/// One fixture file parsed as JSON.
pub fn mapping_json(name: &str) -> Value {
    serde_json::from_str(&mapping_fixture(name)).unwrap()
}
