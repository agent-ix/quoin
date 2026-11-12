// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-125: immutable content-addressed binary attachments for measurement
//! collections. Every repository is a real temporary directory and every
//! collection goes through `write_measurement_collection`.
//!
//! Provenance: PLAT-165

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use ix_trace_rs::trace;
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::store::{measurement_path, read_measurement_collections};
use quoin_measurement::types::ids::CollectionId;
use quoin_measurement::{
    MAX_COLLECTION_ATTACHMENTS, MeasurementError, collection_attachments, read_attachment,
    retain_attachment, verify_collection_attachments, write_measurement_collection,
};
use serde_json::{Value, json};

const PLAN: &str = "---\n\
id: MP-001\n\
title: Example metric\n\
type: MeasurementPlan\n\
status: active\n\
owner: test\n\
stage: branch-comparison\n\
metric: quality.example\n\
definition_version: quality.example-v1\n\
---\n\
\n\
# Example metric\n";

fn repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root");
    std::fs::write(assurance.join("MP-001.md"), PLAN).expect("the plan");
    temporary
}

fn collection(attachments: &Value) -> Value {
    let mut value = json!({
        "schemaVersion": 2,
        "collectionId": "run-001",
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1",
        "timestamp": "2026-08-26T00:00:00.000Z",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"],
            "artifacts": ["config"],
        },
        "observations": [{
            "metric": "quality.example",
            "planId": "MP-001",
            "definitionVersion": "quality.example-v1",
            "state": "measured",
            "value": 0.5,
            "unit": "fraction",
            "shape": "ratio",
            "population": { "examined": 2, "matched": 1, "complete": true, "identity": ["a", "b"] },
        }],
        "rawEvidence": { "payload": [1, 2] },
    });
    if !attachments.is_null() {
        value["attachments"] = attachments.clone();
    }
    value
}

fn publish(root: &Path, attachments: &Value) -> Result<std::path::PathBuf, MeasurementError> {
    write_measurement_collection(
        root,
        &from_serde(&collection(attachments)).expect("the candidate crosses the bridge"),
    )
}

fn refusal(result: Result<std::path::PathBuf, MeasurementError>) -> MeasurementError {
    result.expect_err("the publish must refuse")
}

fn reference_json(root: &Path, bytes: &[u8]) -> Value {
    let reference = retain_attachment(root, "application/octet-stream", bytes).expect("retained");
    quoin_measurement::json_bridge::to_serde(&reference.to_json().expect("json")).expect("serde")
}

fn retained_file(root: &Path, reference: &Value) -> std::path::PathBuf {
    let digest = reference["digest"].as_str().expect("digest");
    root.join("spec/evidence/attachments")
        .join(digest.strip_prefix("sha256:").expect("prefix"))
}

fn no_collection(root: &Path) -> bool {
    !measurement_path(root, &CollectionId::parse("run-001").unwrap()).exists()
}

/// The binary payload used throughout: not UTF-8, with NULs.
const BINARY: &[u8] = &[0, 159, 146, 150, 255, 0, 1, 2];

#[trace("FR-125-AC-1")]
#[test]
fn tc_165_001_the_address_is_the_sha256_of_the_bytes_and_round_trips() {
    let temporary = repository();
    let root = temporary.path();
    let first = retain_attachment(root, "application/x-trace", BINARY).unwrap();
    let again = retain_attachment(root, "application/x-trace", BINARY).unwrap();
    let other = retain_attachment(
        root,
        "application/x-trace",
        &[0, 159, 146, 150, 255, 0, 1, 3],
    )
    .unwrap();
    assert_eq!(first, again);
    assert_ne!(first.digest_hex(), other.digest_hex());
    assert_eq!(first.byte_length(), 8);
    assert_eq!(read_attachment(root, &first).unwrap(), BINARY);
    assert_eq!(
        std::fs::read_dir(root.join("spec/evidence/attachments"))
            .unwrap()
            .count(),
        2,
        "identical bytes are retained once"
    );
}

#[trace("FR-125-AC-2")]
#[test]
fn tc_165_002_every_unsafe_reference_refuses_before_bytes_are_admitted() {
    let temporary = repository();
    let root = temporary.path();
    let good = "sha256:".to_owned() + &"a".repeat(64);
    let entry = |digest: &str, media: &str, length: Value| json!({ "digest": digest, "mediaType": media, "byteLength": length });
    let many: Vec<Value> = (0..=MAX_COLLECTION_ATTACHMENTS)
        .map(|index| entry(&format!("sha256:{index:064x}"), "a/b", json!(1)))
        .collect();
    let cases = [
        json!("not an array"),
        json!([entry("../../etc/passwd", "a/b", json!(1))]),
        json!([entry(
            &format!("sha256:{}", "A".repeat(64)),
            "a/b",
            json!(1)
        )]),
        json!([entry("sha256:abc", "a/b", json!(1))]),
        json!([entry(&format!("md5:{}", "a".repeat(64)), "a/b", json!(1))]),
        json!([entry(&good, "", json!(1))]),
        json!([entry(&good, "a/b", json!(-1))]),
        json!([entry(&good, "a/b", json!(1.5))]),
        json!([entry(&good, "a/b", json!(64 * 1024 * 1024 + 1))]),
        json!([entry(&good, "a/b", json!(1)), entry(&good, "a/b", json!(1))]),
        Value::Array(many),
    ];
    for attachments in &cases {
        let error = refusal(publish(root, attachments));
        assert_eq!(
            error.code(),
            MeasurementErrorCode::AttachmentInvalid,
            "{attachments}"
        );
        assert!(no_collection(root));
        assert!(!root.join("spec/evidence/attachments").exists());
    }
    assert_eq!(
        retain_attachment(root, "", BINARY).unwrap_err().code(),
        MeasurementErrorCode::AttachmentInvalid
    );
    assert!(
        collection_attachments(&from_serde(&collection(&Value::Null)).unwrap())
            .unwrap()
            .is_empty()
    );
}

