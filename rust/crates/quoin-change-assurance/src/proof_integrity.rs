// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Proof-integrity statements and the fail-closed retention/checkpoint seams (FR-210).
//!
//! This module binds exact input bytes to one selected proof. It does not claim
//! that any configured retention provider is durable or immutable; that requires
//! separate repeatable evidence against the selected deployment.

use std::fmt;

use quoin_store::{JsonNumber, JsonObject, JsonValue, canonical_bytes, parse_strict_json};
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
    record_id: String,
    /// The selected record revision.
    record_revision: u64,
    /// The candidate revision.
    candidate_revision: String,
    /// The selected proof identity.
    proof_id: String,
    /// The selected attestation identity.
    attestation_id: String,
    /// SHA-256 of the exact accepted attestation bytes.
    attestation_sha256: String,
    /// SHA-256 of the exact retained output bytes.
    output_sha256: String,
    /// Exact retained output length in bytes.
    output_size_bytes: u64,
    /// Media type recorded by the accepted attestation.
    output_media_type: String,
    /// SHA-256 of the domain-separated canonical statement preimage.
    statement_digest: String,
    canonical_bytes: Vec<u8>,
}

impl ProofIntegrityStatement {
    /// Read one stored statement, requiring its exact canonical bytes and digest.
    ///
    /// # Errors
    ///
    /// Refuses duplicate names, unknown/missing fields, noncanonical JSON,
    /// malformed values, or a statement digest that does not match its fields.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ProofIntegrityError> {
        let value = parse_strict_json(bytes).map_err(|_| ProofIntegrityError::StatementInvalid)?;
        let object = value
            .as_object()
            .map_err(|_| ProofIntegrityError::StatementInvalid)?;
        let expected_names = [
            "schema_version",
            "purpose",
            "record_id",
            "record_revision",
            "candidate_revision",
            "proof_id",
            "attestation_id",
            "attestation_sha256",
            "output_sha256",
            "output_size_bytes",
            "output_media_type",
            "statement_digest",
        ];
        if object.len() != expected_names.len()
            || object.names().any(|name| !expected_names.contains(&name))
            || integer_field(object, "schema_version")? != 1
            || text_field(object, "purpose")? != STATEMENT_PURPOSE
        {
            return Err(ProofIntegrityError::StatementInvalid);
        }
        let record_id = text_field(object, "record_id")?.to_owned();
        let proof_id = text_field(object, "proof_id")?.to_owned();
        let attestation_id = text_field(object, "attestation_id")?.to_owned();
        let candidate_revision = text_field(object, "candidate_revision")?.to_owned();
        let output_media_type = text_field(object, "output_media_type")?.to_owned();
        if !crate::ids::Identity::is_valid(&record_id)
            || !crate::ids::Identity::is_valid(&proof_id)
            || !crate::ids::Identity::is_valid(&attestation_id)
            || candidate_revision.is_empty()
            || output_media_type.is_empty()
        {
            return Err(ProofIntegrityError::StatementInvalid);
        }
        let record_revision = integer_field(object, "record_revision")?;
        let output_size_bytes = integer_field(object, "output_size_bytes")?;
        if record_revision == 0 || output_size_bytes > MAX_EXACT_JSON_INTEGER {
            return Err(ProofIntegrityError::StatementInvalid);
        }
        let attestation_sha256 = text_field(object, "attestation_sha256")?.to_owned();
        let output_sha256 = text_field(object, "output_sha256")?.to_owned();
        let statement_digest = text_field(object, "statement_digest")?.to_owned();
        if !is_digest(&attestation_sha256)
            || !is_digest(&output_sha256)
            || !is_digest(&statement_digest)
        {
            return Err(ProofIntegrityError::StatementInvalid);
        }
        let canonical =
            canonical_bytes(&value).map_err(|_| ProofIntegrityError::StatementInvalid)?;
        if canonical != bytes {
            return Err(ProofIntegrityError::StatementInvalid);
        }
        let mut unsigned = object.clone();
        unsigned.remove("statement_digest");
        let unsigned_bytes = canonical_bytes(&JsonValue::Object(unsigned))
            .map_err(|_| ProofIntegrityError::StatementInvalid)?;
        if statement_digest_for(&unsigned_bytes)? != statement_digest {
            return Err(ProofIntegrityError::StatementInvalid);
        }
        Ok(Self {
            record_id,
            record_revision,
            candidate_revision,
            proof_id,
            attestation_id,
            attestation_sha256,
            output_sha256,
            output_size_bytes,
            output_media_type,
            statement_digest,
            canonical_bytes: canonical,
        })
    }

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
        let statement_digest = statement_digest_for(&unsigned_bytes)?;
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

    /// Record identity bound by this statement.
    #[must_use]
    pub fn record_id(&self) -> &str {
        &self.record_id
    }
    /// Record revision bound by this statement.
    #[must_use]
    pub const fn record_revision(&self) -> u64 {
        self.record_revision
    }
    /// Candidate revision bound by this statement.
    #[must_use]
    pub fn candidate_revision(&self) -> &str {
        &self.candidate_revision
    }
    /// Proof identity bound by this statement.
    #[must_use]
    pub fn proof_id(&self) -> &str {
        &self.proof_id
    }
    /// Attestation identity bound by this statement.
    #[must_use]
    pub fn attestation_id(&self) -> &str {
        &self.attestation_id
    }
    /// Digest of exact accepted attestation bytes.
    #[must_use]
    pub fn attestation_sha256(&self) -> &str {
        &self.attestation_sha256
    }
    /// Digest of exact output bytes.
    #[must_use]
    pub fn output_sha256(&self) -> &str {
        &self.output_sha256
    }
    /// Exact output byte length.
    #[must_use]
    pub const fn output_size_bytes(&self) -> u64 {
        self.output_size_bytes
    }
    /// Output media type.
    #[must_use]
    pub fn output_media_type(&self) -> &str {
        &self.output_media_type
    }
    /// Digest of the unsigned canonical statement preimage.
    #[must_use]
    pub fn statement_digest(&self) -> &str {
        &self.statement_digest
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

/// Signature outcome returned by Engineering Assurance FR-080 for one tag object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointSignature {
    /// A configured trusted signer verified the signature.
    Verified,
    /// The signed tag's signature is malformed or cryptographically bad.
    BadSignature,
    /// The signature is valid but the signer is not in configured trust.
    UntrustedSigner,
    /// The tag object carries no signature.
    Unsigned,
    /// The signature could not be evaluated.
    Unverifiable,
}

