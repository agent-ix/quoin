// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The crate has one canonicaliser, one base64 codec and no oracle.
//!
//! Trace: FR-100-AC-8
//! Provenance: quoin#476
//!
//! # Why a source census and not a review note
//!
//! FR-100-AC-8 is not "the port should reuse the store's canonical JSON". It is
//! that `canonicalGraphPortfolioJson` delegates and that **no second
//! implementation exists**, which is a property of the tree rather than of one
//! call site. A reviewer reading a diff sees the delegation; only a census sees
//! the copy someone adds next month in a module the diff never touched.
//!
//! The same argument applies to the other two seams this wave could have
//! duplicated and did not: the base64 decoder W8 landed (quoin#465) and the
//! JSON-value bridge.
//!
//! # The ported surface census
//!
//! The retained `graph-portfolio.ts` was deleted by quoin#480, so the surface
//! it exported can no longer be counted from it. What survives the deletion is
//! the obligation that every one of those five entry points still has a Rust
//! home: the census below reads `src/lib.rs` and names them, so dropping one on
//! a later refactor is a failing test rather than a silent omission.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The entry points `src/measurement/graph-portfolio.ts` exported before
/// quoin#480 deleted it, each of which this crate must still re-export.
const PORTED_ENTRY_POINTS: [&str; 5] = [
    "parse_graph_portfolio_mappings",
    "build_governed_graph_portfolio_from",
    "compare_graph_quality_collections",
    "canonical_graph_portfolio_json",
    "render_governed_graph_portfolio",
];

/// The crate's one canonical-JSON module. Every other module reaches canonical
/// JSON through it, and it reaches the store.
const CANONICAL_MODULE: &str = "canonical.rs";

/// The crate's one base64 module (quoin#475).
const BASE64_MODULE: &str = "base64.rs";

/// Every `.rs` file under `src/`, as `(relative path, contents)`.
fn modules() -> Vec<(String, String)> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(directory).expect("src/ is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                into.push((
                    path.strip_prefix(root)
                        .expect("a path under src/")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    std::fs::read_to_string(&path).expect("a source file"),
                ));
            }
        }
    }
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}

/// Source lines with the `//` comment lines removed: a doc comment naming a
/// function is a reference, not a second implementation.
fn code_of(contents: &str) -> String {
    contents
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The one place canonical JSON is written, and the one place it comes from.
///
/// Trace: FR-100-AC-8
/// Provenance: quoin#476
#[test]
fn tc_476_020_there_is_exactly_one_canonical_json_writer() {
    let mut writers: Vec<String> = Vec::new();
    for (name, contents) in modules() {
        let code = code_of(&contents);
        let emits_json = code.contains("canonical_json(")
            || code.contains("canonical_json_bytes(")
            || code.contains("to_string_pretty")
            || code.contains("serde_json::to_string");
        if emits_json {
            writers.push(name);
        }
    }
    assert_eq!(
        writers,
        vec![CANONICAL_MODULE.to_owned()],
        "canonical JSON must be written in exactly one module (FR-100-AC-8); \
         `canonical_graph_portfolio_json` delegates there and nothing else may reimplement it"
    );

    let canonical = modules()
        .into_iter()
        .find(|(name, _)| name == CANONICAL_MODULE)
        .expect("the crate has a canonical module")
        .1;
    assert!(
        canonical.contains("quoin_store::canonical_json"),
        "{CANONICAL_MODULE} must delegate to the store rather than spell the format itself"
    );
    for forbidden in ["fn sort_keys", "b\"  \"", "JSON.stringify"] {
        assert!(
            !canonical.contains(forbidden),
            "{CANONICAL_MODULE} contains {forbidden}: it must delegate, not reimplement"
        );
    }
}

/// One base64 codec (quoin#475's), one bridge.
///
/// Trace: FR-100-AC-8
/// Provenance: quoin#476
#[test]
fn tc_476_021_there_is_no_second_codec_or_bridge() {
    let mut base64: Vec<String> = Vec::new();
    let mut bridges: Vec<String> = Vec::new();
    for (name, contents) in modules() {
        let code = code_of(&contents);
        if code.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz") {
            base64.push(name.clone());
        }
        if code.contains("fn from_serde(") || code.contains("fn to_serde(") {
            bridges.push(name);
        }
    }
    assert_eq!(
        base64,
        vec![BASE64_MODULE.to_owned()],
        "the base64 alphabet may appear in exactly one module; \
         `scorer.rs` calls `crate::base64::decode` rather than writing a second decoder"
    );
    assert!(
        bridges.is_empty(),
        "the `quoin_store::JsonValue` <-> `serde_json::Value` crossing is \
         `quoin_measurement::json_bridge` and nowhere else; these define a second: {bridges:?}"
    );
}

/// No test in this crate can spawn a TypeScript runtime.
///
/// Provenance: quoin#476
#[test]
fn tc_476_022_no_test_runs_the_retained_typescript() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for entry in std::fs::read_dir(&root).expect("tests/ is readable") {
        let path = entry.expect("a directory entry").path();
        // This census names the forbidden spellings in order to forbid them.
        let is_this_file = path
            .file_name()
            .is_some_and(|name| name == "tc_476_boundary.rs");
        if path.extension().is_some_and(|extension| extension == "rs") && !is_this_file {
            let code = code_of(&std::fs::read_to_string(&path).expect("a test file"));
            for forbidden in ["std::process::Command", "ts-node", "node --"] {
                assert!(
                    !code.contains(forbidden),
                    "{}: contains {forbidden}; no test spawns an oracle, \
                     the TypeScript is deleted",
                    path.display()
                );
            }
        }
    }
}

/// Every entry point the deleted module exported still has a Rust home.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#476
#[test]
fn tc_476_023_every_ported_entry_point_is_still_exported() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = std::fs::read_to_string(root.join("src/lib.rs")).expect("lib.rs is readable");
    for exported in PORTED_ENTRY_POINTS {
        assert!(
            lib.contains(exported),
            "the crate does not re-export {exported}, which the ported module exported"
        );
    }
    assert!(
        !root
            .join("../../../src/measurement/graph-portfolio.ts")
            .exists(),
        "src/measurement/graph-portfolio.ts is back; this census is written for a tree in \
         which it is deleted (quoin#480)"
    );
}
