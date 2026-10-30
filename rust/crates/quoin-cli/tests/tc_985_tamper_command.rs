// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin measurement verify` catches three kinds of tamper that its own
//! read of the current store, alone, cannot see: a protected
//! apparatus genuinely edited between two real runs, a collection deleted
//! after being counted, and a collection's stored file edited after the
//! commit that added it (PLAT-985).
//!
//! Every test here drives the real `quoin` binary and real `git`, the same
//! way `tc_961_verify_command.rs` does — this crate's whole reason for
//! existing is that only it has git, so the checker itself
//! (`quoin_measurement::verify`) cannot be exercised end to end without it.
//!
//! Provenance: PLAT-985

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

fn init_with_plan(repo: &Path, plan_file: &str) {
    let assurance = repo.join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::copy(
        fixtures().join("plans").join(plan_file),
        assurance.join(plan_file),
    )
    .unwrap();
    git(repo, &["init", "--quiet"]);
    git(repo, &["add", "spec"]);
    git(repo, &["commit", "--quiet", "-m", "plan"]);
}

/// Record the fixture collection `name` into `repo`, with its stated
/// timestamp replaced. Mirrors `tc_961_verify_command.rs`'s helper of the
/// same job.
fn record(repo: &Path, name: &str, timestamp: &str) {
    let mut collection: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("wrong/selectively-reported-run").join(name))
            .unwrap(),
    )
    .unwrap();
    collection["timestamp"] = json!(timestamp);
    let stack = collection["verificationStack"].as_object_mut().unwrap();
    stack.remove("protectedApparatus");
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

fn verdict(repo: &Path, plan: &str) -> (Option<i32>, Value, String) {
    let output = quoin(repo, &["measurement", "verify", "--plan", plan]);
    let payload = serde_json::from_slice(&output.stdout).unwrap();
    (
        output.status.code(),
        payload,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn reasons(verdict: &Value) -> Vec<String> {
    verdict["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reason| reason.as_str().unwrap().to_owned())
        .collect()
}

const MEASUREMENTS: &str = "spec/evidence/measurements";

/// A collection once counted and later removed from the store is not simply
/// gone: `quoin measurement verify` reads the store's git history and
/// reports a `collection_deleted` finding, so deleting a regressed run
/// cannot defeat "counts every collection".
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_103_a_collection_deleted_after_being_counted_is_rejected() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");

    record(repo.path(), "1.json", "2026-09-01T00:00:00Z");
    git(
        repo.path(),
        &[
            "add",
            &format!("{MEASUREMENTS}/verify-rerun-regressed.json"),
        ],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "first run"]);
    record(repo.path(), "2.json", "2026-09-02T00:00:00Z");
    git(
        repo.path(),
        &["add", &format!("{MEASUREMENTS}/verify-rerun-pass.json")],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "second run"]);

    let (_, before, _) = verdict(repo.path(), "MP-961");
    assert_eq!(before["counts"]["collectionsConsidered"], 2);

    std::fs::remove_file(
        repo.path()
            .join(MEASUREMENTS)
            .join("verify-rerun-regressed.json"),
    )
    .unwrap();
    git(
        repo.path(),
        &[
            "rm",
            "--quiet",
            &format!("{MEASUREMENTS}/verify-rerun-regressed.json"),
        ],
    );
    git(
        repo.path(),
        &["commit", "--quiet", "-m", "delete the regressed run"],
    );

    let (status, after, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    assert_eq!(after["verdict"], "reject");
    assert_eq!(after["counts"]["collectionsConsidered"], 1);
    assert!(
        reasons(&after).contains(&"collection_deleted".to_owned()),
        "{after}"
    );
}

/// A collection's stored file, edited by a commit after the one that added
/// it, is `collection_edited` — the store's git history shows a hand-made
/// change intake never made.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_104_a_collection_edited_after_intake_is_rejected() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");

    record(repo.path(), "2.json", "2026-09-01T00:00:00Z");
    git(
        repo.path(),
        &["add", &format!("{MEASUREMENTS}/verify-rerun-pass.json")],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "record the pass"]);

    let stored = repo
        .path()
        .join(MEASUREMENTS)
        .join("verify-rerun-pass.json");
    let mut collection: Value =
        serde_json::from_str(&std::fs::read_to_string(&stored).unwrap()).unwrap();
    collection["environment"]["runner"] = json!("edited-after-the-fact");
    std::fs::write(&stored, collection.to_string()).unwrap();
    git(
        repo.path(),
        &["add", &format!("{MEASUREMENTS}/verify-rerun-pass.json")],
    );
    git(
        repo.path(),
        &["commit", "--quiet", "-m", "hand edit the stored record"],
    );

    let (status, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    assert_eq!(payload["verdict"], "reject");
    assert!(
        reasons(&payload).contains(&"collection_edited".to_owned()),
        "{payload}"
    );
}