/// Raw FR-080 result paired with the exact tag bytes supplied to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fr080CheckpointResult {
    /// Purpose parsed from the verified immutable tag object.
    pub purpose: String,
    /// Statement digest parsed from that same object.
    pub statement_digest: String,
    /// Proof scope parsed from that same object.
    pub scope: ProofScope,
    /// Full object id returned for the exact immutable tag object verified.
    pub tag_object_id: String,
    /// SHA-256 of exact tag object bytes FR-080 parsed and verified.
    pub verified_tag_bytes_sha256: String,
    /// Signature outcome FR-080 returned for this same immutable object.
    pub signature: CheckpointSignature,
}

/// Checkpoint evidence accepted only when tied to the exact signed-tag bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedCheckpoint {
    purpose: String,
    statement_digest: String,
    scope: ProofScope,
    tag_object_id: String,
    tag_bytes_sha256: String,
}

impl VerifiedCheckpoint {
    /// Bind an FR-080 result to the exact signed-tag bytes passed for verification.
    ///
    /// # Errors
    ///
    /// Returns a typed invalid condition for a bad signature, untrusted signer,
    /// object-byte mismatch, malformed result or wrong protocol binding. Missing
    /// and unevaluable signatures remain typed incomplete conditions.
    pub fn from_fr080_result(
        exact_tag_bytes: &[u8],
        result: Fr080CheckpointResult,
        expected_scope: &ProofScope,
        expected_statement_digest: &str,
    ) -> Result<Self, CheckpointFailure> {
        if result.verified_tag_bytes_sha256 != digest(exact_tag_bytes) {
            return Err(CheckpointFailure::ObjectBytesMismatch);
        }
        if result.tag_object_id.is_empty()
            || result.purpose != STATEMENT_PURPOSE
            || result.statement_digest != expected_statement_digest
            || result.scope != *expected_scope
        {
            return Err(CheckpointFailure::BindingMismatch);
        }
        match result.signature {
            CheckpointSignature::Verified => Ok(Self {
                purpose: result.purpose,
                statement_digest: result.statement_digest,
                scope: result.scope,
                tag_object_id: result.tag_object_id,
                tag_bytes_sha256: result.verified_tag_bytes_sha256,
            }),
            CheckpointSignature::BadSignature => Err(CheckpointFailure::BadSignature),
            CheckpointSignature::UntrustedSigner => Err(CheckpointFailure::UntrustedSigner),
            CheckpointSignature::Unsigned => Err(CheckpointFailure::SignatureMissing),
            CheckpointSignature::Unverifiable => Err(CheckpointFailure::SignatureUnverifiable),
        }
    }

