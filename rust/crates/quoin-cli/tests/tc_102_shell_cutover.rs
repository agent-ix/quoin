// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The final executable boundary contains no Node or oclif escape hatch.

#![allow(
    clippy::expect_used,
    reason = "test setup failures are the intended failure report"
)]

use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("quoin-cli lives at rust/crates/quoin-cli")
        .to_path_buf()
}

fn typescript_files(path: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).expect("the source directory is readable") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            typescript_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "ts") {
            found.push(path);
        }
    }
}

/// Trace: FR-102-AC-6
/// Provenance: quoin#373, quoin#521
#[test]
fn tc_1654_the_native_binary_has_no_oclif_or_typescript_shell() {
    let root = repository_root();
    for retired in [
        "bin/quoin.js",
        "package.json",
        "pnpm-lock.yaml",
        "rust/crates/quoin-schemas",
        "src/base.ts",
        "src/cli.ts",
        "src/commands",
        "src/core",
        "src/flow-command.ts",
        "src/hooks",
        "vite.config.ts",
    ] {
        assert!(
            !root.join(retired).exists(),
            "the retired Node/oclif shell path remains: {retired}"
        );
    }

    let source = root.join("src");
    let mut typescript = Vec::new();
    if source.exists() {
        typescript_files(&source, &mut typescript);
    }
    assert!(
        typescript.is_empty(),
        "retired TypeScript sources: {typescript:?}"
    );
}
