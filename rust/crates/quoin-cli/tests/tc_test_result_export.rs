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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ix_trace_rs::trace;
use serde_json::{Value, json};
use tempfile::TempDir;

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

/// Write `suites.md` and a provenance envelope for the store as it is now.
///
/// `required` is the population the signed matrix declares, chosen by the test
/// (never read back from the bindings the export is judging). Each suite in
/// `suites` is listed with the digest of the one record under
/// `runs/<suite>/`, or a signed null when it has none.
fn write_provenance(
    evidence: &Path,
    required: &[&str],
    suites: &[&str],
    revision: &str,
) -> PathBuf {
    let suites_md = b"# suites
";
    fs::write(evidence.join("suites.md"), suites_md).expect("suites written");
    let runs: Vec<Value> = suites
        .iter()
        .map(|suite| {
            let directory = evidence.join("runs").join(suite);
            let record = fs::read_dir(&directory)
                .ok()
                .and_then(|mut entries| entries.next())
                .map(|entry| entry.expect("entry").path());
            match record {
                Some(path) => json!({
                    "suite": suite,
                    "recordPath": path.strip_prefix(evidence).expect("under evidence").to_str().unwrap(),
                    "sha256": quoin_store::attachment::sha256_hex(&fs::read(&path).expect("record")),
                }),
                None => json!({
                    "suite": suite,
                    "recordPath": format!("runs/{suite}/absent.json"),
                    "sha256": null,
                }),
            }
        })
        .collect();
    let payload = json!({
        "version": 2,
        "sourceRevision": revision,
        "suiteDeclarationsSha256": quoin_store::attachment::sha256_hex(suites_md),
        "testMatrix": [{
            "document": "spec/functional/FR-1.md",
            "id": "FR-1",
            "criteria": required
                .iter()
                .map(|id| json!({"id": id, "status": "tagged"}))
                .collect::<Vec<_>>(),
        }],
        "runs": runs,
    });
    let envelope = json!({
        "payloadType": "application/vnd.agent-ix.execution-provenance.v2+json",
        "payload": base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            payload.to_string(),
        ),
        "signatures": [{"sig": "AAAA"}],
    });
    let path = evidence.parent().expect("parent").join("provenance.json");
    fs::write(&path, envelope.to_string()).expect("provenance written");
    path
}

/// The default scenario's population, written down independently of the bindings.
const FIXTURE_POPULATION: [&str; 3] = ["FR-1-AC-1", "FR-1-AC-2", "FR-1-AC-3"];
const FIXTURE_SUITES: [&str; 3] = ["suite-pass", "suite-fail", "suite-missing"];

fn invoke(subject: &Path, output: &Path, evidence: &Path) -> Output {
    let provenance = write_provenance(evidence, &FIXTURE_POPULATION, &FIXTURE_SUITES, REVISION);
    invoke_with(subject, output, evidence, &provenance)
}