/// The plan document's own objective, estimator, decision rule or protected
/// apparatus changing between two committed revisions that share a
/// `definition_version` is `definition_changed_without_version_bump`
/// (engineering-assurance FR-021's `definition_change_without_version_bump`,
/// wired here with access to the plan's git history).
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_105_a_decision_rule_edited_without_a_version_bump_is_rejected() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    record(repo.path(), "2.json", "2026-09-01T00:00:00Z");
    git(
        repo.path(),
        &["add", &format!("{MEASUREMENTS}/verify-rerun-pass.json")],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "record the pass"]);

    // Loosen the gate's threshold without touching `definition_version`.
    let plan_path = repo.path().join("spec/assurance/MP-961-gate.md");
    let plan_text = std::fs::read_to_string(&plan_path).unwrap();
    let edited = plan_text.replace("threshold: 0.8", "threshold: 0.1");
    assert_ne!(edited, plan_text, "the fixture still states threshold: 0.8");
    std::fs::write(&plan_path, edited).unwrap();
    git(repo.path(), &["add", "spec"]);
    git(
        repo.path(),
        &[
            "commit",
            "--quiet",
            "-m",
            "loosen the gate without a version bump",
        ],
    );

    let (status, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    assert_eq!(payload["verdict"], "reject");
    assert!(
        reasons(&payload).contains(&"definition_changed_without_version_bump".to_owned()),
        "{payload}"
    );
}

fn commit_all(repo: &Path, message: &str) {
    git(repo, &["add", "-A", "spec"]);
    git(repo, &["commit", "--quiet", "-m", message]);
}

fn stored(repo: &Path, id: &str) -> PathBuf {
    repo.join(MEASUREMENTS).join(format!("{id}.json"))
}

fn rewrite(repo: &Path, id: &str, edit: impl FnOnce(&mut Value)) {
    let path = stored(repo, id);
    let mut collection: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(&mut collection);
    std::fs::write(&path, collection.to_string()).unwrap();
}

/// Deleting a collection and re-adding a different file under the same id,
/// in two commits, is `collection_edited` — `--no-renames` alone would read
/// the re-add as a fresh intake.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_107_a_delete_and_re_add_under_the_same_id_is_edited() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    record(repo.path(), "2.json", "2026-09-01T00:00:00Z");
    commit_all(repo.path(), "record the pass");
    let original = std::fs::read_to_string(stored(repo.path(), "verify-rerun-pass")).unwrap();

    git(
        repo.path(),
        &[
            "rm",
            "--quiet",
            &format!("{MEASUREMENTS}/verify-rerun-pass.json"),
        ],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "remove it"]);
    std::fs::create_dir_all(repo.path().join(MEASUREMENTS)).unwrap();
    std::fs::write(stored(repo.path(), "verify-rerun-pass"), original).unwrap();
    rewrite(repo.path(), "verify-rerun-pass", |collection| {
        collection["environment"]["runner"] = json!("re-added");
    });
    commit_all(repo.path(), "re-add it, changed");

    let (status, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    let reasons = reasons(&payload);
    assert!(
        reasons.contains(&"collection_edited".to_owned()),
        "{payload}"
    );
    assert!(
        !reasons.contains(&"collection_deleted".to_owned()),
        "{payload}"
    );
}

/// An edit that re-targets a run's observations away from the plan removes
/// it from the plan's runs; it is still `collection_edited` for the plan the
/// run was first added under.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_108_an_edit_moving_a_run_off_the_plan_is_still_edited() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    record(repo.path(), "1.json", "2026-09-01T00:00:00Z");
    commit_all(repo.path(), "record the regressed run");

    rewrite(repo.path(), "verify-rerun-regressed", |collection| {
        for observation in collection["observations"].as_array_mut().unwrap() {
            observation["definitionVersion"] = json!("some-other-version");
        }
    });
    commit_all(repo.path(), "move the run off the plan");

    let (_, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(payload["verdict"], "reject", "{payload}");
    assert!(
        reasons(&payload).contains(&"collection_edited".to_owned()),
        "{payload}"
    );
}

