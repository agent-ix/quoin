// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Retained proof content is bound by content, not only by length (PLAT-1788,
//! FR-064-AC-3, FR-065-AC-23).
//!
//! Every rejection here has a healthy control in the same test: the intact
//! pair verifies, then one thing changes. Removing the commitment check in
//! `verify::proofs` or `intake` makes these fail.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::fmt::Write as _;

use ix_trace_rs::trace;
use quoin_change_assurance::attestations::attestation_bytes;
use quoin_change_assurance::error::{ChangeAssuranceError, Subject};
use quoin_change_assurance::ids::AttestationId;
use quoin_change_assurance::ids::{ProofId, RecordId, Revision};
use quoin_change_assurance::intake::disk::DiskEvidenceStore;
use quoin_change_assurance::intake::memory::MemoryEvidenceStore;
use quoin_change_assurance::proof_integrity::ProofIntegrityError;
use quoin_change_assurance::proof_integrity::ProofIntegrityStatement;
use quoin_change_assurance::verify::input::{Selection, VerificationInput};
use quoin_change_assurance::verify::verify_change_assurance;
use quoin_change_assurance::{
    Outcome, ProofAttestation, Reason, VerificationReceipt, intake_attestation, read_attestation,
};
use quoin_store::{JsonValue, canonical_bytes};

use crate::common::{
    OUTPUT, attestation_text, base_input, json, member, names, replacing, sealed, without,
};

fn verify(input: &VerificationInput) -> VerificationReceipt {
    verify_change_assurance(input).expect("a verification always produces a receipt")
}

fn reasons(receipt: &VerificationReceipt) -> Vec<Reason> {
    receipt.proofs[0].reasons.clone()
}

/// The scenario attestation, sealed over `output` after `edit`.
fn input_with(edit: impl Fn(&JsonValue) -> JsonValue, output: &[u8]) -> VerificationInput {
    let mut input = base_input();
    input.attestations[0].attestation = sealed(&edit(&json(&attestation_text())), output);
    input.attestations[0].output = Some(output.to_vec());
    input
}

fn att_id() -> AttestationId {
    AttestationId::parse("att-1", Subject::Attestation, "attestation_id").unwrap()
}

fn corrupted() -> Vec<u8> {
    let mut bytes = OUTPUT.to_vec();
    bytes[0] ^= 0x01;
    assert_eq!(bytes.len(), OUTPUT.len());
    bytes
}

/// The statement binds exact canonical attestation bytes and exact output bytes.
#[trace("FR-210-AC-1", "FR-210-CON-3")]
#[test]
fn tc_1788_proof_integrity_statement_binds_the_selected_exact_bytes() {
    let attestation = ProofAttestation::from_json(&json(&attestation_text())).unwrap();
    let exact_attestation = attestation_bytes(&attestation).unwrap();
    let record_id = RecordId::parse("change-1", Subject::Record, "record_id").unwrap();
    let proof_id = ProofId::parse("proof-1", Subject::Attestation, "proof_id").unwrap();
    let revision = Revision::new(1, Subject::Record, "record_revision").unwrap();
    let statement = ProofIntegrityStatement::create(
        &record_id,
        revision,
        "candidate-1",
        &proof_id,
        &attestation,
        &exact_attestation,
        OUTPUT,
    )
    .unwrap();
    assert_eq!(statement.record_id(), "change-1");
    assert_eq!(statement.record_revision(), 1);
    assert_eq!(statement.candidate_revision(), "candidate-1");
    assert_eq!(statement.proof_id(), "proof-1");
    assert_eq!(statement.attestation_id(), "att-1");
    assert_eq!(statement.output_media_type(), "text/plain");
    assert_eq!(
        statement.output_size_bytes(),
        u64::try_from(OUTPUT.len()).unwrap()
    );
    assert_eq!(
        statement.attestation_sha256(),
        format!(
            "sha256:{}",
            quoin_store::attachment::sha256_hex(&exact_attestation)
        )
    );
    assert_eq!(
        statement.output_sha256(),
        format!("sha256:{}", quoin_store::attachment::sha256_hex(OUTPUT))
    );

    let changed_attestation = ProofAttestation::from_json(&replacing(
        &json(&attestation_text()),
        "attestation_id",
        JsonValue::string("att-substitute"),
    ))
    .unwrap();
    let changed = attestation_bytes(&changed_attestation).unwrap();
    assert_eq!(
        ProofIntegrityStatement::create(
            &record_id,
            revision,
            "candidate-1",
            &proof_id,
            &attestation,
            &changed,
            OUTPUT,
        ),
        Err(ProofIntegrityError::AttestationBytesMismatch),
        "typed attestation metadata cannot be paired with bytes for another attestation"
    );
    let parsed = ProofIntegrityStatement::from_canonical_bytes(statement.as_bytes()).unwrap();
    assert_eq!(parsed, statement);
}