fn invoke_with(subject: &Path, output: &Path, evidence: &Path, provenance: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence",
            "export-test-result",
            "--execution-provenance",
            provenance.to_str().expect("UTF-8 path"),
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
        json!([{ "name": "artifact", "digest": { "sha256": "a".repeat(64) }, "annotations": { "revision": REVISION } }]).to_string(),
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
    for (suite, symbol, outcome, obligation) in [
        ("suite-pass", "pass", "pass", "FR-1-AC-1"),
        ("suite-fail", "fail", "fail", "FR-1-AC-2"),
    ] {
        fs::write(
            evidence.join(format!("runs/{suite}/2026-11-13T12~3A00~3A00Z.json")),
            json!({
                "schemaVersion": 1,
                "suite": suite,
                "tool": "cargo test",
                "evidenceKind": "test",
                "timestamp": "2026-11-13T12:00:00Z",
                "entries": [{
                    "symbol": symbol,
                    "outcome": outcome,
                    "traceIds": [obligation]
                }]
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

/// Read the exported statement's three lists.
fn lists(output: &Path) -> (String, Value, Value, Value) {
    let statement: Value = serde_json::from_slice(&fs::read(output).expect("statement exists"))
        .expect("statement is JSON");
    let get = |name: &str| {
        statement
            .pointer(&format!("/predicate/{name}"))
            .cloned()
            .expect(name)
    };
    (
        get("result").as_str().expect("result").to_owned(),
        get("passedTests"),
        get("warnedTests"),
        get("failedTests"),
    )
}

/// A store where every declared obligation passes, so a complete PASSED is
/// reachable; used as the healthy control for every refusal below.
fn all_passing() -> (TempDir, PathBuf, PathBuf) {
    let (root, subject, evidence) = fixture();
    fs::write(
        evidence.join("bindings.json"),
        json!({
            "schemaVersion": 1,
            "bindings": [{"obligation":"FR-1-AC-1","suite":"suite-pass","symbols":["pass"]}]
        })
        .to_string(),
    )
    .expect("bindings");
    fs::remove_dir_all(evidence.join("runs/suite-fail")).expect("no failing suite");
    (root, subject, evidence)
}

/// Complete PASSED needs the declared population: the genuine one passes (the
/// control); an empty graph, an omitted obligation, a zero-symbol binding and
/// an empty matrix cannot, and each is checked against a population written
/// down in the test.
#[trace("FR-185-AC-10")]
#[test]
fn tc_1792_100_complete_pass_needs_the_declared_population() {
    let (root, subject, evidence) = all_passing();
    let output = root.path().join("statement.json");

    let provenance = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(0)
    );
    let (result, passed, _, failed) = lists(&output);
    assert_eq!(
        (result.as_str(), passed, failed),
        ("PASSED", json!(["FR-1-AC-1"]), json!([]))
    );

    // Omitted obligation: declared in the matrix, absent from the bindings.
    let provenance = write_provenance(
        &evidence,
        &["FR-1-AC-1", "FR-1-AC-9"],
        &["suite-pass"],
        REVISION,
    );
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(0)
    );
    let (result, passed, _, failed) = lists(&output);
    assert_eq!(result, "FAILED");
    assert_eq!(passed, json!(["FR-1-AC-1"]));
    assert_eq!(failed, json!(["FR-1-AC-9"]));

    // Empty graph: no bindings at all, population still declared.
    fs::write(
        evidence.join("bindings.json"),
        json!({"schemaVersion": 1, "bindings": []}).to_string(),
    )
    .expect("empty graph");
    let provenance = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(0)
    );
    let (result, passed, _, failed) = lists(&output);
    assert_eq!(
        (result.as_str(), passed, failed),
        ("FAILED", json!([]), json!(["FR-1-AC-1"]))
    );

    // Zero-symbol binding.
    fs::write(
        evidence.join("bindings.json"),
        json!({"schemaVersion": 1, "bindings": [
            {"obligation":"FR-1-AC-1","suite":"suite-pass","symbols":[]}
        ]})
        .to_string(),
    )
    .expect("zero-symbol binding");
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(0)
    );
    let (result, _, _, failed) = lists(&output);
    assert_eq!((result.as_str(), failed), ("FAILED", json!(["FR-1-AC-1"])));

    // An empty signed matrix refuses rather than passing vacuously.
    let empty = write_provenance(&evidence, &[], &["suite-pass"], REVISION);
    fs::remove_file(&output).expect("remove");
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &empty)
            .status
            .code(),
        Some(2)
    );
    assert!(!output.exists());
}

/// Revision binding: a provenance for another revision refuses (the
/// old-run/new-subject case) and an unrelated subject refuses; the matching
/// revision is the healthy control. Nothing is written on refusal.
#[trace("FR-185-AC-12")]
#[test]
fn tc_1792_101_subject_and_provenance_revision_must_agree() {
    let (root, subject, evidence) = all_passing();
    let output = root.path().join("statement.json");
    let good = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &good)
            .status
            .code(),
        Some(0)
    );
    fs::remove_file(&output).expect("remove");

    let old = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], &"f".repeat(40));
    let refused = invoke_with(&subject, &output, &evidence, &old);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("evidence_provenance_invalid"));
    assert!(!output.exists());

    // A subject with no revision annotation is an invalid subject.
    fs::write(
        &subject,
        json!([{ "name": "artifact", "digest": { "sha256": "a".repeat(64) } }]).to_string(),
    )
    .expect("subject");
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &good)
            .status
            .code(),
        Some(2)
    );
    assert!(!output.exists());
}

