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
        "signed_at": "2026-08-31T10:20:30Z",
        "signed_at_source": "tagger_date",
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
    assert!(
        human.contains("trust comes from the relying party, never from the claim's trust, the record or a revision it names"),
        "{human}"
    );
    assert!(
        human.contains("signed_approval.required is false is not relied on"),
        "{human}"
    );
    assert!(human.contains("reviewer@example.test"), "{human}");
}

/// The receipt help names a non-null claim as a claim not verified by Quoin,
/// names `approval-verify` for re-verification with trust the relying party
/// supplies and not taken from the claim, the record or a revision the record
/// names, and documents a claim under an optional policy as not relied on; the
/// write-record and record-digest pages describe the digest.
#[trace("FR-068-AC-13")]
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
        "the receipt's `record_id`",
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

/// No command executes the attested command, invokes Git or performs a network
/// request, and the change-assurance help pages and description make no
/// authorization, non-repudiation, authenticity or certification claim of
/// their own: each such word sits within two lines of a denial.
#[trace("FR-068-AC-11")]
#[test]
fn tc_1511_the_surface_runs_nothing_and_claims_nothing() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/change_assurance.rs"),
    )
    .expect("the surface source");
    for forbidden in [
        "std::process",
        "TcpStream",
        "reqwest",
        "UdpSocket",
        "\"git\"",
    ] {
        assert!(
            !source.contains(forbidden),
            "the surface reaches for `{forbidden}`"
        );
    }

    let capture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/retained-command-help.json"),
        )
        .expect("the help capture"),
    )
    .expect("JSON");
    let mut pages: Vec<String> = capture["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .filter(|case| case["argv"][0] == "change-assurance")
        .map(|case| case["stdout"].as_str().expect("stdout").to_owned())
        .collect();
    assert!(pages.len() >= 8, "every change-assurance page is captured");
    let output = quoin(&[]);
    pages.push(text(&output.stdout));
    let claims = [
        "authorization",
        "authorize",
        "non-repudiation",
        "authenticity",
        "certification",
        "certified",
    ];
    let denials = ["not", "no", "nothing", "never", "nor", "without"];
    for page in &pages {
        let lines: Vec<&str> = page.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let lowered = line.to_lowercase();
            if !claims.iter().any(|claim| lowered.contains(claim)) {
                continue;
            }
            let window = lines[index.saturating_sub(2)..lines.len().min(index + 2)]
                .join(" ")
                .to_lowercase();
            assert!(
                window
                    .split(|c: char| !c.is_ascii_alphabetic() && c != '-')
                    .any(|word| denials.contains(&word)),
                "a claim word is used without a denial: {line}"
            );
        }
    }
}

/// Run `receipt` over the fixture record with `claim` merged into an approving
/// event, and return the plain output.
fn plain_receipt_with(claim_time: &serde_json::Value) -> String {
    let root = tempfile::tempdir().expect("a temporary directory");
    let repo = root.path().to_str().expect("UTF-8");
    let digest = written(repo);
    let mut history: serde_json::Value = serde_json::from_slice(
        &std::fs::read(fixtures().join("decisions.json")).expect("decisions fixture"),
    )
    .expect("JSON");
    let mut claim = serde_json::json!({
        "protocol": "engineering-assurance.approval-verify-result/v1",
        "status": "approved",
        "tag": "approval/change-1/1",
        "record_id": "change-1",
        "record_digest": digest,
        "trust": {"allowed_signers": "/keys/allowed_signers"},
        "approver": "reviewer@example.test",
        "meaning": ["reviewed", "tested"],
        "signature_kind": "ssh",
    });
    for (name, value) in claim_time.as_object().expect("time members") {
        claim[name] = value.clone();
    }
    history["events"][0]["payload"]["verified_approval"] = claim;
    let decisions = root.path().join("timed-decisions.json");
    std::fs::write(
        &decisions,
        serde_json::to_vec(&history).expect("serialises"),
    )
    .expect("written");
    let output = quoin(&[
        "receipt",
        "--repo",
        repo,
        "--record",
        "change-1",
        "--revision",
        "1",
        "--candidate-revision",
        "candidate-1",
        "--decisions",
        decisions.to_str().expect("UTF-8"),
    ]);
    text(&output.stdout)
}

/// The plain receipt shows the approver, the time as the signer's claim with
/// its source, and the meaning together; a claim with no time shows `time
/// missing` and its reason, and no other instant stands in for it.
#[trace("FR-068-AC-14", "FR-065-AC-22")]
#[test]
fn tc_1592_the_receipt_shows_person_time_and_meaning_together() {
    let timed = plain_receipt_with(&serde_json::json!({
        "signed_at": "2026-08-31T10:20:30Z",
        "signed_at_source": "tagger_date",
    }));
    assert!(timed.contains("approver: reviewer@example.test"), "{timed}");
    assert!(
        timed.contains(
            "signing time claimed by the signer: 2026-08-31T10:20:30Z (source: tagger_date)"
        ),
        "{timed}"
    );
    assert!(timed.contains("meaning: reviewed; tested"), "{timed}");
    assert!(!timed.contains("time missing"), "{timed}");

    let missing = plain_receipt_with(&serde_json::json!({
        "signed_at_missing": "tagger_date_unreadable",
    }));
    assert!(
        missing
            .contains("signing time claimed by the signer: time missing (tagger_date_unreadable)"),
        "{missing}"
    );
    assert!(
        missing.contains("approver: reviewer@example.test"),
        "{missing}"
    );
    assert!(missing.contains("meaning: reviewed; tested"), "{missing}");
    assert!(!missing.contains("2026-"), "{missing}");
}

/// The help names the time as the signer's own claim, not attested and never
/// replaced by another time.
#[trace("FR-068-AC-14")]
#[test]
fn tc_1592_the_receipt_help_describes_the_claimed_time() {
    let output = quoin(&["receipt", "--help"]);
    let help = text(&output.stdout).replace("\n  ", " ");
    for literal in [
        "signed_at_source",
        "signed_at_missing",
        "the signer's own claim",
        "time missing",
        "never replaced by another time",
    ] {
        assert!(
            help.contains(literal),
            "receipt help must contain {literal}: {help}"
        );
    }
}