/// Provenance: PLAT-1801
#[trace("FR-210-AC-1", "FR-210-CON-3")]
#[test]
fn tc_1788_statement_reader_refuses_duplicate_and_unknown_members() {
    let attestation = ProofAttestation::from_json(&json(&attestation_text())).unwrap();
    let record_id = RecordId::parse("change-1", Subject::Record, "record_id").unwrap();
    let proof_id = ProofId::parse("proof-1", Subject::Attestation, "proof_id").unwrap();
    let revision = Revision::new(1, Subject::Record, "record_revision").unwrap();
    let exact_attestation = attestation_bytes(&attestation).unwrap();
    let statement = ProofIntegrityStatement::create(
        &record_id,
        revision,
        "candidate-1",
        &proof_id,
        &attestation,
        &exact_attestation,
        OUTPUT,
    )
    .unwrap();
    let mut duplicate = statement.as_bytes().strip_suffix(b"}").unwrap().to_vec();
    duplicate.extend_from_slice(b",\"record_id\":\"other-record\"}");
    assert!(ProofIntegrityStatement::from_canonical_bytes(&duplicate).is_err());
    let mut unknown = statement.as_bytes().strip_suffix(b"}").unwrap().to_vec();
    unknown.extend_from_slice(b",\"unexpected\":true}");
    assert!(ProofIntegrityStatement::from_canonical_bytes(&unknown).is_err());

    let canonical = String::from_utf8(statement.as_bytes().to_vec()).unwrap();
    let multiple_proofs = canonical.replace(
        "\"proof_id\":\"proof-1\"",
        "\"proof_id\":\"proof-1\",\"proof_ids\":[\"proof-1\",\"proof-2\"]",
    );
    assert_ne!(multiple_proofs, canonical, "proof_id field is present");
    assert!(
        ProofIntegrityStatement::from_canonical_bytes(multiple_proofs.as_bytes()).is_err(),
        "the statement wire format accepts exactly one proof id"
    );
}

/// Provenance: PLAT-1801
#[trace("FR-210-AC-6")]
#[test]
fn tc_1788_statement_reader_refuses_field_mutation_without_recomputed_digest() {
    let attestation = ProofAttestation::from_json(&json(&attestation_text())).unwrap();
    let record_id = RecordId::parse("change-1", Subject::Record, "record_id").unwrap();
    let proof_id = ProofId::parse("proof-1", Subject::Attestation, "proof_id").unwrap();
    let revision = Revision::new(1, Subject::Record, "record_revision").unwrap();
    let exact_attestation = attestation_bytes(&attestation).unwrap();
    let statement = ProofIntegrityStatement::create(
        &record_id,
        revision,
        "candidate-1",
        &proof_id,
        &attestation,
        &exact_attestation,
        OUTPUT,
    )
    .unwrap();
    let mut changed = statement.as_bytes().to_vec();
    let digest_start = changed
        .windows(statement.statement_digest().len())
        .position(|window| window == statement.statement_digest().as_bytes())
        .unwrap();
    let digit = digest_start + statement.statement_digest().len() - 1;
    changed[digit] = if changed[digit] == b'0' { b'1' } else { b'0' };
    assert_eq!(
        ProofIntegrityStatement::from_canonical_bytes(&changed),
        Err(ProofIntegrityError::StatementInvalid),
        "editing canonical metadata while retaining its old digest must be refused"
    );
}

