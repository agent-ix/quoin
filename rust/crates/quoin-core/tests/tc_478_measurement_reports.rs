// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The seven measurement routes that READ, each driven through the real binary
//! over the committed fixture trees.
//!
//! # Why a build/render pair is asserted apart
//!
//! Four of these seven are `build_*`/`render_*` pairs taking the SAME request,
//! so a swapped match arm inside a pair is invisible to the drift guard and to
//! any test that only checks a run succeeded. Each pair is therefore asserted
//! on the shape only its own half answers with: a `build_*` returns the
//! canonical JSON document, a `render_*` returns `{ "rendered": … }`, and
//! nothing else can.
//!
//! The two portfolio pairs answer over the same document type and would survive
//! a swap ACROSS the pairs, so the graph pair is asserted on the two members
//! the plain portfolio's report does not carry — `graph` and `graphQuality`.
//!
//! # The trees are the ones the library crates already report over
//!
//! `quoin-measurement/tests/fixtures/portfolio-tree` and
//! `quoin-measurement-graph/tests/fixtures/graph-loader-tree` are committed
//! fixtures with their own frozen oracles. Nothing here writes a tree and reads
//! it back: reading a store this file had just written would measure this file.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// One invocation of the boundary binary.
struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, stdin: &str) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin-core"))
        .arg(op)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status: out.status.code().unwrap(),
    }
}

/// The payload of a run that must have succeeded.
fn ok(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    serde_json::from_str(&result.stdout).unwrap()
}

/// The markdown a `render_*` route answered with.
fn rendered(result: &Run) -> String {
    let payload = ok(result);
    payload["rendered"]
        .as_str()
        .unwrap_or_else(|| panic!("the payload carries no rendered document: {payload}"))
        .to_owned()
}

/// The committed measurement fixture tree.
fn tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-measurement/tests/fixtures/portfolio-tree")
}

/// The committed graph fixture tree, whose repositories carry graph documents.
fn graph_tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../quoin-measurement-graph/tests/fixtures/graph-loader-tree")
}

fn repo(root: &Path, name: &str) -> String {
    root.join(name).to_string_lossy().into_owned()
}

/// A `<repository>=<document>` mapping, as the command line spells one.
fn mapping(name: &str, file: &str) -> String {
    format!(
        "{}={}",
        repo(&graph_tree(), name),
        graph_tree().join(name).join("graph").join(file).display()
    )
}

/// The report pair over one store: the document, and the markdown.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_200_the_report_pair_reads_one_store() {
    let request = json!({ "repo": repo(&tree(), "alpha") }).to_string();

    let report = ok(&run("measurement.build_report", &request));
    let current = report["current"]
        .as_array()
        .unwrap_or_else(|| panic!("the report states no current observations: {report}"));
    assert!(!current.is_empty(), "{report}");
    let plans: Vec<&str> = current
        .iter()
        .filter_map(|entry| entry["planId"].as_str())
        .collect();
    assert!(plans.contains(&"ap-actionability"), "{plans:?}");
    assert_eq!(report["corpusGaps"], 3, "{report}");

    let markdown = rendered(&run("measurement.render_report", &request));
    assert!(
        markdown.starts_with("# QA measurement report"),
        "{markdown}"
    );
    assert!(markdown.contains("ap-actionability"), "{markdown}");
}

/// The comparison pair: the same locating request, one earlier revision.
///
/// `measurement.render_comparison` is the twelfth route, and the one quoin#478's
/// list of eleven does not name; `src/commands/report.ts` cannot be rewired
/// without it.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_201_the_comparison_pair_names_both_collections() {
    let request = json!({
        "repo": repo(&tree(), "alpha"),
        "before_revision": "revision-one",
    })
    .to_string();

    let comparison = ok(&run("measurement.build_comparison", &request));
    assert_eq!(comparison["before"]["collectionId"], "a-2026-01");
    assert_eq!(comparison["after"]["collectionId"], "c-2026-03");

    let markdown = rendered(&run("measurement.render_comparison", &request));
    assert!(
        markdown.starts_with("# QA measurement comparison"),
        "{markdown}"
    );
    assert!(markdown.contains("| Metric | Status |"), "{markdown}");
}

/// A series is one metric's history, oldest first.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_202_a_series_is_one_metrics_history() {
    let points = ok(&run(
        "measurement.build_series",
        &json!({
            "repo": repo(&tree(), "alpha"),
            "metric": "finding_recall",
        })
        .to_string(),
    ));
    let points = points
        .as_array()
        .unwrap_or_else(|| panic!("a series is an array: {points}"));
    assert!(!points.is_empty(), "the series is empty");
    for point in points {
        assert_eq!(point["observation"]["metric"], "finding_recall", "{point}");
    }
    assert_eq!(points[0]["sourceRevision"], "revision-one", "{}", points[0]);
}

/// The portfolio pair spans several stores, and its repository entries carry no
/// graph.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_203_the_portfolio_pair_spans_several_stores() {
    let request = json!({
        "locations": [repo(&tree(), "alpha"), repo(&tree(), "bravo")],
    })
    .to_string();

    let portfolio = ok(&run("measurement.build_portfolio", &request));
    let repositories = portfolio["repositories"]
        .as_array()
        .unwrap_or_else(|| panic!("the portfolio names no repositories: {portfolio}"));
    assert_eq!(repositories.len(), 2, "{portfolio}");
    let names: Vec<&str> = repositories
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(names, vec!["alpha", "bravo"], "{portfolio}");
    assert!(
        repositories
            .iter()
            .all(|entry| entry.get("graph").is_none()),
        "the plain portfolio does not read a graph: {portfolio}"
    );

    let markdown = rendered(&run("measurement.render_portfolio", &request));
    assert!(markdown.starts_with("# QA portfolio report"), "{markdown}");
    assert!(markdown.contains("## alpha"), "{markdown}");
}

/// The graph portfolio pair reads the mapped graph documents over the same
/// stores, and says what the plain pair cannot.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_204_the_graph_portfolio_pair_carries_the_graph() {
    let root = graph_tree();
    let request = json!({
        "locations": [repo(&root, "alpha"), repo(&root, "bravo")],
        "graph_exports": [mapping("alpha", "export.json")],
        "graph_premises": [mapping("alpha", "premises.json")],
        "graph_audits": [mapping("alpha", "audit.json")],
        "changed": [format!("{}=FR-001", repo(&root, "alpha"))],
        "cwd": root.to_string_lossy(),
    })
    .to_string();

    let portfolio = ok(&run("measurement.build_graph_portfolio", &request));
    let repositories = portfolio["repositories"]
        .as_array()
        .unwrap_or_else(|| panic!("the portfolio names no repositories: {portfolio}"));
    assert_eq!(repositories.len(), 2, "{portfolio}");
    let alpha = repositories
        .iter()
        .find(|entry| entry["name"] == "alpha")
        .unwrap_or_else(|| panic!("alpha is absent: {portfolio}"));
    // The two members the plain portfolio's report does not carry. A run that
    // reached `build_portfolio` instead would answer without them.
    assert_eq!(alpha["graph"]["availability"], "available", "{alpha}");
    assert!(alpha.get("graphQuality").is_some(), "{alpha}");

    let markdown = rendered(&run("measurement.render_graph_portfolio", &request));
    assert!(markdown.starts_with("# QA portfolio report"), "{markdown}");
    assert!(markdown.contains("graph"), "{markdown}");
}
