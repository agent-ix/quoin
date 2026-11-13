// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `quoin change-assurance record-digest` and the claimed approval a receipt
//! reports (FR-068), against the real binary.
//!
//! A claimed approval is the event writer's claim; Quoin never verifies it.
//! Every fixture is hand-built and fictional.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/change-assurance-cli")
        .canonicalize()
        .expect("the change-assurance CLI fixtures are present")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

fn quoin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("change-assurance")
        .args(args)
        .output()
        .expect("the native quoin binary runs")
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("the command emitted JSON")
}

fn record_path() -> String {
    fixtures()
        .join("record-body.json")
        .to_str()
        .expect("UTF-8")
        .to_owned()
}

/// Write the fixture record into a fresh repository and return its digest.
fn written(repo: &str) -> String {
    let output = quoin(&[
        "write-record",
        "--input",
        &record_path(),
        "--repo",
        repo,
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    json(&output)["record_digest"]
        .as_str()
        .expect("write-record reports the record digest")
        .to_owned()
}

fn stored_files(root: &Path) -> Vec<PathBuf> {
    fn walk(at: &Path, into: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(at).into_iter().flatten().flatten() {
            if entry.path().is_dir() {
                walk(&entry.path(), into);
            } else {
                into.push(entry.path());
            }
        }
    }
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    files
}

/// `write-record` and `record-digest` report the same digest for a stored
/// revision; `record-digest` of an unstored revision exits 2 and writes
/// nothing, and a successful call writes nothing either.
#[trace("FR-068-AC-12")]
#[test]
fn tc_1511_write_record_and_record_digest_report_one_digest() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let repo = root.path().to_str().expect("UTF-8");
    let digest = written(repo);
    assert!(
        digest.starts_with("sha256:")
            && digest.len() == 71
            && digest[7..]
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "{digest}"
    );

    let before = stored_files(root.path());
    let reported = quoin(&[
        "record-digest",
        "--record",
        "change-1",
        "--revision",
        "1",
        "--repo",
        repo,
        "--json",
    ]);
    assert_eq!(
        reported.status.code(),
        Some(0),
        "{}",
        text(&reported.stderr)
    );
    assert_eq!(json(&reported)["record_digest"], digest);
    let plain = quoin(&[
        "record-digest",
        "--record",
        "change-1",
        "--revision",
        "1",
        "--repo",
        repo,
    ]);
    assert_eq!(text(&plain.stdout).trim_end(), digest);

    let unstored = quoin(&[
        "record-digest",
        "--record",
        "change-1",
        "--revision",
        "2",
        "--repo",
        repo,
    ]);
    assert_eq!(
        unstored.status.code(),
        Some(2),
        "{}",
        text(&unstored.stderr)
    );
    assert!(unstored.stdout.is_empty());
    assert_eq!(stored_files(root.path()), before);
}

/// A receipt whose event carries a claim bound to the stored digest reports it
/// in its JSON, and its plain output labels it as the event writer's claim that
/// Quoin has not verified, naming `approval-verify` as the way to re-verify.
#[trace("FR-068-AC-13", "FR-065-AC-12")]
#[test]
fn tc_1511_a_receipt_labels_the_claimed_approval_as_an_unverified_claim() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let repo = root.path().to_str().expect("UTF-8");
    let digest = written(repo);

    let mut history: serde_json::Value = serde_json::from_slice(
        &std::fs::read(fixtures().join("decisions.json")).expect("decisions fixture"),
    )
    .expect("JSON");
    history["events"][0]["payload"]["verified_approval"] = serde_json::json!({
        "protocol": "engineering-assurance.approval-verify-result/v1",
        "status": "approved",
        "tag": "approval/change-1/1",
        "record_id": "change-1",
        "record_digest": digest,
        "trust": {"allowed_signers": "/keys/allowed_signers"},
        "approver": "reviewer@example.test",
        "meaning": ["reviewed"],
        "signature_kind": "ssh",
    });
    let decisions = root.path().join("claiming-decisions.json");
    std::fs::write(
        &decisions,
        serde_json::to_vec(&history).expect("serialises"),
    )
    .expect("written");
    let receipt_argv = |json: bool| {
        let mut argv = vec![
            "receipt".to_owned(),
            "--repo".to_owned(),
            repo.to_owned(),
            "--record".to_owned(),
            "change-1".to_owned(),
            "--revision".to_owned(),
            "1".to_owned(),
            "--candidate-revision".to_owned(),
            "candidate-1".to_owned(),
            "--decisions".to_owned(),
            decisions.to_str().expect("UTF-8").to_owned(),
        ];
        if json {
            argv.push("--json".to_owned());
        }
        argv
    };
    let run = |json: bool| {
        let argv = receipt_argv(json);
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        quoin(&refs)
    };

    let machine = run(true);
    let receipt = json(&machine);
    let claim = &receipt["decision_event"]["claimed_approval"];
    assert_eq!(claim["approver"], "reviewer@example.test", "{receipt}");
    assert_eq!(claim["record_digest"], digest);

    let human = text(&run(false).stdout);
    assert!(
        human.contains("claimed approval, the event writer's claim and not verified by Quoin"),
        "{human}"
    );
    assert!(human.contains("approval-verify"), "{human}");
    assert!(human.contains("reviewer@example.test"), "{human}");
}

/// The receipt help names a non-null claim as a claim not verified by Quoin,
/// names `approval-verify` for re-verification with trust the relying party
/// supplies and not taken from the claim, the record or a revision the record
/// names, and documents a claim under an optional policy as not relied on; the
/// write-record and record-digest pages describe the digest.
#[trace("FR-068-AC-13", "FR-068-AC-11")]
#[test]
fn tc_1511_the_help_pages_describe_the_claim_and_the_digest() {
    let help = |argv: &[&str]| {
        let mut full = argv.to_vec();
        full.push("--help");
        let output = quoin(&full);
        assert_eq!(output.status.code(), Some(0));
        text(&output.stdout)
    };
    let receipt = help(&["receipt"]).replace("\n  ", " ");
    for literal in [
        "claimed_approval",
        "has not been verified by Quoin",
        "approval-verify",
        "trust you supply",
        "Take no trust from the claim's `trust`, from the record, or from any revision the record names",
        "never rely on a claim under a record whose `signed_approval.required` is false",
    ] {
        assert!(
            receipt.contains(literal),
            "receipt help must contain {literal}: {receipt}"
        );
    }
    let digest = help(&["record-digest"]);
    assert!(digest.contains("Approved-Record-Digest"), "{digest}");
    assert!(digest.contains("writes nothing"), "{digest}");
    let written = help(&["write-record"]);
    assert!(written.contains("record digest"), "{written}");
    let listing = help(&[]);
    assert!(listing.contains("record-digest"), "{listing}");
}
