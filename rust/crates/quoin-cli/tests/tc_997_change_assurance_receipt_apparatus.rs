// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-111's apparatus judgment, reached through `quoin change-assurance
//! receipt` itself — the real binary, not just the library or the in-process
//! runtime (PLAT-997).
//!
//! Before this ticket, `change_assurance.receipt`'s wire request carried no
//! diff and no plan link, so none of FR-111's reasons could ever fire through
//! this command (the known gap FR-111-CON-3 recorded). This file drives the
//! whole producer path — `seal-record`, `seal-attestation`, `intake`, then
//! `receipt` with `--diff-path` and `--plan` — against a real repository on
//! disk, exactly as a caller would.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The retained fixture bodies `tc_1322_change_assurance_exit_grammar.rs`
/// also reads.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/change-assurance-cli")
        .canonicalize()
        .expect("the retained change-assurance CLI fixtures are present")
}

fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name)).expect("the fixture is present")
}

/// A repository root that exists only for one test.
fn repo() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

/// Run one `quoin change-assurance <args>` invocation, capturing its output.
fn quoin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("change-assurance")
        .args(args)
        .output()
        .expect("the native quoin binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
    path
}

/// A minimal `MeasurementPlan` document protecting `entries`, written under
/// `root`'s `spec/assurance/` — the root `quoin-measurement`'s plan intake
/// walks (PLAT-975, PLAT-997).
fn write_protecting_plan(root: &Path, id: &str, entries: &[&str]) {
    let mut list = String::new();
    for entry in entries {
        let _ = writeln!(list, "  - {entry}");
    }
    let document = format!(
        "---\nid: {id}\ntitle: PLAT-997 CLI fixture\ntype: MeasurementPlan\nstatus: active\n\
         stage: observe\nmetric: change_assurance.apparatus_touched\n\
         definition_version: v1\nprotected_apparatus:\n{list}---\n\n# PLAT-997 CLI fixture\n"
    );
    write(
        root,
        &format!("spec/assurance/{id}.md"),
        document.as_bytes(),
    );
}

/// Seal the fixture record, retain it, and answer its digest.
fn seal_record(root: &Path) -> String {
    let record_path = write(root, "record-body.json", &read_fixture("record-body.json"));
    let output = quoin(&[
        "seal-record",
        "--input",
        record_path.to_str().unwrap(),
        "--repo",
        root.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "seal-record: stderr {}",
        text(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("--json emits one document");
    payload["digest"].as_str().expect("a digest").to_owned()
}

/// Seal and retain the fixture attestation and a fresh output file, and
/// answer the sealed attestation's digest.
///
/// `attestation-body.json`'s own `record_digest` is a fixed value computed
/// over `record-body.json`'s fixed content; `seal_record` above must answer
/// the same value or the fixtures have drifted apart.
fn intake_attestation(root: &Path) -> String {
    let attestation_body = write(
        root,
        "attestation-body.json",
        &read_fixture("attestation-body.json"),
    );
    let output_bytes = write(root, "output.bin", b"passed\n");
    let sealed = quoin(&[
        "seal-attestation",
        "--input",
        attestation_body.to_str().unwrap(),
        "--output",
        output_bytes.to_str().unwrap(),
        "--media-type",
        "text/plain",
    ]);
    assert_eq!(
        sealed.status.code(),
        Some(0),
        "seal-attestation: stderr {}",
        text(&sealed.stderr)
    );
    let sealed_path = write(root, "sealed-attestation.json", &sealed.stdout);
    let payload: serde_json::Value =
        serde_json::from_str(text(&sealed.stdout).trim()).expect("seal-attestation emits JSON");
    let digest = payload["digest"].as_str().expect("a digest").to_owned();

    let intake = quoin(&[
        "intake",
        "--attestation",
        sealed_path.to_str().unwrap(),
        "--output",
        output_bytes.to_str().unwrap(),
        "--repo",
        root.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        intake.status.code(),
        Some(0),
        "intake: stderr {}",
        text(&intake.stderr)
    );
    digest
}

/// `quoin change-assurance receipt` with `--diff-path` and `--plan` reports
/// `apparatus_touched` and refuses the receipt, through the real binary — the
/// gap FR-111-CON-3 recorded until this ticket.
///
/// Trace: FR-111-AC-1, TC-1885
/// Provenance: PLAT-997
#[test]
fn tc_997_620_receipt_reports_apparatus_touched_through_the_real_binary() {
    let root = repo();
    let record_digest = seal_record(root.path());
    assert_eq!(
        record_digest, "1972d0d5d3ee3fb06a2a8ca2cc97f3e239d956958368d209ebbe0a52f9b61630",
        "record-body.json and attestation-body.json's record_digest have drifted apart"
    );
    let attestation_digest = intake_attestation(root.path());
    write_protecting_plan(root.path(), "MP-997-CLI-A", &["checker/config.toml"]);
    let decisions = write(
        root.path(),
        "decisions.json",
        &read_fixture("decisions.json"),
    );
    let audits = write(root.path(), "audits.json", &read_fixture("audits.json"));

    let output = quoin(&[
        "receipt",
        "--repo",
        root.path().to_str().unwrap(),
        "--record",
        &record_digest,
        "--candidate-revision",
        "candidate-1",
        "--select",
        &format!("proof-1={attestation_digest}"),
        "--decisions",
        decisions.to_str().unwrap(),
        "--audits",
        audits.to_str().unwrap(),
        "--diff-path",
        "src/lib.rs",
        "--diff-path",
        "checker/config.toml",
        "--plan",
        "MP-997-CLI-A",
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an invalid receipt is a complete payload with a failing outcome, not \
         a command failure: stderr {}",
        text(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("--json emits one document");
    assert_eq!(payload["outcome"], serde_json::json!("invalid"));
    assert_eq!(payload["reasons"], serde_json::json!(["apparatus_touched"]));
}

/// A plan that protects apparatus, checked with no `--diff-path` at all, is
/// `diff_missing` and `incomplete` through the real binary — never vacuously
/// `valid` because the caller supplied no diff (FR-111-AC-4).
///
/// Trace: FR-111-AC-4, TC-1886
/// Provenance: PLAT-997
#[test]
fn tc_997_621_receipt_reports_diff_missing_through_the_real_binary() {
    let root = repo();
    let record_digest = seal_record(root.path());
    let attestation_digest = intake_attestation(root.path());
    write_protecting_plan(root.path(), "MP-997-CLI-B", &["checker/config.toml"]);
    let decisions = write(
        root.path(),
        "decisions.json",
        &read_fixture("decisions.json"),
    );
    let audits = write(root.path(), "audits.json", &read_fixture("audits.json"));

    let output = quoin(&[
        "receipt",
        "--repo",
        root.path().to_str().unwrap(),
        "--record",
        &record_digest,
        "--candidate-revision",
        "candidate-1",
        "--select",
        &format!("proof-1={attestation_digest}"),
        "--decisions",
        decisions.to_str().unwrap(),
        "--audits",
        audits.to_str().unwrap(),
        "--plan",
        "MP-997-CLI-B",
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        text(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("--json emits one document");
    assert_eq!(payload["outcome"], serde_json::json!("incomplete"));
    assert_eq!(payload["reasons"], serde_json::json!(["diff_missing"]));
}
