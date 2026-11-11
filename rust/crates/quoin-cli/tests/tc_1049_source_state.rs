// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin measurement record` does not store a self-attested source state it
//! could not check: a fabricated revision or a `clean` claim on a dirty tree
//! is refused, and outside git the claim is stored as `unverified`
//! (PLAT-1049).
//!
//! Provenance: PLAT-1049

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

const ZEROS: &str = "0000000000000000000000000000000000000000";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-measurement/tests/fixtures/verify")
}

fn git(repo: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
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
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn with_plan(repo: &Path) {
    let assurance = repo.join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::copy(
        fixtures().join("plans/MP-961-gate.md"),
        assurance.join("MP-961-gate.md"),
    )
    .unwrap();
}

fn init_git(repo: &Path) -> String {
    git(repo, &["init", "--quiet"]);
    git(repo, &["add", "spec"]);
    git(repo, &["commit", "--quiet", "-m", "plan"]);
    git(repo, &["rev-parse", "HEAD"])
}

fn record(repo: &Path, revision: &str, state: &str) -> Output {
    let mut collection: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("wrong/selectively-reported-run/1.json")).unwrap(),
    )
    .unwrap();
    let stack = collection["verificationStack"].as_object_mut().unwrap();
    stack.remove("protectedApparatus");
    stack.insert(
        "sources".to_owned(),
        json!({ "subject": { "revision": revision, "sourceState": state } }),
    );
    // Outside the repo: an untracked input file would itself make it dirty.
    let scratch = tempfile::tempdir().unwrap();
    let input = scratch.path().join("input.json");
    std::fs::write(&input, collection.to_string()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["measurement", "record", "--input", input.to_str().unwrap()])
        .args(["--repo", repo.to_str().unwrap()])
        .output()
        .unwrap()
}

fn stored_state(repo: &Path) -> String {
    let store = repo.join("spec/evidence/measurements");
    let file = std::fs::read_dir(store)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let stored: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    stored["verificationStack"]["sources"]["subject"]["sourceState"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Outside git nothing can be checked: the claim is kept out of the store as
/// `unverified`, never as `clean`, and the record is not refused.
#[trace("FR-044", "PLAT-1049")]
#[test]
fn tc_1049_001_outside_git_a_clean_claim_is_stored_unverified() {
    let repo = tempfile::tempdir().unwrap();
    with_plan(repo.path());
    let output = record(repo.path(), ZEROS, "clean");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stored_state(repo.path()), "unverified");
}

/// A revision that is not a commit in `--repo` may belong to another
/// repository: the record is accepted and the claim stored as `unverified`.
#[trace("FR-044", "PLAT-1049")]
#[test]
fn tc_1049_002_an_unresolvable_revision_in_git_is_stored_unverified() {
    let repo = tempfile::tempdir().unwrap();
    with_plan(repo.path());
    init_git(repo.path());
    let output = record(repo.path(), ZEROS, "clean");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stored_state(repo.path()), "unverified");
}

#[trace("FR-044", "PLAT-1049")]
#[test]
fn tc_1049_003_a_clean_claim_on_a_dirty_tree_is_refused() {
    let repo = tempfile::tempdir().unwrap();
    with_plan(repo.path());
    let head = init_git(repo.path());
    std::fs::write(
        repo.path().join("spec/assurance/MP-961-gate.md"),
        "edited\n",
    )
    .unwrap();
    let output = record(repo.path(), &head, "clean");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("sources.subject.sourceState"),
        "{}",
        stderr(&output)
    );
}

#[trace("FR-044", "PLAT-1049")]
#[test]
fn tc_1049_004_a_dirty_claim_on_a_dirty_tree_and_a_clean_claim_on_a_clean_tree_are_kept() {
    let repo = tempfile::tempdir().unwrap();
    with_plan(repo.path());
    let head = init_git(repo.path());
    let output = record(repo.path(), &head, "clean");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stored_state(repo.path()), "clean");

    let other = tempfile::tempdir().unwrap();
    with_plan(other.path());
    let head = init_git(other.path());
    std::fs::write(other.path().join("untracked.txt"), "x\n").unwrap();
    let output = record(other.path(), &head, "dirty");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stored_state(other.path()), "dirty");
}
