// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading proof attestations (FR-064).
//!
//! An attestation is addressed by its caller-assigned `attestation_id`; it
//! carries a SHA-256 content commitment over its metadata and the retained
//! output bytes (FR-064), and the retained output is checked by length and by
//! that commitment.

use quoin_store::{JsonValue, canonical_bytes};

use crate::error::ChangeAssuranceError;
use crate::model::attestation::ProofAttestation;

/// Validate an attestation.
///
/// # Errors
///
/// Every refusal of the v2 attestation schema.
pub fn read_attestation_json(value: &JsonValue) -> Result<ProofAttestation, ChangeAssuranceError> {
    ProofAttestation::from_json(value)
}

/// The canonical bytes an attestation is retained as.
///
/// # Errors
///
/// As [`quoin_store::canonical_bytes`].
pub fn attestation_bytes(attestation: &ProofAttestation) -> Result<Vec<u8>, ChangeAssuranceError> {
    Ok(canonical_bytes(&attestation.to_json()?)?)
}
