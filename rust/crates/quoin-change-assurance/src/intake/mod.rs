// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Retaining and reading change-assurance evidence (FR-064).
//!
//! # The one seam
//!
//! Everything in this module that *decides* anything is written against
//! [`EvidenceStore`] and never against a filesystem. There are two
//! implementations — [`disk::DiskEvidenceStore`] and
//! [`memory::MemoryEvidenceStore`] — and one analysis over both, so the rules
//! that make retained evidence trustworthy (output bytes must have the declared
//! size; an identity-named slot that already holds different bytes is a
//! refusal, never an overwrite; an attestation and its output become visible
//! together or not at all) are stated once and tested without a disk.
//!
//! # What is new here, and what is not
//!
//! Record storage is [`quoin_store::store::write_new_or_identical`] and
//! nothing more: one file at `records/<record_id>/<revision>.json`, written to
//! a temporary and hard-linked into place so that a losing racer compares
//! rather than clobbers. A successor revision is a new file.
//!
//! An attestation is a *pair* — the attestation and the bytes it attests to —
//! and a pair cannot be published by a file rename. That composition (stage
//! both files in an invisible directory, fsync it, rename the directory into
//! place, fsync the parent) is the genuinely new work, and it is the reason
//! [`EvidenceStore::recover_staging`] exists: a store's recovery pass must
//! remove staging *directories*, which `quoin-store`'s file-granular
//! [`recover_orphaned_temporaries`](quoin_store::recover_orphaned_temporaries)
//! does not.

pub mod disk;
pub mod memory;

use quoin_store::parse_strict_json;

use crate::attestations::{attestation_bytes, read_attestation_json};
use crate::error::{ChangeAssuranceError, Subject};
use crate::ids::{AttestationId, RecordId, Revision};
use crate::model::attestation::ProofAttestation;
use crate::model::record::ChangeAssuranceRecord;
use crate::records::{read_change_record_json, record_bytes};

/// Whether a publish added evidence or found it already retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Published {
    /// The evidence was not there and now is.
    Retained,
    /// The evidence was already there, byte-for-byte.
    AlreadyRetained,
}

/// One retained attestation and the output it attests to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedPair {
    /// The attestation's canonical bytes, as retained.
    pub attestation: Vec<u8>,
    /// The output bytes, as retained.
    pub output: Vec<u8>,
}

/// Where change-assurance evidence is kept.
///
/// Every method is addressed by identity: a record by `(record_id, revision)`
/// and an attestation by its `attestation_id`. Nothing is addressed by content.
pub trait EvidenceStore {
    /// The record retained at `(record_id, revision)`, if one is.
    ///
    /// # Errors
    ///
    /// Any failure reading retained evidence.
    fn record(
        &self,
        record_id: &RecordId,
        revision: Revision,
    ) -> Result<Option<Vec<u8>>, ChangeAssuranceError>;

    /// Retain `bytes` at `(record_id, revision)`, or confirm the same bytes
    /// are there.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::ContentCollision`] when the slot already holds
    /// different bytes, and any failure retaining them.
    fn publish_record(
        &mut self,
        record_id: &RecordId,
        revision: Revision,
        bytes: &[u8],
    ) -> Result<Published, ChangeAssuranceError>;

    /// The attestation pair retained under `attestation_id`, if one is.
    ///
    /// # Errors
    ///
    /// Any failure reading retained evidence, including a directory holding
    /// only one half of a pair.
    fn attestation(
        &self,
        attestation_id: &AttestationId,
    ) -> Result<Option<RetainedPair>, ChangeAssuranceError>;

    /// Retain a pair under `attestation_id`, or confirm the same pair is
    /// there.
    ///
    /// Both halves must become visible together: a reader must never observe
    /// an attestation whose output is not yet there.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::ContentCollision`] when the slot already holds
    /// a different pair, and any failure retaining it.
    fn publish_attestation(
        &mut self,
        attestation_id: &AttestationId,
        pair: &RetainedPair,
    ) -> Result<Published, ChangeAssuranceError>;

    /// Remove staging an interrupted publish left behind, returning how many.
    ///
    /// Never called on another writer's behalf by a publish: staging that
    /// belongs to a publish still in flight is indistinguishable from an
    /// orphan, so this is always an explicit pass.
    ///
    /// # Errors
    ///
    /// Any failure listing or removing staging.
    fn recover_staging(&mut self) -> Result<usize, ChangeAssuranceError>;
}

