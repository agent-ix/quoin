// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Immutable, content-addressed binary attachments for a measurement
//! collection (FR-125).
//!
//! An attachment is opaque bytes a producer cannot embed losslessly as JSON: a
//! profile, a trace, sanitizer output. [`retain_attachment`] stores the bytes
//! once under their SHA-256 ([`quoin_store::attachment`]) through the store's
//! no-replace publish, and returns the [`AttachmentReference`] a collection
//! states in its `attachments` member. [`read_attachment`] gives the bytes back
//! only if they still hash to that address and have the declared length.
//!
//! Quoin never interprets the bytes: a verified attachment means the bytes are
//! the ones retained, not that they are a valid profile or trace.

use std::path::Path;

use quoin_store::attachment::{attachment_path, is_sha256_hex, sha256_hex};
use quoin_store::bounded_read::read_regular_file_bounded;
use quoin_store::store::{Content, StorePath, publish};
use quoin_store::{JsonObject, JsonValue, StoreError, parse_strict_json};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::store::measurement_path;
use crate::types::ids::CollectionId;

/// The largest attachment retained, in bytes.
pub const MAX_ATTACHMENT_BYTES: u64 = 64 * 1024 * 1024;

/// The most attachments one collection may reference.
pub const MAX_COLLECTION_ATTACHMENTS: usize = 256;

/// The largest collection document read back for verification.
const MAX_COLLECTION_BYTES: u64 = 64 * 1024 * 1024;

const DIGEST_PREFIX: &str = "sha256:";

/// A collection's pointer at one retained attachment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachmentReference {
    digest: String,
    media_type: String,
    byte_length: u64,
}

impl AttachmentReference {
    /// The lowercase hex SHA-256 of the bytes.
    #[must_use]
    pub fn digest_hex(&self) -> &str {
        &self.digest
    }

    /// The producer's media type for the bytes.
    #[must_use]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// The declared length in bytes.
    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// The reference as a collection states it:
    /// `{ "digest": "sha256:<hex>", "mediaType": ..., "byteLength": ... }`.
    ///
    /// # Errors
    ///
    /// Never in practice; the length is a finite integer.
    pub fn to_json(&self) -> Result<JsonValue, MeasurementError> {
        let mut object = JsonObject::new();
        object.set(
            "digest",
            JsonValue::string(format!("{DIGEST_PREFIX}{}", self.digest)),
        );
        object.set("mediaType", JsonValue::string(self.media_type.clone()));
        // The length is bounded by `MAX_ATTACHMENT_BYTES`, far inside f64's
        // exact integer range.
        #[allow(clippy::cast_precision_loss, reason = "bounded well below 2^53")]
        let length = self.byte_length as f64;
        object.set("byteLength", JsonValue::number(length)?);
        Ok(JsonValue::Object(object))
    }

    fn from_json(value: &JsonValue, index: usize) -> Result<Self, MeasurementError> {
        let invalid = |what: &str| {
            MeasurementError::new(
                MeasurementErrorCode::AttachmentInvalid,
                format!("attachments[{index}]: {what}"),
            )
        };
        let JsonValue::Object(object) = value else {
            return Err(invalid("must be an object"));
        };
        let digest = object
            .get("digest")
            .and_then(JsonValue::as_str)
            .and_then(|text| text.strip_prefix(DIGEST_PREFIX))
            .filter(|hex| is_sha256_hex(hex))
            .ok_or_else(|| invalid("`digest` must be `sha256:` and 64 lowercase hex digits"))?;
        let media_type = object
            .get("mediaType")
            .and_then(JsonValue::as_str)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| invalid("`mediaType` must be a non-empty string"))?;
        let length = object
            .get("byteLength")
            .and_then(JsonValue::as_f64)
            .ok_or_else(|| invalid("`byteLength` must be a number"))?;
        #[allow(
            clippy::cast_precision_loss,
            reason = "compared against a bound far below 2^53"
        )]
        let in_range =
            length.fract() == 0.0 && (0.0..=MAX_ATTACHMENT_BYTES as f64).contains(&length);
        if !in_range {
            return Err(invalid(&format!(
                "`byteLength` must be a whole number of at most {MAX_ATTACHMENT_BYTES}"
            )));
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked whole, non-negative and within MAX_ATTACHMENT_BYTES"
        )]
        let byte_length = length as u64;
        Ok(Self {
            digest: digest.to_owned(),
            media_type: media_type.to_owned(),
            byte_length,
        })
    }
}

/// Retain `bytes` as an immutable attachment and return its reference.
///
/// Retaining the same bytes again is idempotent. Nothing is ever replaced: if
/// the name already holds different bytes the store's content has been altered
/// and the call refuses.
///
/// # Errors
///
/// [`MeasurementErrorCode::AttachmentInvalid`] for an empty media type or bytes
/// past [`MAX_ATTACHMENT_BYTES`], [`MeasurementErrorCode::AttachmentTampered`]
/// when the address already holds different bytes, and
/// [`MeasurementErrorCode::Store`] for a failed write.
pub fn retain_attachment(
    repo: &Path,
    media_type: &str,
    bytes: &[u8],
) -> Result<AttachmentReference, MeasurementError> {
    if media_type.is_empty() {
        return Err(MeasurementError::new(
            MeasurementErrorCode::AttachmentInvalid,
            "attachment mediaType must be non-empty",
        ));
    }
    let byte_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if byte_length > MAX_ATTACHMENT_BYTES {
        return Err(MeasurementError::new(
            MeasurementErrorCode::AttachmentInvalid,
            format!("attachment is {byte_length} bytes; the limit is {MAX_ATTACHMENT_BYTES}"),
        ));
    }
    let digest = sha256_hex(bytes);
    let path = attachment_path(repo, &digest).ok_or_else(|| {
        MeasurementError::new(MeasurementErrorCode::AttachmentInvalid, "malformed digest")
    })?;
    publish(&StorePath::ContentAddressed(path), Content::Artifact(bytes)).map_err(|error| {
        match error {
            StoreError::ContentCollision { .. } => MeasurementError::new(
                MeasurementErrorCode::AttachmentTampered,
                format!("attachment sha256:{digest} already holds different bytes"),
            ),
            other => MeasurementError::from(other),
        }
    })?;
    Ok(AttachmentReference {
        digest,
        media_type: media_type.to_owned(),
        byte_length,
    })
}

