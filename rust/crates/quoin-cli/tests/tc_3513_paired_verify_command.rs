// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `quoin measurement verify` and `quoin report` evaluate a plan that
//! declares `paired_run` over its latest two recorded collections
//! (FR-132-AC-8, AC-13; PLAT-1404).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::Path;
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const PLAN: &str = "---\nid: MP-001\ntitle: Determinism\ntype: MeasurementPlan\nstatus: active\n\
    owner: test\nstage: branch-comparison\nmetric: quality.example\n\
    definition_version: quality.example-v1\npaired_run:\n  relation: equal\n  minimum_pairs: 1\n\
    ---\n\n# Determinism\n";

fn quoin(repo: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(arguments)
        .args(["--repo", repo.to_str().unwrap()])
        .output()
        .unwrap()
}

fn git(repo: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success(), "git {arguments:?}");
}

/// Record a collection whose one slice has `value`, and commit it.
fn record(repo: &Path, id: &str, value: f64) {
    let collection = json!({
        "schemaVersion": 2, "collectionId": id, "subject": "fixture",
        "scope": { "cases": 1 }, "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1", "timestamp": "2026-08-26T00:00:00.000Z",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"], "artifacts": ["config"],
        },
        "observations": [{
            "metric": "quality.example", "planId": "MP-001",
            "definitionVersion": "quality.example-v1", "state": "measured",
            "value": value, "unit": "count", "shape": "count",
            "dimensions": { "root": "r1" },
            "population": { "examined": 1, "complete": true },
        }],
        "rawEvidence": { "payload": [1] },
    });
    let input = repo.join("input.json");
    std::fs::write(&input, collection.to_string()).unwrap();
    let output = quoin(
        repo,
        &["measurement", "record", "--input", input.to_str().unwrap()],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_file(input).unwrap();
    git(repo, &["add", "spec"]);
    git(repo, &["commit", "--quiet", "-m", id]);
}

fn verify(repo: &Path) -> (Option<i32>, Value) {
    let output = quoin(repo, &["measurement", "verify", "--plan", "MP-001"]);
    (
        output.status.code(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[trace("FR-132-AC-8", "FR-132-AC-10", "FR-132-AC-13", "TC-3513")]
#[test]
fn tc_3513_verify_and_report_evaluate_a_paired_plan() {
    let repo = tempfile::tempdir().unwrap();
    let assurance = repo.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::write(assurance.join("MP-001.md"), PLAN).unwrap();
    git(repo.path(), &["init", "--quiet"]);
    git(repo.path(), &["add", "spec"]);
    git(repo.path(), &["commit", "--quiet", "-m", "plan"]);

    record(repo.path(), "run-a", 1.0);
    let (status, alone) = verify(repo.path());
    assert_eq!(status, Some(1));
    assert_eq!(alone["reasons"], json!(["paired_run_unpaired"]));

    record(repo.path(), "run-b", 1.0);
    let (status, equal) = verify(repo.path());
    assert_eq!(status, Some(0), "{equal}");
    assert_eq!(equal["verdict"], json!("accept"));
    assert_eq!(equal["decisions"][0]["estimate"].as_f64(), Some(0.0));

    let report = quoin(repo.path(), &["report"]);
    let text = String::from_utf8_lossy(&report.stdout);
    assert!(
        text.contains("| quality.example | MP-001 | equal | observed | 0 | 1 | 1 |"),
        "{text}"
    );

    record(repo.path(), "run-c", 2.0);
    let (status, differing) = verify(repo.path());
    assert_eq!(status, Some(1));
    assert_eq!(differing["reasons"], json!(["paired_run_differs"]));
    assert_eq!(differing["decisions"][0]["estimate"].as_f64(), Some(1.0));

    // Three collections: the report attests no order and defers to `verify`.
    let report = quoin(repo.path(), &["report"]);
    let text = String::from_utf8_lossy(&report.stdout);
    assert!(text.contains("| equal | inconclusive | - |"), "{text}");
    assert!(text.contains("order_unattested"), "{text}");
}
