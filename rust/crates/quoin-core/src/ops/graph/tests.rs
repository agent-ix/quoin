// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `graph` domain, decided against a tree in memory.
//!
//! Every test here substitutes a [`FakeTree`] for the granted reader, so the
//! whole domain — the request shapes, the ceilings, the exit mapping and the
//! two renderings — is exercised with no disk at all. That is the property
//! `tests/tc_library_containment.rs` makes a rule: `ops/graph/` is handed what
//! it may read and acquires nothing.
//!
//! What the three views COMPUTE is `quoin-graph-analysis`' and is pinned there
//! against the captured oracle corpus (`tc_385_parity.rs`). What is pinned here
//! is the boundary: that a request reaches those functions unchanged, that a
//! refusal reaches the caller as the status `src/commands/graph/*` exited with,
//! and that the payload is the bytes the renderer wrote.
//!
//! Provenance: quoin#500

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_graph_analysis::GraphInputReader;
use serde_json::{Value, json};

use crate::capabilities::Capabilities;
use crate::error::CoreErrorCode;
use crate::protocol::Response;

use super::{MAX_SCALAR_BYTES, change_impact, churn, fan_out};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SCHEMA_DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

/// A tree of files, and nothing else, standing in for the filesystem.
struct FakeTree(BTreeMap<PathBuf, String>);

impl GraphInputReader for FakeTree {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        self.0.get(path).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("{}", path.display()))
        })
    }
}

fn modules() -> Value {
    json!([{
        "name": "example",
        "version": "1.0.0",
        "schemas": [{ "archetype": "FR", "schema_digest": SCHEMA_DIGEST }],
    }])
}

fn premises() -> Value {
    json!({ "format": "quire-assurance", "format_version": 1, "modules": modules() })
}

fn export() -> Value {
    json!({
        "format": "quire-assurance",
        "format_version": 1,
        "modules": modules(),
        "source": { "repository": "agent-ix/example", "revision": REVISION },
        "artifacts": [{
            "id": "FR-001",
            "artifact_type": "FR",
            "locator": { "path": "spec/FR-001.md", "line": 1, "digest": DIGEST },
        }],
        "obligations": [{
            "source": "acceptance-criterion",
            "id": "FR-001-AC-1",
            "document": "spec/FR-001.md",
            "statement": "The command reports fan-out.",
            "statement_hash": DIGEST,
            "target_ids": ["TC-1249"],
            "locator": { "path": "spec/FR-001.md", "line": 20, "digest": DIGEST },
        }],
        "symbols": [],
        "relation_kinds": quoin_graph_analysis::DEFAULT_RELATION_KINDS
            .iter()
            .map(|kind| json!({
                "kind": kind,
                "availability": "available",
                "sources": ["module_vocabulary"],
            }))
            .collect::<Vec<_>>(),
        "relations": [],
        "relation_observations": [],
    })
}

fn audit() -> Value {
    json!({
        "format": "quoin-audit-envelope",
        "format_version": 1,
        "source": { "repository": "agent-ix/example", "revision": REVISION },
        "export": premises(),
        "report": { "findings": [], "healthy": ["FR-001-AC-1"], "unevaluated": [] },
    })
}

fn bindings() -> Value {
    json!({
        "schemaVersion": 1,
        "bindings": [{
            "obligation": "FR-001-AC-1",
            "statementHashAtBinding": DIGEST,
            "suite": "unit",
            "commit": REVISION,
            "symbols": ["fan-out test"],
        }],
    })
}

/// The four paths a request names, and the tree that satisfies them.
fn tree(with_bindings: bool) -> FakeTree {
    let mut files = BTreeMap::new();
    files.insert(PathBuf::from("/in/export.json"), export().to_string());
    files.insert(PathBuf::from("/in/premises.json"), premises().to_string());
    files.insert(PathBuf::from("/in/audit.json"), audit().to_string());
    if with_bindings {
        let path = quoin_store::store::store_root(Path::new("/repo"))
            .join(quoin_evidence::paths::bindings_path());
        files.insert(path, bindings().to_string());
    }
    FakeTree(files)
}

fn request(json_output: bool) -> Value {
    json!({
        "repo": "/repo",
        "export_path": "/in/export.json",
        "premises_path": "/in/premises.json",
        "audit_path": "/in/audit.json",
        "json": json_output,
    })
}

/// The one member every `graph.*` payload carries.
fn rendered(response: &Response) -> &str {
    response.payload["rendered"]
        .as_str()
        .expect("the payload carries a rendered document")
}

/// `graph.fan_out` renders the canonical JSON the retained command printed.
///
/// Trace: FR-062-AC-1
/// Provenance: quoin#500
#[test]
fn fan_out_answers_with_the_canonical_json_report() {
    let reader = tree(true);
    let response = fan_out(&request(true), &Capabilities::with_graph(&reader)).unwrap();
    assert_eq!(response.outcome.code(), 0);
    let report: Value = serde_json::from_str(rendered(&response)).expect("canonical JSON");
    assert_eq!(report["view"], "fan-out");
    assert_eq!(report["state"], "complete");
    assert_eq!(report["rows"][0]["suite"], "unit");
    assert_eq!(report["rows"][0]["obligationCount"], 1);
}

