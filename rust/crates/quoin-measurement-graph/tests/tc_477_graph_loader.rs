// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The governed graph portfolio *loader*, against the retained TypeScript.
//!
//! Trace: FR-067-AC-1
//! Trace: FR-100-AC-4
//! Trace: FR-101-AC-5
//! Provenance: quoin#477
//!
//! # What this gate is
//!
//! `tests/goldens/graph-loader-oracle.json` was captured **once** from
//! `src/measurement/graph-portfolio-load.ts` by
//! `oracle/capture-graph-loader-oracle.mjs`, over the committed fixture tree
//! under `tests/fixtures/graph-loader-tree/`. Both the source and the capture
//! script were deleted at the quoin#480 cutover; the golden is what survives.
//! This test runs
//! [`build_governed_graph_portfolio`] over the same tree and compares. No node
//! process is spawned here, and `tc_476_boundary.rs` asserts none can be.
//!
//! # Two comparisons, and the line between them
//!
//! [`byte_identical`](tc_477_010_byte_identical_report) compares the whole
//! report — canonical JSON and rendered text — byte for byte, over the three
//! repositories whose every sentence both sides can produce.
//!
//! [`verdicts`](tc_477_011_graph_refusal_verdicts) compares availability and
//! path only, over the two repositories whose graph refusal quotes an operating
//! system or a schema validator. `DIVERGENCE.md` §9 states exactly which
//! sentences those are and why neither side can be made to write the other's.
//! The split is deliberate: an exception inside a byte gate is not a byte gate.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::PathBuf;

use quoin_measurement_graph::input::NormalizedStructuralGraph;
use quoin_measurement_graph::mapping::GraphPortfolioMappingOptions;
use quoin_measurement_graph::{
    build_governed_graph_portfolio, canonical_graph_portfolio_json, render_governed_graph_portfolio,
};
use serde_json::Value;

/// The token the capture substituted for the fixture tree's absolute root.
const TREE_TOKEN: &str = "@@TREE@@";

/// The repositories that name graph documents at all.
///
/// `bravo` and `echo` name none, which is the mapping refusal
/// `load_structural_graph` answers without opening a file. The capture script
/// holds the same set and the two must agree or the golden describes a
/// different run.
const MAPPED: [&str; 3] = ["alpha", "charlie", "delta"];

/// The lowest number of repositories the byte-identical case may carry.
///
/// A case list that silently emptied would pass every assertion below. Three
/// is what the capture wrote: one whose graph loads, one with no mapping, one
/// whose collection is undated.
const BYTE_IDENTICAL_FLOOR: usize = 3;

/// As [`BYTE_IDENTICAL_FLOOR`], for the graph refusal verdicts.
const VERDICT_FLOOR: usize = 2;

/// The lowest number of gaps the byte-identical report must state.
///
/// One per unmapped graph export, plus the undated collection and the
/// repository with no active plan. A report that stated none would compare
/// equal to a golden that also stated none.
const GAP_FLOOR: usize = 4;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tree() -> PathBuf {
    crate_root().join("tests/fixtures/graph-loader-tree")
}

fn repo(name: &str) -> PathBuf {
    tree().join(name)
}

fn graph_document(name: &str, file: &str) -> PathBuf {
    tree().join(name).join("graph").join(file)
}

fn golden() -> Value {
    let path = crate_root().join("tests/goldens/graph-loader-oracle.json");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{}: unreadable golden: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("{}: golden is not JSON: {error}", path.display()))
}

/// Replace this checkout's fixture root with the captured token, which is the
/// capture's one normalisation and this file's one normalisation.
fn normalise(text: &str) -> String {
    text.replace(tree().to_string_lossy().as_ref(), TREE_TOKEN)
}

/// The mappings the capture script built, rebuilt from the same rule.
fn options(names: &[&str]) -> GraphPortfolioMappingOptions {
    let mapped: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| MAPPED.contains(name))
        .collect();
    let mapping = |file: &str| -> Vec<String> {
        mapped
            .iter()
            .map(|name| {
                format!(
                    "{}={}",
                    repo(name).display(),
                    graph_document(name, file).display()
                )
            })
            .collect()
    };
    GraphPortfolioMappingOptions {
        graph_exports: mapping("export.json"),
        graph_premises: mapping("premises.json"),
        graph_audits: mapping("audit.json"),
        changed: if names.contains(&"alpha") {
            vec![
                format!("{}=FR-001", repo("alpha").display()),
                format!("{}=FR-002", repo("alpha").display()),
            ]
        } else {
            Vec::new()
        },
        cwd: Some(tree()),
    }
}