    /// Full immutable tag object id returned by FR-080.
    #[must_use]
    pub fn tag_object_id(&self) -> &str {
        &self.tag_object_id
    }
    /// Statement digest verified by FR-080 from this exact tag object.
    #[must_use]
    pub fn statement_digest(&self) -> &str {
        &self.statement_digest
    }
}

/// Typed FR-080 checkpoint failure, preserving invalid versus incomplete evidence.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum CheckpointFailure {
    /// The configured verifier is not available.
    #[error("checkpoint verifier is unavailable")]
    VerifierUnavailable,
    /// No signature was present.
    #[error("checkpoint signature is missing")]
    SignatureMissing,
    /// The signature could not be evaluated.
    #[error("checkpoint signature is unverifiable")]
    SignatureUnverifiable,
    /// The signature is bad.
    #[error("checkpoint signature is bad")]
    BadSignature,
    /// The signature is valid, but signer trust does not include this signer.
    #[error("checkpoint signer is untrusted")]
    UntrustedSigner,
    /// Parsed statement purpose, digest or proof scope does not match the request.
    #[error("checkpoint binding does not match the requested proof")]
    BindingMismatch,
    /// The verifier result does not name the exact tag bytes supplied for verification.
    #[error("checkpoint result refers to different tag bytes")]
    ObjectBytesMismatch,
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
        /// Statement digest qualified by the exact checkpoint.
        statement_digest: String,
        /// Exact tag object id returned by FR-080.
        tag_object_id: String,
        /// Signature status returned by FR-080 for that object.
        checkpoint_signature: CheckpointSignature,
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
    checkpoint: Result<Option<&VerifiedCheckpoint>, CheckpointFailure>,
    attestation: &[u8],
    output: &[u8],
) -> Qualification {
    let Some(provider) = provider else {
        return Qualification::Incomplete("retention_provider_unconfigured");
    };
    let Some(tag_object) = tag_object else {
        return Qualification::Incomplete("signed_tag_object_missing");
    };
    let checkpoint = match checkpoint {
        Ok(Some(checkpoint)) => checkpoint,
        Ok(None)
        | Err(
            CheckpointFailure::VerifierUnavailable
            | CheckpointFailure::SignatureMissing
            | CheckpointFailure::SignatureUnverifiable,
        ) => {
            return Qualification::Incomplete("checkpoint_unavailable");
        }
        Err(CheckpointFailure::BadSignature) => {
            return Qualification::Invalid("checkpoint_signature_bad");
        }
        Err(CheckpointFailure::UntrustedSigner) => {
            return Qualification::Invalid("checkpoint_signer_untrusted");
        }
        Err(CheckpointFailure::BindingMismatch | CheckpointFailure::ObjectBytesMismatch) => {
            return Qualification::Invalid("checkpoint_binding_mismatch");
        }
    };
    if checkpoint.purpose != STATEMENT_PURPOSE
        || checkpoint.statement_digest != statement.statement_digest
        || checkpoint.scope != *scope
        || checkpoint.tag_object_id.is_empty()
        || checkpoint.tag_bytes_sha256 != digest(tag_object)
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
    Qualification::Verified {
        object_ids,
        statement_digest: statement.statement_digest.clone(),
        tag_object_id: checkpoint.tag_object_id.clone(),
        checkpoint_signature: CheckpointSignature::Verified,
    }
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
    /// A stored statement was malformed, noncanonical or digest-mismatched.
    #[error("proof-integrity statement is invalid")]
    StatementInvalid,
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

fn statement_digest_for(unsigned_bytes: &[u8]) -> Result<String, ProofIntegrityError> {
    let length = u64::try_from(unsigned_bytes.len())
        .map_err(|_| ProofIntegrityError::UnrepresentableSize)?;
    let mut preimage = Vec::with_capacity(DIGEST_DOMAIN.len() + 8 + unsigned_bytes.len());
    preimage.extend_from_slice(DIGEST_DOMAIN);
    preimage.extend_from_slice(&length.to_be_bytes());
    preimage.extend_from_slice(unsigned_bytes);
    Ok(digest(&preimage))
}

fn text_field<'a>(object: &'a JsonObject, name: &str) -> Result<&'a str, ProofIntegrityError> {
    object
        .get(name)
        .and_then(JsonValue::as_str)
        .ok_or(ProofIntegrityError::StatementInvalid)
}

