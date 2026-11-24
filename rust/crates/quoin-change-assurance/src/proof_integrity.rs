// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Proof-integrity statements and the fail-closed retention/checkpoint seams (FR-210).
//!
//! This module binds exact input bytes to one selected proof. It does not claim
//! that any configured retention provider is durable or immutable; that requires
//! separate repeatable evidence against the selected deployment.

use std::fmt;

use quoin_store::{JsonNumber, JsonObject, JsonValue, canonical_bytes};
use thiserror::Error;

use crate::ids::{ProofId, RecordId, Revision};
use crate::model::attestation::ProofAttestation;

/// The statement purpose required by FR-210 and Engineering Assurance FR-080.
pub const STATEMENT_PURPOSE: &str = "proof-integrity-statement/v1";
const DIGEST_DOMAIN: &[u8] = b"quoin-proof-integrity-statement/v1\0";
const MAX_EXACT_JSON_INTEGER: u64 = 9_007_199_254_740_991;

/// A canonical, single-proof binding over exact retained source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofIntegrityStatement {
    /// The record identity.
    pub record_id: String,
    /// The selected record revision.
    pub record_revision: u64,
    /// The candidate revision.
    pub candidate_revision: String,
    /// The selected proof identity.
    pub proof_id: String,
    /// The selected attestation identity.
    pub attestation_id: String,
    /// SHA-256 of the exact accepted attestation bytes.
    pub attestation_sha256: String,
    /// SHA-256 of the exact retained output bytes.
    pub output_sha256: String,
    /// Exact retained output length in bytes.
    pub output_size_bytes: u64,
    /// Media type recorded by the accepted attestation.
    pub output_media_type: String,
    /// SHA-256 of the domain-separated canonical statement preimage.
    pub statement_digest: String,
    canonical_bytes: Vec<u8>,
}

impl ProofIntegrityStatement {
    /// Build the statement from the selected binding and exact retained bytes.
    ///
    /// The output is checked against the attestation before it can be bound.
    ///
    /// # Errors
    ///
    /// Returns [`ProofIntegrityError::OutputMismatch`] when size or the
    /// attestation's content commitment does not match, and
    /// [`ProofIntegrityError::UnrepresentableSize`] when a length cannot be
    /// represented exactly by the workspace's I-JSON number model.
    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is a distinct FR-210 binding field or exact source byte slice"
    )]
    pub fn create(
        record_id: &RecordId,
        record_revision: Revision,
        candidate_revision: &str,
        proof_id: &ProofId,
        attestation: &ProofAttestation,
        exact_attestation_bytes: &[u8],
        exact_output_bytes: &[u8],
    ) -> Result<Self, ProofIntegrityError> {
        let output_size_bytes = u64::try_from(exact_output_bytes.len())
            .map_err(|_| ProofIntegrityError::UnrepresentableSize)?;
        if output_size_bytes > MAX_EXACT_JSON_INTEGER
            || attestation.retained_output.size_bytes != output_size_bytes
            || !attestation.commits_to(exact_output_bytes).unwrap_or(false)
            || attestation.record_id != *record_id
            || attestation.record_revision != record_revision
            || attestation.candidate_revision.as_str() != candidate_revision
            || attestation.proof_id != *proof_id
        {
            return Err(ProofIntegrityError::OutputMismatch);
        }
        let attestation_id = &attestation.attestation_id;
        let attestation_sha256 = digest(exact_attestation_bytes);
        let output_sha256 = digest(exact_output_bytes);
        let fields = Fields {
            record_id: record_id.as_str(),
            record_revision: record_revision.get(),
            candidate_revision,
            proof_id: proof_id.as_str(),
            attestation_id: attestation_id.as_str(),
            attestation_sha256: &attestation_sha256,
            output_sha256: &output_sha256,
            output_size_bytes,
            output_media_type: attestation.retained_output.media_type.as_str(),
        };
        let unsigned = fields.json()?;
        let unsigned_bytes = canonical_bytes(&JsonValue::Object(unsigned))
            .map_err(|_| ProofIntegrityError::Canonicalization)?;
        let preimage_len = u64::try_from(unsigned_bytes.len())
            .map_err(|_| ProofIntegrityError::UnrepresentableSize)?;
        let mut preimage = Vec::with_capacity(DIGEST_DOMAIN.len() + 8 + unsigned_bytes.len());
        preimage.extend_from_slice(DIGEST_DOMAIN);
        preimage.extend_from_slice(&preimage_len.to_be_bytes());
        preimage.extend_from_slice(&unsigned_bytes);
        let statement_digest = digest(&preimage);
        let mut complete = fields.json()?;
        insert(
            &mut complete,
            "statement_digest",
            string(statement_digest.clone()),
        )?;
        let canonical_bytes = canonical_bytes(&JsonValue::Object(complete))
            .map_err(|_| ProofIntegrityError::Canonicalization)?;
        Ok(Self {
            record_id: record_id.as_str().to_owned(),
            record_revision: record_revision.get(),
            candidate_revision: candidate_revision.to_owned(),
            proof_id: proof_id.as_str().to_owned(),
            attestation_id: attestation_id.as_str().to_owned(),
            attestation_sha256: fields.attestation_sha256.to_owned(),
            output_sha256: fields.output_sha256.to_owned(),
            output_size_bytes,
            output_media_type: fields.output_media_type.to_owned(),
            statement_digest,
            canonical_bytes,
        })
    }

    /// The exact RFC 8785 bytes whose digest is represented by this statement.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