fn names_at(case: &Value) -> Vec<String> {
    case.get("repositories")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("golden case has no repositories array"))
        .iter()
        .map(|name| {
            name.as_str()
                .unwrap_or_else(|| panic!("a repository name is not a string"))
                .to_owned()
        })
        .collect()
}

fn text_at<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("golden has no {key} string"))
}

/// The whole report, byte for byte, for the repositories whose every sentence
/// both trees produce.
///
/// Trace: FR-067-AC-1, FR-100-AC-4, FR-101-AC-5
/// Provenance: quoin#477
#[test]
fn tc_477_010_byte_identical_report() {
    let golden = golden();
    let case = golden
        .get("byte_identical")
        .unwrap_or_else(|| panic!("golden has no byte_identical case"));
    let names = names_at(case);
    assert!(
        names.len() >= BYTE_IDENTICAL_FLOOR,
        "the byte-identical case carries {} repositories, below the floor of {BYTE_IDENTICAL_FLOOR}",
        names.len()
    );

    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    let locations: Vec<PathBuf> = borrowed.iter().map(|name| repo(name)).collect();
    let report = build_governed_graph_portfolio(&locations, &options(&borrowed))
        .unwrap_or_else(|error| panic!("the loader refused the fixture tree: {error}"));

    let gaps: usize = report
        .repositories
        .iter()
        .map(|entry| entry.gaps.len())
        .sum();
    assert!(
        gaps >= GAP_FLOOR,
        "the report states {gaps} gaps, below the floor of {GAP_FLOOR}"
    );

    let json = canonical_graph_portfolio_json(&report)
        .unwrap_or_else(|error| panic!("the report would not canonicalise: {error}"));
    assert_eq!(normalise(&json), text_at(case, "json"), "canonical JSON");

    let rendered = render_governed_graph_portfolio(&report)
        .unwrap_or_else(|error| panic!("the report would not render: {error}"));
    assert_eq!(
        normalise(&rendered),
        text_at(case, "rendered"),
        "rendered text"
    );
}

/// Availability and path for the two graph refusals whose sentence is an
/// operating system's or a schema validator's. `DIVERGENCE.md` §9.
///
/// Trace: FR-067-AC-1, FR-100-AC-4, FR-101-AC-5
/// Provenance: quoin#477
#[test]
fn tc_477_011_graph_refusal_verdicts() {
    let golden = golden();
    let case = golden
        .get("verdicts")
        .unwrap_or_else(|| panic!("golden has no verdicts case"));
    let expected = case
        .get("graphs")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("golden verdicts case has no graphs array"));
    assert!(
        expected.len() >= VERDICT_FLOOR,
        "the verdict case carries {} graphs, below the floor of {VERDICT_FLOOR}",
        expected.len()
    );

    let names = names_at(case);
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    let locations: Vec<PathBuf> = borrowed.iter().map(|name| repo(name)).collect();
    let report = build_governed_graph_portfolio(&locations, &options(&borrowed))
        .unwrap_or_else(|error| panic!("the loader refused the fixture tree: {error}"));

    let observed: Vec<Value> = report
        .repositories
        .iter()
        .map(|entry| {
            let path = match entry.graph {
                NormalizedStructuralGraph::Available { ref path, .. }
                | NormalizedStructuralGraph::Unavailable { ref path, .. } => path.as_deref(),
            };
            serde_json::json!({
                "root": normalise(&entry.base.root),
                "availability": entry.graph.availability(),
                "path": path.map(normalise),
            })
        })
        .collect();
    assert_eq!(&observed, expected, "graph refusal verdicts");

    // Anti-vacuity: the two refusals must be different from each other, or a
    // loader that answered one verdict for everything would pass.
    let availabilities: std::collections::BTreeSet<&str> = observed
        .iter()
        .filter_map(|entry| entry.get("availability").and_then(Value::as_str))
        .collect();
    assert!(
        availabilities.len() >= 2,
        "every graph refusal is {availabilities:?}; the case distinguishes nothing"
    );
}

/// The capture recorded where it came from (FR-101-AC-11).
///
/// Trace: FR-101-AC-5
/// Provenance: quoin#477
#[test]
fn tc_477_012_golden_states_its_provenance() {
    let golden = golden();
    for key in [
        "produced_by",
        "produced_by_digest",
        "produced_from_revision",
        "produced_by_node",
        "capture_script",
        "fixture_tree",
        "tree_token",
    ] {
        assert!(
            !text_at(&golden, key).is_empty(),
            "the golden states no {key}"
        );
    }
    assert_eq!(text_at(&golden, "tree_token"), TREE_TOKEN);
    assert_eq!(
        text_at(&golden, "produced_by"),
        "src/measurement/graph-portfolio-load.ts"
    );
}
