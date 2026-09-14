// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The five measurement routes that WRITE, each driven through the real binary
//! against a real store.
//!
//! # Route coverage is not handler coverage (quoin#447)
//!
//! `measurement.record` is one wire name over three intakes, and a suite that
//! exercised the route once would leave two of the three never executed with
//! every gate green. So it is driven three times here, and each run asserts the
//! path only its own intake can return:
//!
//! | intake | published at |
//! | --- | --- |
//! | collection | `spec/evidence/measurements/<collection id>.json` |
//! | intervention | `spec/evidence/interventions/p-<record id>.json` |
//! | operational | `spec/evidence/operational/<digest>.json` |
//!
//! The two producers are held apart the same way: an agent-eval intervention
//! lands under `interventions/`, and a GitHub release pair under
//! `operational/pairs/`, which is neither of the other two.
//!
//! # Where the inputs come from
//!
//! Nothing below writes its own fixture and then reads it back. Every candidate
//! is a document this repository already retains — the committed collection
//! fixture under `quoin-measurement/tests/fixtures/`, the retained intervention
//! record, the retained operational pair and the two retained producer
//! definitions — copied into a temporary tree so the store may be written to.
//! A self-written candidate would prove only that the boundary agrees with this
//! file.

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

/// The path a publishing or producing route answered with.
fn published(result: &Run) -> PathBuf {
    let payload = ok(result);
    PathBuf::from(
        payload["path"]
            .as_str()
            .unwrap_or_else(|| panic!("the payload carries no path: {payload}")),
    )
}

/// This repository's root: `crates/quoin-core` -> `crates` -> `rust` -> root.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

/// The committed measurement fixture tree `quoin-measurement` reports over.
fn fixture_tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-measurement/tests/fixtures/portfolio-tree")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a destination directory");
    for entry in std::fs::read_dir(from).expect("a readable source directory") {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a copied file");
        }
    }
}

fn read_json(path: &Path) -> Value {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The name of the directory a published path rests in.
fn directory(path: &Path) -> &str {
    path.parent()
        .and_then(Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_else(|| panic!("{} has no parent directory", path.display()))
}

/// A workspace holding this repository's governing plans and the retained
/// evidence directories named, and no published record of its own.
///
/// Not a copy of the whole of `spec/`: the retained measurements alone are
/// tens of megabytes, and none of the routes below reads them. What is copied
/// is exactly the producer's and the intake's input contract.
fn workspace(evidence: &[&str]) -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    copy_tree(
        &repo().join("spec/assurance"),
        &temporary.path().join("spec/assurance"),
    );
    for name in evidence {
        copy_tree(
            &repo().join("spec/evidence").join(name),
            &temporary.path().join("spec/evidence").join(name),
        );
    }
    temporary
}

/// The collection intake: a candidate carrying no `record_type` is a
/// collection, and it is published under `measurements/`.
///
/// The candidate is the committed schema-version-2 fixture narrowed to the one
/// observation whose plan is active at the definition version it names — the
/// store refuses a collection naming a retired or drifted plan, and refusing is
/// not what this test is measuring.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_100_a_collection_is_published_under_measurements() {
    let alpha = fixture_tree().join("alpha");
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
    copy_tree(&alpha.join("spec"), &root.join("spec"));

    let mut candidate = read_json(&alpha.join("spec/evidence/measurements/b-2026-02.json"));
    candidate["collectionId"] = json!("d-2026-04");
    let recall = candidate["observations"][0].clone();
    assert_eq!(recall["metric"], "finding_recall", "{recall}");
    candidate["observations"] = json!([recall]);

    let path = published(&run(
        "measurement.record",
        &json!({ "repo": root.to_string_lossy(), "record": candidate }).to_string(),
    ));
    assert_eq!(directory(&path), "measurements", "{}", path.display());
    assert_eq!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some("d-2026-04.json"),
        "{}",
        path.display()
    );
    assert!(path.exists(), "{} was not written", path.display());
}

/// The intervention intake: `record_type: intervention_experiment` is published
/// under `interventions/`, at the `p-` prefixed name the store mints.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_101_an_intervention_record_is_published_under_interventions() {
    let temporary = workspace(&["agent-evals"]);
    let root = temporary.path();
    let candidate = read_json(
        &repo().join("spec/evidence/interventions/quoin-270-cli-eval-sentinel-contract.json"),
    );
    assert_eq!(candidate["record_type"], "intervention_experiment");

    let path = published(&run(
        "measurement.record",
        &json!({ "repo": root.to_string_lossy(), "record": candidate }).to_string(),
    ));
    assert_eq!(directory(&path), "interventions", "{}", path.display());
    assert_eq!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some("p-quoin-270-cli-eval-sentinel-contract.json"),
        "{}",
        path.display()
    );
    assert!(path.exists(), "{} was not written", path.display());
}

