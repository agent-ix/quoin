// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `change_assurance` domain, decided with no disk at all.
//!
//! Every test here runs the real operations against
//! `quoin_change_assurance::intake::memory::MemoryEvidenceStore`, which is what
//! makes the refusal, ceiling, mapping and verification paths reachable without
//! a filesystem - the property `tests/tc_library_containment.rs` enforces and
//! `src/capabilities.rs` explains.
//!
//! **These tests call the handlers directly and therefore prove nothing about
//! ROUTING.** A swapped match arm in `dispatch` is invisible from here. That is
//! the quoin#447 trap, and the answer to it is
//! `tests/tc_457_change_assurance_boundary.rs`, which hands every operation to
//! the real binary by its wire spelling.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::cell::RefCell;
use std::path::Path;

use quoin_change_assurance::intake::memory::MemoryEvidenceStore;
use quoin_change_assurance::{
    AttestationId, ChangeAssuranceError, EvidenceStore, Published, RecordId, RetainedPair,
    Revision, Subject,
};
use serde_json::{Value, json};

use super::{
    MAX_INTAKE_BYTES, MAX_PREPARE_ATTESTATION_BYTES, MAX_RECEIPT_BYTES, MAX_RECOVER_BYTES,
    MAX_SCHEMA_BYTES, MAX_VERIFY_RECEIPT_BYTES, MAX_WRITE_RECORD_BYTES, intake,
    prepare_attestation, receipt, recover, schema, verify_receipt, write_record,
};
use crate::capabilities::{Capabilities, ChangeAssuranceHost};
use crate::error::CoreErrorCode;

/// The bytes the attestation's retained output holds.
const OUTPUT: &[u8] = b"all tests passed\n";

/// The record body a caller writes.
fn record_body() -> Value {
    json!({
        "schema_version": 1,
        "record_type": "change_assurance",
        "record_id": "change-1",
        "revision": 1,
        "subject": {"repository": "agent-ix/quoin", "base_revision": "abc123", "scope": ["src/evidence"]},
        "source_connections": [{"source_id": "FR-063", "kind": "requirement", "revision": "1"}],
        "impact_snapshot": {"identity": "impact-1", "revision": "1", "completeness": "complete", "truncated": false, "gaps": []},
        "definition": {
            "requirements": [{"id": "FR-063", "statement": "record it", "source_ids": ["FR-063"]}],
            "preservation_constraints": [],
            "proof_obligations": [{
                "proof_id": "proof-1",
                "statement": "tests pass",
                "obligation_ids": ["FR-063-AC-1"],
                "evidence_kind": "Unit",
                "command": {"argv": ["cargo", "test"], "working_directory": "."},
                "tool_identity": "cargo",
                "configuration_ref": "config/test.toml"
            }],
            "unknowns": []
        },
        "review_workflow": {"run_id": "run-1", "decision_event_kind": "change_assurance.review_decided"}
    })
}

/// The attestation body a caller writes, before `retained_output` is derived.
fn attestation_body() -> Value {
    json!({
        "schema_version": 1,
        "record_type": "proof_attestation",
        "attestation_id": "att-1",
        "record_id": "change-1",
        "record_revision": 1,
        "candidate_revision": "candidate-1",
        "proof_id": "proof-1",
        "command": {"argv": ["cargo", "test"], "working_directory": "."},
        "tool": {"identity": "cargo", "version": "1.90.0", "configuration_ref": "config/test.toml"},
        "environment": {"os": "test"},
        "observed_at": "2026-08-31T00:00:00Z",
        "result": "passed",
        "strength": "tested"
    })
}

/// An approving decision history.
fn decisions() -> Value {
    json!({
        "run_id": "run-1",
        "events": [{
            "id": "event-1", "ts": "2026-08-31T00:00:00.000Z", "sequence": 1,
            "actor": {"id": "reviewer-1", "kind": "human"},
            "kind": "change_assurance.review_decided",
            "payload": {"schema_version": 1, "record_id": "change-1", "revision": 1, "decision": "approved"}
        }]
    })
}

