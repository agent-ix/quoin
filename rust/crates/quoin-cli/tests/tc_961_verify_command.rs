// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin measurement verify` orders collections by the git commit that
//! first added each one, not by the producer's stated timestamp (FR-108-AC-2,
//! PLAT-961).
//!
//! The regressed run below states a timestamp AFTER the passing one, so
//! ordering by stated time would make the regression the candidate. It was
//! committed first, so it is history, and the pass that followed it with the
//! same apparatus is a rerun until it passed.
//!
//! Provenance: PLAT-961

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-measurement/tests/fixtures/verify")
}

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

/// Record the fixture collection `name` into `repo`, with its stated
/// timestamp replaced.
fn record(repo: &Path, name: &str, timestamp: &str) {
    let mut collection: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("wrong/selectively-reported-run").join(name))
            .unwrap(),
    )
    .unwrap();
    collection["timestamp"] = json!(timestamp);
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
}

fn verdict(repo: &Path) -> (Option<i32>, Value, String) {
    let output = quoin(repo, &["measurement", "verify", "--plan", "MP-961"]);
    let payload = serde_json::from_slice(&output.stdout).unwrap();
    (
        output.status.code(),
        payload,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Trace: FR-108-AC-2, FR-108-AC-3, FR-108-AC-7, FR-108-AC-8
/// Provenance: PLAT-961
#[test]
fn tc_961_020_git_intake_order_decides_the_candidate_over_a_stated_timestamp() {
    let repo = tempfile::tempdir().unwrap();
    let assurance = repo.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::copy(
        fixtures().join("plans/MP-961-gate.md"),
        assurance.join("MP-961-gate.md"),
    )
    .unwrap();
    git(repo.path(), &["init", "--quiet"]);
    git(repo.path(), &["add", "spec"]);
    git(repo.path(), &["commit", "--quiet", "-m", "plan"]);

    record(repo.path(), "1.json", "2030-01-01T00:00:00Z");
    record(repo.path(), "2.json", "2026-09-01T00:00:00Z");
    // Uncommitted, the two runs have no attested order at all.
    let (status, uncommitted, _) = verdict(repo.path());
    assert_eq!(status, Some(1));
    assert_eq!(uncommitted["counts"]["orderUnattested"], 2);
    assert!(
        uncommitted["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("order_unattested"))
    );

    let measurements = "spec/evidence/measurements";
    git(
        repo.path(),
        &[
            "add",
            &format!("{measurements}/verify-rerun-regressed.json"),
        ],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "first run"]);
    git(
        repo.path(),
        &["add", &format!("{measurements}/verify-rerun-pass.json")],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "second run"]);

    let (status, committed, _) = verdict(repo.path());
    assert_eq!(status, Some(1));
    assert_eq!(committed["orderSource"], "git-first-parent-add");
    assert_eq!(committed["candidate"], "verify-rerun-pass");
    assert_eq!(committed["verdict"], "reject");
    // The regressed run states a later time than the pass it was committed
    // before: the two orders disagree, so the candidate's place is not
    // attested either.
    assert_eq!(
        committed["reasons"],
        json!(["order_unattested", "rerun_until_pass"])
    );
    assert_eq!(
        committed["regressedRuns"],
        json!(["verify-rerun-regressed"])
    );
    assert_eq!(committed["counts"]["orderAttested"], 2);

    // A shallow clone has lost the commits that first added the files.
    let clone = tempfile::tempdir().unwrap();
    let shallow = clone.path().join("shallow");
    let source = format!("file://{}", repo.path().display());
    let status = Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["clone", "--quiet", "--depth", "1", &source])
        .arg(&shallow)
        .status()
        .unwrap();
    assert!(status.success());
    let (status, cloned, stderr) = verdict(&shallow);
    assert_eq!(status, Some(1));
    assert_eq!(cloned["orderSource"], "git-shallow");
    assert_eq!(cloned["counts"]["orderAttested"], 0);
    assert!(
        cloned["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("order_unattested"))
    );
    assert!(stderr.contains("shallow clone"), "{stderr}");
}
