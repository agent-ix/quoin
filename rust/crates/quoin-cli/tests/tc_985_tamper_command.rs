// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin measurement verify` catches four kinds of tamper that its own
//! read of the current store, alone, cannot see: a protected-apparatus
//! digest forged by hand rather than resolved by intake, a protected
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

fn git_output(repo: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
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
    // The fixture's hand-pasted `artifacts` digest for the plan document is
    // stale once a test has edited the plan file: intake's own truth-check
    // (`declared`/PLAT-931) refuses a stated digest that disagrees with the
    // real file, so this recomputes it the way `--digest-from-file` would.
    let plan_bytes = std::fs::read(repo.join("spec/assurance/MP-961-gate.md")).unwrap();
    let plan_digest = quoin_store::digest_bytes_sha256(&plan_bytes).to_stored();
    stack["artifacts"]["spec/assurance/MP-961-gate.md"] = json!(plan_digest);
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

/// A hand-written collection — never passed through `quoin measurement
/// record`, which always resolves `protectedApparatus` itself from the real
/// file — can state any digest it likes for the plan document it protects.
/// `quoin measurement verify` catches the forgery by checking the recorded
/// digest against `git show <sourceRevision>:<path>`, the file's real bytes
/// at the commit the collection claims to be from.
///
/// Trace: FR-108-AC-9
/// Provenance: PLAT-985
#[test]
fn tc_985_101_a_forged_protected_apparatus_digest_is_rejected() {
    let repo = tempfile::tempdir().unwrap();
    init_with_plan(repo.path(), "MP-961-gate.md");
    let plan_commit = git_output(repo.path(), &["rev-parse", "HEAD"]);

    // A collection nobody's intake ever wrote: hand-assembled from the
    // fixture accepted collection, claiming the plan commit as its source
    // revision and a digest for the protected file that is not its real
    // sha256.
    let mut collection: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("right/accept/1.json")).unwrap(),
    )
    .unwrap();
    collection["collectionId"] = json!("tamper-forged");
    collection["sourceRevision"] = json!(plan_commit);
    let wrong_digest = format!("sha256:{}", "f".repeat(64));
    collection["verificationStack"]["protectedApparatus"]["MP-961"]["spec/assurance/MP-961-gate.md"] =
        json!(wrong_digest);
    collection["verificationStack"]["artifacts"]["spec/assurance/MP-961-gate.md"] =
        json!(wrong_digest);

    let measurements = repo.path().join(MEASUREMENTS);
    std::fs::create_dir_all(&measurements).unwrap();
    std::fs::write(
        measurements.join("tamper-forged.json"),
        collection.to_string(),
    )
    .unwrap();
    git(
        repo.path(),
        &["add", &format!("{MEASUREMENTS}/tamper-forged.json")],
    );
    git(
        repo.path(),
        &["commit", "--quiet", "-m", "hand-written collection"],
    );

    let (status, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    assert_eq!(payload["verdict"], "reject");
    assert!(
        reasons(&payload).contains(&"apparatus_forged".to_owned()),
        "{payload}"
    );
}

/// A protected apparatus genuinely edited between two real runs — recorded
/// honestly by `quoin measurement record` each time, from the disk as it
/// stood — is `apparatus_edit`: FR-110's own mechanism (PLAT-975), exercised
/// here through the real intake-then-verify CLI path rather than a unit
/// test's `Ranked` construction.
///
/// Trace: FR-110-AC-7
/// Provenance: PLAT-975, PLAT-985
#[test]
fn tc_985_102_a_real_apparatus_edit_between_two_runs_is_rejected_end_to_end() {
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

    // The plan document is its own protected apparatus; editing it changes
    // the digest the next `record` resolves, without bumping
    // `definition_version`.
    let plan_path = repo.path().join("spec/assurance/MP-961-gate.md");
    let plan_text = std::fs::read_to_string(&plan_path).unwrap();
    std::fs::write(
        &plan_path,
        format!("{plan_text}\nEdited after the first run.\n"),
    )
    .unwrap();
    git(repo.path(), &["add", "spec"]);
    git(
        repo.path(),
        &["commit", "--quiet", "-m", "edit the answer key"],
    );

    record(repo.path(), "2.json", "2026-09-02T00:00:00Z");
    git(
        repo.path(),
        &["add", &format!("{MEASUREMENTS}/verify-rerun-pass.json")],
    );
    git(repo.path(), &["commit", "--quiet", "-m", "second run"]);

    let (status, payload, _) = verdict(repo.path(), "MP-961");
    assert_eq!(status, Some(1));
    assert_eq!(payload["verdict"], "reject");
    assert!(
        reasons(&payload).contains(&"apparatus_edit".to_owned()),
        "{payload}"
    );
}

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