struct Fields<'a> {
    record_id: &'a str,
    record_revision: u64,
    candidate_revision: &'a str,
    proof_id: &'a str,
    attestation_id: &'a str,
    attestation_sha256: &'a str,
    output_sha256: &'a str,
    output_size_bytes: u64,
    output_media_type: &'a str,
}

impl Fields<'_> {
    fn json(&self) -> Result<JsonObject, ProofIntegrityError> {
        let record_revision = exact_json_number(self.record_revision)?;
        let output_size_bytes = exact_json_number(self.output_size_bytes)?;
        let mut object = JsonObject::new();
        for (key, value) in [
            ("schema_version", number(1.0)?),
            ("purpose", string(STATEMENT_PURPOSE.to_owned())),
            ("record_id", string(self.record_id.to_owned())),
            ("record_revision", number(record_revision)?),
            (
                "candidate_revision",
                string(self.candidate_revision.to_owned()),
            ),
            ("proof_id", string(self.proof_id.to_owned())),
            ("attestation_id", string(self.attestation_id.to_owned())),
            (
                "attestation_sha256",
                string(self.attestation_sha256.to_owned()),
            ),
            ("output_sha256", string(self.output_sha256.to_owned())),
            ("output_size_bytes", number(output_size_bytes)?),
            (
                "output_media_type",
                string(self.output_media_type.to_owned()),
            ),
        ] {
            insert(&mut object, key, value)?;
        }
        Ok(object)
    }
}

/// A typed statement retention port. Implementations must refuse replacement.
pub trait ProofRetention {
    /// Retain one immutable object and return its provider object identifier.
    ///
    /// # Errors
    ///
    /// Returns a typed configuration, availability, digest, scope or
    /// immutability refusal.
    fn retain(
        &mut self,
        scope: &ProofScope,
        object: &RetainedObject<'_>,
    ) -> Result<String, RetentionFailure>;
    /// Retrieve one retained object by provider identifier.
    ///
    /// # Errors
    ///
    /// Returns a typed provider refusal rather than converting it to absence.
    fn retrieve(&self, object_id: &str) -> Result<Option<Vec<u8>>, RetentionFailure>;
    /// Exercise the provider's refusal of an attempted object replacement.
    ///
    /// # Errors
    ///
    /// Returns [`RetentionFailure::Immutable`] when the provider refuses the
    /// update, or another typed provider failure.
    fn attempt_replace(&mut self, object_id: &str, bytes: &[u8]) -> Result<(), RetentionFailure>;
    /// Exercise the provider's refusal of an attempted object deletion.
    ///
    /// # Errors
    ///
    /// Returns [`RetentionFailure::Immutable`] when the provider refuses the
    /// deletion, or another typed provider failure.
    fn attempt_delete(&mut self, object_id: &str) -> Result<(), RetentionFailure>;
}

