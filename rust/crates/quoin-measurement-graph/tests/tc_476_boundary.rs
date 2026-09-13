// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The crate has one canonicaliser, one base64 codec, one digest and no oracle.
//!
//! Trace: FR-100-AC-8
//! Trace: FR-101-AC-5
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
//! The same argument applies to the other three seams this wave could have
//! duplicated and did not: the base64 decoder W8 landed (quoin#465), the SHA-256
//! the scorer-integrity check needs, and the JSON-value bridge.
//!
//! # The retained declaration census
//!
//! quoin#476 states "107 zod declarations" in `graph-portfolio.ts`. That number
//! matches no measurement of that file: it declares **no zod at all** — it is
//! pure TypeScript types over values other modules validated. What it does
//! declare is counted here from both sides, so a lost export is a failing test
//! rather than a silent omission.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The retained module this wave ports, relative to the crate root.
const RETAINED: &str = "../../../src/measurement/graph-portfolio.ts";

/// `export function` declarations in the retained module.
const RETAINED_EXPORTED_FUNCTIONS: usize = 5;

/// Module-private `function` declarations in the retained module.
const RETAINED_PRIVATE_FUNCTIONS: usize = 22;

/// `export type` and `export interface` declarations in the retained module.
const RETAINED_EXPORTED_SHAPES: usize = 17;

/// `z.` constructor calls in the retained module. The ticket says 107; the
/// file says this.
const RETAINED_ZOD_CONSTRUCTORS: usize = 0;

/// Below this many source modules the census is not measuring the crate.
const SOURCE_FLOOR: usize = 10;

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
    assert!(
        found.len() >= SOURCE_FLOOR,
        "anti-vacuity floor: at least {SOURCE_FLOOR} modules expected, saw {} — a census that \
         finds nothing proves nothing",
        found.len()
    );
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

/// One base64 codec (quoin#475's), one SHA-256 (the store's), one bridge.
///
/// Trace: FR-100-AC-8
/// Provenance: quoin#476
#[test]
fn tc_476_021_there_is_no_second_codec_digest_or_bridge() {
    let mut base64: Vec<String> = Vec::new();
    let mut digests: Vec<String> = Vec::new();
    let mut bridges: Vec<String> = Vec::new();
    for (name, contents) in modules() {
        let code = code_of(&contents);
        if code.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz") {
            base64.push(name.clone());
        }
        // Type *names* naming the algorithm are references
        // (`Sha256Reference` wraps the store's parsed digest); an `sha2` crate
        // import or a hasher construction would be a second implementation.
        if code.contains("sha2::") || code.contains("Sha256::new") || code.contains("Digest::new") {
            digests.push(name.clone());
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
        digests.is_empty(),
        "no module may reach a SHA-256 implementation directly; the digest is \
         `quoin_store::digest_bytes_sha256`, and these do otherwise: {digests:?}"
    );
    assert!(
        bridges.is_empty(),
        "the `quoin_store::JsonValue` <-> `serde_json::Value` crossing is \
         `quoin_measurement::json_bridge` and nowhere else; these define a second: {bridges:?}"
    );
}

/// No test in this crate can spawn a TypeScript runtime.
///
/// Trace: FR-101-AC-5
/// Provenance: quoin#476
#[test]
fn tc_476_022_no_test_runs_the_retained_typescript() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut checked = 0_usize;
    for entry in std::fs::read_dir(&root).expect("tests/ is readable") {
        let path = entry.expect("a directory entry").path();
        // This census names the forbidden spellings in order to forbid them.
        let is_this_file = path
            .file_name()
            .is_some_and(|name| name == "tc_476_boundary.rs");
        if path.extension().is_some_and(|extension| extension == "rs") && !is_this_file {
            let code = code_of(&std::fs::read_to_string(&path).expect("a test file"));
            checked += 1;
            for forbidden in ["std::process::Command", "ts-node", "node --"] {
                assert!(
                    !code.contains(forbidden),
                    "{}: contains {forbidden}; the oracle is a committed golden, \
                     captured once (FR-101-AC-5)",
                    path.display()
                );
            }
        }
    }
    assert!(
        checked >= 4,
        "anti-vacuity floor: at least 4 test files expected, saw {checked}"
    );
}

/// What `graph-portfolio.ts` actually declares, counted rather than believed.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#476
#[test]
fn tc_476_023_the_retained_module_is_measured_not_quoted() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(RETAINED);
    let retained = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
    let starting = |prefix: &str| {
        retained
            .lines()
            .filter(|line| line.starts_with(prefix))
            .count()
    };

    assert_eq!(
        starting("export function "),
        RETAINED_EXPORTED_FUNCTIONS,
        "the retained module's exported surface changed"
    );
    assert_eq!(
        starting("function "),
        RETAINED_PRIVATE_FUNCTIONS,
        "the retained module's private surface changed"
    );
    assert_eq!(
        starting("export type ") + starting("export interface "),
        RETAINED_EXPORTED_SHAPES,
        "the retained module's exported shapes changed"
    );
    assert_eq!(
        retained.matches("z.").count(),
        RETAINED_ZOD_CONSTRUCTORS,
        "quoin#476 says the module holds 107 zod declarations; it holds none, and \
         the census asserts what is there rather than what the ticket says is there"
    );

    // The five exports are the five this crate re-exports, by name.
    let lib = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("lib.rs is readable");
    for exported in [
        "parse_graph_portfolio_mappings",
        "build_governed_graph_portfolio_from",
        "compare_graph_quality_collections",
        "canonical_graph_portfolio_json",
        "render_governed_graph_portfolio",
    ] {
        assert!(
            lib.contains(exported),
            "the crate does not re-export {exported}, which the retained module exports"
        );
    }
}
