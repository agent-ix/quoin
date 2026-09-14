// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The three `graph.*` routes, each driven through the real binary.
//!
//! # Why by name, through the process
//!
//! `graph.fan_out`, `graph.churn` and `graph.change_impact` take the SAME
//! request apart from the two change-impact members, so a swapped match arm in
//! `dispatch` would answer every one of them with a well-formed report of the
//! wrong view and leave every unit test green — the condition
//! `tc_447_operation_census.rs` exists to make impossible. Each route is
//! therefore run by its own name here and asserted on the one member only its
//! own handler can produce: `view`.
//!
//! # The tree is a committed fixture with no bindings store
//!
//! `quoin-measurement-graph/tests/fixtures/graph-loader-tree/alpha` carries the
//! oracle's own base export, premises and audit, and no retained evidence
//! store. That is the interesting state at this boundary rather than a
//! shortcoming: an absent store is a REPORT that says so, not a refusal, and a
//! boundary that turned it into one would be the FR-062-AC-9 failure. What the
//! views compute when a store IS there is `quoin-graph-analysis`' own parity
//! corpus, not this file's business.

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

/// The report a successful run rendered, parsed back from its one member.
fn report(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    let payload: Value = serde_json::from_str(&result.stdout).unwrap();
    let rendered = payload["rendered"]
        .as_str()
        .unwrap_or_else(|| panic!("the payload carries no rendered report: {payload}"));
    serde_json::from_str(rendered)
        .unwrap_or_else(|error| panic!("the rendered report is not JSON ({error}): {rendered}"))
}

/// The committed graph fixture tree.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../quoin-measurement-graph/tests/fixtures/graph-loader-tree/alpha")
}

fn path(file: &str) -> String {
    repo().join("graph").join(file).display().to_string()
}

/// The three declared input paths and the spelling, as every route takes them.
fn request(json_output: bool) -> Value {
    json!({
        "repo": repo().display().to_string(),
        "export_path": path("export.json"),
        "premises_path": path("premises.json"),
        "audit_path": path("audit.json"),
        "json": json_output,
    })
}

/// Each named route answers with its own view, and an absent retained store is
/// a report rather than a refusal.
///
/// Trace: FR-062-AC-9, FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#500
#[test]
fn tc_500_300_each_route_answers_with_its_own_view() {
    let mut seeded = request(true);
    seeded["requirements"] = json!(["FR-001"]);

    // Spelled out one call at a time rather than looped over a table: the
    // operation census reads `run("<op>"` out of this file's own source, and a
    // name reached through a loop variable is a name no reader of the census
    // can see.
    let views = [
        (
            "graph.fan_out",
            report(&run("graph.fan_out", &request(true).to_string())),
            "fan-out",
        ),
        (
            "graph.churn",
            report(&run("graph.churn", &request(true).to_string())),
            "churn",
        ),
        (
            "graph.change_impact",
            report(&run("graph.change_impact", &seeded.to_string())),
            "change-impact",
        ),
    ];
    for (op, report, view) in views {
        assert_eq!(report["view"], view, "{op} answered with another view");
        assert_eq!(
            report["state"], "not_computed",
            "{op}: the fixture tree has no retained store"
        );
        assert!(
            report["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gap| gap["kind"] == "absent-bindings-store"),
            "{op}: an absent store must be named as a gap, not implied: {report}"
        );
        assert_eq!(
            report["source"]["repository"], "agent-ix/example",
            "{op}: the accepted source identity is repeated back"
        );
    }
}

/// An absent `relations` member is the eight defaults; an empty one is none.
///
/// The distinction is the whole reason `relations` is an `Option<Vec<String>>`
/// with `#[serde(default)]` rather than a `Vec` — a boundary that normalised
/// the absent case into `[]` would silently turn every default walk into a walk
/// with no edges.
///
/// Trace: FR-062-AC-3
/// Provenance: quoin#500
#[test]
fn tc_500_301_an_absent_relation_list_is_not_an_empty_one() {
    let mut absent = request(true);
    absent["requirements"] = json!(["FR-001"]);
    let mut empty = absent.clone();
    empty["relations"] = json!([]);

    let absent = report(&run("graph.change_impact", &absent.to_string()));
    let empty = report(&run("graph.change_impact", &empty.to_string()));

    assert_eq!(
        absent["relationKinds"],
        json!([
            "depends_on",
            "derives_from",
            "implements",
            "mitigates",
            "refines",
            "requires",
            "satisfies",
            "traces_to"
        ]),
        "an absent relation list must be the eight defaults"
    );
    assert_eq!(
        empty["relationKinds"],
        json!([]),
        "an empty relation list must stay empty"
    );
}

/// Both spellings come from one report: the markdown is the same view.
///
/// Trace: FR-062-AC-10
/// Provenance: quoin#500
#[test]
fn tc_500_302_the_markdown_spelling_is_the_same_view() {
    let result = run("graph.fan_out", &request(false).to_string());
    assert_eq!(result.status, 0, "{}", result.stderr);
    let payload: Value = serde_json::from_str(&result.stdout).unwrap();
    let markdown = payload["rendered"].as_str().unwrap();
    assert!(
        markdown.starts_with("# Evidence graph: fan-out"),
        "{markdown}"
    );
    assert!(
        markdown.contains("0000000000000000000000000000000000000000"),
        "the markdown carries the full accepted revision: {markdown}"
    );
}

/// A declared input that is not there is REFUSED by name, not an internal
/// fault and not an empty report.
///
/// Trace: FR-062-AC-9
/// Provenance: quoin#500
#[test]
fn tc_500_303_a_missing_declared_input_is_refused_by_name() {
    let mut broken = request(true);
    broken["premises_path"] = json!(repo().join("graph/not-here.json").display().to_string());

    let result = run("graph.fan_out", &broken.to_string());
    assert_eq!(result.status, 2, "REFUSED, not INTERNAL: {}", result.stderr);
    assert!(
        result.stdout.is_empty(),
        "a refusal carries no payload: {}",
        result.stdout
    );
    let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
    let text = diagnostics.to_string();
    assert!(
        text.contains("premises"),
        "the refusal must name which of the three inputs it is about: {text}"
    );
}

/// A member the request type does not declare is INVALID, so a caller that
/// misspells one is told rather than quietly ignored.
///
/// Provenance: quoin#500
#[test]
fn tc_500_304_an_undeclared_member_is_invalid() {
    let mut extra = request(true);
    extra["premisis_path"] = json!("a misspelling");

    let result = run("graph.fan_out", &extra.to_string());
    assert_eq!(result.status, 3, "INVALID: {}", result.stderr);
    assert!(result.stdout.is_empty(), "{}", result.stdout);
}
