// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Exercise the evidence-store-to-file Test Result export command (FR-185).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures panic with the failed setup step"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::{Value, json};
use tempfile::TempDir;

fn invoke(subject: &Path, output: &Path, evidence: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence",
            "export-test-result",
            "--subject",
            subject.to_str().expect("UTF-8 path"),
            "--output",
            output.to_str().expect("UTF-8 path"),
            "--evidence-dir",
            evidence.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("quoin runs")
}

fn subject_file(root: &Path) -> PathBuf {
    let path = root.join("subject.json");
    fs::write(
        &path,
        json!([{ "name": "artifact", "digest": { "sha256": "a".repeat(64) } }]).to_string(),
    )
    .expect("subject written");
    path
}

fn fixture() -> (TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().expect("temporary root");
    let evidence = root.path().join("evidence");
    fs::create_dir_all(evidence.join("runs/suite-pass")).expect("pass run dir");
    fs::create_dir_all(evidence.join("runs/suite-fail")).expect("fail run dir");
    fs::write(
        evidence.join("bindings.json"),
        json!({
            "schemaVersion": 1,
            "bindings": [
                {"obligation":"FR-1-AC-1","suite":"suite-pass","symbols":["pass"]},
                {"obligation":"FR-1-AC-2","suite":"suite-fail","symbols":["fail"]},
                {"obligation":"FR-1-AC-3","suite":"suite-missing","symbols":["missing"]}
            ]
        })
        .to_string(),
    )
    .expect("bindings written");
    for (suite, symbol, outcome) in [
        ("suite-pass", "pass", "pass"),
        ("suite-fail", "fail", "fail"),
    ] {
        fs::write(
            evidence.join(format!("runs/{suite}/2026-11-13T12~3A00~3A00Z.json")),
            json!({
                "schemaVersion": 1,
                "suite": suite,
                "tool": "cargo test",
                "evidenceKind": "test",
                "timestamp": "2026-11-13T12:00:00Z",
                "entries": [{"symbol": symbol, "outcome": outcome}]
            })
            .to_string(),
        )
        .expect("run written");
    }
    let subject = subject_file(root.path());
    (root, subject, evidence)
}

#[trace("FR-185-AC-1", "FR-185-AC-8")]
#[test]
fn tc_185_cli_exports_pass_fail_and_missing_run_classifications() {
    let (root, subject, evidence) = fixture();
    let output = root.path().join("statement.json");
    let result = invoke(&subject, &output, &evidence);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(output).expect("statement exists");
    assert!(bytes.starts_with(b"{\"_type\":\"https://in-toto.io/Statement/v1\",\"predicate\":"));
    assert_eq!(bytes.last(), Some(&b'}'));
    let statement: Value = serde_json::from_slice(&bytes).expect("statement is JSON");
    assert_eq!(
        statement.pointer("/predicate/result"),
        Some(&json!("FAILED"))
    );
    assert_eq!(
        statement.pointer("/predicate/passedTests"),
        Some(&json!(["FR-1-AC-1"]))
    );
    assert_eq!(
        statement.pointer("/predicate/failedTests"),
        Some(&json!(["FR-1-AC-2", "FR-1-AC-3"]))
    );
    assert_eq!(
        statement.pointer("/subject/0/digest/sha256"),
        Some(&json!("a".repeat(64)))
    );
}

#[trace("FR-185-AC-1", "FR-185-AC-8")]
#[test]
fn tc_185_a_later_duplicate_failure_cannot_be_hidden_by_a_pass() {
    let (root, subject, evidence) = fixture();
    fs::write(
        evidence.join("runs/suite-pass/2026-11-13T12~3A00~3A00Z.json"),
        json!({
            "schemaVersion": 1,
            "suite": "suite-pass",
            "tool": "cargo test",
            "evidenceKind": "test",
            "timestamp": "2026-11-13T12:00:00Z",
            "entries": [
                {"symbol": "pass", "outcome": "pass"},
                {"symbol": "pass", "outcome": "fail"}
            ]
        })
        .to_string(),
    )
    .expect("duplicate entries written");

    let output = root.path().join("statement.json");
    let result = invoke(&subject, &output, &evidence);
    assert_eq!(result.status.code(), Some(0));
    let statement: Value = serde_json::from_slice(&fs::read(output).expect("statement exists"))
        .expect("statement is JSON");
    assert_eq!(
        statement.pointer("/predicate/result"),
        Some(&json!("FAILED"))
    );
    assert_eq!(
        statement.pointer("/predicate/passedTests"),
        Some(&json!([]))
    );
    assert_eq!(
        statement.pointer("/predicate/failedTests"),
        Some(&json!(["FR-1-AC-1", "FR-1-AC-2", "FR-1-AC-3"]))
    );
}