/// Intact output verifies and the receipt carries the commitment checked; an
/// equal-length substitute is invalid, names the integrity reason (not the
/// size reason) and carries no digest.
#[trace("FR-065-AC-23")]
#[test]
fn tc_1788_001_equal_length_substitution_is_invalid() {
    let good = verify(&base_input());
    assert_eq!(good.proofs[0].outcome, Outcome::Valid);
    let attestation = ProofAttestation::from_json(&json(&attestation_text())).unwrap();
    assert_eq!(
        good.proofs[0].content_digest.as_deref(),
        Some(attestation.content_commitment.digest.as_str())
    );

    let mut bad = base_input();
    bad.attestations[0].output = Some(corrupted());
    let receipt = verify(&bad);
    assert_eq!(receipt.proofs[0].outcome, Outcome::Invalid);
    assert_eq!(reasons(&receipt), vec![Reason::OutputIntegrityMismatch]);
    assert_eq!(receipt.proofs[0].content_digest, None);
}

/// A changed length is a size mismatch, decided before the commitment.
#[trace("FR-065-AC-23")]
#[test]
fn tc_1788_002_changed_length_is_a_size_mismatch() {
    for output in [b"short".to_vec(), [OUTPUT, b"!"].concat()] {
        let mut input = base_input();
        input.attestations[0].output = Some(output);
        assert_eq!(reasons(&verify(&input)), vec![Reason::OutputSizeMismatch]);
    }
}