/// Read a retained attachment back, verified against its reference.
///
/// The read is bounded by the declared length before any bytes are allocated.
///
/// # Errors
///
/// [`MeasurementErrorCode::AttachmentUnavailable`] when nothing is retained or
/// the file cannot be read as a regular file, and
/// [`MeasurementErrorCode::AttachmentTampered`] when the bytes are longer or
/// shorter than declared or no longer hash to the address. Both name the
/// attachment.
pub fn read_attachment(
    repo: &Path,
    reference: &AttachmentReference,
) -> Result<Vec<u8>, MeasurementError> {
    let name = format!("sha256:{}", reference.digest);
    let path = attachment_path(repo, &reference.digest).ok_or_else(|| {
        MeasurementError::new(
            MeasurementErrorCode::AttachmentInvalid,
            format!("attachment {name}: malformed digest"),
        )
    })?;
    let bytes = read_regular_file_bounded(&path, reference.byte_length).map_err(|error| {
        let code = match &error {
            StoreError::ReadSourceTooLarge { .. } => MeasurementErrorCode::AttachmentTampered,
            _ => MeasurementErrorCode::AttachmentUnavailable,
        };
        MeasurementError::new(code, format!("attachment {name}: {error}"))
    })?;
    if u64::try_from(bytes.len()).ok() != Some(reference.byte_length)
        || sha256_hex(&bytes) != reference.digest
    {
        return Err(MeasurementError::new(
            MeasurementErrorCode::AttachmentTampered,
            format!("attachment {name}: retained bytes do not match the address"),
        ));
    }
    Ok(bytes)
}

/// The attachment references a collection states in its `attachments` member.
///
/// A collection with no such member references none. The count is bounded and
/// a repeated address is refused, before any file is touched.
///
/// # Errors
///
/// [`MeasurementErrorCode::AttachmentInvalid`] for a member that is not an
/// array of well-formed references within the limits.
pub fn collection_attachments(
    candidate: &JsonValue,
) -> Result<Vec<AttachmentReference>, MeasurementError> {
    let JsonValue::Object(root) = candidate else {
        return Ok(Vec::new());
    };
    let Some(member) = root.get("attachments") else {
        return Ok(Vec::new());
    };
    let JsonValue::Array(items) = member else {
        return Err(MeasurementError::new(
            MeasurementErrorCode::AttachmentInvalid,
            "`attachments` must be an array",
        ));
    };
    if items.len() > MAX_COLLECTION_ATTACHMENTS {
        return Err(MeasurementError::new(
            MeasurementErrorCode::AttachmentInvalid,
            format!(
                "`attachments` holds {} entries; the limit is {MAX_COLLECTION_ATTACHMENTS}",
                items.len()
            ),
        ));
    }
    let mut references: Vec<AttachmentReference> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let reference = AttachmentReference::from_json(item, index)?;
        if references
            .iter()
            .any(|seen| seen.digest == reference.digest)
        {
            return Err(MeasurementError::new(
                MeasurementErrorCode::AttachmentInvalid,
                format!(
                    "attachments[{index}]: sha256:{} is referenced twice",
                    reference.digest
                ),
            ));
        }
        references.push(reference);
    }
    Ok(references)
}

/// Verify every attachment a candidate references is retained and intact.
pub(crate) fn verify_candidate_attachments(
    repo: &Path,
    candidate: &JsonValue,
) -> Result<(), MeasurementError> {
    for reference in collection_attachments(candidate)? {
        read_attachment(repo, &reference)?;
    }
    Ok(())
}

/// Verify the attachments of a sealed collection against the retained bytes.
///
/// Editing or deleting a retained attachment after the collection was written
/// makes this refuse, naming the attachment.
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionUnreadable`] when the collection cannot be
/// read, and the refusals of [`read_attachment`] for each attachment.
pub fn verify_collection_attachments(
    repo: &Path,
    collection_id: &CollectionId,
) -> Result<(), MeasurementError> {
    let path = measurement_path(repo, collection_id);
    let unreadable = |error: &dyn std::fmt::Display| {
        MeasurementError::new(
            MeasurementErrorCode::CollectionUnreadable,
            format!("{}: {error}", path.display()),
        )
    };
    let bytes = read_regular_file_bounded(&path, MAX_COLLECTION_BYTES)
        .map_err(|error| unreadable(&error))?;
    let value = parse_strict_json(&bytes).map_err(|error| unreadable(&error))?;
    verify_candidate_attachments(repo, &value)
}
