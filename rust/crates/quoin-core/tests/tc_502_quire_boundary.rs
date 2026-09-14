// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The two `quire.*` routes, each driven through the real binary.
//!
//! # Why by name, through the process
//!
//! `src/ops/quire/tests.rs` proves what the domain DECIDES, against an
//! in-memory host. It cannot prove that the wire name `quire.coverage` reaches
//! the coverage handler, because it calls the handler directly — the condition
//! `tc_447_operation_census.rs` exists to make impossible. Both operations take
//! a `scope` and a `modules` list, so a swapped match arm in `dispatch` would
//! deserialise either request happily and answer with the other operation's
//! payload. Each route is therefore run by its own name here and asserted on a
//! member only its own handler produces.
//!
//! # And why against a real repository
//!
//! These two operations exist BECAUSE a repository cannot ride on stdin. The
//! host is the only part of the domain a unit test cannot stand in for, so the
//! one thing this file adds is that the granted host really walks a tree:
//! `tests/fixtures/quire-repo` is both the scope and the module, which is the
//! `ScopeOrAmbient` shape a repository carrying its own module has.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::io::Write as _;
use std::path::PathBuf;
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

fn payload(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    serde_json::from_str(&result.stdout).unwrap()
}

/// The fixture repository, which is also the fixture module.
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("quire-repo")
}

/// A CLOSED module set naming the fixture, never ambient: an ambient selection
/// makes the answer depend on which modules the machine running the suite
/// happens to have installed.
fn request() -> Value {
    let repo = repo().display().to_string();
    json!({ "scope": repo, "modules": [repo] })
}

/// `quire.coverage` derives the obligations the module's `obligations:` source
/// declares, over a real `spec/` tree.
///
/// Trace: FR-099
/// Provenance: quoin#502
#[test]
fn tc_502_010_coverage_derives_the_obligations_the_repository_states() {
    let result = run("quire.coverage", &request().to_string());
    let payload = payload(&result);

    let obligations = payload["obligations"].as_array().unwrap();
    assert_eq!(
        obligations.len(),
        2,
        "the fixture states two criteria; an empty census would assert nothing"
    );
    assert_eq!(obligations[0]["id"], "FR-001-AC-1");
    assert_eq!(
        obligations[0]["statement"],
        "Every stored payload shall round-trip through the reader unchanged."
    );
    assert_eq!(obligations[0]["method"], "Test");
    assert_eq!(obligations[0]["target_ids"], json!(["TC-001"]));
    assert!(
        obligations[0]["statement_hash"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64),
        "the statement hash is what suspect-link detection compares"
    );
    assert!(
        payload.get("shapes").is_none(),
        "this name must not reach the properties handler"
    );
}

/// `quire.properties` classifies the same criteria and answers with the shape
/// map, keyed on the obligation id.
///
/// Trace: FR-052
/// Provenance: quoin#502
#[test]
fn tc_502_011_properties_classifies_the_documents_under_spec() {
    // No `documents`: the empty list asks for the `spec/**/*.md` the retained
    // `quoin advise` passed as a glob, and the walk that answers it is the
    // host's, which is exactly what this file is here to exercise.
    let result = run("quire.properties", &request().to_string());
    let payload = payload(&result);

    let shapes = payload["shapes"].as_object().unwrap();
    assert_eq!(
        shapes.len(),
        2,
        "both criteria carry a row id, so both have a shape to offer"
    );
    assert_eq!(shapes["FR-001-AC-1"]["archetype"], "FR");
    assert!(
        shapes["FR-001-AC-1"]["property"].is_string(),
        "every classified criterion carries a shape token, `unclassified` included"
    );
    assert_eq!(payload["unresolved"], json!([]));
    assert!(
        payload.get("obligations").is_none(),
        "this name must not reach the coverage handler"
    );
}

/// A scope with no `spec/` is the world declining, not the request being
/// malformed: exit 2, naming the engine rule.
///
/// Trace: FR-099
/// Provenance: quoin#502
#[test]
fn tc_502_012_a_repository_without_a_document_root_is_refused_by_name() {
    let mut request = request();
    let empty = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    request["scope"] = json!(empty.display().to_string());

    let result = run("quire.coverage", &request.to_string());
    assert_eq!(result.status, 2, "{}", result.stderr);
    assert_eq!(result.stdout, "");
    let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
    assert_eq!(diagnostics[0]["context"]["op"], "quire.coverage");
    assert_eq!(diagnostics[0]["context"]["quire_code"], "QQ-1002");
}
