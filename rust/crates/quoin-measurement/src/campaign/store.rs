// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Immutable campaign definitions, runs, executor results and raw bytes (FR-114).

use std::path::{Path, PathBuf};

use quoin_store::{
    RawFileSha256Digest, canonical_bytes, digest_bytes_sha256, parse_strict_json,
    store::write_content_addressed,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;

use crate::types::ids::CollectionId;

/// An invalid, missing, edited or unreadable campaign evidence file.
#[derive(Debug, Error)]
pub enum CampaignStoreError {
    /// The run id cannot name one safe file.
    #[error("invalid campaign run id: {0}")]
    RunId(String),
    /// An evidence file could not be read or published.
    #[error("campaign evidence I/O at {path}: {source}")]
    Io {
        /// The path attempted.
        path: PathBuf,
        /// The underlying filesystem error.
        source: std::io::Error,
    },
    /// A retained file is linked, not regular, or exceeds the size ceiling.
    #[error("campaign evidence is not an admissible regular file: {0}")]
    File(PathBuf),
    /// A retained digest does not match its file's bytes.
    #[error("campaign evidence digest mismatch at {0}")]
    Digest(PathBuf),
    /// The retained file is not strict or typed JSON.
    #[error("invalid campaign JSON at {path}: {message}")]
    Json {
        /// The file path.
        path: PathBuf,
        /// The parse or typed validation failure.
        message: String,
    },
    /// Quoin's durable write-once store refused publication.
    #[error("campaign evidence store refused {path}: {message}")]
    Store {
        /// The file path.
        path: PathBuf,
        /// The store refusal.
        message: String,
    },
}

/// Maximum bytes of one retained campaign record or raw artifact.
pub const MAX_CAMPAIGN_EVIDENCE_BYTES: u64 = 64 * 1024 * 1024;

/// The campaign evidence root beneath Quoin's existing store.
#[must_use]
pub fn campaigns_root(repo: &Path) -> PathBuf {
    quoin_store::store::store_root(repo).join("campaigns")
}

/// One immutable campaign run, named by its caller-provided safe id.
///
/// # Errors
/// Refuses a run id that could escape the campaign directory.
pub fn run_path(repo: &Path, id: &str) -> Result<PathBuf, CampaignStoreError> {
    CollectionId::parse(id).map_err(|error| CampaignStoreError::RunId(error.to_string()))?;
    Ok(campaigns_root(repo).join("runs").join(format!("{id}.json")))
}

/// A content-addressed retained record path. `digest` is bare SHA-256 hex.
///
/// # Errors
/// Refuses a malformed digest before constructing a path.
pub fn digest_path(
    repo: &Path,
    kind: &'static str,
    digest: &str,
    extension: &'static str,
) -> Result<PathBuf, CampaignStoreError> {
    if !matches!(
        kind,
        "definitions"
            | "requests"
            | "results"
            | "raw"
            | "inputs"
            | "domain-verdicts"
            | "verdicts"
            | "bundles"
    ) || !matches!(extension, "json" | "bin")
    {
        return Err(CampaignStoreError::RunId(
            "invalid campaign evidence namespace".to_owned(),
        ));
    }
    let stored = format!("sha256:{digest}");
    RawFileSha256Digest::parse_stored(&stored)
        .map_err(|_| CampaignStoreError::RunId(format!("malformed {kind} digest")))?;
    Ok(campaigns_root(repo)
        .join(kind)
        .join(format!("{digest}.{extension}")))
}

/// Retain exact bytes under a SHA-256 path without replacing prior evidence.
///
/// # Errors
/// Refuses oversized bytes or a collision with different retained bytes.
pub fn retain_bytes(
    repo: &Path,
    kind: &'static str,
    extension: &'static str,
    bytes: &[u8],
) -> Result<(String, PathBuf), CampaignStoreError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CAMPAIGN_EVIDENCE_BYTES {
        return Err(CampaignStoreError::File(campaigns_root(repo).join(kind)));
    }
    let digest = digest_bytes_sha256(bytes).as_hex().to_owned();
    let path = digest_path(repo, kind, &digest, extension)?;
    write_content_addressed(&path, bytes).map_err(|error| CampaignStoreError::Store {
        path: path.clone(),
        message: error.to_string(),
    })?;
    Ok((digest, path))
}