/// Run records are bound by digest: a changed record, a missing record under a
/// non-null digest, or provenance/suite-declaration tampering refuses; a
/// signed null digest is a missing run (FAILED), not a refusal.
#[trace("FR-185-AC-11", "FR-185-AC-12")]
#[test]
fn tc_1792_102_runs_and_declarations_are_bound_by_digest() {
    let (root, subject, evidence) = all_passing();
    let output = root.path().join("statement.json");
    let provenance = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(0)
    );
    fs::remove_file(&output).expect("remove");

    // Suite declarations changed after signing.
    fs::write(
        evidence.join("suites.md"),
        b"# tampered
",
    )
    .expect("tamper");
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(2)
    );
    assert!(!output.exists());
    fs::write(
        evidence.join("suites.md"),
        b"# suites
",
    )
    .expect("restore");

    // A run record rewritten after signing (old run content, same path).
    let record = evidence.join("runs/suite-pass/2026-11-13T12~3A00~3A00Z.json");
    let original = fs::read(&record).expect("record");
    fs::write(
        &record,
        String::from_utf8_lossy(&original).replace("cargo test", "other tool"),
    )
    .expect("rewrite");
    let changed = invoke_with(&subject, &output, &evidence, &provenance);
    assert_eq!(changed.status.code(), Some(3));
    assert!(!output.exists());

    // Non-null digest, record gone.
    fs::remove_file(&record).expect("remove record");
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(3)
    );
    assert!(!output.exists());

    // Signed null digest: a missing run, exported as FAILED.
    let null = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &null)
            .status
            .code(),
        Some(0)
    );
    let (result, _, _, failed) = lists(&output);
    assert_eq!((result.as_str(), failed), ("FAILED", json!(["FR-1-AC-1"])));

    // Missing provenance file refuses.
    fs::remove_file(&output).expect("remove");
    let absent = root.path().join("nope.json");
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &absent)
            .status
            .code(),
        Some(2)
    );
    assert!(!output.exists());
}

/// A failed write leaves an existing output byte-for-byte unchanged.
#[cfg(unix)]
#[trace("FR-185-AC-6")]
#[test]
fn tc_1792_103_a_refused_or_failed_export_leaves_existing_output_unchanged() {
    let (root, subject, evidence) = all_passing();
    let output = root.path().join("statement.json");
    fs::write(&output, b"previous statement").expect("existing output");

    // Refusal (stale provenance revision) leaves it alone.
    let old = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], &"e".repeat(40));
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &old)
            .status
            .code(),
        Some(2)
    );
    assert_eq!(fs::read(&output).expect("output"), b"previous statement");

    // Write failure: the destination directory is read-only.
    let good = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    let locked = root.path().join("locked");
    fs::create_dir(&locked).expect("dir");
    let target = locked.join("statement.json");
    fs::write(&target, b"previous statement").expect("existing");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o500)).expect("read-only");
    let failed = invoke_with(&subject, &target, &evidence, &good);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).expect("restore");
    if failed.status.code() == Some(4) {
        assert_eq!(fs::read(&target).expect("output"), b"previous statement");
    } else {
        // Running as a user that bypasses directory permissions: no failure to observe.
        assert_eq!(failed.status.code(), Some(0));
    }
}