#[trace("FR-185-AC-3")]
#[test]
fn tc_185_invalid_subject_exits_two_without_output() {
    let root = tempfile::tempdir().expect("temporary root");
    let subject = root.path().join("subject.json");
    fs::write(
        &subject,
        r#"[{"name":"artifact","digest":{"sha256":"not-hex"}}]"#,
    )
    .expect("subject written");
    let output = root.path().join("statement.json");
    let result = invoke(&subject, &output, &root.path().join("missing-evidence"));
    assert_eq!(result.status.code(), Some(2));
    assert!(!output.exists());
}

#[trace("FR-185-AC-6")]
#[test]
fn tc_185_invalid_store_names_path_and_leaves_no_output() {
    let (root, subject, evidence) = fixture();
    fs::write(evidence.join("runs/suite-pass/broken.json"), "{").expect("malformed run");
    let output = root.path().join("statement.json");
    let result = invoke(&subject, &output, &evidence);
    assert_eq!(result.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("evidence_store_invalid"), "{stderr}");
    assert!(stderr.contains("broken.json"), "{stderr}");
    assert!(!output.exists());

    fs::remove_file(evidence.join("runs/suite-pass/broken.json")).expect("remove malformed");
    fs::remove_file(evidence.join("bindings.json")).expect("remove bindings");
    let missing = invoke(&subject, &output, &evidence);
    assert_eq!(missing.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("evidence_store_invalid"), "{stderr}");
    assert!(stderr.contains("bindings.json"), "{stderr}");
    assert!(!output.exists());
}

#[cfg(unix)]
#[trace("FR-185-AC-6")]
#[test]
fn tc_185_an_unreadable_runs_directory_is_invalid_and_leaves_no_output() {
    use std::os::unix::fs::PermissionsExt;

    let (root, subject, evidence) = fixture();
    let runs = evidence.join("runs");
    fs::set_permissions(&runs, fs::Permissions::from_mode(0o000)).expect("runs are unreadable");
    let output = root.path().join("statement.json");
    let result = invoke(&subject, &output, &evidence);
    fs::set_permissions(&runs, fs::Permissions::from_mode(0o700)).expect("runs restored");

    assert_eq!(result.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("evidence_store_invalid"), "{stderr}");
    assert!(stderr.contains("runs"), "{stderr}");
    assert!(!output.exists());
}

#[trace("FR-185-AC-3")]
#[test]
fn tc_185_output_write_failure_exits_four() {
    let (root, subject, evidence) = fixture();
    let output = root.path().join("statement.json");
    fs::create_dir(&output).expect("output path is a directory");
    let result = invoke(&subject, &output, &evidence);
    assert_eq!(result.status.code(), Some(4));
    assert!(output.is_dir());
    assert!(String::from_utf8_lossy(&result.stderr).contains("evidence_output_write_failed"));
}

#[trace("FR-185-AC-4")]
#[test]
fn tc_185_reordered_store_records_produce_identical_bytes() {
    let (root, subject, evidence) = fixture();
    let first = root.path().join("first.json");
    let initial = invoke(&subject, &first, &evidence);
    assert_eq!(initial.status.code(), Some(0));

    let binding_path = evidence.join("bindings.json");
    let mut bindings: Value =
        serde_json::from_slice(&fs::read(&binding_path).expect("bindings")).expect("bindings JSON");
    bindings
        .get_mut("bindings")
        .and_then(Value::as_array_mut)
        .expect("binding array")
        .reverse();
    fs::write(&binding_path, bindings.to_string()).expect("reordered bindings");
    let second = root.path().join("second.json");
    let reordered = invoke(&subject, &second, &evidence);
    assert_eq!(reordered.status.code(), Some(0));
    assert_eq!(
        fs::read(first).expect("first output"),
        fs::read(second).expect("second output")
    );
}

#[trace("FR-185-AC-7")]
#[test]
fn tc_185_cli_export_path_has_no_git_network_or_process_api() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/evidence/test_result.rs"),
    )
    .expect("export adapter source");
    for forbidden in [
        "std::process",
        "TcpStream",
        "UdpSocket",
        "reqwest",
        "Command::new(\"git\")",
    ] {
        assert!(
            !source.contains(forbidden),
            "export path contains {forbidden}"
        );
    }
}
