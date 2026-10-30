// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The FR-068 exit grammar of `quoin change-assurance`, against the real
//! binary.
//!
//! TC-1322 and TC-1324 were carried by `tests/change-assurance-command-surface.test.ts`
//! until the native cutover deleted it (quoin#457, quoin#524) without
//! reproducing them here. With no test left holding the grammar, every
//! refusal collapsed onto the shell's generic exit 1 and a consumer could no
//! longer tell a REFUSED document from a receipt that verified `invalid`
//! (quoin#543; agent-ix/tl-mltl and agent-ix/quire-contract-runtime#32 both
//! assert the difference).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository-root fixture directory the retained suite also read.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/change-assurance-cli")
        .canonicalize()
        .expect("the retained change-assurance CLI fixtures are present")
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

/// A repository holding the retained record and one retained attestation.
fn retained_repo() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary directory");
    let repo = root.path().to_str().expect("a UTF-8 path").to_owned();
    let fixture = |name: &str| fixtures().join(name).to_str().expect("UTF-8").to_owned();
    let written = quoin(&[
        "write-record",
        "--input",
        &fixture("record-body.json"),
        "--repo",
        &repo,
    ]);
    assert_eq!(written.status.code(), Some(0), "{}", text(&written.stderr));
    let output = root.path().join("output.bin");
    std::fs::write(&output, b"passed\n").expect("the output file");
    let prepared = quoin(&[
        "prepare-attestation",
        "--input",
        &fixture("attestation-body.json"),
        "--output",
        output.to_str().expect("UTF-8"),
        "--media-type",
        "text/plain",
    ]);
    assert_eq!(
        prepared.status.code(),
        Some(0),
        "{}",
        text(&prepared.stderr)
    );
    let attestation = root.path().join("attestation.json");
    std::fs::write(&attestation, &prepared.stdout).expect("the attestation file");
    let intake = quoin(&[
        "intake",
        "--attestation",
        attestation.to_str().expect("UTF-8"),
        "--output",
        output.to_str().expect("UTF-8"),
        "--repo",
        &repo,
    ]);
    assert_eq!(intake.status.code(), Some(0), "{}", text(&intake.stderr));
    root
}

/// The shared receipt/verify-receipt inputs for one candidate revision.
fn inputs(root: &Path, candidate: &str) -> Vec<String> {
    vec![
        "--repo".to_owned(),
        root.to_str().expect("UTF-8").to_owned(),
        "--record".to_owned(),
        "change-1".to_owned(),
        "--revision".to_owned(),
        "1".to_owned(),
        "--candidate-revision".to_owned(),
        candidate.to_owned(),
        "--select".to_owned(),
        "proof-1=att-1".to_owned(),
        "--decisions".to_owned(),
        fixtures()
            .join("decisions.json")
            .to_str()
            .expect("UTF-8")
            .to_owned(),
        "--audits".to_owned(),
        fixtures()
            .join("audits.json")
            .to_str()
            .expect("UTF-8")
            .to_owned(),
        "--json".to_owned(),
    ]
}

fn run_with(subcommand: &str, leading: &[&str], rest: &[String]) -> Output {
    let mut argv: Vec<&str> = vec![subcommand];
    argv.extend_from_slice(leading);
    argv.extend(rest.iter().map(String::as_str));
    quoin(&argv)
}

/// Emit a receipt for `candidate` and write it beside the repository.
fn emitted_receipt(root: &Path, candidate: &str) -> (PathBuf, i32) {
    let output = run_with("receipt", &[], &inputs(root, candidate));
    let code = output.status.code().expect("the receipt command exits");
    let path = root.join(format!("receipt-{candidate}.json"));
    std::fs::write(&path, &output.stdout).expect("the receipt file");
    (path, code)
}

fn verify(root: &Path, receipt: &Path, candidate: &str) -> Output {
    run_with(
        "verify-receipt",
        &["--input", receipt.to_str().expect("UTF-8")],
        &inputs(root, candidate),
    )
}