fn integer_field(object: &JsonObject, name: &str) -> Result<u64, ProofIntegrityError> {
    let value = object
        .get(name)
        .and_then(JsonValue::as_f64)
        .ok_or(ProofIntegrityError::StatementInvalid)?;
    if !value.is_finite() || value < 0.0 || value > 9_007_199_254_740_991.0 || value.fract() != 0.0
    {
        return Err(ProofIntegrityError::StatementInvalid);
    }
    format!("{value:.0}")
        .parse()
        .map_err(|_| ProofIntegrityError::StatementInvalid)
}

fn is_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
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
        CheckpointSignature, Fr080CheckpointResult, ProofIntegrityStatement, ProofRetention,
        ProofScope, Qualification, RetainedObject, RetentionFailure, STATEMENT_PURPOSE,
        VerifiedCheckpoint, qualify,
    };

    /// Test double only: this in-memory map is not a durability or immutability claim.
    #[derive(Default)]
    struct TestDoubleRetention {
        objects: BTreeMap<String, Vec<u8>>,
        missing_readback: bool,
        retain_failure: Option<RetentionFailure>,
    }

    impl ProofRetention for TestDoubleRetention {
        fn retain(
            &mut self,
            _scope: &ProofScope,
            object: &RetainedObject<'_>,
        ) -> Result<String, RetentionFailure> {
            if let Some(failure) = self.retain_failure {
                return Err(failure);
            }
            let bytes = object.bytes();
            let id = super::digest(bytes);
            match self.objects.get(&id) {
                Some(existing) if existing != bytes => Err(RetentionFailure::DigestMismatch),
                Some(_) => Ok(id),
                None => {
                    self.objects.insert(id.clone(), bytes.to_vec());
                    Ok(id)
                }
            }
        }

        fn retrieve(&self, object_id: &str) -> Result<Option<Vec<u8>>, RetentionFailure> {
            if self.missing_readback {
                Ok(None)
            } else {
                Ok(self.objects.get(object_id).cloned())
            }
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

    fn checkpoint(tag_bytes: &[u8], tag_object_id: &str) -> VerifiedCheckpoint {
        VerifiedCheckpoint::from_fr080_result(
            tag_bytes,
            Fr080CheckpointResult {
                purpose: STATEMENT_PURPOSE.to_owned(),
                statement_digest: statement().statement_digest,
                scope: scope(),
                tag_object_id: tag_object_id.to_owned(),
                verified_tag_bytes_sha256: super::digest(tag_bytes),
                signature: CheckpointSignature::Verified,
            },
            &scope(),
            &statement().statement_digest,
        )
        .unwrap()
    }

    fn checkpoint_result(tag_bytes: &[u8], tag_object_id: &str) -> Fr080CheckpointResult {
        Fr080CheckpointResult {
            purpose: STATEMENT_PURPOSE.to_owned(),
            statement_digest: statement().statement_digest,
            scope: scope(),
            tag_object_id: tag_object_id.to_owned(),
            verified_tag_bytes_sha256: super::digest(tag_bytes),
            signature: CheckpointSignature::Verified,
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
            Ok(Some(&checkpoint(b"tag", "immutable-tag-object-id"))),
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
        let mut wrong = checkpoint_result(b"tag", "immutable-tag-object-id");
        wrong.scope.proof_id = "other-proof".to_owned();
        let mut provider = TestDoubleRetention::default();
        let wrong = VerifiedCheckpoint::from_fr080_result(
            b"tag",
            wrong,
            &scope(),
            &statement().statement_digest,
        );
        let result = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag"),
            wrong.as_ref().map(Some).map_err(|error| *error),
            b"attestation",
            b"output",
        );
        assert_eq!(
            result,
            Qualification::Invalid("checkpoint_binding_mismatch")
        );
        assert!(
            provider.objects.is_empty(),
            "a failed checkpoint must not write retention objects"
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-3")]
    #[trace("FR-210-AC-6")]
    #[test]
    fn test_double_round_trips_exact_objects_and_refuses_mutation_without_claiming_durability() {
        let mut provider = TestDoubleRetention::default();
        let result = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag"),
            Ok(Some(&checkpoint(b"tag", "immutable-tag-object-id"))),
            b"attestation",
            b"output",
        );
        assert!(
            matches!(result, Qualification::Verified { object_ids, .. } if object_ids.len() == 4)
        );
        assert_eq!(
            provider.objects.len(),
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
    #[trace("FR-210-AC-6")]
    #[test]
    fn test_double_missing_readback_is_incomplete_and_digest_or_scope_refusal_is_invalid() {
        let checkpoint = checkpoint(b"tag", "tag-object");
        let mut missing = TestDoubleRetention {
            missing_readback: true,
            ..TestDoubleRetention::default()
        };
        let missing_result = qualify(
            Some(&mut missing),
            &scope(),
            &statement(),
            Some(b"tag"),
            Ok(Some(&checkpoint)),
            b"attestation",
            b"output",
        );
        assert_eq!(
            missing_result,
            Qualification::Incomplete("retained_object_missing")
        );

        for failure in [
            RetentionFailure::DigestMismatch,
            RetentionFailure::ScopeMismatch,
        ] {
            let mut refusing = TestDoubleRetention {
                retain_failure: Some(failure),
                ..TestDoubleRetention::default()
            };
            let result = qualify(
                Some(&mut refusing),
                &scope(),
                &statement(),
                Some(b"tag"),
                Ok(Some(&checkpoint)),
                b"attestation",
                b"output",
            );
            assert_eq!(
                result,
                Qualification::Invalid("retention_binding_mismatch"),
                "{failure:?}"
            );
        }
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
            Ok(Some(&checkpoint(b"tag", "immutable-tag-object-id"))),
            b"changed attestation",
            b"output",
        );
        assert_eq!(result, Qualification::Invalid("statement_content_mismatch"));
        assert!(
            provider.objects.is_empty(),
            "mismatched bytes must be refused before provider writes"
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-5")]
    #[test]
    fn output_substitution_replay_and_missing_checkpoint_never_qualify() {
        let checkpoint = checkpoint(b"tag", "tag-object");
        let mut provider = TestDoubleRetention::default();
        let changed_output = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag"),
            Ok(Some(&checkpoint)),
            b"attestation",
            b"mutant",
        );
        assert_eq!(
            changed_output,
            Qualification::Invalid("statement_content_mismatch")
        );
        assert!(
            provider.objects.is_empty(),
            "mutated output cannot reach retention"
        );

        let missing_checkpoint = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag"),
            Ok(None),
            b"attestation",
            b"output",
        );
        assert_eq!(
            missing_checkpoint,
            Qualification::Incomplete("checkpoint_unavailable")
        );
        let missing_tag = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            None,
            Ok(Some(&checkpoint)),
            b"attestation",
            b"output",
        );
        assert_eq!(
            missing_tag,
            Qualification::Incomplete("signed_tag_object_missing")
        );

        let mut replayed_scope = scope();
        replayed_scope.record_revision += 1;
        let replay = qualify(
            Some(&mut provider),
            &replayed_scope,
            &statement(),
            Some(b"tag"),
            Ok(Some(&checkpoint)),
            b"attestation",
            b"output",
        );
        assert_eq!(
            replay,
            Qualification::Invalid("checkpoint_binding_mismatch")
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-6")]
    #[test]
    fn substituted_tag_bytes_do_not_match_the_exact_object_fr080_verified() {
        let checkpoint = checkpoint(b"tag object A", "tag-object-A");
        let mut provider = TestDoubleRetention::default();
        let result = qualify(
            Some(&mut provider),
            &scope(),
            &statement(),
            Some(b"tag object B"),
            Ok(Some(&checkpoint)),
            b"attestation",
            b"output",
        );
        assert_eq!(
            result,
            Qualification::Invalid("checkpoint_binding_mismatch")
        );
        assert!(
            provider.objects.is_empty(),
            "substituted tag bytes must be refused before retention"
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-7")]
    #[test]
    fn bad_or_untrusted_signatures_are_invalid_and_missing_verification_is_incomplete() {
        for (signature, expected) in [
            (
                CheckpointSignature::BadSignature,
                Qualification::Invalid("checkpoint_signature_bad"),
            ),
            (
                CheckpointSignature::UntrustedSigner,
                Qualification::Invalid("checkpoint_signer_untrusted"),
            ),
            (
                CheckpointSignature::Unsigned,
                Qualification::Incomplete("checkpoint_unavailable"),
            ),
            (
                CheckpointSignature::Unverifiable,
                Qualification::Incomplete("checkpoint_unavailable"),
            ),
        ] {
            let raw = Fr080CheckpointResult {
                signature,
                ..checkpoint_result(b"tag", "tag-object").clone()
            };
            let checkpoint = VerifiedCheckpoint::from_fr080_result(
                b"tag",
                raw,
                &scope(),
                &statement().statement_digest,
            );
            let result = qualify(
                Some(&mut TestDoubleRetention::default()),
                &scope(),
                &statement(),
                Some(b"tag"),
                checkpoint.as_ref().map(Some).map_err(|error| *error),
                b"attestation",
                b"output",
            );
            assert_eq!(result, expected, "signature result {signature:?}");
        }
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-7")]
    #[test]
    fn qualification_preserves_integrity_facts_without_claiming_approval_or_correctness() {
        let checkpoint = checkpoint(b"tag", "full-immutable-object-id");
        let result = qualify(
            Some(&mut TestDoubleRetention::default()),
            &scope(),
            &statement(),
            Some(b"tag"),
            Ok(Some(&checkpoint)),
            b"attestation",
            b"output",
        );
        assert_eq!(
            result,
            Qualification::Verified {
                object_ids: vec![
                    super::digest(b"statement"),
                    super::digest(b"tag"),
                    super::digest(b"attestation"),
                    super::digest(b"output"),
                ],
                statement_digest: statement().statement_digest,
                tag_object_id: "full-immutable-object-id".to_owned(),
                checkpoint_signature: CheckpointSignature::Verified,
            }
        );
    }

    /// Provenance: PLAT-1801
    #[trace("FR-210-AC-8")]
    #[test]
    fn separate_valid_tag_objects_for_the_same_checkpoint_keep_distinct_ids() {
        for (tag_bytes, object_id) in [
            (b"tag object one".as_slice(), "tag-object-one"),
            (b"tag object two".as_slice(), "tag-object-two"),
        ] {
            let checkpoint = checkpoint(tag_bytes, object_id);
            let result = qualify(
                Some(&mut TestDoubleRetention::default()),
                &scope(),
                &statement(),
                Some(tag_bytes),
                Ok(Some(&checkpoint)),
                b"attestation",
                b"output",
            );
            assert!(
                matches!(result, Qualification::Verified { tag_object_id, .. } if tag_object_id == object_id)
            );
        }
    }
}
