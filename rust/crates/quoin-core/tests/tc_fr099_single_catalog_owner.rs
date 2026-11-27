// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The catalog and installed-module registry retain one implementation owner (FR-099).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a duplicate architecture owner is a failing test"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn rust_sources(directory: &Path, output: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(directory)
        .expect("crate source directory is readable")
        .map(|entry| entry.expect("source entry is readable").path())
        .collect();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate is under crates")
        .parent()
        .expect("crates is under rust")
        .to_path_buf()
}

#[trace("FR-099-CON-1")]
#[test]
fn catalog_projection_and_module_registry_have_one_public_owner_each() {
    let workspace = workspace_root();
    let crates = workspace.join("crates");
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(&crates).expect("workspace crates are readable") {
        let crate_root = entry.expect("crate entry is readable").path();
        let source_root = crate_root.join("src");
        if source_root.is_dir() {
            rust_sources(&source_root, &mut sources);
        }
    }

    let declarations = [
        (
            "pub struct Catalog",
            workspace.join("crates/quoin-catalog/src/lib.rs"),
        ),
        (
            "pub struct ModuleRegistry",
            workspace.join("crates/quoin-modules/src/registry.rs"),
        ),
    ];
    for (declaration, expected_owner) in declarations {
        let owners: Vec<_> = sources
            .iter()
            .filter_map(|path| {
                let source = std::fs::read_to_string(path).expect("production source is readable");
                source
                    .lines()
                    .any(|line| line.trim_start().starts_with(declaration))
                    .then_some(path)
            })
            .collect();
        assert_eq!(
            owners,
            [&expected_owner],
            "{declaration} must have exactly one production owner"
        );
    }

    let boundary_catalog =
        std::fs::read_to_string(workspace.join("crates/quoin-core/src/ops/catalog.rs"))
            .expect("the boundary catalog adapter is readable");
    assert!(boundary_catalog.contains("quoin_catalog::build("));

    let module_adapter =
        std::fs::read_to_string(workspace.join("crates/quoin-core/src/ops/modules/mod.rs"))
            .expect("the boundary module adapter is readable");
    assert!(module_adapter.contains("quoin_modules::"));
}
