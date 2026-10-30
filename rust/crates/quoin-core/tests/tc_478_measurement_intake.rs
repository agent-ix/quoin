// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The measurement routes that WRITE, each driven through the production runtime
//! against a real store.
//!
//! # Route coverage is not handler coverage (quoin#447)
//!
//! `measurement.record` is one wire name over several intakes, and a suite that
//! exercised the route once would leave two of the three never executed with
//! every gate green. So it is driven once per intake here, and each run asserts the
//! path only its own intake can return:
//!
//! | intake | published at |
//! | --- | --- |
//! | collection | `spec/evidence/measurements/<collection id>.json` |
//! | intervention | `spec/evidence/interventions/p-<record id>.json` |
//!
//! The agent-eval producer lands an intervention under `interventions/`.
//!
//! # Where the inputs come from
//!
//! Nothing below writes its own fixture and then reads it back. Each candidate
//! comes from the committed measurement-store fixtures, copied into a temporary
//! tree so the store may be written to. The release-producer inputs are
//! synthetic and built in the test.
//! A self-written candidate would prove only that the boundary agrees with this
//! file.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::{Path, PathBuf};

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

/// One invocation of the production runtime.
struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, stdin: &str) -> Run {
    let request: Value = serde_json::from_str(stdin).unwrap();
    let response =
        dispatch(op, &request, &RuntimeSettings::default()).unwrap_or_else(|error| Response {
            payload: Value::Null,
            diagnostics: vec![Diagnostic::from(&error)],
            outcome: error.outcome(),
        });
    let stdout = if response.outcome.carries_payload() {
        canonical_json(&response.payload).unwrap()
    } else {
        String::new()
    };
    let stderr = if response.diagnostics.is_empty() {
        String::new()
    } else {
        canonical_json(&response.diagnostics).unwrap()
    };
    Run {
        stdout,
        stderr,
        status: i32::from(response.outcome.code()),
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

/// A workspace holding this repository's governing plans and selected evidence
/// fixtures, and no published record of its own.
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
            &repo()
                .join("tests/fixtures/evidence/measurement-store")
                .join(name),
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
/// Trace: FR-096-AC-2
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
/// Trace: FR-096-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_101_an_intervention_record_is_published_under_interventions() {
    let temporary = workspace(&["agent-evals"]);
    let root = temporary.path();
    let candidate = read_json(&repo().join(
        "tests/fixtures/evidence/measurement-store/interventions/\
             quoin-270-cli-eval-sentinel-contract.json",
    ));
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

/// `measurement.produce_agent_eval_intervention` produces a record from a
/// retained definition and publishes it.
///
/// Trace: FR-096-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_103_the_agent_eval_producer_publishes_an_intervention() {
    let temporary = workspace(&["agent-evals"]);
    let root = temporary.path();
    let definition = read_json(&repo().join(
        "tests/fixtures/evidence/measurement-store/agent-evals/quoin-270-sentinel-definition.json",
    ));

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

/// A repository holding a synthetic governing plan and synthetic GitHub release
/// exports, built here, and the producer definition that names them.
fn synthetic_release_workspace() -> (tempfile::TempDir, Value) {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
    let assurance = root.join("spec/assurance");
    std::fs::create_dir_all(&assurance).expect("an assurance directory");
    std::fs::write(
        assurance.join("MP-9001-synthetic-release-operational-evidence.md"),
        "---\nid: MP-9001\ntitle: Synthetic release operational evidence\ntype: MeasurementPlan\n\
status: active\nowner: engineering assurance\nstage: gate\nmetric: release_operational_evidence\n\
definition_version: github-actions.release-operational-v2\nground_truth_kind: mechanical\n\
protected_apparatus:\n  - spec/evidence/github-actions/synthetic-workflow.yml\nnegative_controls:\n  - kind: apparatus-edit\n    description: an edit to the protected apparatus without a new definition version rejects\nrelationships: []\n---\n\n# Synthetic release operational evidence\n\n## Decision Use\n\nDecide.\n",
    )
    .expect("a governing plan");
    let actions = root.join("spec/evidence/github-actions");
    std::fs::create_dir_all(&actions).expect("an exports directory");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |elapsed| elapsed.as_nanos());
    let revision = format!("{nanos:040x}");
    let job = |name: &str, started: &str, completed: &str| {
        json!({
            "run_id": 7, "run_attempt": 1, "head_sha": revision, "status": "completed",
            "conclusion": "success", "started_at": started, "completed_at": completed,
            "name": name
        })
    };
    let write = |name: &str, text: String| {
        std::fs::write(actions.join(name), text).expect("a writable export");
    };
    write(
        "synthetic-run.json",
        json!({
            "id": 7, "head_sha": revision, "path": ".github/workflows/release.yml",
            "event": "workflow_dispatch", "status": "completed", "conclusion": "success",
            "html_url": "https://github.example.test/runs/7", "updated_at": "2026-08-29T23:11:38Z",
            "run_attempt": 1, "actor": { "login": "release-actor" }
        })
        .to_string(),
    );
    write(
        "synthetic-jobs.json",
        json!({ "jobs": [job("Publish", "2026-08-29T23:11:00Z", "2026-08-29T23:11:37Z")] })
            .to_string(),
    );
    write(
        "synthetic-workflow.yml",
        "name: Release\non:\n  workflow_dispatch:\njobs:\n  publish:\n    name: Publish\n    steps: []\n"
            .to_owned(),
    );
    let definition = json!({
        "record_prefix": "synthetic-release",
        "workflow_path": ".github/workflows/release.yml",
        "release_job": "Publish",
        "accepted_event": "workflow_dispatch",
        "control_id": "synthetic-release-control",
        "subject": { "id": "synthetic/subject" },
        "producer": {
            "tool_identity": "synthetic-adapter", "tool_version": "0.0.1",
            "configuration_ref": "github-actions/synthetic-definition.json",
            "environment": {},
            "definition_version": "github-actions.release-operational-v2"
        },
        "scope": { "service": "synthetic", "environment": "test", "population": "synthetic" },
        "supported_transition": "tagged-to-published",
        "authorized_roles": ["maintainer"],
        "coverage": "synthetic coverage",
        "limitations": ["synthetic"],
        "owner": "engineering assurance",
        "gaps": ["synthetic"],
        "actions": ["synthetic"],
        "clock_deadline_seconds": 600,
        "workflow_evidence_path": "github-actions/synthetic-workflow.yml",
        "run_evidence_path": "github-actions/synthetic-run.json",
        "jobs_evidence_path": "github-actions/synthetic-jobs.json"
    });
    (temporary, definition)
}

/// `measurement.produce_github_release_operational` produces the linked pair
/// from synthetic exports and publishes it under `operational/pairs/`.
///
/// Trace: FR-096-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_104_the_release_producer_publishes_an_operational_pair() {
    let (temporary, definition) = synthetic_release_workspace();
    let root = temporary.path();

    let path = published(&run(
        "measurement.produce_github_release_operational",
        &json!({ "repo": root.to_string_lossy(), "definition": definition }).to_string(),
    ));
    assert_eq!(directory(&path), "pairs", "{}", path.display());
    let produced = read_json(&path);
    assert_eq!(produced["records"].as_array().map(Vec::len), Some(2));
}

