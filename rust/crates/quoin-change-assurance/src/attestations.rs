// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Sealing and verifying proof attestations (FR-064).
//!
//! The port of `sealAttestation`, `verifyAttestation` and `attestationBytes`.
//! The digest rule is the record's rule, applied to the other shape: it is
//! taken over the attestation's canonical bytes with the top-level `digest`
//! member removed, and nothing else.

use quoin_store::{JsonValue, canonical_bytes, digest_canonical_value};

use crate::error::{ChangeAssuranceError, Subject};
use crate::model::attestation::{ProofAttestation, UnsealedAttestation};

/// Seal an unsealed attestation: validate it, attach its digest, verify it.
///
/// # Errors
///
/// Every refusal of the v1 attestation schema, and
/// [`ChangeAssuranceError::DigestMismatch`] if the attached digest does not
/// verify.
pub fn seal_attestation(input: &JsonValue) -> Result<ProofAttestation, ChangeAssuranceError> {
    let mut unsealed = input.as_object()?.clone();
    unsealed.remove("digest");
    let unsealed = JsonValue::Object(unsealed);
    UnsealedAttestation::from_json(&unsealed)?;
    let digest = digest_canonical_value(&unsealed)?;
    let mut sealed = unsealed.as_object()?.clone();
    sealed.set("digest", JsonValue::string(digest.as_hex()));
    verify_attestation(&JsonValue::Object(sealed))
}

/// Validate a sealed attestation and check its digest against its own bytes.
///
/// # Errors
///
/// Every refusal of the v1 attestation schema, then
/// [`ChangeAssuranceError::DigestMismatch`].
pub fn verify_attestation(value: &JsonValue) -> Result<ProofAttestation, ChangeAssuranceError> {
    let attestation = ProofAttestation::read_sealed(value)?;
    let mut unsigned = value.as_object()?.clone();
    unsigned.remove("digest");
    let recomputed = digest_canonical_value(&JsonValue::Object(unsigned))?;
    if recomputed == attestation.digest {
        Ok(attestation)
    } else {
        Err(ChangeAssuranceError::DigestMismatch {
            subject: Subject::Attestation,
            stored: attestation.digest.as_hex().to_owned(),
            recomputed: recomputed.as_hex().to_owned(),
        })
    }
}

/// The canonical bytes an attestation is retained as.
///
/// # Errors
///
/// As [`quoin_store::canonical_bytes`].
pub fn attestation_bytes(attestation: &ProofAttestation) -> Result<Vec<u8>, ChangeAssuranceError> {
    Ok(canonical_bytes(&attestation.to_json()?)?)
}