/// The binding all retained objects and checkpoints must share.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofScope {
    /// Record identity.
    pub record_id: String,
    /// Record revision.
    pub record_revision: u64,
    /// Candidate revision.
    pub candidate_revision: String,
    /// Proof identity.
    pub proof_id: String,
    /// Attestation identity.
    pub attestation_id: String,
}

/// One exact object to retain.
pub enum RetainedObject<'a> {
    /// The canonical proof-integrity statement.
    Statement(&'a [u8]),
    /// The immutable signed tag object bytes returned by FR-080.
    TagObject(&'a [u8]),
    /// The exact accepted attestation bytes.
    Attestation(&'a [u8]),
    /// The exact retained output bytes.
    Output(&'a [u8]),
}

/// A provider result used only after exact FR-080 checkpoint verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedCheckpoint {
    /// Purpose parsed from the verified immutable tag object.
    pub purpose: String,
    /// Statement digest parsed from that same object.
    pub statement_digest: String,
    /// Proof scope parsed from that same object.
    pub scope: ProofScope,
    /// Full object id for the exact immutable tag object verified.
    pub tag_object_id: String,
    /// Whether FR-080 verified the tag signature under configured trust.
    pub signature_verified: bool,
}

/// Why qualification stopped before a proof could be valid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Qualification {
    /// A configured provider or an object needed from it was absent.
    Incomplete(&'static str),
    /// Available evidence contradicts the statement or requested scope.
    Invalid(&'static str),
    /// The statement, exact tag object, and exact source bytes agree.
    Verified {
        /// Provider identifiers returned for the four retained exact objects.
        object_ids: Vec<String>,
    },
}

/// Check provider availability, retain and read back exact bytes, and validate an FR-080 result.
///
/// `None` is the explicit unconfigured-provider case and always refuses
/// qualification as incomplete.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "qualification consumes the provider and each distinct FR-210 evidence input"
)]
pub fn qualify(
    provider: Option<&mut dyn ProofRetention>,
    scope: &ProofScope,
    statement: &ProofIntegrityStatement,
    tag_object: Option<&[u8]>,
    checkpoint: Option<&VerifiedCheckpoint>,
    attestation: &[u8],
    output: &[u8],
) -> Qualification {
    let Some(provider) = provider else {
        return Qualification::Incomplete("retention_provider_unconfigured");
    };
    let Some(tag_object) = tag_object else {
        return Qualification::Incomplete("signed_tag_object_missing");
    };
    let Some(checkpoint) = checkpoint else {
        return Qualification::Incomplete("checkpoint_unavailable");
    };
    if checkpoint.purpose != STATEMENT_PURPOSE
        || checkpoint.statement_digest != statement.statement_digest
        || checkpoint.scope != *scope
        || checkpoint.tag_object_id.is_empty()
    {
        return Qualification::Invalid("checkpoint_binding_mismatch");
    }
    if statement.record_id != scope.record_id
        || statement.record_revision != scope.record_revision
        || statement.candidate_revision != scope.candidate_revision
        || statement.proof_id != scope.proof_id
        || statement.attestation_id != scope.attestation_id
        || statement.attestation_sha256 != digest(attestation)
        || statement.output_sha256 != digest(output)
        || statement.output_size_bytes != u64::try_from(output.len()).unwrap_or(u64::MAX)
    {
        return Qualification::Invalid("statement_content_mismatch");
    }
    if !checkpoint.signature_verified {
        return Qualification::Incomplete("checkpoint_signature_unverified");
    }
    let objects = [
        RetainedObject::Statement(statement.as_bytes()),
        RetainedObject::TagObject(tag_object),
        RetainedObject::Attestation(attestation),
        RetainedObject::Output(output),
    ];
    let mut object_ids = Vec::with_capacity(objects.len());
    for object in objects {
        let expected = object.bytes();
        let object_id = match provider.retain(scope, &object) {
            Ok(id) if !id.is_empty() => id,
            Ok(_) | Err(RetentionFailure::MissingConfiguration | RetentionFailure::Unavailable) => {
                return Qualification::Incomplete("retention_provider_unavailable");
            }
            Err(RetentionFailure::DigestMismatch | RetentionFailure::ScopeMismatch) => {
                return Qualification::Invalid("retention_binding_mismatch");
            }
            Err(RetentionFailure::Immutable) => {
                return Qualification::Invalid("retention_provider_rejected_write");
            }
        };
        match provider.retrieve(&object_id) {
            Ok(Some(bytes)) if bytes == expected => object_ids.push(object_id),
            Ok(Some(_)) => return Qualification::Invalid("retained_bytes_mismatch"),
            Ok(None)
            | Err(RetentionFailure::MissingConfiguration | RetentionFailure::Unavailable) => {
                return Qualification::Incomplete("retained_object_missing");
            }
            Err(RetentionFailure::DigestMismatch | RetentionFailure::ScopeMismatch) => {
                return Qualification::Invalid("retention_binding_mismatch");
            }
            Err(RetentionFailure::Immutable) => {
                return Qualification::Invalid("retention_provider_retrieval_refused");
            }
        }
    }
    Qualification::Verified { object_ids }
}

/// A provider refusal, classified without consulting display text.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RetentionFailure {
    /// No provider configuration is available.
    #[error("retention provider configuration is missing")]
    MissingConfiguration,
    /// The provider could not be reached.
    #[error("retention provider is unavailable")]
    Unavailable,
    /// A digest supplied to the provider disagreed with the exact bytes.
    #[error("retention digest mismatch")]
    DigestMismatch,
    /// The provider rejected the requested proof scope.
    #[error("retention scope mismatch")]
    ScopeMismatch,
    /// The provider correctly refused an update or deletion of retained data.
    #[error("retained object is immutable")]
    Immutable,
}