/// The operational intake: `record_type: operational_evidence` is published
/// under `operational/`, at a name derived from the record identity.
///
/// Trace: FR-096-AC-2
/// Provenance: quoin#478
#[test]
fn tc_478_102_an_operational_record_is_published_under_operational() {
    let (source, definition) = synthetic_release_workspace();
    let pair = published(&run(
        "measurement.produce_github_release_operational",
        &json!({ "repo": source.path().to_string_lossy(), "definition": definition }).to_string(),
    ));
    let candidate = read_json(&pair)["records"][0].clone();
    assert_eq!(candidate["record_type"], "operational_evidence");

    let (target, _) = synthetic_release_workspace();
    let path = published(&run(
        "measurement.record",
        &json!({ "repo": target.path().to_string_lossy(), "record": candidate }).to_string(),
    ));
    assert_eq!(directory(&path), "operational", "{}", path.display());
    assert_eq!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some("synthetic-release-capability.json")
    );
    assert!(path.exists(), "{} was not written", path.display());
}

/// A request past its intake's ceiling is refused by the PROCESS, with the
/// declared refusal status and a diagnostic naming the operation — and the
/// repository it names does not exist, so a store opened before the ceiling was
/// applied would answer with an I/O fault instead.
///
/// Trace: FR-096-AC-4
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
