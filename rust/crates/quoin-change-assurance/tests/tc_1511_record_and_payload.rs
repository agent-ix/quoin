// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The v2 record policy, the v2 decision payload and the record digest
//! (FR-063).
//!
//! A claim in a decision payload is data the event's writer supplied. Every
//! fixture here is hand-built and fictional.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use ix_trace_rs::trace;

use quoin_change_assurance::error::Subject;
use quoin_change_assurance::ids::{RecordId, Revision};
use quoin_change_assurance::intake::EvidenceStore;
use quoin_change_assurance::intake::memory::MemoryEvidenceStore;
use quoin_change_assurance::model::outcome::Outcome;
use quoin_change_assurance::records::record_bytes;
use quoin_change_assurance::verify::verify_change_assurance;
use quoin_change_assurance::{
    prepare_change_record, read_change_record_json, read_change_record_with_digest, record_digest,
    write_change_record,
};
use quoin_store::JsonValue;

use crate::common::{base_input, digest_of, history, json, record_text, replacing};

const OFF: &str = r#"{"required": false, "signature_kinds": [], "approvers": []}"#;

fn record_with(policy: &str) -> JsonValue {
    json(&record_text().replace(OFF, policy))
}

fn ids() -> (RecordId, Revision) {
    (
        RecordId::parse("change-1", Subject::Record, "record_id").unwrap(),
        Revision::new(1, Subject::Record, "revision").unwrap(),
    )
}

/// The scenario with one event whose payload ends with `tail`.
fn decided(decision: &str, tail: &str) -> quoin_change_assurance::VerificationInput {
    let mut input = base_input();
    let event = format!(
        r#"{{"id": "event-1", "ts": "2026-08-31T00:00:00.000Z", "sequence": 1,
          "actor": {{"id": "reviewer-1", "kind": "human"}}, "kind": "change_assurance.review_decided",
          "payload": {{"schema_version": 2, "record_id": "change-1", "revision": 1, "decision": "{decision}"{tail}}}}}"#
    );
    input.decision_history = Some(history(&[event]));
    input
}

fn approved_claim(digest: &str) -> String {
    format!(
        r#"{{"protocol": "engineering-assurance.approval-verify-result/v1", "status": "approved", "tag": "t-1", "record_id": "change-1", "record_digest": "{digest}", "trust": {{"allowed_signers": "/keys/allowed"}}, "approver": "a@example.test", "meaning": ["reviewed"], "signature_kind": "ssh"}}"#
    )
}

/// Whether verification found exactly one matching, intact decision event.
fn is_candidate(input: &quoin_change_assurance::VerificationInput) -> bool {
    let receipt = verify_change_assurance(input).unwrap();
    !receipt
        .reasons
        .iter()
        .any(|reason| reason.as_str() == "decision_mismatch")
}