/// Changing any committed metadata member without re-sealing fails, even
/// though the output bytes are untouched.
#[trace("FR-065-AC-23")]
#[test]
fn tc_1788_003_metadata_only_change_is_invalid() {
    let sealed_text = json(&attestation_text());
    for (name, value) in [
        ("observed_at", JsonValue::string("2027-01-01T00:00:00Z")),
        ("strength", JsonValue::string("observed")),
        ("environment", json(r#"{"os": "other"}"#)),
    ] {
        let mut input = base_input();
        input.attestations[0].attestation = replacing(&sealed_text, name, value);
        let receipt = verify(&input);
        assert_eq!(receipt.proofs[0].outcome, Outcome::Invalid, "{name}");
        assert_eq!(
            reasons(&receipt),
            vec![Reason::OutputIntegrityMismatch],
            "{name}"
        );
    }
    // Control: the same members, re-sealed, verify.
    let resealed = input_with(
        |a| replacing(a, "observed_at", JsonValue::string("2027-01-01T00:00:00Z")),
        OUTPUT,
    );
    assert_eq!(verify(&resealed).proofs[0].outcome, Outcome::Valid);
}

/// A missing output and another attestation's output are both refused.
#[trace("FR-065-AC-23")]
#[test]
fn tc_1788_004_missing_or_wrong_artifact_is_refused() {
    let mut missing = base_input();
    missing.attestations[0].output = None;
    assert_eq!(reasons(&verify(&missing)), vec![Reason::OutputMissing]);

    // A different attestation's output of the same length: att-2 legitimately
    // commits to its own bytes, but cannot vouch for att-1's slot.
    let other_output = b"all tests FAILED\n".to_vec();
    assert_eq!(other_output.len(), OUTPUT.len());
    let other = input_with(Clone::clone, &other_output);
    assert_eq!(verify(&other).proofs[0].outcome, Outcome::Valid);
    let mut wrong = base_input();
    wrong.attestations[0].output = other.attestations[0].output.clone();
    assert_eq!(
        reasons(&verify(&wrong)),
        vec![Reason::OutputIntegrityMismatch]
    );
}

/// A commitment is bound to its record, revision and attestation: replaying
/// the sealed attestation against another record or revision, or relabelling
/// it, cannot satisfy the proof.
#[trace("FR-065-AC-23")]
#[test]
fn tc_1788_005_a_commitment_cannot_be_reused_across_records_or_revisions() {
    for (name, value) in [
        ("record_id", JsonValue::string("change-2")),
        ("record_revision", JsonValue::number(2.0).unwrap()),
        ("attestation_id", JsonValue::string("att-9")),
    ] {
        let mut input = base_input();
        input.attestations[0].attestation =
            replacing(&json(&attestation_text()), name, value.clone());
        let receipt = verify(&input);
        assert_eq!(receipt.proofs[0].outcome, Outcome::Invalid, "{name}");
        assert_eq!(receipt.proofs[0].content_digest, None, "{name}");
        assert!(
            reasons(&receipt).contains(&Reason::OutputIntegrityMismatch),
            "{name}: replay without re-sealing keeps the old commitment"
        );
        // Even re-sealed by someone who can rewrite it, the binding fields
        // still refuse the proof.
        let resealed = input_with(|a| replacing(a, name, value.clone()), OUTPUT);
        assert_eq!(
            verify(&resealed).proofs[0].outcome,
            Outcome::Invalid,
            "{name}"
        );
    }
}

/// Two retained attestations under one id, and two selections for one proof,
/// both refuse; a single of each is the healthy control.
#[trace("FR-065-AC-5")]
#[test]
fn tc_1788_006_duplicate_ids_and_multiple_selections_refuse() {
    assert_eq!(verify(&base_input()).proofs[0].outcome, Outcome::Valid);

    let mut duplicate_id = base_input();
    let copy = duplicate_id.attestations[0].clone();
    duplicate_id.attestations.push(copy);
    let receipt = verify(&duplicate_id);
    assert_eq!(receipt.proofs[0].outcome, Outcome::Invalid);
    assert!(reasons(&receipt).contains(&Reason::AttestationSchemaInvalid));
    assert_eq!(receipt.proofs[0].content_digest, None);

    let mut two_selected = base_input();
    let selection: Selection = two_selected.selections[0].clone();
    two_selected.selections.push(selection);
    let receipt = verify(&two_selected);
    assert_eq!(receipt.proofs[0].outcome, Outcome::Invalid);
    assert!(reasons(&receipt).contains(&Reason::AttestationSchemaInvalid));
}

/// A zero-size output is a present, empty output; absent is not the same, and
/// non-empty bytes against size 0 are a size mismatch.
#[trace("FR-065-AC-5")]
#[test]
fn tc_1788_007_size_zero_differs_from_missing() {
    let zero = |output: Option<Vec<u8>>| {
        let mut input = input_with(
            |a| {
                replacing(
                    a,
                    "retained_output",
                    json(r#"{"media_type": "text/plain", "size_bytes": 0}"#),
                )
            },
            b"",
        );
        input.attestations[0].output = output;
        verify(&input)
    };
    assert_eq!(zero(Some(Vec::new())).proofs[0].outcome, Outcome::Valid);
    assert_eq!(reasons(&zero(None)), vec![Reason::OutputMissing]);
    assert_eq!(
        reasons(&zero(Some(b"x".to_vec()))),
        vec![Reason::OutputSizeMismatch]
    );
}

/// A declared size no output can have never passes: `u64::MAX` is not a
/// readable size, and a huge readable one is a size mismatch.
#[trace("FR-065-AC-5")]
#[test]
fn tc_1788_008_an_unreachable_declared_size_never_passes() {
    let with_size = |size: &str| {
        replacing(
            &json(&attestation_text()),
            "retained_output",
            json(&format!(
                r#"{{"media_type": "text/plain", "size_bytes": {size}}}"#
            )),
        )
    };
    assert!(ProofAttestation::from_json(&with_size("18446744073709551615")).is_err());

    let mut input = base_input();
    input.attestations[0].attestation = with_size("9007199254740991");
    let receipt = verify(&input);
    assert_eq!(receipt.proofs[0].outcome, Outcome::Invalid);
    assert!(reasons(&receipt).contains(&Reason::OutputSizeMismatch));
}

/// Intake refuses an equal-length substitute on both stores and retains
/// nothing; the intact pair is retained.
#[trace("FR-064-AC-3")]
#[test]
fn tc_1788_009_intake_refuses_an_equal_length_substitute() {
    let attestation = canonical_bytes(&json(&attestation_text())).unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut disk = DiskEvidenceStore::new(root.path());
    let mut memory = MemoryEvidenceStore::new();

    let refusal = intake_attestation(&mut memory, &attestation, &corrupted()).unwrap_err();
    assert!(matches!(refusal, ChangeAssuranceError::OutputCommitment));
    let refusal = intake_attestation(&mut disk, &attestation, &corrupted()).unwrap_err();
    assert!(matches!(refusal, ChangeAssuranceError::OutputCommitment));
    assert!(read_attestation(&memory, &att_id()).unwrap().is_none());
    assert!(read_attestation(&disk, &att_id()).unwrap().is_none());

    intake_attestation(&mut memory, &attestation, OUTPUT).unwrap();
    intake_attestation(&mut disk, &attestation, OUTPUT).unwrap();
    assert!(read_attestation(&memory, &att_id()).unwrap().is_some());
    assert!(read_attestation(&disk, &att_id()).unwrap().is_some());
}

/// After retention, copying the store and reopening it still verifies; an
/// equal-length overwrite of the copied output is then refused on read.
#[trace("FR-064-AC-3")]
#[test]
fn tc_1788_010_verification_survives_copy_and_reopen() {
    let attestation = canonical_bytes(&json(&attestation_text())).unwrap();
    let first = tempfile::tempdir().unwrap();
    intake_attestation(
        &mut DiskEvidenceStore::new(first.path()),
        &attestation,
        OUTPUT,
    )
    .unwrap();

    let second = tempfile::tempdir().unwrap();
    copy_tree(first.path(), second.path());
    let reopened = DiskEvidenceStore::new(second.path());
    let (read, output) = read_attestation(&reopened, &att_id()).unwrap().unwrap();
    assert!(read.commits_to(&output).unwrap());
    assert_eq!(output, OUTPUT);

    let mut tampered = false;
    walk(second.path(), &mut |path| {
        if path.file_name().is_some_and(|n| n == "output.bin") {
            std::fs::write(path, corrupted()).unwrap();
            tampered = true;
        }
    });
    assert!(
        tampered,
        "the retained output.bin was found and overwritten"
    );
    let reopened = DiskEvidenceStore::new(second.path());
    assert!(matches!(
        read_attestation(&reopened, &att_id()),
        Err(ChangeAssuranceError::OutputCommitment)
    ));
}

/// The stored commitment matches only the exact output it was sealed over.
#[trace("FR-064-AC-4")]
#[test]
fn tc_1788_011_the_commitment_changes_with_any_input() {
    let attestation = ProofAttestation::from_json(&json(&attestation_text())).unwrap();
    assert!(attestation.commits_to(OUTPUT).unwrap());
    assert!(!attestation.commits_to(&corrupted()).unwrap());
    assert!(!attestation.commits_to(b"").unwrap());
    let digest = &attestation.content_commitment.digest;
    assert!(digest.starts_with("sha256:") && digest.len() == "sha256:".len() + 64);
}

/// A known answer: the preimage is built here byte by byte from the
/// specification, not by the code under test, so dropping the domain
/// separator, the length framing or the order changes the digest and fails.
/// Member order and whitespace of the supplied JSON do not matter (RFC 8785),
/// and duplicate member names are refused before canonicalization.
#[trace("FR-064-AC-4")]
#[test]
fn tc_1788_012_the_commitment_matches_the_specified_preimage() {
    let attestation = json(&attestation_text());
    let metadata = canonical_bytes(&without(&attestation, "content_commitment")).unwrap();
    let mut preimage = b"quoin-proof-attestation-content/v2\0".to_vec();
    preimage.extend_from_slice(&u64::try_from(metadata.len()).unwrap().to_be_bytes());
    preimage.extend_from_slice(&metadata);
    preimage.extend_from_slice(&u64::try_from(OUTPUT.len()).unwrap().to_be_bytes());
    preimage.extend_from_slice(OUTPUT);
    let expected = format!("sha256:{}", quoin_store::attachment::sha256_hex(&preimage));
    let read = ProofAttestation::from_json(&attestation).unwrap();
    assert_eq!(read.content_commitment.digest, expected);

    // Reverse the member order and pad with whitespace: same commitment.
    let mut reversed = String::from("{ ");
    let members: Vec<String> = names(&attestation).into_iter().rev().collect();
    for (index, name) in members.iter().enumerate() {
        if index > 0 {
            reversed.push_str(" ,\n ");
        }
        let value =
            String::from_utf8(canonical_bytes(member(&attestation, name)).unwrap()).unwrap();
        write!(reversed, "\"{name}\" : {value}").unwrap();
    }
    reversed.push_str(" }");
    let reread = ProofAttestation::from_json(&json(&reversed)).unwrap();
    assert!(reread.commits_to(OUTPUT).unwrap());

    let duplicated = reversed.replacen("{ ", "{ \"proof_id\": \"proof-1\", ", 1);
    let mut store = MemoryEvidenceStore::new();
    assert!(intake_attestation(&mut store, duplicated.as_bytes(), OUTPUT).is_err());
    assert!(intake_attestation(&mut store, reversed.as_bytes(), OUTPUT).is_ok());
}

fn walk(dir: &std::path::Path, visit: &mut dyn FnMut(&std::path::Path)) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, visit);
        } else {
            visit(&path);
        }
    }
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