/// Corrupting a collection's file before deleting it still attributes the
/// deletion to the plan intake first added it under.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_109_a_collection_corrupted_then_deleted_is_still_attributed() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    record(repo.path(), "1.json", "2026-09-01T00:00:00Z");
    commit_all(repo.path(), "first run");
    record(repo.path(), "2.json", "2026-09-02T00:00:00Z");
    commit_all(repo.path(), "second run");

    std::fs::write(stored(repo.path(), "verify-rerun-regressed"), "{}").unwrap();
    commit_all(repo.path(), "corrupt the regressed run");
    std::fs::remove_file(stored(repo.path(), "verify-rerun-regressed")).unwrap();
    commit_all(repo.path(), "delete it");

    let (status, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    assert!(
        reasons(&payload).contains(&"collection_deleted".to_owned()),
        "{payload}"
    );
}

/// Renaming the plan document in the same commit that changes its decision
/// rule, without a `definition_version` bump, is still
/// `definition_changed_without_version_bump`: each revision is read at the
/// path it had in its own commit.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_110_a_rename_does_not_hide_the_revisions_before_it() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    git(
        repo.path(),
        &[
            "mv",
            "spec/assurance/MP-961-gate.md",
            "spec/assurance/MP-961-renamed.md",
        ],
    );
    let path = repo.path().join("spec/assurance/MP-961-renamed.md");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replace("threshold: 0.8", "threshold: 0.1")
            .replace("MP-961-gate.md", "MP-961-renamed.md"),
    )
    .unwrap();
    commit_all(repo.path(), "rename and loosen the gate");

    let (_, payload, _) = verdict(repo.path(), "MP-961");
    assert!(
        reasons(&payload).contains(&"definition_changed_without_version_bump".to_owned()),
        "{payload}"
    );
}

/// `measurement.verify` reads the work tree, so an uncommitted edit to a
/// stored collection or to the plan's decision rule is caught as surely as a
/// committed one.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_111_uncommitted_work_tree_tampering_is_caught() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    record(repo.path(), "2.json", "2026-09-01T00:00:00Z");
    commit_all(repo.path(), "record the pass");

    rewrite(repo.path(), "verify-rerun-pass", |collection| {
        collection["environment"]["runner"] = json!("edited, not committed");
    });
    let plan_path = repo.path().join("spec/assurance/MP-961-gate.md");
    let plan_text = std::fs::read_to_string(&plan_path).unwrap();
    std::fs::write(
        &plan_path,
        plan_text.replace("threshold: 0.8", "threshold: 0.1"),
    )
    .unwrap();

    let (status, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    let reasons = reasons(&payload);
    assert!(
        reasons.contains(&"collection_edited".to_owned()),
        "{payload}"
    );
    assert!(
        reasons.contains(&"definition_changed_without_version_bump".to_owned()),
        "{payload}"
    );
}

/// No false positive: an honestly recorded, committed run, a new
/// uncommitted intake, and a committed plan edit that does bump
/// `definition_version` raise none of the four tamper reasons.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_112_honest_history_raises_no_tamper_reason() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    record(repo.path(), "1.json", "2026-09-01T00:00:00Z");
    commit_all(repo.path(), "first run");
    record(repo.path(), "2.json", "2026-09-02T00:00:00Z");
    let plan_path = repo.path().join("spec/assurance/MP-961-gate.md");
    let plan_text = std::fs::read_to_string(&plan_path).unwrap();
    std::fs::write(
        &plan_path,
        plan_text
            .replace("threshold: 0.8", "threshold: 0.7")
            .replace("gate.pass-rate-v1", "gate.pass-rate-v2"),
    )
    .unwrap();
    git(repo.path(), &["add", "spec/assurance"]);
    git(
        repo.path(),
        &["commit", "--quiet", "-m", "a genuine version bump"],
    );

    let (_, payload, stderr) = verdict(repo.path(), "MP-961");
    let reasons = reasons(&payload);
    for tamper in [
        "collection_deleted",
        "collection_edited",
        "definition_changed_without_version_bump",
    ] {
        assert!(
            !reasons.contains(&tamper.to_owned()),
            "{tamper}: {payload} {stderr}"
        );
    }
}