/// The audit report that finds the one proof healthy.
fn audits() -> Value {
    json!([{
        "proof_id": "proof-1",
        "report": {"findings": [], "healthy": ["FR-063-AC-1"], "unevaluated": []}
    }])
}

/// Lowercase hex of some bytes, the encoding every document crosses in.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, byte| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// Hex of a JSON document's compact bytes.
fn hex_json(value: &Value) -> String {
    hex(&serde_json::to_vec(value).unwrap())
}

/// One `MemoryEvidenceStore` shared across every `store()` call of one host.
///
/// `ChangeAssuranceHost::store` hands back a fresh handle per operation, which
/// is right for production - a process answers one operation and exits - and
/// wrong for a test that retains in one call and reads in the next. The handle
/// borrows instead of owning, so a test can drive a whole write-then-verify
/// sequence against one store and then look at it.
struct SharedStore<'a>(&'a RefCell<MemoryEvidenceStore>);

impl EvidenceStore for SharedStore<'_> {
    fn record(
        &self,
        record_id: &RecordId,
        revision: Revision,
    ) -> Result<Option<Vec<u8>>, ChangeAssuranceError> {
        self.0.borrow().record(record_id, revision)
    }

    fn publish_record(
        &mut self,
        record_id: &RecordId,
        revision: Revision,
        bytes: &[u8],
    ) -> Result<Published, ChangeAssuranceError> {
        self.0
            .borrow_mut()
            .publish_record(record_id, revision, bytes)
    }

    fn attestation(
        &self,
        attestation_id: &AttestationId,
    ) -> Result<Option<RetainedPair>, ChangeAssuranceError> {
        self.0.borrow().attestation(attestation_id)
    }

    fn publish_attestation(
        &mut self,
        attestation_id: &AttestationId,
        pair: &RetainedPair,
    ) -> Result<Published, ChangeAssuranceError> {
        self.0
            .borrow_mut()
            .publish_attestation(attestation_id, pair)
    }

    fn recover_staging(&mut self) -> Result<usize, ChangeAssuranceError> {
        self.0.borrow_mut().recover_staging()
    }
}

/// A host granting one in-memory store, whatever repository is named.
struct TestHost {
    inner: RefCell<MemoryEvidenceStore>,
}

impl TestHost {
    fn new() -> Self {
        Self {
            inner: RefCell::new(MemoryEvidenceStore::new()),
        }
    }
}

impl ChangeAssuranceHost for TestHost {
    fn store<'a>(&'a self, _repo: &Path) -> Box<dyn EvidenceStore + 'a> {
        Box::new(SharedStore(&self.inner))
    }
}

/// The payload of a successful operation.
fn payload(response: crate::protocol::Response) -> Value {
    assert_eq!(
        response.outcome.code(),
        0,
        "diagnostics: {:?}",
        response.diagnostics
    );
    response.payload
}

/// Write the record, prepare and take in the attestation, so a receipt can be
/// requested over them.
fn retain_everything(capabilities: &Capabilities<'_>) {
    let _ = payload(
        write_record(
            &json!({ "repo": "/repo", "record_hex": hex_json(&record_body()) }),
            capabilities,
        )
        .unwrap(),
    );
    let prepared = payload(
        prepare_attestation(&json!({
            "attestation_hex": hex_json(&attestation_body()),
            "output_hex": hex(OUTPUT),
            "media_type": "text/plain",
        }))
        .unwrap(),
    );
    let _ = payload(
        intake(
            &json!({
                "repo": "/repo",
                "attestation_hex": hex_json(&prepared["attestation"]),
                "output_hex": hex(OUTPUT),
            }),
            capabilities,
        )
        .unwrap(),
    );
}

/// The receipt request every test varies.
fn receipt_request() -> Value {
    json!({
        "repo": "/repo",
        "record_id": "change-1",
        "revision": 1,
        "candidate_revision": "candidate-1",
        "selections": [{"proof_id": "proof-1", "attestation_id": "att-1"}],
        "decisions_hex": hex_json(&decisions()),
        "audits_hex": hex_json(&audits()),
    })
}