/// Retain a record at its `(record_id, revision)`.
///
/// # Errors
///
/// [`ChangeAssuranceError::ContentCollision`] when that revision already holds
/// different bytes, and any failure retaining them.
pub fn write_change_record<S: EvidenceStore + ?Sized>(
    store: &mut S,
    record: &ChangeAssuranceRecord,
) -> Result<Published, ChangeAssuranceError> {
    let bytes = record_bytes(record)?;
    store.publish_record(&record.record_id, record.revision, &bytes)
}

/// Read the record retained at `(record_id, revision)`.
///
/// # Errors
///
/// Every refusal of the record schema, and
/// [`ChangeAssuranceError::PathIdentityMismatch`] when the retained record
/// declares an identity other than the one it is filed under.
pub fn read_change_record<S: EvidenceStore + ?Sized>(
    store: &S,
    record_id: &RecordId,
    revision: Revision,
) -> Result<Option<ChangeAssuranceRecord>, ChangeAssuranceError> {
    let Some(bytes) = store.record(record_id, revision)? else {
        return Ok(None);
    };
    let record = read_change_record_json(&parse_strict_json(&bytes)?)?;
    if &record.record_id == record_id && record.revision == revision {
        Ok(Some(record))
    } else {
        Err(ChangeAssuranceError::PathIdentityMismatch {
            what: Subject::Record,
            expected: format!("{record_id}/{revision}"),
            found: format!("{}/{}", record.record_id, record.revision),
        })
    }
}

/// Take in an attestation and the output it attests to.
///
/// The output length is checked against the attestation **before** anything is
/// retained: a pair whose output does not have the declared size is refused
/// rather than filed, because retaining it would put evidence in the store that
/// verification could only ever reject.
///
/// The bytes retained are the attestation's own canonical bytes, not the bytes
/// supplied - two spellings of one attestation are one attestation, and
/// retaining the spelling would make the second one a collision.
///
/// # Errors
///
/// A malformed or invalid attestation, [`ChangeAssuranceError::OutputIntegrity`]
/// when the output size disagrees with the attestation, and
/// [`ChangeAssuranceError::ContentCollision`] when the id already names a
/// different pair.
pub fn intake_attestation<S: EvidenceStore + ?Sized>(
    store: &mut S,
    raw_attestation: &[u8],
    output: &[u8],
) -> Result<Published, ChangeAssuranceError> {
    let attestation = read_attestation_json(&parse_strict_json(raw_attestation)?)?;
    let expected = attestation_bytes(&attestation)?;
    check_output_size(&attestation, output)?;
    store.publish_attestation(
        &attestation.attestation_id,
        &RetainedPair {
            attestation: expected,
            output: output.to_vec(),
        },
    )
}

/// Read the attestation retained under `attestation_id`, with its output.
///
/// Re-checks the output length on the way out: retained evidence that has
/// drifted is refused rather than returned.
///
/// # Errors
///
/// Every refusal of the attestation schema,
/// [`ChangeAssuranceError::PathIdentityMismatch`] when the attestation is
/// filed under another id, and [`ChangeAssuranceError::OutputIntegrity`] when
/// the retained output no longer has the declared size.
pub fn read_attestation<S: EvidenceStore + ?Sized>(
    store: &S,
    attestation_id: &AttestationId,
) -> Result<Option<(ProofAttestation, Vec<u8>)>, ChangeAssuranceError> {
    let Some(pair) = store.attestation(attestation_id)? else {
        return Ok(None);
    };
    let attestation = read_attestation_json(&parse_strict_json(&pair.attestation)?)?;
    if &attestation.attestation_id != attestation_id {
        return Err(ChangeAssuranceError::PathIdentityMismatch {
            what: Subject::Attestation,
            expected: attestation_id.as_str().to_owned(),
            found: attestation.attestation_id.as_str().to_owned(),
        });
    }
    check_output_size(&attestation, &pair.output)?;
    Ok(Some((attestation, pair.output)))
}

fn check_output_size(
    attestation: &ProofAttestation,
    output: &[u8],
) -> Result<(), ChangeAssuranceError> {
    let observed_size = u64::try_from(output.len()).unwrap_or(u64::MAX);
    if attestation.retained_output.size_bytes == observed_size {
        Ok(())
    } else {
        Err(ChangeAssuranceError::OutputIntegrity {
            declared_size: attestation.retained_output.size_bytes,
            observed_size,
        })
    }
}

/// The claim this family's records do and do not make.
///
/// A record here is retained content and recorded attribution. It is not
/// authentication, not authorization, not a signature, and not a claim about a
/// person.
pub const INTEGRITY_BOUNDARY: &str = "retained content and recorded attribution only";