/// Retain JSON bytes after strict parsing and RFC 8785 canonicalization.
///
/// # Errors
/// Refuses duplicate-key/invalid JSON or a durable write collision.
pub fn retain_json_bytes(
    repo: &Path,
    kind: &'static str,
    raw: &[u8],
) -> Result<(String, PathBuf), CampaignStoreError> {
    let parsed = parse_strict_json(raw).map_err(|error| CampaignStoreError::Json {
        path: campaigns_root(repo).join(kind),
        message: error.to_string(),
    })?;
    let bytes = canonical_bytes(&parsed).map_err(|error| CampaignStoreError::Json {
        path: campaigns_root(repo).join(kind),
        message: error.to_string(),
    })?;
    retain_bytes(repo, kind, "json", &bytes)
}

/// Retain a typed EA or Quoin record after the store's canonical writer has
/// encoded its structural JSON value. Callers compare the returned digest to
/// the type owner's independently minted identity before granting credit.
///
/// # Errors
/// Refuses any numeric value the store cannot represent or a durable collision.
pub fn retain_value(
    repo: &Path,
    kind: &'static str,
    value: &Value,
) -> Result<(String, PathBuf), CampaignStoreError> {
    let parsed =
        crate::json_bridge::from_serde(value).map_err(|error| CampaignStoreError::Json {
            path: campaigns_root(repo).join(kind),
            message: error.to_string(),
        })?;
    let bytes = canonical_bytes(&parsed).map_err(|error| CampaignStoreError::Json {
        path: campaigns_root(repo).join(kind),
        message: error.to_string(),
    })?;
    retain_bytes(repo, kind, "json", &bytes)
}

/// Read a bounded, regular evidence file and check the digest in its name.
///
/// # Errors
/// Refuses a link, non-file, oversized file, unreadable file or mismatch.
pub fn read_digest_bytes(
    repo: &Path,
    kind: &'static str,
    digest: &str,
    extension: &'static str,
) -> Result<Vec<u8>, CampaignStoreError> {
    let path = digest_path(repo, kind, digest, extension)?;
    let bytes = read_bounded(&path)?;
    if digest_bytes_sha256(&bytes).as_hex() != digest {
        return Err(CampaignStoreError::Digest(path));
    }
    Ok(bytes)
}

/// Read a strict JSON record through its generated contract type.
///
/// # Errors
/// Refuses duplicate JSON keys and any invalid generated type field.
pub fn read_typed<T: DeserializeOwned>(path: &Path) -> Result<T, CampaignStoreError> {
    let bytes = read_bounded(path)?;
    let strict = parse_strict_json(&bytes).map_err(|error| CampaignStoreError::Json {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let canonical =
        quoin_store::canonical_json_bytes(&strict).map_err(|error| CampaignStoreError::Json {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    serde_json::from_slice(&canonical).map_err(|error| CampaignStoreError::Json {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Read a bounded, regular retained file without following an obvious link.
///
/// # Errors
/// Refuses missing, linked, nonregular, oversized or unreadable files.
pub fn read_bounded(path: &Path) -> Result<Vec<u8>, CampaignStoreError> {
    quoin_store::read_regular_file_bounded(path, MAX_CAMPAIGN_EVIDENCE_BYTES).map_err(|error| {
        match error {
            quoin_store::StoreError::Io { source, .. } => CampaignStoreError::Io {
                path: path.to_path_buf(),
                source,
            },
            other => CampaignStoreError::Store {
                path: path.to_path_buf(),
                message: other.to_string(),
            },
        }
    })
}
