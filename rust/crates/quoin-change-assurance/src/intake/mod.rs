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
//! that make retained evidence trustworthy (bytes must reproduce their digest;
//! a digest-named slot that already holds different bytes is a refusal, never
//! an overwrite; an attestation and its output become visible together or not
//! at all) are stated once and tested without a disk.
//!
//! # What is new here, and what is not
//!
//! Record storage is [`quoin_store::write_content_addressed`] and nothing
//! more: one file, named by its digest, written to a temporary and hard-linked
//! into place so that a losing racer compares rather than clobbers.
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

use quoin_store::{CanonicalDigest, digest_raw_bytes, parse_strict_json};

use crate::attestations::{attestation_bytes, verify_attestation};
use crate::error::{ChangeAssuranceError, Subject};
use crate::model::attestation::ProofAttestation;
use crate::model::record::ChangeAssuranceRecord;
use crate::records::{record_bytes, verify_change_record};

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
/// Every method is addressed by digest. There is no method taking a path, a
/// name or a handle, because evidence that can be addressed by anything other
/// than its content is evidence that can be substituted.
pub trait EvidenceStore {
    /// The record retained under `digest`, if one is.
    ///
    /// # Errors
    ///
    /// Any failure reading retained evidence.
    fn record(&self, digest: &CanonicalDigest) -> Result<Option<Vec<u8>>, ChangeAssuranceError>;

    /// Retain `bytes` under `digest`, or confirm the same bytes are there.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::ContentCollision`] when the slot already holds
    /// different bytes, and any failure retaining them.
    fn publish_record(
        &mut self,
        digest: &CanonicalDigest,
        bytes: &[u8],
    ) -> Result<Published, ChangeAssuranceError>;

    /// The attestation pair retained under `digest`, if one is.
    ///
    /// # Errors
    ///
    /// Any failure reading retained evidence, including a directory holding
    /// only one half of a pair.
    fn attestation(
        &self,
        digest: &CanonicalDigest,
    ) -> Result<Option<RetainedPair>, ChangeAssuranceError>;

    /// Retain a pair under `digest`, or confirm the same pair is there.
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
        digest: &CanonicalDigest,
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

/// Retain a sealed record.
///
/// # Errors
///
/// [`ChangeAssuranceError::ContentCollision`] when the digest already names
/// different bytes, and any failure retaining them.
pub fn write_change_record<S: EvidenceStore + ?Sized>(
    store: &mut S,
    record: &ChangeAssuranceRecord,
) -> Result<Published, ChangeAssuranceError> {
    let bytes = record_bytes(record)?;
    store.publish_record(&record.digest, &bytes)
}

/// Read the record retained under `digest`, verifying it.
///
/// # Errors
///
/// Every refusal of the record schema, a digest that does not verify, and
/// [`ChangeAssuranceError::PathDigestMismatch`] when the retained record
/// declares a digest other than the one it is filed under.
pub fn read_change_record<S: EvidenceStore + ?Sized>(
    store: &S,
    digest: &CanonicalDigest,
) -> Result<Option<ChangeAssuranceRecord>, ChangeAssuranceError> {
    let Some(bytes) = store.record(digest)? else {
        return Ok(None);
    };
    let record = verify_change_record(&parse_strict_json(&bytes)?)?;
    if &record.digest == digest {
        Ok(Some(record))
    } else {
        Err(ChangeAssuranceError::PathDigestMismatch {
            what: Subject::Record,
            expected: digest.as_hex().to_owned(),
            found: record.digest.as_hex().to_owned(),
        })
    }
}

/// Take in an attestation and the output it attests to.
///
/// The output is checked against the attestation **before** anything is
/// retained: a pair whose output does not reproduce the declared digest and
/// size is refused rather than filed, because retaining it would put evidence
/// in the store that verification could only ever reject.
///
/// The bytes retained are the attestation's own canonical bytes, not the bytes
/// supplied — two spellings of one attestation are one attestation, and
/// retaining the spelling would make the second one a collision.
///
/// # Errors
///
/// A malformed or invalid attestation, [`ChangeAssuranceError::OutputIntegrity`]
/// when the output disagrees with the attestation, and
/// [`ChangeAssuranceError::ContentCollision`] when the digest already names a
/// different pair.
pub fn intake_attestation<S: EvidenceStore + ?Sized>(
    store: &mut S,
    raw_attestation: &[u8],
    output: &[u8],
) -> Result<Published, ChangeAssuranceError> {
    let attestation = verify_attestation(&parse_strict_json(raw_attestation)?)?;
    let expected = attestation_bytes(&attestation)?;
    let observed_size = u64::try_from(output.len()).unwrap_or(u64::MAX);
    let observed_digest = digest_raw_bytes(output);
    if attestation.retained_output.size_bytes != observed_size
        || attestation.retained_output.digest != observed_digest
    {
        return Err(ChangeAssuranceError::OutputIntegrity {
            declared_size: attestation.retained_output.size_bytes,
            declared_digest: attestation.retained_output.digest.as_hex().to_owned(),
            observed_size,
            observed_digest: observed_digest.as_hex().to_owned(),
        });
    }
    store.publish_attestation(
        &attestation.digest,
        &RetainedPair {
            attestation: expected,
            output: output.to_vec(),
        },
    )
}

/// Read the attestation retained under `digest`, with its output.
///
/// Re-checks the output against the attestation on the way out: retained
/// evidence that has drifted is refused rather than returned, because a caller
/// that trusted it would attribute the drift to the producer.
///
/// # Errors
///
/// Every refusal of the attestation schema,
/// [`ChangeAssuranceError::PathDigestMismatch`] when the attestation is filed
/// under another digest, and [`ChangeAssuranceError::OutputIntegrity`] when
/// the retained output no longer reproduces the declared digest and size.
pub fn read_attestation<S: EvidenceStore + ?Sized>(
    store: &S,
    digest: &CanonicalDigest,
) -> Result<Option<(ProofAttestation, Vec<u8>)>, ChangeAssuranceError> {
    let Some(pair) = store.attestation(digest)? else {
        return Ok(None);
    };
    let attestation = verify_attestation(&parse_strict_json(&pair.attestation)?)?;
    if &attestation.digest != digest {
        return Err(ChangeAssuranceError::PathDigestMismatch {
            what: Subject::Attestation,
            expected: digest.as_hex().to_owned(),
            found: attestation.digest.as_hex().to_owned(),
        });
    }
    let observed_size = u64::try_from(pair.output.len()).unwrap_or(u64::MAX);
    let observed_digest = digest_raw_bytes(&pair.output);
    if attestation.retained_output.size_bytes != observed_size
        || attestation.retained_output.digest != observed_digest
    {
        return Err(ChangeAssuranceError::OutputIntegrity {
            declared_size: attestation.retained_output.size_bytes,
            declared_digest: attestation.retained_output.digest.as_hex().to_owned(),
            observed_size,
            observed_digest: observed_digest.as_hex().to_owned(),
        });
    }
    Ok(Some((attestation, pair.output)))
}

/// The claim this family's digests do and do not make.
///
/// Retained verbatim from the TypeScript's
/// `CHANGE_ASSURANCE_INTEGRITY_BOUNDARY`: a digest here is content integrity
/// and recorded attribution. It is not authentication, not authorization, not
/// a signature, and not a claim about a person.
pub const INTEGRITY_BOUNDARY: &str = "content integrity and recorded attribution only";