#[trace("FR-125-AC-3")]
#[test]
fn tc_165_003_missing_stale_or_mismatched_bytes_refuse_the_publish() {
    let temporary = repository();
    let root = temporary.path();
    let present = reference_json(root, BINARY);

    let missing = json!([{
        "digest": format!("sha256:{}", "b".repeat(64)),
        "mediaType": "a/b",
        "byteLength": 4,
    }]);
    let error = refusal(publish(root, &missing));
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentUnavailable);
    assert!(error.subject().contains(&"b".repeat(64)), "{error}");

    let mut wrong_length = present.clone();
    wrong_length["byteLength"] = json!(7);
    let error = refusal(publish(root, &json!([wrong_length])));
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentTampered);

    let mut longer = present.clone();
    longer["byteLength"] = json!(9);
    let error = refusal(publish(root, &json!([longer])));
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentTampered);

    std::fs::write(retained_file(root, &present), b"swapped").unwrap();
    let error = refusal(publish(root, &json!([present])));
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentTampered);
    assert!(no_collection(root));
}

#[trace("FR-125-AC-4")]
#[test]
fn tc_165_004_post_seal_edit_substitution_or_deletion_is_red_and_names_it() {
    let temporary = repository();
    let root = temporary.path();
    let reference = reference_json(root, BINARY);
    publish(root, &json!([reference.clone()])).unwrap();
    let id = CollectionId::parse("run-001").unwrap();
    verify_collection_attachments(root, &id).unwrap();
    let file = retained_file(root, &reference);
    let hex = file.file_name().unwrap().to_string_lossy().into_owned();

    // Edit in place (same length, different bytes).
    std::fs::write(&file, [0, 159, 146, 150, 255, 0, 1, 3]).unwrap();
    let error = verify_collection_attachments(root, &id).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentTampered);
    assert!(error.subject().contains(&hex), "{error}");

    // Re-retaining the original bytes does not replace the edited file.
    let error = retain_attachment(root, "application/octet-stream", BINARY).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentTampered);
    assert_eq!(
        std::fs::read(&file).unwrap(),
        [0, 159, 146, 150, 255, 0, 1, 3]
    );

    // Symlink substitution, even to the right bytes.
    let elsewhere = root.join("elsewhere.bin");
    std::fs::write(&elsewhere, BINARY).unwrap();
    std::fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &file).unwrap();
    let error = verify_collection_attachments(root, &id).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentUnavailable);
    assert!(error.subject().contains(&hex), "{error}");

    // Deletion.
    std::fs::remove_file(&file).unwrap();
    let error = verify_collection_attachments(root, &id).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentUnavailable);
    assert!(error.subject().contains(&hex), "{error}");
}

#[trace("FR-125-AC-5")]
#[test]
fn tc_165_005_a_collection_binds_and_recovers_binary_attachments() {
    let temporary = repository();
    let root = temporary.path();
    let first = reference_json(root, BINARY);
    let second = reference_json(root, &[255, 254, 0, 0]);
    publish(root, &json!([first, second])).unwrap();

    let stored =
        read_measurement_collections(&quoin_measurement::DiskMeasurement::new(root)).unwrap();
    assert_eq!(stored.len(), 1);
    let id = CollectionId::parse("run-001").unwrap();
    verify_collection_attachments(root, &id).unwrap();

    let text = std::fs::read_to_string(measurement_path(root, &id)).unwrap();
    let document: Value = serde_json::from_str(&text).unwrap();
    let references = collection_attachments(&from_serde(&document).unwrap()).unwrap();
    assert_eq!(references.len(), 2);
    assert_eq!(read_attachment(root, &references[0]).unwrap(), BINARY);
    assert_eq!(
        read_attachment(root, &references[1]).unwrap(),
        [255, 254, 0, 0]
    );
}

#[trace("FR-125-AC-4")]
#[test]
fn tc_165_006_a_symlink_at_any_component_of_the_retained_path_refuses() {
    let temporary = repository();
    let root = temporary.path();
    let reference = retain_attachment(root, "a/b", BINARY).unwrap();

    // The attachments directory itself is a link to a copy holding the bytes.
    let store = root.join("spec/evidence");
    let real = store.join("attachments");
    let moved = root.join("moved");
    std::fs::rename(&real, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &real).unwrap();
    let error = read_attachment(root, &reference).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentUnavailable);
    assert!(error.subject().contains("symlink"), "{error}");
    let error = retain_attachment(root, "a/b", BINARY).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentUnavailable);

    // The final component is a link to correct bytes: store refuses it too.
    std::fs::remove_file(&real).unwrap();
    std::fs::rename(&moved, &real).unwrap();
    let file = real.join(reference.digest_hex());
    let copy = root.join("copy.bin");
    std::fs::rename(&file, &copy).unwrap();
    std::os::unix::fs::symlink(&copy, &file).unwrap();
    let error = retain_attachment(root, "a/b", BINARY).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::AttachmentUnavailable);
}