#[trace("FR-185-AC-9")]
#[test]
fn tc_185_cli_does_not_export_a_pass_for_an_unrelated_trace_id() {
    let (root, subject, evidence) = fixture();
    fs::write(
        evidence.join("bindings.json"),
        json!({
            "schemaVersion": 1,
            "bindings": [
                {"obligation":"FR-1-AC-1","suite":"suite-pass","symbols":["pass"]}
            ]
        })
        .to_string(),
    )
    .expect("single binding written");
    fs::write(
        evidence.join("runs/suite-pass/2026-11-13T12~3A00~3A00Z.json"),
        json!({
            "schemaVersion": 1,
            "suite": "suite-pass",
            "tool": "cargo test",
            "evidenceKind": "test",
            "timestamp": "2026-11-13T12:00:00Z",
            "entries": [{
                "symbol": "pass",
                "outcome": "pass",
                "traceIds": ["FR-1-AC-OTHER"]
            }]
        })
        .to_string(),
    )
    .expect("mismatched run written");

    let output = root.path().join("statement.json");
    let provenance = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    let result = invoke_with(&subject, &output, &evidence, &provenance);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
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
        Some(&json!(["FR-1-AC-1"]))
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
                {"symbol": "pass", "outcome": "pass", "traceIds": ["FR-1-AC-1"]},
                {"symbol": "pass", "outcome": "fail", "traceIds": ["FR-1-AC-1"]}
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

#[trace("FR-185-AC-2")]
#[test]
fn tc_185_a_later_duplicate_skip_is_not_hidden_by_a_pass() {
    let (root, subject, evidence) = fixture();
    fs::write(
        evidence.join("bindings.json"),
        json!({
            "schemaVersion": 1,
            "bindings": [
                {"obligation":"FR-1-AC-1","suite":"suite-pass","symbols":["pass"]}
            ]
        })
        .to_string(),
    )
    .expect("single binding written");
    fs::write(
        evidence.join("runs/suite-pass/2026-11-13T12~3A00~3A00Z.json"),
        json!({
            "schemaVersion": 1,
            "suite": "suite-pass",
            "tool": "cargo test",
            "evidenceKind": "test",
            "timestamp": "2026-11-13T12:00:00Z",
            "entries": [
                {"symbol": "pass", "outcome": "pass", "traceIds": ["FR-1-AC-1"]},
                {"symbol": "pass", "outcome": "skip", "traceIds": ["FR-1-AC-1"]}
            ]
        })
        .to_string(),
    )
    .expect("duplicate entries written");

    let output = root.path().join("statement.json");
    let provenance = write_provenance(&evidence, &["FR-1-AC-1"], &["suite-pass"], REVISION);
    let result = invoke_with(&subject, &output, &evidence, &provenance);
    assert_eq!(result.status.code(), Some(0));
    let statement: Value = serde_json::from_slice(&fs::read(output).expect("statement exists"))
        .expect("statement is JSON");
    assert_eq!(
        statement.pointer("/predicate/result"),
        Some(&json!("WARNED"))
    );
    assert_eq!(
        statement.pointer("/predicate/passedTests"),
        Some(&json!([]))
    );
    assert_eq!(
        statement.pointer("/predicate/warnedTests"),
        Some(&json!(["FR-1-AC-1"]))
    );
    assert_eq!(
        statement.pointer("/predicate/failedTests"),
        Some(&json!([]))
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
    let result = invoke_with(
        &subject,
        &output,
        &root.path().join("missing-evidence"),
        &root.path().join("provenance.json"),
    );
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

/// A record for another suite cannot stand in for the suite the provenance
/// names: another suite's bytes listed under a suite refuse, rather than
/// overriding that suite's signed absence.
#[trace("FR-185-AC-12")]
#[test]
fn tc_1792_104_a_run_record_must_be_for_the_suite_that_names_it() {
    let (root, subject, evidence) = fixture();
    let output = root.path().join("statement.json");
    let pass_path = "runs/suite-pass/2026-11-13T12~3A00~3A00Z.json";
    let pass_digest =
        quoin_store::attachment::sha256_hex(&fs::read(evidence.join(pass_path)).expect("record"));
    let suites_md = b"# suites\n";
    fs::write(evidence.join("suites.md"), suites_md).expect("suites");
    let envelope = |runs: Value| {
        let payload = json!({
            "version": 2,
            "sourceRevision": REVISION,
            "suiteDeclarationsSha256": quoin_store::attachment::sha256_hex(suites_md),
            "testMatrix": [{"document": "d", "id": "FR-1",
                "criteria": [{"id": "FR-1-AC-1", "status": "tagged"}]}],
            "runs": runs,
        });
        let path = root.path().join("lying.json");
        fs::write(
            &path,
            json!({
                "payloadType": "application/vnd.agent-ix.execution-provenance.v2+json",
                "payload": base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    payload.to_string(),
                ),
                "signatures": [{"sig": "AAAA"}],
            })
            .to_string(),
        )
        .expect("envelope");
        path
    };
    // Control: the record listed under its own suite exports.
    let honest = envelope(json!([
        {"suite": "suite-pass", "recordPath": pass_path, "sha256": pass_digest}
    ]));
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &honest)
            .status
            .code(),
        Some(0)
    );
    fs::remove_file(&output).expect("remove");

    // suite-pass's bytes filed under suite-fail's name.
    let lying = envelope(json!([
        {"suite": "suite-fail", "recordPath": pass_path, "sha256": pass_digest}
    ]));
    let refused = invoke_with(&subject, &output, &evidence, &lying);
    assert_eq!(refused.status.code(), Some(3));
    assert!(!output.exists());
}

/// `bindings.json` is not signed: re-pointing an obligation at a passing suite
/// cannot hide a digest-bound failure traced to that obligation.
#[trace("FR-185-AC-8")]
#[test]
fn tc_1792_105_edited_bindings_cannot_hide_a_bound_failure() {
    let (root, subject, evidence) = fixture();
    let output = root.path().join("statement.json");
    let provenance = write_provenance(
        &evidence,
        &["FR-1-AC-1", "FR-1-AC-2"],
        &["suite-pass", "suite-fail"],
        REVISION,
    );
    fs::write(
        evidence.join("bindings.json"),
        json!({"schemaVersion": 1, "bindings": [
            {"obligation": "FR-1-AC-1", "suite": "suite-pass", "symbols": ["pass"]},
            {"obligation": "FR-1-AC-2", "suite": "suite-pass", "symbols": ["pass"]}
        ]})
        .to_string(),
    )
    .expect("edited bindings");
    assert_eq!(
        invoke_with(&subject, &output, &evidence, &provenance)
            .status
            .code(),
        Some(0)
    );
    let (result, passed, _, failed) = lists(&output);
    assert_eq!(result, "FAILED");
    assert_eq!(passed, json!(["FR-1-AC-1"]));
    assert_eq!(failed, json!(["FR-1-AC-2"]));
}
