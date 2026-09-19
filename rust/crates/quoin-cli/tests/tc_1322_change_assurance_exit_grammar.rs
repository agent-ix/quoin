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
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use quoin_store::{digest_canonical_value, parse_strict_json_str};

/// The repository-root fixture directory the retained suite also read.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/change-assurance-cli")
        .canonicalize()
        .expect("the retained change-assurance CLI fixtures are present")
}

fn receipt() -> serde_json::Value {
    let bytes = std::fs::read(fixtures().join("receipt.json")).expect("the sealed receipt fixture");
    serde_json::from_slice(&bytes).expect("the sealed receipt fixture is JSON")
}

/// Run `quoin change-assurance verify-receipt` over a receipt supplied on
/// standard input, exactly as the reporting consumers do.
fn verify(document: &serde_json::Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "change-assurance",
            "verify-receipt",
            "--input",
            "-",
            "--json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the native quoin binary runs");
    child
        .stdin
        .as_mut()
        .expect("the child holds a piped stdin")
        .write_all(
            serde_json::to_string(document)
                .expect("the receipt serialises")
                .as_bytes(),
        )
        .expect("the receipt reaches the child");
    child.wait_with_output().expect("the child terminates")
}

/// Re-seal a receipt body so the only thing under test is its verified
/// outcome, not its integrity.
///
/// `digest` is excluded from its own preimage, which is what
/// `verify::verify_receipt` recomputes.
fn reseal(mut body: serde_json::Value) -> serde_json::Value {
    let object = body.as_object_mut().expect("a receipt is a JSON object");
    object.remove("digest");
    let preimage =
        parse_strict_json_str(&serde_json::to_string(&body).expect("the receipt body serialises"))
            .expect("the receipt body is strict JSON");
    let digest = digest_canonical_value(&preimage).expect("the body digests");
    body.as_object_mut()
        .expect("a receipt is a JSON object")
        .insert(
            "digest".to_owned(),
            serde_json::Value::String(digest.as_hex().to_owned()),
        );
    body
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

/// Exit 0 is a `valid` receipt, and the receipt is still the machine-readable
/// result.
///
/// Trace: FR-068-AC-6, FR-068-AC-7, TC-1322, TC-1323
#[test]
fn tc_1322_a_sealed_valid_receipt_exits_zero() {
    let output = verify(&receipt());
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        text(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("--json emits one JSON document");
    assert_eq!(payload["outcome"], serde_json::json!("valid"));
}

/// Exit 1 is reserved for a receipt whose own verified outcome is `invalid` or
/// `incomplete`. The document is intact; the verdict it carries is not a pass.
///
/// Trace: FR-068-AC-6, TC-1322
#[test]
fn tc_1322_an_intact_receipt_with_a_failing_outcome_exits_one() {
    for (outcome, check, reason) in [
        ("invalid", "review", "review_rejected"),
        ("incomplete", "lineage", "parent_missing"),
    ] {
        let mut body = receipt();
        body["outcome"] = serde_json::json!(outcome);
        body["reasons"] = serde_json::json!([reason]);
        body["checks"][check] = serde_json::json!({
            "outcome": outcome,
            "reasons": [reason],
        });
        let output = verify(&reseal(body));
        assert_eq!(
            output.status.code(),
            Some(1),
            "{outcome}: stderr: {}",
            text(&output.stderr)
        );
        let payload: serde_json::Value = serde_json::from_str(text(&output.stdout).trim())
            .expect("--json emits one JSON document");
        assert_eq!(payload["outcome"], serde_json::json!(outcome));
    }
}

/// Exit 2 is a refused document. Both refusal paths reach it: a semantic edit
/// that trips the consistency rule before any hashing, and an edit to a
/// digest-covered field that leaves the document self-consistent so that only
/// its seal is wrong.
///
/// A fix for one path alone would leave the other reporting the verdict
/// grammar's exit 1 (quoin#543).
///
/// Trace: FR-068-AC-6, FR-068-AC-7, TC-1322, TC-1323
#[test]
fn tc_1322_an_edited_receipt_exits_two_on_both_refusal_paths() {
    let mut precedence = receipt();
    precedence["outcome"] = serde_json::json!("invalid");
    let precedence = verify(&precedence);
    assert_eq!(
        precedence.status.code(),
        Some(2),
        "stderr: {}",
        text(&precedence.stderr)
    );
    assert!(precedence.stdout.is_empty());
    assert!(text(&precedence.stderr).contains("outcome disagrees with reason precedence"));

    let mut seal = receipt();
    seal["candidate_revision"] = serde_json::json!("1".repeat(40));
    let seal = verify(&seal);
    assert_eq!(
        seal.status.code(),
        Some(2),
        "stderr: {}",
        text(&seal.stderr)
    );
    assert!(seal.stdout.is_empty());
    assert!(text(&seal.stderr).contains("digest mismatch"));
}

/// A parse error and an unknown schema name are refusals of the document the
/// caller supplied, not command failures: both exit 2.
///
/// Trace: FR-068-AC-6, FR-068-AC-8, TC-1322, TC-1324
#[test]
fn tc_1324_unparseable_input_and_an_unknown_schema_name_exit_two() {
    let output = verify(&serde_json::json!("not a receipt"));
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
