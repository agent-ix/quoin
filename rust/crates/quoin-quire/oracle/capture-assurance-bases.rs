// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One-time capture: the **base documents** of the `assurance-v1` parity
//! corpus (quoin#474).
//!
//! # Why a capture step exists at all
//!
//! The measurement schemas' parity corpus (`quoin-jsonschema`) is built from
//! records retained under `spec/evidence/`, so its `.mjs` oracle reads them off
//! disk. There is **no retained `assurance-v1` document in this repository** —
//! `git grep '"quire-assurance"'` finds the schema and nothing else — and the
//! only producer of one is the linked Rust engine, which a Node script cannot
//! call.
//!
//! So the base documents are produced here, once, by
//! `quire_rs::build_assurance_export` over the committed fixture corpus, and
//! committed as `tests/goldens/assurance-bases.json` with the engine revision
//! and quoin revision that produced them. `capture-ajv-verdicts.mjs` then reads
//! that file, applies the mutation ladder, and records the **oracle's** verdict
//! — `validateAssurance` from `src/quire/validate.ts`.
//!
//! This is not a self-fixture: the bases are the engine's output, and the thing
//! under test is the schema check, which had no hand in producing them.
//!
//! Run from `rust/`:
//!
//! ```text
//! cargo run -p quoin-quire --example capture-assurance-bases
//! ```
//!
//! Deleted at the cutover commit per FR-101-AC-5.

use std::path::{Path, PathBuf};
use std::process::Command;

use quoin_quire::{ModuleSelection, ScopeRoot, assurance, ids};
use serde_json::{Value, json};

/// One base: a scope, the module set it is read under, and the name the corpus
/// knows it by.
struct Base {
    name: &'static str,
    /// The scope root, relative to the crate manifest directory.
    scope: &'static str,
    /// The module roots, relative to the crate manifest directory.
    modules: &'static [&'static str],
    /// The repository identity written into the source premise.
    repository: &'static str,
    /// The revision written into the source premise. Forty hex digits, and a
    /// literal rather than this checkout's HEAD so the capture is reproducible.
    revision: &'static str,
}

/// The committed fixture corpora that export.
///
/// Two, and they differ in shape rather than in labels: read under its own
/// module the fixture repository has archetypes, a traceability model and one
/// bound trace marker, so every collection is populated; read under
/// `module-no-traceability` it has none, so `artifacts`, `obligations`,
/// `relation_kinds` and `relations` come back empty. A corpus built from one
/// shape cannot separate a schema that requires a populated collection from one
/// that merely tolerates it.
const BASES: &[Base] = &[
    Base {
        name: "fixture-repo",
        scope: "tests/fixtures/repo",
        modules: &["tests/fixtures/repo"],
        repository: "agent-ix/quoin",
        revision: "0000000000000000000000000000000000000000",
    },
    Base {
        name: "fixture-repo-under-a-module-with-no-traceability",
        scope: "tests/fixtures/repo",
        modules: &["tests/fixtures/module-no-traceability"],
        repository: "agent-ix/quoin",
        revision: "1111111111111111111111111111111111111111",
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut documents = Vec::new();
    for base in BASES {
        let scope_path = manifest.join(base.scope);
        let mut roots = Vec::new();
        for module in base.modules {
            roots.push(ids::ModuleRoot::open(manifest.join(module))?);
        }
        let outcome = assurance::build(&assurance::Request {
            scope: ScopeRoot::open(&scope_path)?,
            modules: ModuleSelection::Closed(roots),
            repository: ids::RepositoryId::new(base.repository)?,
            revision: ids::RevisionId::new(base.revision)?,
        })?;
        let document: Value = serde_json::from_slice(&outcome.export.to_json_bytes()?)?;
        documents.push(json!({
            "name": base.name,
            "origin": format!("{}/{}", "rust/crates/quoin-quire", base.scope),
            "document": document,
        }));
    }

    let captured = json!({
        "provenance": {
            "producer": "rust/crates/quoin-quire/oracle/capture-assurance-bases.rs",
            "engine": engine_pin(&manifest)?,
            "quoin_revision": revision(&manifest)?,
        },
        "bases": documents,
    });

    let out = manifest
        .join("tests")
        .join("goldens")
        .join("assurance-bases.json");
    let mut text = serde_json::to_string_pretty(&captured)?;
    text.push('\n');
    std::fs::write(&out, text)?;
    println!("wrote {} bases to {}", BASES.len(), out.display());
    Ok(())
}

/// The `quire-rs` rev this build linked, read from the manifest rather than
/// restated — the same rule `build.rs` follows.
fn engine_pin(manifest: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(manifest.join("Cargo.toml"))?;
    let line = text
        .lines()
        .find(|line| line.starts_with("quire-rs = "))
        .ok_or("Cargo.toml no longer declares quire-rs on one line")?;
    Ok(line.trim().to_string())
}

fn revision(manifest: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["-C", &manifest.display().to_string(), "rev-parse", "HEAD"])
        .output()?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