/// Errors constructing a statement.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ProofIntegrityError {
    /// Retained output or selected attestation binding did not match.
    #[error("proof-integrity input bytes or scope mismatch")]
    OutputMismatch,
    /// A byte length cannot be represented as an exact JSON integer.
    #[error("proof-integrity size exceeds exact JSON integer range")]
    UnrepresentableSize,
    /// Shared JCS encoding refused a statement.
    #[error("proof-integrity statement canonicalization failed")]
    Canonicalization,
}

impl RetainedObject<'_> {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Statement(bytes)
            | Self::TagObject(bytes)
            | Self::Attestation(bytes)
            | Self::Output(bytes) => bytes,
        }
    }
}

impl fmt::Display for ProofScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}:{}:{}",
            self.record_id,
            self.record_revision,
            self.candidate_revision,
            self.proof_id,
            self.attestation_id
        )
    }
}

fn insert(
    object: &mut JsonObject,
    name: &str,
    value: JsonValue,
) -> Result<(), ProofIntegrityError> {
    object
        .insert(name, value)
        .map_err(|_| ProofIntegrityError::Canonicalization)
}

fn string(value: String) -> JsonValue {
    JsonValue::string(value)
}
fn number(value: f64) -> Result<JsonValue, ProofIntegrityError> {
    JsonNumber::new(value)
        .map(JsonValue::Number)
        .map_err(|_| ProofIntegrityError::UnrepresentableSize)
}
fn exact_json_number(value: u64) -> Result<f64, ProofIntegrityError> {
    if value > MAX_EXACT_JSON_INTEGER {
        return Err(ProofIntegrityError::UnrepresentableSize);
    }
    value
        .to_string()
        .parse::<f64>()
        .map_err(|_| ProofIntegrityError::UnrepresentableSize)
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", quoin_store::attachment::sha256_hex(bytes))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in tests, a panic is the failure report"
)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;

    use super::{
        ProofIntegrityStatement, ProofRetention, ProofScope, Qualification, RetainedObject,
        RetentionFailure, STATEMENT_PURPOSE, VerifiedCheckpoint, qualify,
    };

    /// Test double only: this in-memory map is not a durability or immutability claim.
    #[derive(Default)]
    struct TestDoubleRetention(BTreeMap<String, Vec<u8>>);

    impl ProofRetention for TestDoubleRetention {
        fn retain(
            &mut self,
            _scope: &ProofScope,
            object: &RetainedObject<'_>,
        ) -> Result<String, RetentionFailure> {
            let bytes = object.bytes();
            let id = super::digest(bytes);
            match self.0.get(&id) {
                Some(existing) if existing != bytes => Err(RetentionFailure::DigestMismatch),
                Some(_) => Ok(id),
                None => {
                    self.0.insert(id.clone(), bytes.to_vec());
                    Ok(id)
                }
            }
        }

        fn retrieve(&self, object_id: &str) -> Result<Option<Vec<u8>>, RetentionFailure> {
            Ok(self.0.get(object_id).cloned())
        }

        fn attempt_replace(
            &mut self,
            _object_id: &str,
            _bytes: &[u8],
        ) -> Result<(), RetentionFailure> {
            Err(RetentionFailure::Immutable)
        }

        fn attempt_delete(&mut self, _object_id: &str) -> Result<(), RetentionFailure> {
            Err(RetentionFailure::Immutable)
        }
    }

    fn statement() -> ProofIntegrityStatement {
        ProofIntegrityStatement {
            record_id: "record-1".to_owned(),
            record_revision: 2,
            candidate_revision: "candidate-1".to_owned(),
            proof_id: "proof-1".to_owned(),
            attestation_id: "attestation-1".to_owned(),
            attestation_sha256: super::digest(b"attestation"),
            output_sha256: super::digest(b"output"),
            output_size_bytes: 6,
            output_media_type: "application/octet-stream".to_owned(),
            statement_digest: super::digest(b"statement"),
            canonical_bytes: b"statement".to_vec(),
        }
    }

    fn scope() -> ProofScope {
        ProofScope {
            record_id: "record-1".to_owned(),
            record_revision: 2,
            candidate_revision: "candidate-1".to_owned(),
            proof_id: "proof-1".to_owned(),
            attestation_id: "attestation-1".to_owned(),
        }
    }

    fn checkpoint() -> VerifiedCheckpoint {
        VerifiedCheckpoint {
            purpose: STATEMENT_PURPOSE.to_owned(),
            statement_digest: statement().statement_digest,
            scope: scope(),
            tag_object_id: "immutable-tag-object-id".to_owned(),
            signature_verified: true,
        }
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-3")]
    #[test]
    fn missing_provider_refuses_qualification_as_incomplete() {
        let result = qualify(
            None,
            &scope(),
            &statement(),
            Some(b"tag"),
            Some(&checkpoint()),
            b"attestation",
            b"output",
        );
        assert_eq!(
            result,
            Qualification::Incomplete("retention_provider_unconfigured")
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-2")]
    #[test]
    fn checkpoint_with_wrong_scope_is_invalid() {
        let mut wrong = checkpoint();
        wrong.scope.proof_id = "other-proof".to_owned();
        let mut provider = TestDoubleRetention::default();
        let result = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag"),
            Some(&wrong),
            b"attestation",
            b"output",
        );
        assert_eq!(
            result,
            Qualification::Invalid("checkpoint_binding_mismatch")
        );
        assert!(
            provider.0.is_empty(),
            "a failed checkpoint must not write retention objects"
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-3")]
    #[test]
    fn test_double_round_trips_exact_objects_and_refuses_mutation_without_claiming_durability() {
        let mut provider = TestDoubleRetention::default();
        let result = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag"),
            Some(&checkpoint()),
            b"attestation",
            b"output",
        );
        assert!(matches!(result, Qualification::Verified { object_ids } if object_ids.len() == 4));
        assert_eq!(
            provider.0.len(),
            4,
            "the four statement and source objects are read back"
        );
        let object_id = super::digest(b"output");
        assert_eq!(
            provider.attempt_replace(&object_id, b"changed"),
            Err(RetentionFailure::Immutable)
        );
        assert_eq!(
            provider.attempt_delete(&object_id),
            Err(RetentionFailure::Immutable)
        );
        assert_eq!(
            provider.retrieve(&object_id).unwrap().as_deref(),
            Some(b"output".as_slice())
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-5")]
    #[test]
    fn changed_source_bytes_are_invalid_even_with_a_verified_checkpoint() {
        let mut provider = TestDoubleRetention::default();
        let result = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag"),
            Some(&checkpoint()),
            b"changed attestation",
            b"output",
        );
        assert_eq!(result, Qualification::Invalid("statement_content_mismatch"));
        assert!(
            provider.0.is_empty(),
            "mismatched bytes must be refused before provider writes"
        );
    }
}