// --- Ceilings -------------------------------------------------------------

/// Every operation's whole-request bound is applied, and applied BEFORE the
/// store is consulted: the host below grants nothing, so an operation that
/// reached one would fault internally (4) rather than refuse (2).
#[test]
fn every_operation_refuses_a_request_past_its_own_ceiling() {
    let capabilities = Capabilities::none();
    let filler = |limit: usize| json!({ "padding": "x".repeat(limit) });

    for (name, error) in [
        (
            "write_record",
            write_record(&filler(MAX_WRITE_RECORD_BYTES), &capabilities).unwrap_err(),
        ),
        (
            "prepare_attestation",
            prepare_attestation(&filler(MAX_PREPARE_ATTESTATION_BYTES)).unwrap_err(),
        ),
        (
            "intake",
            intake(&filler(MAX_INTAKE_BYTES), &capabilities).unwrap_err(),
        ),
        (
            "recover",
            recover(&filler(MAX_RECOVER_BYTES), &capabilities).unwrap_err(),
        ),
        (
            "receipt",
            receipt(&filler(MAX_RECEIPT_BYTES), &capabilities).unwrap_err(),
        ),
        (
            "verify_receipt",
            verify_receipt(&filler(MAX_VERIFY_RECEIPT_BYTES), &capabilities).unwrap_err(),
        ),
        ("schema", schema(&filler(MAX_SCHEMA_BYTES)).unwrap_err()),
    ] {
        assert_eq!(error.code, CoreErrorCode::Refused, "{name}");
        assert_eq!(error.outcome().code(), 2, "{name}");
        assert_eq!(error.context["op"], format!("change_assurance.{name}"));
    }
}

/// A scalar past its own bound is refused naming the field, not the whole
/// request: the two bounds catch different shapes of oversized input.
#[test]
fn a_repository_path_past_the_scalar_bound_is_refused_naming_the_field() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let error = recover(
        &json!({ "repo": "r".repeat(super::MAX_SCALAR_BYTES + 1) }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["field"], "repo");
}

// --- The grant ------------------------------------------------------------

/// An operation that needs the store and was granted none reports a BUILD
/// fault, not a refusal: the caller did nothing wrong.
#[test]
fn an_operation_that_needs_a_store_faults_internally_when_granted_none() {
    let none = Capabilities::none();
    for (name, error) in [
        (
            "write_record",
            write_record(
                &json!({ "repo": ".", "record_hex": hex_json(&record_body()) }),
                &none,
            )
            .unwrap_err(),
        ),
        (
            "recover",
            recover(&json!({ "repo": "." }), &none).unwrap_err(),
        ),
    ] {
        assert_eq!(error.code, CoreErrorCode::Io, "{name}");
        assert_eq!(error.outcome().code(), 4, "{name}");
        assert_eq!(error.context["op"], format!("change_assurance.{name}"));
    }
}

// --- Writing and preparing -------------------------------------------------

/// A record is retained at its `(record_id, revision)`, and writing it again is
/// not an error.
#[test]
fn a_record_is_retained_under_its_identity_and_revision() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let request = json!({ "repo": "/repo", "record_hex": hex_json(&record_body()) });
    let first = payload(write_record(&request, &capabilities).unwrap());
    assert_eq!(first["retained"], json!(true));
    assert!(first["path"].as_str().unwrap().ends_with("change-1/1.json"));
    assert!(first["record"].get("digest").is_none());

    let again = payload(write_record(&request, &capabilities).unwrap());
    assert_eq!(again["retained"], json!(false));
}

/// A body that carries a `digest` is refused: it is an undeclared member.
#[test]
fn write_refuses_a_body_that_supplies_a_digest() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let mut body = record_body();
    body["digest"] = json!("a".repeat(64));
    let error = write_record(
        &json!({ "repo": "/repo", "record_hex": hex_json(&body) }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.context["change_assurance_code"], "QCA-RECORD-INVALID");
}