/// The operational intake: `record_type: operational_evidence` is published
/// under `operational/`, at a name derived from the record identity's digest.
///
/// The candidate is the standing capability of the retained pair, published
/// into a store that holds no pair — the retained pair FILE is what
/// `read_operational_entries` would answer with for a record already inside
/// one, so a store carrying it would not exercise this path at all.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_102_an_operational_record_is_published_under_operational() {
    let temporary = workspace(&["github-actions"]);
    let root = temporary.path();
    let pair = read_json(&repo().join(
        "spec/evidence/operational/pairs/\
         c1b30a188d4d03bbe316e0fdb7582eff3fa314c55268a5f71f81f99f8ea2acf8.json",
    ));
    let candidate = pair["records"][0].clone();
    assert_eq!(candidate["record_type"], "operational_evidence");
    assert_eq!(
        candidate["record_id"],
        "quoin-271-release-v0.22.5-capability"
    );

    let path = published(&run(
        "measurement.record",
        &json!({ "repo": root.to_string_lossy(), "record": candidate }).to_string(),
    ));
    assert_eq!(directory(&path), "operational", "{}", path.display());
    let name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_else(|| panic!("{} has no file name", path.display()));
    let stem = name.trim_end_matches(".json");
    assert_eq!(stem.len(), 64, "{name} is not a digest basename");
    assert!(
        stem.chars().all(|c| c.is_ascii_hexdigit()),
        "{name} is not a digest basename"
    );
    assert!(path.exists(), "{} was not written", path.display());
}

/// `measurement.produce_agent_eval_intervention` produces a record from a
/// retained definition and publishes it.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_103_the_agent_eval_producer_publishes_an_intervention() {
    let temporary = workspace(&["agent-evals"]);
    let root = temporary.path();
    let definition =
        read_json(&repo().join("spec/evidence/agent-evals/quoin-270-sentinel-definition.json"));

    let path = published(&run(
        "measurement.produce_agent_eval_intervention",
        &json!({ "repo": root.to_string_lossy(), "definition": definition }).to_string(),
    ));
    assert_eq!(directory(&path), "interventions", "{}", path.display());
    assert_eq!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some("p-quoin-270-cli-eval-sentinel-contract.json"),
        "{}",
        path.display()
    );
    let produced = read_json(&path);
    assert_eq!(produced["record_type"], "intervention_experiment");
    assert_eq!(
        produced["record_id"],
        "quoin-270-cli-eval-sentinel-contract"
    );
}

/// `measurement.produce_github_release_operational` produces the linked pair
/// and publishes it under `operational/pairs/` — which is neither of the two
/// directories the record route writes to.
///
/// Trace: FR-096-AC-2, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_104_the_release_producer_publishes_an_operational_pair() {
    let temporary = workspace(&["github-actions"]);
    let root = temporary.path();
    let definition = read_json(
        &repo().join("spec/evidence/github-actions/quoin-271-release-v0.22.5-definition.json"),
    );

    let path = published(&run(
        "measurement.produce_github_release_operational",
        &json!({ "repo": root.to_string_lossy(), "definition": definition }).to_string(),
    ));
    assert_eq!(directory(&path), "pairs", "{}", path.display());
    assert_eq!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some("c1b30a188d4d03bbe316e0fdb7582eff3fa314c55268a5f71f81f99f8ea2acf8.json"),
        "{}",
        path.display()
    );
    let produced = read_json(&path);
    assert_eq!(produced["records"].as_array().map(Vec::len), Some(2));
}

/// A request past its intake's ceiling is refused by the PROCESS, with the
/// declared refusal status and a diagnostic naming the operation — and the
/// repository it names does not exist, so a store opened before the ceiling was
/// applied would answer with an I/O fault instead.
///
/// Trace: FR-096-AC-4, FR-101-AC-7
/// Provenance: quoin#478
#[test]
fn tc_478_105_an_oversized_record_is_refused_by_the_process() {
    let oversized = json!({
        "repo": "/nonexistent/quoin-478",
        "record": { "padding": "x".repeat(quoin_core::ops::measurement::MAX_COLLECTION_BYTES) },
    });
    let result = run("measurement.record", &oversized.to_string());
    assert_eq!(result.status, 2, "{}", result.stderr);
    assert_eq!(result.stdout, "", "a refusal carries no payload");
    let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
    assert_eq!(diagnostics[0]["context"]["op"], "measurement.record");
    assert_eq!(
        diagnostics[0]["context"]["limit_bytes"],
        quoin_core::ops::measurement::MAX_COLLECTION_BYTES.to_string()
    );
}
