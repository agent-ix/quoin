// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Customer artifacts: records and documents only the customer holds (FR-030).
//!
//! A pack clause can expect a record or document no toolchain run produces. A
//! person retains it here as a description and a locator, and a discharge fact
//! cites it as `evidence://customer-artifacts/<artifact-id>` (FR-211). The
//! record vouches that the customer supplied the artifact, not that the
//! artifact is adequate or unchanged: nothing here copies, opens, fetches,
//! hashes or sizes what `location` names (FR-030-CON-4, FR-030-CON-5).

use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::EvidenceError;
use crate::paths::customer_artifact_path;
use crate::source::EvidenceSource;
use crate::store::codec::canonical_bytes_of;
use crate::timestamp::parse_timestamp;
use crate::types::STORE_SCHEMA_VERSION;

const LABEL: &str = "customer artifact";

/// Longest `artifactId`, in bytes.
const MAX_ID_BYTES: usize = 64;
/// Longest `title` or `suppliedBy`, in bytes.
const MAX_NAME_BYTES: usize = 256;
/// Longest `location`, in bytes.
const MAX_LOCATION_BYTES: usize = 2048;

/// Why an identifier was refused. Spellings are contractual (FR-130's rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CustomerArtifactRefusal {
    /// The `artifactId` is outside 1 to 64 ASCII letters, digits and `-`.
    InvalidIdentifier,
    /// The `artifactId` differs from a filed one only in letter case.
    CaseCollision,
}

impl CustomerArtifactRefusal {
    /// Every kind, in declaration order.
    pub const ALL: &'static [Self] = &[Self::InvalidIdentifier, Self::CaseCollision];

    /// The kind's contractual spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "invalid-identifier",
            Self::CaseCollision => "case-collision",
        }
    }
}

/// One retained customer artifact, exactly as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CustomerArtifact {
    /// Always [`STORE_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The id as submitted; only the file name is lowercased.
    pub artifact_id: String,
    /// Names the artifact.
    pub title: String,
    /// An opaque locator in the customer's own system. Never opened.
    pub location: String,
    /// The person or role at the customer who supplied the artifact.
    pub supplied_by: String,
    /// Canonical UTC timestamp.
    pub supplied_at: String,
}

/// The outcome of one recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCustomerArtifact {
    /// The accepted record.
    pub record: CustomerArtifact,
    /// Where it was filed, relative to the store root.
    pub path: String,
    /// Whether this call created the file, or found identical bytes there.
    pub created: bool,
}

fn refuse(kind: CustomerArtifactRefusal, detail: impl Into<String>) -> EvidenceError {
    EvidenceError::CustomerArtifactRefused {
        kind,
        detail: detail.into(),
    }
}

fn check_identifier(id: &str) -> Result<(), EvidenceError> {
    let in_charset = id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if id.is_empty() || id.len() > MAX_ID_BYTES || !in_charset {
        return Err(refuse(
            CustomerArtifactRefusal::InvalidIdentifier,
            format!(
                "artifactId {id:?} must be 1 to {MAX_ID_BYTES} bytes of ASCII letters, digits and '-'"
            ),
        ));
    }
    Ok(())
}

fn string<'a>(object: &'a Map<String, Value>, member: &str) -> Result<&'a str, EvidenceError> {
    object
        .get(member)
        .ok_or_else(|| EvidenceError::invalid(LABEL, format!("{member}: is required")))?
        .as_str()
        .ok_or_else(|| EvidenceError::invalid(LABEL, format!("{member}: must be a string")))
}

fn text<'a>(
    object: &'a Map<String, Value>,
    member: &str,
    max_bytes: usize,
) -> Result<&'a str, EvidenceError> {
    let value = string(object, member)?;
    if value.trim().is_empty() {
        return Err(EvidenceError::invalid(
            LABEL,
            format!("{member}: must not be blank"),
        ));
    }
    if value.len() > max_bytes {
        return Err(EvidenceError::invalid(
            LABEL,
            format!("{member}: is over {max_bytes} bytes"),
        ));
    }
    Ok(value)
}

/// Validate one document in full: exactly the five members, as non-blank
/// strings within their bounds, an admissible id and a canonical timestamp.
fn admit(document: &Value) -> Result<CustomerArtifact, EvidenceError> {
    let object = document
        .as_object()
        .ok_or_else(|| EvidenceError::invalid(LABEL, "document: must be a JSON object"))?;
    for member in object.keys() {
        if !matches!(
            member.as_str(),
            "artifactId" | "title" | "location" | "suppliedBy" | "suppliedAt"
        ) {
            return Err(EvidenceError::invalid(
                LABEL,
                format!("{member}: is not a customer artifact member"),
            ));
        }
    }
    let artifact_id = string(object, "artifactId")?;
    check_identifier(artifact_id)?;
    let supplied_at = text(object, "suppliedAt", usize::MAX)?;
    if parse_timestamp(supplied_at).is_none() {
        return Err(EvidenceError::invalid(
            LABEL,
            "suppliedAt: must be a canonical UTC timestamp YYYY-MM-DDTHH:MM:SS[.mmm]Z",
        ));
    }
    Ok(CustomerArtifact {
        schema_version: STORE_SCHEMA_VERSION,
        artifact_id: artifact_id.to_owned(),
        title: text(object, "title", MAX_NAME_BYTES)?.to_owned(),
        location: text(object, "location", MAX_LOCATION_BYTES)?.to_owned(),
        supplied_by: text(object, "suppliedBy", MAX_NAME_BYTES)?.to_owned(),
        supplied_at: supplied_at.to_owned(),
    })
}

