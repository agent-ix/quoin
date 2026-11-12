// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `measurement.attach` and attachment verification in `measurement.verify`
//! through the production runtime (FR-125, PLAT-165).
//!
//! Provenance: PLAT-165

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, request: &Value) -> Run {
    let response =
        dispatch(op, request, &RuntimeSettings::default()).unwrap_or_else(|error| Response {
            payload: Value::Null,
            diagnostics: vec![Diagnostic::from(&error)],
            outcome: error.outcome(),
        });
    Run {
        stdout: if response.outcome.carries_payload() {
            canonical_json(&response.payload).unwrap()
        } else {
            String::new()
        },
        stderr: if response.diagnostics.is_empty() {
            String::new()
        } else {
            canonical_json(&response.diagnostics).unwrap()
        },
        status: i32::from(response.outcome.code()),
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-measurement/tests/fixtures/verify")
}

fn repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().unwrap();
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::copy(
        fixtures().join("plans/MP-961-gate.md"),
        assurance.join("MP-961-gate.md"),
    )
    .unwrap();
    temporary
}

fn attach(repo: &Path, path: &str) -> Run {
    run(
        "measurement.attach",
        &json!({ "repo": repo.to_str().unwrap(), "path": path, "media_type": "application/x-profile" }),
    )
}

#[trace("FR-125-AC-6")]
#[test]
fn tc_165_101_attach_retains_a_regular_file_and_refuses_every_unsafe_source() {
    let temporary = repository();
    let root = temporary.path();
    std::fs::write(root.join("profile.bin"), [0_u8, 159, 255]).unwrap();
    let attached = attach(root, "profile.bin");
    assert_eq!(attached.status, 0, "{}", attached.stderr);
    let reference: Value = serde_json::from_str(&attached.stdout).unwrap();
    assert_eq!(reference["byteLength"], 3);
    assert_eq!(reference["mediaType"], "application/x-profile");
    let hex = reference["digest"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    assert_eq!(
        std::fs::read(root.join("spec/evidence/attachments").join(hex)).unwrap(),
        [0, 159, 255]
    );
    let attachments = root.join("spec/evidence/attachments");
    let before = std::fs::read_dir(&attachments).unwrap().count();

    // Fixtures for the refusals.
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.bin"), b"secret").unwrap();
    std::fs::create_dir_all(root.join("dir")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join("linked-dir")).unwrap();
    std::os::unix::fs::symlink(root.join("profile.bin"), root.join("linked-file")).unwrap();
    let huge = std::fs::File::create(root.join("huge.bin")).unwrap();
    huge.set_len(64 * 1024 * 1024 + 1).unwrap();

    let absolute = outside.path().join("secret.bin");
    for source in [
        absolute.to_str().unwrap(),
        "../escape",
        "dir/../profile.bin",
        "",
        "linked-dir/secret.bin",
        "linked-file",
        "dir",
        "huge.bin",
        "missing.bin",
    ] {
        let refused = attach(root, source);
        assert_ne!(refused.status, 0, "{source:?} must refuse");
        assert!(
            refused.stderr.contains("QM-ATTACHMENT-INVALID"),
            "{source:?}: {}",
            refused.stderr
        );
    }
    assert_eq!(
        std::fs::read_dir(&attachments).unwrap().count(),
        before,
        "a refused source retains nothing"
    );
}

fn recorded_with_attachment(root: &Path) -> String {
    std::fs::write(root.join("trace.bin"), [1_u8, 2, 3, 4]).unwrap();
    let attached = attach(root, "trace.bin");
    assert_eq!(attached.status, 0, "{}", attached.stderr);
    let reference: Value = serde_json::from_str(&attached.stdout).unwrap();
    let mut record: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("right/accept/1.json")).unwrap(),
    )
    .unwrap();
    record["verificationStack"]
        .as_object_mut()
        .unwrap()
        .remove("protectedApparatus");
    record["attachments"] = json!([reference.clone()]);
    let recorded = run(
        "measurement.record",
        &json!({ "repo": root.to_str().unwrap(), "record": record }),
    );
    assert_eq!(recorded.status, 0, "{}", recorded.stderr);
    reference["digest"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap()
        .to_owned()
}

fn verify(root: &Path) -> Run {
    run(
        "measurement.verify",
        &json!({ "repo": root.to_str().unwrap(), "plan": "MP-961", "claimed": "accept" }),
    )
}

#[trace("FR-125-AC-7")]
#[test]
fn tc_165_102_verify_fails_when_a_retained_attachment_is_tampered_or_missing() {
    let temporary = repository();
    let root = temporary.path();
    let hex = recorded_with_attachment(root);
    let clean = verify(root);
    assert_eq!(clean.status, 0, "{}", clean.stderr);

    let file = root.join("spec/evidence/attachments").join(&hex);
    std::fs::write(&file, [9_u8, 9, 9, 9]).unwrap();
    let tampered = verify(root);
    assert_ne!(tampered.status, 0);
    assert!(
        tampered.stderr.contains("QM-ATTACHMENT-TAMPERED"),
        "{}",
        tampered.stderr
    );
    assert!(tampered.stderr.contains(&hex), "{}", tampered.stderr);

    std::fs::remove_file(&file).unwrap();
    let missing = verify(root);
    assert_ne!(missing.status, 0);
    assert!(
        missing.stderr.contains("QM-ATTACHMENT-UNAVAILABLE"),
        "{}",
        missing.stderr
    );
    assert!(missing.stderr.contains(&hex), "{}", missing.stderr);
}
