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

/// The retained measurement corpus `DIVERGENCE.md` §2 is measured over,
/// relative to the crate root. Only this directory: the repository holds
/// several checkouts of itself under `.worktrees/`, and a walk that reached
/// them would count the same observation many times.
const RETAINED_MEASUREMENTS: &str = "../../../spec/evidence/measurements";

/// The floors the §2 census is measured against, one order of magnitude below
/// the figures measured at the revision that wrote them (48 collection files,
/// 14,644 observations, 50,882 dimension values). The corpus grows; these say
/// it has not collapsed.
const RETAINED_COLLECTION_FLOOR: usize = 40;

/// As [`RETAINED_COLLECTION_FLOOR`], for observations.
const RETAINED_OBSERVATION_FLOOR: usize = 12_000;

/// As [`RETAINED_COLLECTION_FLOOR`], for dimension values.
const RETAINED_DIMENSION_VALUE_FLOOR: usize = 40_000;

/// How many offending values the §2 census names before it stops collecting.
const OFFENDER_REPORT_CAP: usize = 10;

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

/// The bound `DIVERGENCE.md` §2 rests on, measured rather than recited.
///
/// §2 declares a divergence that **cannot** be put under the golden: a
/// non-string `dimensions` member throws part-way through the retained
/// renderer, so there are no TypeScript bytes to compare against. Its only
/// bound is a census of the retained corpus, and a bound that lives in a
/// sentence no test evaluates is not a bound — the first draft of that sentence
/// carried figures inflated eleven-fold by a glob that escaped into
/// `.worktrees/`, and nothing caught it. This walks the directory the sentence
/// is about.
///
/// Trace: FR-100-AC-4
/// Trace: FR-101-AC-5
/// Provenance: quoin#476
#[test]
fn tc_476_024_no_retained_dimension_value_is_a_non_string() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(RETAINED_MEASUREMENTS);
    let mut files = 0_usize;
    let mut observations = 0_usize;
    let mut with_dimensions = 0_usize;
    let mut values = 0_usize;
    let mut non_strings = 0_usize;
    let mut offenders: Vec<String> = Vec::new();

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{}: unreadable: {error}", root.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    entries.sort();

    for path in entries {
        files += 1;
        let text = std::fs::read_to_string(&path).expect("a collection file");
        let document: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|error| panic!("{path:?}: {error}"));
        let rows = document
            .get("observations")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("{path:?}: no observations array"));
        for row in rows {
            observations += 1;
            let Some(dimensions) = row.get("dimensions") else {
                continue;
            };
            let dimensions = dimensions
                .as_object()
                .unwrap_or_else(|| panic!("{path:?}: dimensions is not an object"));
            with_dimensions += 1;
            for (name, value) in dimensions {
                values += 1;
                if !value.is_string() {
                    // Capped: one offender names the class, and a corpus-wide
                    // break would otherwise print megabytes before the
                    // assertion is readable.
                    non_strings += 1;
                    if offenders.len() < OFFENDER_REPORT_CAP {
                        let file = path.file_name().unwrap_or(path.as_os_str());
                        offenders.push(format!("{}: {name} = {value}", file.to_string_lossy()));
                    }
                }
            }
        }
    }

    // Anti-vacuity: a directory that emptied, moved, or stopped being read
    // would otherwise make every assertion below pass over nothing.
    assert!(
        files >= RETAINED_COLLECTION_FLOOR
            && observations >= RETAINED_OBSERVATION_FLOOR
            && values >= RETAINED_DIMENSION_VALUE_FLOOR,
        "the census read {files} files, {observations} observations and {values} \
         dimension values; the floors are {RETAINED_COLLECTION_FLOOR}, \
         {RETAINED_OBSERVATION_FLOOR} and {RETAINED_DIMENSION_VALUE_FLOOR}. \
         DIVERGENCE.md §2 quotes this measurement, so a shrunken population is a \
         failure rather than a pass"
    );
    // Not every observation states `dimensions`, and the fallback §2 is about
    // is reached for both spellings. If that stopped being true the sentence in
    // §2 about the absent case would have gone stale too.
    assert!(
        with_dimensions < observations,
        "every retained observation now states dimensions; DIVERGENCE.md §2 says \
         some do not"
    );
    assert_eq!(
        non_strings, 0,
        "DIVERGENCE.md §2 bounds a non-fixturable divergence on there being no \
         non-string dimension value in the retained corpus. {non_strings} of \
         {values} now are, the first of them: {offenders:?}"
    );
}