/// Read a stored file: valid JSON, the stored members, and an `artifactId`
/// that lowercases to the file name.
fn read_file(path: &str, file_stem: &str, body: &str) -> Result<CustomerArtifact, EvidenceError> {
    let mut document: Value =
        serde_json::from_str(body).map_err(|error| EvidenceError::StoreRead {
            path: path.to_owned(),
            detail: error.to_string(),
        })?;
    // Every way a stored file fails admission names the file: an id outside
    // the charset is the file's defect, not a refusal of the caller's request.
    let named = |error: EvidenceError| match error {
        EvidenceError::InvalidRecord { label, detail } => EvidenceError::InvalidRecord {
            label,
            detail: format!("{path}: {detail}"),
        },
        EvidenceError::CustomerArtifactRefused { kind, detail } => EvidenceError::InvalidRecord {
            label: LABEL,
            detail: format!("{path}: {}: {detail}", kind.as_str()),
        },
        other => other,
    };
    let object = document.as_object_mut().ok_or_else(|| {
        named(EvidenceError::invalid(
            LABEL,
            "document: must be a JSON object",
        ))
    })?;
    if object.remove("schemaVersion") != Some(Value::from(STORE_SCHEMA_VERSION)) {
        return Err(named(EvidenceError::invalid(
            LABEL,
            format!("schemaVersion: must be {STORE_SCHEMA_VERSION}"),
        )));
    }
    let record = admit(&document).map_err(named)?;
    if record.artifact_id.to_ascii_lowercase() != file_stem {
        return Err(EvidenceError::RecordIntegrity {
            what: "customer artifact id differs from its file name",
            path: path.to_owned(),
            note: "",
        });
    }
    Ok(record)
}

/// Read one retained customer artifact by id, in any letter case.
///
/// Returns `None` only when the store holds no file for the id. A file that
/// is not valid is an error naming it, never an absent record
/// (FR-030-AC-41).
///
/// # Errors
///
/// [`EvidenceError::CustomerArtifactRefused`] for an inadmissible id,
/// [`EvidenceError::StoreRead`], [`EvidenceError::InvalidRecord`] or
/// [`EvidenceError::RecordIntegrity`] for an invalid stored file, otherwise
/// [`EvidenceError::StoreIo`].
pub fn read_customer_artifact<S: EvidenceSource + ?Sized>(
    source: &S,
    artifact_id: &str,
) -> Result<Option<CustomerArtifact>, EvidenceError> {
    check_identifier(artifact_id)?;
    let path = customer_artifact_path(artifact_id);
    let stem = artifact_id.to_ascii_lowercase();
    source
        .read(&path)?
        .map(|body| read_file(&path, &stem, &body))
        .transpose()
}

/// Validate and file one customer artifact, append-only.
///
/// An identical replay is a no-op; a different document at an occupied file
/// name is refused with [`EvidenceError::RecordIntegrity`], and an id that
/// folds to an occupied name without being equal to its `artifactId` is
/// refused as `case-collision`. Nothing is written unless the whole document
/// is valid, and `location` is never read.
///
/// # Errors
///
/// [`EvidenceError::InvalidRecord`] or [`EvidenceError::CustomerArtifactRefused`]
/// for a document that fails validation, [`EvidenceError::RecordIntegrity`]
/// for a different document at an occupied name, otherwise
/// [`EvidenceError::Canonicalization`] or [`EvidenceError::StoreIo`].
pub fn write_customer_artifact<S: EvidenceSource + ?Sized>(
    source: &mut S,
    document: &Value,
) -> Result<StoredCustomerArtifact, EvidenceError> {
    let record = admit(document)?;
    let path = customer_artifact_path(&record.artifact_id);
    let bytes = canonical_bytes_of(LABEL, &record)?;
    let _lock = source.lock_store()?;
    if let Some(filed) = read_customer_artifact(source, &record.artifact_id)?
        && filed.artifact_id != record.artifact_id
    {
        return Err(refuse(
            CustomerArtifactRefusal::CaseCollision,
            format!(
                "artifactId {:?} differs from the filed {:?} only in letter case",
                record.artifact_id, filed.artifact_id
            ),
        ));
    }
    let created = source.publish(&path, &bytes)?;
    Ok(StoredCustomerArtifact {
        record,
        path,
        created,
    })
}

#[cfg(test)]
mod tests;