/// The size is read off the output bytes; a caller-written `retained_output`
/// is refused rather than overwritten.
#[test]
fn prepare_attestation_derives_the_retained_output_size_from_the_bytes() {
    let prepared = payload(
        prepare_attestation(&json!({
            "attestation_hex": hex_json(&attestation_body()),
            "output_hex": hex(OUTPUT),
            "media_type": "text/plain",
        }))
        .unwrap(),
    );
    assert_eq!(
        prepared["attestation"]["retained_output"],
        json!({"media_type": "text/plain", "size_bytes": OUTPUT.len()})
    );

    let mut supplied = attestation_body();
    supplied["retained_output"] = json!({"media_type": "text/plain", "size_bytes": 1});
    let error = prepare_attestation(&json!({
        "attestation_hex": hex_json(&supplied),
        "output_hex": hex(OUTPUT),
        "media_type": "text/plain",
    }))
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
}

// --- Receipts --------------------------------------------------------------

/// Everything agreeing verifies `valid`, and the receipt re-derives byte for
/// byte; an edited receipt is refused.
#[test]
fn the_receipt_re_derives_and_an_edited_one_is_refused() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    retain_everything(&capabilities);

    let verdict = payload(receipt(&receipt_request(), &capabilities).unwrap());
    let stored = verdict["receipt"].clone();
    assert_eq!(stored["outcome"], json!("valid"), "{stored}");

    let verified = payload(
        verify_receipt(
            &json!({ "receipt_hex": hex_json(&stored), "inputs": receipt_request() }),
            &capabilities,
        )
        .unwrap(),
    );
    assert_eq!(verified["receipt"], stored);

    let mut edited = stored;
    edited["candidate_revision"] = json!("candidate-2");
    let error = verify_receipt(
        &json!({ "receipt_hex": hex_json(&edited), "inputs": receipt_request() }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(
        error.context["change_assurance_code"],
        "QCA-RECEIPT-MISMATCH"
    );
}

/// A verdict is not a failure. An incomplete receipt exits 0 with the receipt
/// as its payload; the exit-1 policy belongs to the caller that asked.
#[test]
fn a_verification_that_finds_no_attestation_selected_is_still_a_clean_answer() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    retain_everything(&capabilities);
    let mut request = receipt_request();
    request["selections"] = json!([]);

    let response = receipt(&request, &capabilities).unwrap();
    assert_eq!(response.outcome.code(), 0);
    assert_ne!(response.payload["receipt"]["outcome"], json!("valid"));
    assert!(
        !response.payload["receipt"]["reasons"]
            .as_array()
            .unwrap()
            .is_empty(),
        "an incomplete receipt names why"
    );
}

/// Evidence the caller named and the store does not hold is REFUSED (2) naming
/// the identity, not reported as an empty verification.
#[test]
fn a_named_record_that_is_not_retained_is_refused_naming_it() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let error = receipt(&receipt_request(), &capabilities).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["field"], "record_id");
    assert_eq!(error.context["id"], "change-1/1");
}

/// Output whose size contradicts the attestation is refused BEFORE anything is
/// retained: a pair that verification could only ever reject must not be
/// filed.
#[test]
fn intake_refuses_output_that_contradicts_the_attestation() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let prepared = payload(
        prepare_attestation(&json!({
            "attestation_hex": hex_json(&attestation_body()),
            "output_hex": hex(OUTPUT),
            "media_type": "text/plain",
        }))
        .unwrap(),
    );

    let error = intake(
        &json!({
            "repo": "/repo",
            "attestation_hex": hex_json(&prepared["attestation"]),
            "output_hex": hex(b"different bytes entirely"),
        }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(
        error.context["change_assurance_code"],
        "QCA-OUTPUT-INTEGRITY-MISMATCH"
    );

    let id = AttestationId::parse("att-1", Subject::Attestation, "attestation_id").unwrap();
    assert!(
        host.inner.borrow().attestation(&id).unwrap().is_none(),
        "a refused intake retained the pair anyway"
    );
}

// --- The transport encoding ----------------------------------------------

/// The reason documents cross as hex: the strict reader runs over the bytes the
/// producer supplied, so a duplicate member is refused HERE. A request carrying
/// a pre-parsed value would have had that member resolved last-wins by whoever
/// parsed it, and this record would be written cleanly.
#[test]
fn a_duplicate_member_in_the_supplied_bytes_is_refused() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let duplicated = br#"{"record_id":"a","record_id":"b"}"#;
    let error = write_record(
        &json!({ "repo": "/repo", "record_hex": hex(duplicated) }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.context["field"], "record_hex");
}

/// A field that is not hexadecimal, or is of odd length, is a bad request and
/// says which field.
#[test]
fn a_field_that_is_not_hexadecimal_names_itself() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    for value in ["zz", "abc"] {
        let error = write_record(
            &json!({ "repo": "/repo", "record_hex": value }),
            &capabilities,
        )
        .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest, "{value}");
        assert_eq!(error.context["field"], "record_hex", "{value}");
    }
}

