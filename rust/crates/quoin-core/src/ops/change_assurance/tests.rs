// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `change_assurance` domain, decided with no disk at all.
//!
//! Every test here runs the real operations against
//! `quoin_change_assurance::intake::memory::MemoryEvidenceStore`, which is what
//! makes the refusal, ceiling, mapping and verification paths reachable without
//! a filesystem — the property `tests/tc_library_containment.rs` enforces and
//! `src/capabilities.rs` explains.
//!
//! **These tests call the handlers directly and therefore prove nothing about
//! ROUTING.** A swapped match arm in `dispatch` is invisible from here. That is
//! the quoin#447 trap, and the answer to it is
//! `tests/tc_457_change_assurance_boundary.rs`, which hands every operation to
//! the real binary by its wire spelling.
//!
//! Nothing below recomputes what the retained TypeScript decided. The digests
//! asserted are read from `tests/fixtures/change-assurance-oracle.json`, which
//! was extracted from the capture `oracle/capture-change-assurance-oracle.mjs`
//! took against `src/change-assurance/` — so "the Rust reaches the same digest"
//! is a comparison against the oracle rather than against this crate's own
//! output (FR-101 AC-5).

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
use quoin_change_assurance::{ChangeAssuranceError, EvidenceStore, Published, RetainedPair};
use quoin_store::CanonicalDigest;
use serde_json::{Value, json};

use super::{
    MAX_INTAKE_BYTES, MAX_RECEIPT_BYTES, MAX_RECOVER_BYTES, MAX_SEAL_ATTESTATION_BYTES,
    MAX_SEAL_RECORD_BYTES, MAX_VERIFY_RECEIPT_BYTES, intake, receipt, recover, seal_attestation,
    seal_record, verify_receipt,
};
use crate::capabilities::{Capabilities, ChangeAssuranceHost};
use crate::error::CoreErrorCode;

/// One named case of the committed extract of the oracle capture.
fn case(name: &str) -> Value {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/change-assurance-oracle.json"
    ))
    .expect("the fixture is JSON");
    oracle["cases"]
        .as_array()
        .expect("the fixture holds cases")
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("the fixture has no case named {name}"))
        .clone()
}