/// The markdown spelling is the other rendering of the SAME report, not a
/// second report.
///
/// Trace: FR-062-AC-10
/// Provenance: quoin#500
#[test]
fn the_two_spellings_render_one_report() {
    let reader = tree(true);
    let grant = Capabilities::with_graph(&reader);
    let markdown = fan_out(&request(false), &grant).unwrap();
    let canonical = fan_out(&request(true), &grant).unwrap();
    assert!(rendered(&markdown).contains("Suite"));
    assert!(rendered(&markdown).contains(REVISION));
    assert!(!rendered(&markdown).starts_with('{'));
    assert!(rendered(&canonical).starts_with('{'));
}

/// `graph.churn` and `graph.change_impact` reach their own views.
///
/// Trace: FR-062-AC-7
/// Provenance: quoin#500
#[test]
fn each_operation_reaches_its_own_view() {
    let reader = tree(true);
    let grant = Capabilities::with_graph(&reader);

    let churned = churn(&request(true), &grant).unwrap();
    let report: Value = serde_json::from_str(rendered(&churned)).expect("canonical JSON");
    assert_eq!(report["view"], "churn");
    assert_eq!(report["rows"][0]["obligation"], "FR-001-AC-1");
    assert_eq!(report["rows"][0]["eventCount"], 0);

    let mut impact_request = request(true);
    impact_request["requirements"] = json!(["FR-001"]);
    let impacted = change_impact(&impact_request, &grant).unwrap();
    let report: Value = serde_json::from_str(rendered(&impacted)).expect("canonical JSON");
    assert_eq!(report["view"], "change-impact");
    assert_eq!(report["requested"], json!(["FR-001"]));
    assert_eq!(report["rows"][0]["requirement"], "FR-001");
    assert_eq!(report["rows"][0]["depth"], 0);
}

/// An absent relationship vocabulary is the eight defaults; an EMPTY one is a
/// walk with no edges. The two are different answers and the wire keeps them
/// apart.
///
/// Trace: FR-062-AC-3
/// Provenance: quoin#500
#[test]
fn an_absent_relation_list_is_not_an_empty_one() {
    let reader = tree(true);
    let grant = Capabilities::with_graph(&reader);

    let mut absent = request(true);
    absent["requirements"] = json!(["FR-001"]);
    let defaulted = change_impact(&absent, &grant).unwrap();
    let report: Value = serde_json::from_str(rendered(&defaulted)).expect("canonical JSON");
    assert_eq!(
        report["relationKinds"].as_array().map(Vec::len),
        Some(quoin_graph_analysis::DEFAULT_RELATION_KINDS.len()),
        "an absent `relations` must mean the default vocabulary"
    );

    let mut empty = absent.clone();
    empty["relations"] = json!([]);
    let none = change_impact(&empty, &grant).unwrap();
    let report: Value = serde_json::from_str(rendered(&none)).expect("canonical JSON");
    assert_eq!(
        report["relationKinds"],
        json!([]),
        "an empty `relations` must mean no edges at all, not the defaults"
    );
}

/// A declared input that is not there is the caller's file, not the caller's
/// request: exit 2, naming which input.
///
/// Trace: FR-062-AC-9
/// Provenance: quoin#500
#[test]
fn a_missing_declared_input_is_refused_by_name() {
    let mut files = tree(true);
    files.0.remove(Path::new("/in/audit.json"));
    let error = fan_out(&request(true), &Capabilities::with_graph(&files)).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["input"], "audit");
    assert_eq!(error.context["graph_code"], "QGA-1001");
}

/// An absent bindings store is NOT a refusal: it is a report that says nothing
/// was computed, which is the distinction `src/graph-analysis/load.ts` drew and
/// the one a healthy zero would erase.
///
/// Trace: FR-062-AC-9
/// Provenance: quoin#500
#[test]
fn an_absent_bindings_store_is_a_report_and_not_a_refusal() {
    let reader = tree(false);
    let response = fan_out(&request(true), &Capabilities::with_graph(&reader)).unwrap();
    let report: Value = serde_json::from_str(rendered(&response)).expect("canonical JSON");
    assert_eq!(report["state"], "not_computed");
    assert_eq!(report["rows"], json!([]));
    assert!(
        !report["gaps"].as_array().expect("gaps").is_empty(),
        "an unavailable store must be reported as a gap rather than as a clean empty result"
    );
}

/// A misspelled member is refused rather than silently dropped (rust-style §11).
///
/// Provenance: quoin#500
#[test]
fn an_unknown_member_is_refused() {
    let reader = tree(true);
    let mut bad = request(true);
    bad["premisis_path"] = json!("/in/premises.json");
    let error = fan_out(&bad, &Capabilities::with_graph(&reader)).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.outcome().code(), 3);
}

/// Every path is bounded before it is opened.
///
/// Provenance: quoin#500
#[test]
fn an_oversized_path_is_refused_before_any_read() {
    let reader = tree(true);
    let mut bad = request(true);
    bad["export_path"] = json!("/".repeat(MAX_SCALAR_BYTES + 1));
    let error = fan_out(&bad, &Capabilities::with_graph(&reader)).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["field"], "export_path");
    assert_eq!(error.context["limit_bytes"], MAX_SCALAR_BYTES.to_string());
}

/// A build that dispatched a graph operation without granting a reader is a
/// build fault (4), never a refusal aimed at the caller (2).
///
/// Provenance: quoin#500
#[test]
fn a_missing_grant_is_an_internal_fault() {
    let error = fan_out(&request(true), &Capabilities::none()).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Io);
    assert_eq!(error.outcome().code(), 4);
    assert_eq!(error.context["op"], "graph.fan_out");
}