/// An undeclared request member is refused rather than dropped: a field the
/// boundary ignores is a field the caller believes it sent.
#[test]
fn an_undeclared_request_member_is_refused() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let error = recover(
        &json!({ "repo": "/repo", "repository": "/repo" }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
}

/// Recovering staging reports what it removed, over a store that actually had
/// some: a count asserted over an empty population is a check that passes
/// whatever the code does.
#[test]
fn recover_reports_the_staging_it_removed() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    host.inner.borrow_mut().strand(
        ".tmp-attestation-1",
        RetainedPair {
            attestation: b"{}".to_vec(),
            output: b"x".to_vec(),
        },
    );
    host.inner.borrow_mut().strand(
        ".tmp-attestation-2",
        RetainedPair {
            attestation: b"{}".to_vec(),
            output: b"y".to_vec(),
        },
    );
    assert_eq!(host.inner.borrow().staged(), 2);

    let removed = payload(recover(&json!({ "repo": "/repo" }), &capabilities).unwrap());
    assert_eq!(removed["removed"], json!(2));
    assert_eq!(host.inner.borrow().staged(), 0);
}

/// Trace: FR-068-AC-8
/// Provenance: quoin#503
///
/// Asked for nothing in particular, the operation answers with the vocabulary
/// and no asset. That is the `quoin change-assurance schema` listing, and it
/// is what tells a caller which names the flag will accept.
#[test]
fn tc_503_the_vocabulary_comes_back_without_an_asset() {
    let response = schema(&json!({})).expect("listing needs no capability");
    let payload = &response.payload;
    assert_eq!(
        payload["schemas"],
        json!([
            "change-assurance-record-v1.schema.json",
            "proof-attestation-v1.schema.json",
            "verification-receipt-v1.schema.json"
        ])
    );
    assert_eq!(payload["schema"], Value::Null);
}

/// Trace: FR-068-AC-8
/// Provenance: quoin#503
///
/// A named asset comes back as the exact text compiled into
/// `quoin-change-assurance`, trailing newline included. Byte-for-byte is the
/// contract: a consumer validates against the same file the writing code was
/// written against.
#[test]
fn tc_503_a_named_asset_comes_back_byte_for_byte() {
    let response = schema(&json!({ "name": "proof-attestation-v1.schema.json" }))
        .expect("a known name resolves");
    assert_eq!(
        response.payload["schema"],
        json!(quoin_change_assurance::schemas::PROOF_ATTESTATION_V1)
    );
}

/// Trace: FR-068-AC-8
/// Provenance: quoin#503
///
/// An unknown name is a caller mistake reported as one, naming what this build
/// does ship. Answering with some other schema would validate a document
/// against a contract nobody asked for.
#[test]
fn tc_503_an_unknown_asset_name_is_refused_naming_the_known_ones() {
    let error = schema(&json!({ "name": "change-assurance-record-v2.schema.json" }))
        .expect_err("an unknown name is refused");
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.context["op"], "change_assurance.schema");
    assert_eq!(
        error.context["name"],
        "change-assurance-record-v2.schema.json"
    );
    assert!(
        error.context["known"].contains("change-assurance-record-v1.schema.json"),
        "the refusal must name what this build ships, said {}",
        error.context["known"]
    );
}