/// A stored receipt that re-derives byte for byte exits with its own
/// verdict: 0 for `valid`, 1 for `invalid` or `incomplete`.
///
/// Trace: FR-068-AC-6, FR-068-AC-7, TC-1322, TC-1323
/// Provenance: quoin#543
#[test]
fn tc_1322_a_rederived_receipt_exits_with_its_verdict() {
    let root = retained_repo();
    // The attestation is bound to `candidate-1`; any other candidate cannot
    // be discharged by it.
    for candidate in ["candidate-1", "candidate-2"] {
        let (receipt, emitted) = emitted_receipt(root.path(), candidate);
        let output = verify(root.path(), &receipt, candidate);
        assert_eq!(
            output.status.code(),
            Some(emitted),
            "{candidate}: stderr: {}",
            text(&output.stderr)
        );
    }
    let (_, failing) = emitted_receipt(root.path(), "candidate-2");
    assert_eq!(failing, 1, "a candidate the attestation does not cover");
}

/// Exit 2 is a refused document: a stored receipt whose bytes differ from the
/// re-derived receipt is refused, whatever member was edited.
///
/// Trace: FR-068-AC-6, FR-068-AC-7, TC-1322, TC-1323
/// Provenance: quoin#543
#[test]
fn tc_1322_an_edited_receipt_exits_two() {
    let root = retained_repo();
    let (receipt, _) = emitted_receipt(root.path(), "candidate-1");
    let mut body: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("the receipt file"))
            .expect("the receipt is JSON");
    body["outcome"] = serde_json::json!("invalid");
    let edited = root.path().join("edited.json");
    std::fs::write(&edited, serde_json::to_vec(&body).expect("serialises")).expect("written");
    let output = verify(root.path(), &edited, "candidate-1");
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    assert_eq!(output.stdout, Vec::<u8>::new(), "a refused document emits nothing");
}

/// A parse error and an unknown schema name are refusals of the document the
/// caller supplied, not command failures: both exit 2.
///
/// Trace: FR-068-AC-6, FR-068-AC-8, TC-1322, TC-1324
/// Provenance: quoin#543
#[test]
fn tc_1324_unparseable_input_and_an_unknown_schema_name_exit_two() {
    let root = retained_repo();
    let junk = root.path().join("junk.json");
    std::fs::write(&junk, b"\"not a receipt\"").expect("written");
    let output = verify(root.path(), &junk, "candidate-1");
    assert_eq!(output.status.code(), Some(2));

    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "change-assurance",
            "schema",
            "--name",
            "not-an-asset.schema.json",
        ])
        .output()
        .expect("the native quoin binary runs");
    assert_eq!(output.status.code(), Some(2));
}

/// A usage error on this surface exits 2 as well. FR-068 groups usage with
/// parse and integrity errors, but the parser refuses an invocation before
/// dispatch ever sees which family it named, so this arm reached the root
/// shell's exit 3 until quoin#543.
///
/// Trace: FR-068-AC-6, TC-1322
/// Provenance: quoin#543
#[test]
fn tc_1322_a_usage_error_on_the_surface_exits_two() {
    for argv in [
        vec![
            "change-assurance",
            "verify-receipt",
            "--input",
            "-",
            "--bogus",
        ],
        vec!["change-assurance", "verify-receipt", "--input"],
        vec!["change-assurance", "verify-receipt"],
        vec!["change-assurance", "not-a-subcommand"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(&argv)
            .output()
            .expect("the native quoin binary runs");
        assert_eq!(output.status.code(), Some(2), "{argv:?}");
    }

    // The same malformed flag on another surface keeps the root shell's exit
    // 3: FR-068's grammar is this family's, not a new global meaning for 2.
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["catalog", "list", "--bogus"])
        .output()
        .expect("the native quoin binary runs");
    assert_eq!(output.status.code(), Some(3));
}