/// A well-formed claim on an `approved` decision is a candidate; each of an
/// undeclared member, another protocol, a missing member, an empty `meaning`,
/// a member outside its status's row and a claim on `rejected` or `revise` is
/// refused as a candidate.
#[trace("FR-063-AC-14")]
#[test]
fn tc_1511_only_a_closed_well_formed_claim_on_an_approval_is_a_candidate() {
    let digest = digest_of(&json(&record_text()));
    let good = approved_claim(&digest);
    let tail = |claim: &str| format!(r#", "verified_approval": {claim}"#);
    assert!(is_candidate(&decided("approved", &tail(&good))));

    let gitsign = good.replace(
        r#"{"allowed_signers": "/keys/allowed"}"#,
        r#"{"allowed_signers": "/keys/allowed", "gitsign": {"certificate_identity": "a@example.test", "certificate_oidc_issuer": "https://issuer.example.test", "principal": "a"}}"#,
    );
    assert!(is_candidate(&decided("approved", &tail(&gitsign))));

    let refused = [
        good.replace(r#""tag": "t-1""#, r#""tag": "t-1", "extra": 1"#),
        good.replace("approval-verify-result/v1", "approval-verify-result/v2"),
        good.replace(r#""trust": {"allowed_signers": "/keys/allowed"}, "#, ""),
        good.replace(r#", "approver": "a@example.test""#, ""),
        good.replace(r#", "meaning": ["reviewed"]"#, ""),
        good.replace(r#", "signature_kind": "ssh""#, ""),
        good.replace(r#"["reviewed"]"#, "[]"),
        good.replace(r#""signature_kind": "ssh""#, r#""signature_kind": "pgp""#),
        good.replace(r#""status": "approved""#, r#""status": "approvedish""#),
        good.replace(
            r#""allowed_signers": "/keys/allowed""#,
            r#""allowed_signers": """#,
        ),
        good.replace(&digest, ""),
        good.replace(&digest, &"d".repeat(257)),
        good.replace(r#""record_id": "change-1""#, r#""record_id": "-bad""#),
        gitsign.replace(r#""principal": "a""#, r#""principal": """#),
        // A non-approved status without its `trust`.
        good.replace(r#""trust": {"allowed_signers": "/keys/allowed"}, "#, "")
            .replace(r#""status": "approved""#, r#""status": "signed_bad""#)
            .replace(r#", "approver": "a@example.test""#, "")
            .replace(r#", "meaning": ["reviewed"]"#, "")
            .replace(r#", "signature_kind": "ssh""#, ""),
        // A member outside its row: `approver` on a status that is not approved.
        good.replace(r#""status": "approved""#, r#""status": "signed_bad""#),
    ];
    for claim in &refused {
        assert!(
            !is_candidate(&decided("approved", &tail(claim))),
            "{claim} must be refused as a candidate"
        );
    }
    for decision in ["rejected", "revise"] {
        assert!(
            !is_candidate(&decided(decision, &tail(&good))),
            "a claim on {decision} must be refused as a candidate"
        );
    }
    // The same non-approved status with its row complete is a candidate.
    let bad_signature = good
        .replace(r#""status": "approved""#, r#""status": "signed_bad""#)
        .replace(r#", "approver": "a@example.test""#, "")
        .replace(r#", "meaning": ["reviewed"]"#, "")
        .replace(r#", "signature_kind": "ssh""#, "");
    assert!(is_candidate(&decided("approved", &tail(&bad_signature))));
    let at_limit = good.replace(&digest, &"d".repeat(256));
    assert!(is_candidate(&decided("approved", &tail(&at_limit))));
}

/// A payload `schema_version` other than 2 cannot approve the revision.
#[trace("FR-063-AC-10")]
#[test]
fn tc_1511_a_payload_that_is_not_version_two_cannot_approve() {
    for version in ["1", "3", "\"2\""] {
        let mut input = decided("approved", "");
        let wrong = format!("\"schema_version\": {version}");
        let history_text = common::event_text("event-1", 1, "change_assurance.review_decided")
            .replace("\"schema_version\": 2", &wrong);
        input.decision_history = Some(history(&[history_text]));
        let receipt = verify_change_assurance(&input).unwrap();
        assert_eq!(receipt.outcome, Outcome::Invalid, "version {version}");
        assert!(receipt.decision_event.is_none());
    }
}

/// The record digest is `sha256:` and the lowercase hex of the stored bytes.
#[trace("FR-063-AC-15")]
#[test]
fn tc_1511_the_digest_is_sha256_of_the_stored_bytes() {
    assert_eq!(
        record_digest(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        record_digest(b""),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

/// Two writes of one body store identical bytes and report one digest; a
/// one-byte change reports another; the record body, the schema and the stored
/// file hold no digest member.
#[trace("FR-063-AC-15")]
#[test]
fn tc_1511_the_digest_is_stable_across_writes_and_sensitive_to_a_byte() {
    let record = prepare_change_record(&json(&record_text())).unwrap();
    let (id, revision) = ids();
    let mut first = MemoryEvidenceStore::new();
    let mut second = MemoryEvidenceStore::new();
    write_change_record(&mut first, &record).unwrap();
    write_change_record(&mut second, &record).unwrap();
    let stored = first.record(&id, revision).unwrap().unwrap();
    assert_eq!(Some(stored.clone()), second.record(&id, revision).unwrap());
    let (_, digest) = read_change_record_with_digest(&first, &id, revision)
        .unwrap()
        .unwrap();
    let (_, again) = read_change_record_with_digest(&second, &id, revision)
        .unwrap()
        .unwrap();
    assert_eq!(digest, again);
    assert_eq!(digest, record_digest(&stored));
    assert!(digest.starts_with("sha256:") && digest.len() == "sha256:".len() + 64);

    let changed =
        prepare_change_record(&json(&record_text().replace("record it", "record it!"))).unwrap();
    assert_ne!(record_digest(&record_bytes(&changed).unwrap()), digest);

    let text = String::from_utf8(stored).unwrap();
    assert!(!text.contains("digest"), "{text}");
    assert!(!quoin_change_assurance::schemas::CHANGE_ASSURANCE_RECORD_V2.contains("digest"));
    assert!(
        read_change_record_json(&replacing(
            &json(&record_text()),
            "digest",
            JsonValue::string(digest)
        ))
        .is_err()
    );
}

/// The digest hashes the bytes as stored: a trailing newline or a CRLF that a
/// checkout might add changes it, and nothing is normalized.
#[trace("FR-063-AC-15")]
#[test]
fn tc_1511_the_digest_applies_no_newline_or_end_of_line_normalization() {
    let record = prepare_change_record(&json(&record_text())).unwrap();
    let canonical = record_bytes(&record).unwrap();
    let (id, revision) = ids();
    let mut digests = vec![record_digest(&canonical)];
    for suffix in [&b"\n"[..], &b"\r\n"[..]] {
        let mut stored = canonical.clone();
        stored.extend_from_slice(suffix);
        let mut store = MemoryEvidenceStore::new();
        store.publish_record(&id, revision, &stored).unwrap();
        let (_, digest) = read_change_record_with_digest(&store, &id, revision)
            .unwrap()
            .unwrap();
        assert_eq!(digest, record_digest(&stored));
        digests.push(digest);
    }
    digests.sort();
    digests.dedup();
    assert_eq!(
        digests.len(),
        3,
        "each spelling of the stored bytes has its own digest"
    );
}

/// A claim bound to the digest of the canonical bytes no longer matches once
/// the stored bytes carry a trailing newline: the verification input carries
/// what the reader hashed.
#[trace("FR-063-AC-15", "FR-065-AC-14")]
#[test]
fn tc_1511_a_changed_stored_byte_breaks_a_claim_bound_to_the_old_digest() {
    let claim = approved_claim(&digest_of(&json(&record_text())));
    let mut input = decided("approved", &format!(r#", "verified_approval": {claim}"#));
    assert_eq!(
        verify_change_assurance(&input).unwrap().outcome,
        Outcome::Valid
    );
    input.record_digest = record_digest(b"the record as a checkout rewrote it");
    let receipt = verify_change_assurance(&input).unwrap();
    assert_eq!(receipt.outcome, Outcome::Invalid);
    assert_eq!(
        receipt
            .reasons
            .iter()
            .map(|r| r.as_str())
            .collect::<Vec<_>>(),
        ["approval_record_mismatch"]
    );
}

/// Policy shapes: `required` true takes a kind, `required` false takes none, a
/// kind is unique and known, an approver is a unique non-empty string, and the
/// block is mandatory.
#[trace("FR-063-AC-16")]
#[test]
fn tc_1511_the_policy_is_closed_and_narrowing_only() {
    for good in [
        OFF,
        r#"{"required": true, "signature_kinds": ["ssh"], "approvers": []}"#,
        r#"{"required": true, "signature_kinds": ["gitsign", "ssh"], "approvers": ["a", "b"]}"#,
    ] {
        read_change_record_json(&record_with(good)).unwrap();
    }
    for bad in [
        r#"{"required": true, "signature_kinds": [], "approvers": []}"#,
        r#"{"required": false, "signature_kinds": ["ssh"], "approvers": []}"#,
        r#"{"required": false, "signature_kinds": [], "approvers": ["a"]}"#,
        r#"{"required": true, "signature_kinds": ["ssh", "ssh"], "approvers": []}"#,
        r#"{"required": true, "signature_kinds": ["pgp"], "approvers": []}"#,
        r#"{"required": true, "signature_kinds": ["ssh"], "approvers": [""]}"#,
        r#"{"required": true, "signature_kinds": ["ssh"], "approvers": ["a", "a"]}"#,
        // Unsorted lists are refused like every other record array.
        r#"{"required": true, "signature_kinds": ["ssh", "gitsign"], "approvers": []}"#,
        r#"{"required": true, "signature_kinds": ["ssh"], "approvers": ["b", "a"]}"#,
        r#"{"required": true, "signature_kinds": ["ssh"], "approvers": [], "allowed_signers": "f"}"#,
        r#"{"required": true, "signature_kinds": ["ssh"]}"#,
        r#"{"required": "yes", "signature_kinds": [], "approvers": []}"#,
    ] {
        assert!(
            read_change_record_json(&record_with(bad)).is_err(),
            "{bad} must be refused"
        );
    }
    let without = record_text().replace(&format!(r#", "signed_approval": {OFF}"#), "");
    assert!(read_change_record_json(&json(&without)).is_err());
    let v1 = record_text().replace("\"schema_version\": 2", "\"schema_version\": 1");
    assert!(read_change_record_json(&json(&v1)).is_err());
}

/// Changing `signed_approval` changes the record digest, and the members are
/// written in sorted order whatever order the author listed them in.
#[trace("FR-063-AC-16")]
#[test]
fn tc_1511_the_policy_is_part_of_the_digest_and_written_sorted() {
    let digests: Vec<String> = [
        OFF,
        r#"{"required": true, "signature_kinds": ["ssh"], "approvers": []}"#,
        r#"{"required": true, "signature_kinds": ["ssh"], "approvers": ["a"]}"#,
    ]
    .iter()
    .map(|policy| digest_of(&record_with(policy)))
    .collect();
    assert_eq!(
        digests
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );

    let unsorted =
        r#"{"required": true, "signature_kinds": ["ssh", "gitsign"], "approvers": ["b", "a"]}"#;
    let written = prepare_change_record(&record_with(unsorted)).unwrap();
    let text = String::from_utf8(record_bytes(&written).unwrap()).unwrap();
    assert!(
        text.contains(
            r#""approvers":["a","b"],"required":true,"signature_kinds":["gitsign","ssh"]"#
        ),
        "{text}"
    );
}

/// The reader accepts v2 and a record without the policy is not v2.
#[trace("FR-063-AC-1")]
#[test]
fn tc_1511_a_v2_record_requires_its_signed_approval_policy() {
    let record = read_change_record_json(&json(&record_text())).unwrap();
    assert!(!record.review_workflow.signed_approval.required);
    assert!(
        record
            .review_workflow
            .signed_approval
            .signature_kinds
            .is_empty()
    );
    let digestful = replacing(&json(&record_text()), "digest", JsonValue::string("x"));
    assert!(prepare_change_record(&digestful).is_err());
}