/// The `everything-agrees` case, flattened into the single-attestation shape
/// most of these tests drive.
///
/// `attestation_body` is the SEALED attestation with the two members
/// `change_assurance.seal_attestation` derives removed, so a test can hand back
/// exactly what a caller would have written and compare the seal against the
/// oracle's.
fn oracle() -> Value {
    let mut fixture = case("everything-agrees");
    let sealed = fixture["attestations"][0]["attestation"].clone();
    let output = fixture["attestations"][0]["output"].clone();
    let mut body = sealed.clone();
    let object = body.as_object_mut().expect("an attestation is an object");
    let retained = object
        .remove("retained_output")
        .expect("a sealed attestation declares its retained output");
    object.remove("digest");
    fixture["attestation_body"] = body;
    fixture["attestation_media_type"] = retained["media_type"].clone();
    fixture["attestation_digest"] = sealed["digest"].clone();
    fixture["attestation_output_bytes"] = output;
    fixture
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

/// The oracle's retained output, as bytes.
fn output_bytes(fixture: &Value) -> Vec<u8> {
    fixture["attestation_output_bytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
        .collect()
}

/// One `MemoryEvidenceStore` shared across every `store()` call of one host.
///
/// `ChangeAssuranceHost::store` hands back a fresh handle per operation, which
/// is right for production — a process answers one operation and exits — and
/// wrong for a test that retains in one call and reads in the next. The handle
/// borrows instead of owning, so a test can drive a whole intake-then-verify
/// sequence against one store and then look at it.
struct SharedStore<'a>(&'a RefCell<MemoryEvidenceStore>);

impl EvidenceStore for SharedStore<'_> {
    fn record(&self, digest: &CanonicalDigest) -> Result<Option<Vec<u8>>, ChangeAssuranceError> {
        self.0.borrow().record(digest)
    }

    fn publish_record(
        &mut self,
        digest: &CanonicalDigest,
        bytes: &[u8],
    ) -> Result<Published, ChangeAssuranceError> {
        self.0.borrow_mut().publish_record(digest, bytes)
    }

    fn attestation(
        &self,
        digest: &CanonicalDigest,
    ) -> Result<Option<RetainedPair>, ChangeAssuranceError> {
        self.0.borrow().attestation(digest)
    }

    fn publish_attestation(
        &mut self,
        digest: &CanonicalDigest,
        pair: &RetainedPair,
    ) -> Result<Published, ChangeAssuranceError> {
        self.0.borrow_mut().publish_attestation(digest, pair)
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
            "seal_record",
            seal_record(&filler(MAX_SEAL_RECORD_BYTES), &capabilities).unwrap_err(),
        ),
        (
            "seal_attestation",
            seal_attestation(&filler(MAX_SEAL_ATTESTATION_BYTES)).unwrap_err(),
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
            verify_receipt(&filler(MAX_VERIFY_RECEIPT_BYTES)).unwrap_err(),
        ),
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
    let fixture = oracle();
    for (name, error) in [
        (
            "seal_record",
            seal_record(
                &json!({ "repo": ".", "record_hex": hex_json(&fixture["record_body"]) }),
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

// --- Sealing --------------------------------------------------------------

/// The sealed record reaches the digest the TypeScript oracle recorded, and it
/// is retained under it.
#[test]
fn a_record_seals_to_the_oracles_digest_and_is_retained() {
    let fixture = oracle();
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);

    let first = payload(
        seal_record(
            &json!({ "repo": "/repo", "record_hex": hex_json(&fixture["record_body"]) }),
            &capabilities,
        )
        .unwrap(),
    );
    assert_eq!(first["record"]["digest"], fixture["record_digest"]);
    assert_eq!(first["retained"], json!(true));
    assert_eq!(
        first["path"].as_str().unwrap(),
        format!(
            "/repo/spec/evidence/change-assurance/records/{}.json",
            fixture["record_digest"].as_str().unwrap()
        )
    );

    // Re-sealing byte-identical input succeeds and retains nothing new. That is
    // the contract `writeChangeRecord` had, and "already there" is not an error.
    let again = payload(
        seal_record(
            &json!({ "repo": "/repo", "record_hex": hex_json(&fixture["record_body"]) }),
            &capabilities,
        )
        .unwrap(),
    );
    assert_eq!(again["retained"], json!(false));
    assert_eq!(again["record"]["digest"], fixture["record_digest"]);
}

/// A body that supplies a member the seal derives is refused rather than
/// overwritten: a caller handing in a wrong digest must be told, not corrected.
#[test]
fn seal_refuses_a_body_that_supplies_a_derived_member() {
    let fixture = oracle();
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);

    let mut record = fixture["record_body"].clone();
    record["digest"] = json!("0".repeat(64));
    let error = seal_record(
        &json!({ "repo": "/repo", "record_hex": hex_json(&record) }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);

    let mut attestation = fixture["attestation_body"].clone();
    attestation["retained_output"] = json!({});
    let error = seal_attestation(&json!({
        "attestation_hex": hex_json(&attestation),
        "output_hex": hex(&output_bytes(&fixture)),
        "media_type": fixture["attestation_media_type"],
    }))
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.context["supplied"], "retained_output");
}

/// The only members `seal_attestation` derives are read off the output bytes,
/// and they reach the oracle's values — over bytes that are not valid UTF-8,
/// which is why they cross as hex rather than as text.
#[test]
fn seal_attestation_derives_the_retained_output_from_the_bytes() {
    let fixture = oracle();
    let bytes = output_bytes(&fixture);
    // The fixture's output is binary: it carries a NUL and a literal U+FFFD,
    // which is the byte sequence a lossy text decode MANUFACTURES. Carried as
    // text it would be indistinguishable from output that had been corrupted;
    // carried as hex the digest below is over the producer's own bytes.
    assert!(
        bytes.contains(&0),
        "the fixture's output must carry a NUL byte"
    );
    assert!(
        bytes.windows(3).any(|w| w == [0xef, 0xbf, 0xbd]),
        "the fixture's output must already carry a U+FFFD"
    );

    let sealed = payload(
        seal_attestation(&json!({
            "attestation_hex": hex_json(&fixture["attestation_body"]),
            "output_hex": hex(&bytes),
            "media_type": fixture["attestation_media_type"],
        }))
        .unwrap(),
    );
    assert_eq!(
        sealed["attestation"]["digest"],
        fixture["attestation_digest"]
    );
    assert_eq!(
        sealed["attestation"]["retained_output"]["size_bytes"],
        json!(bytes.len())
    );
    assert_eq!(
        sealed["attestation"]["retained_output"]["media_type"],
        fixture["attestation_media_type"]
    );
}

// --- Retaining and verifying ---------------------------------------------

/// The whole sequence a `quoin change-assurance receipt` run is: seal the
/// record, retain the attestation and its output, then verify — and the receipt
/// reaches the digest and the verdict the TypeScript oracle recorded.
#[test]
fn the_retained_evidence_verifies_to_the_oracles_receipt() {
    let fixture = oracle();
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let bytes = output_bytes(&fixture);

    let _ = payload(
        seal_record(
            &json!({ "repo": "/repo", "record_hex": hex_json(&fixture["record_body"]) }),
            &capabilities,
        )
        .unwrap(),
    );
    let sealed = payload(
        seal_attestation(&json!({
            "attestation_hex": hex_json(&fixture["attestation_body"]),
            "output_hex": hex(&bytes),
            "media_type": fixture["attestation_media_type"],
        }))
        .unwrap(),
    );
    let retained = payload(
        intake(
            &json!({
                "repo": "/repo",
                "attestation_hex": hex_json(&sealed["attestation"]),
                "output_hex": hex(&bytes),
            }),
            &capabilities,
        )
        .unwrap(),
    );
    assert_eq!(retained["size_bytes"], json!(bytes.len()));
    assert_eq!(retained["retained"], json!(true));
    assert_eq!(
        retained["directory"].as_str().unwrap(),
        format!(
            "/repo/spec/evidence/change-assurance/attestations/{}",
            fixture["attestation_digest"].as_str().unwrap()
        )
    );

    let verdict = payload(
        receipt(
            &json!({
                "repo": "/repo",
                "record_digest": fixture["record_digest"],
                "candidate_revision": fixture["candidate_revision"],
                "parent_digests": [],
                "selections": fixture["selections"],
                "decisions_hex": hex_json(&fixture["decision_history"]),
                "audits_hex": hex_json(&fixture["audits"]),
            }),
            &capabilities,
        )
        .unwrap(),
    );
    assert_eq!(verdict["receipt"]["outcome"], fixture["receipt_outcome"]);
    assert_eq!(verdict["receipt"]["digest"], fixture["receipt_digest"]);

    // And the receipt re-verifies as a document, which is the other command.
    let reverified =
        payload(verify_receipt(&json!({ "receipt_hex": hex_json(&verdict["receipt"]) })).unwrap());
    assert_eq!(reverified["receipt"]["digest"], fixture["receipt_digest"]);
}

/// A verdict is not a failure. An `invalid` receipt exits 0 with the receipt as
/// its payload; the exit-1 policy belongs to the caller that asked.
#[test]
fn a_verification_that_finds_nothing_retained_is_still_a_clean_answer() {
    let fixture = oracle();
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let _ = payload(
        seal_record(
            &json!({ "repo": "/repo", "record_hex": hex_json(&fixture["record_body"]) }),
            &capabilities,
        )
        .unwrap(),
    );

    let response = receipt(
        &json!({
            "repo": "/repo",
            "record_digest": fixture["record_digest"],
            "candidate_revision": fixture["candidate_revision"],
            "parent_digests": [],
            "selections": [],
            "decisions_hex": hex_json(&fixture["decision_history"]),
            "audits_hex": null,
        }),
        &capabilities,
    )
    .unwrap();
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
/// the digest, not reported as an empty verification.
#[test]
fn a_named_record_that_is_not_retained_is_refused_naming_it() {
    let fixture = oracle();
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let error = receipt(
        &json!({
            "repo": "/repo",
            "record_digest": "0".repeat(64),
            "candidate_revision": fixture["candidate_revision"],
            "parent_digests": [],
            "selections": [],
            "decisions_hex": hex_json(&fixture["decision_history"]),
            "audits_hex": null,
        }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["digest"], "0".repeat(64));
    assert_eq!(error.context["field"], "record_digest");
}

/// Intake refuses a pair whose output does not reproduce the attestation, and
/// the store is left holding nothing: a pair that verification could only ever
/// reject must not be filed.
#[test]
fn intake_refuses_output_that_contradicts_the_attestation() {
    let fixture = oracle();
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let sealed = payload(
        seal_attestation(&json!({
            "attestation_hex": hex_json(&fixture["attestation_body"]),
            "output_hex": hex(&output_bytes(&fixture)),
            "media_type": fixture["attestation_media_type"],
        }))
        .unwrap(),
    );

    let error = intake(
        &json!({
            "repo": "/repo",
            "attestation_hex": hex_json(&sealed["attestation"]),
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

    let digest =
        CanonicalDigest::parse_stored(fixture["attestation_digest"].as_str().unwrap()).unwrap();
    assert!(
        host.inner.borrow().attestation(&digest).unwrap().is_none(),
        "a refused intake retained the pair anyway"
    );
}

// --- The transport encoding ----------------------------------------------

/// The reason documents cross as hex: the strict reader runs over the bytes the
/// producer supplied, so a duplicate member is refused HERE. A request carrying
/// a pre-parsed value would have had that member resolved last-wins by whoever
/// parsed it, and this record would seal cleanly.
#[test]
fn a_duplicate_member_in_the_supplied_bytes_is_refused() {
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let duplicated = br#"{"record_id":"a","record_id":"b"}"#;
    let error = seal_record(
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
        let error = seal_record(
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

/// An edited receipt is refused rather than read back as fact.
#[test]
fn verify_receipt_refuses_an_edited_verdict() {
    let fixture = oracle();
    let host = TestHost::new();
    let capabilities = Capabilities::with_change_assurance(&host);
    let _ = payload(
        seal_record(
            &json!({ "repo": "/repo", "record_hex": hex_json(&fixture["record_body"]) }),
            &capabilities,
        )
        .unwrap(),
    );
    let bytes = output_bytes(&fixture);
    let sealed = payload(
        seal_attestation(&json!({
            "attestation_hex": hex_json(&fixture["attestation_body"]),
            "output_hex": hex(&bytes),
            "media_type": fixture["attestation_media_type"],
        }))
        .unwrap(),
    );
    let _ = payload(
        intake(
            &json!({
                "repo": "/repo",
                "attestation_hex": hex_json(&sealed["attestation"]),
                "output_hex": hex(&bytes),
            }),
            &capabilities,
        )
        .unwrap(),
    );
    let verdict = payload(
        receipt(
            &json!({
                "repo": "/repo",
                "record_digest": fixture["record_digest"],
                "candidate_revision": fixture["candidate_revision"],
                "parent_digests": [],
                "selections": fixture["selections"],
                "decisions_hex": hex_json(&fixture["decision_history"]),
                "audits_hex": hex_json(&fixture["audits"]),
            }),
            &capabilities,
        )
        .unwrap(),
    );

    let mut edited = verdict["receipt"].clone();
    assert_eq!(edited["outcome"], json!("valid"));
    edited["outcome"] = json!("invalid");
    let error = verify_receipt(&json!({ "receipt_hex": hex_json(&edited) })).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(
        error.context["change_assurance_code"],
        "QCA-RECEIPT-INVALID"
    );
}
