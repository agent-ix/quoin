// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Campaign run records and bounded strict-JSON reading (FR-114).

use std::path::{Path, PathBuf};

use quoin_store::{canonical_bytes, parse_strict_json};
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

/// Encode a structural JSON value as RFC 8785 canonical bytes.
///
/// # Errors
/// Refuses any numeric value the store cannot represent.
pub fn encode_value(repo: &Path, value: &Value) -> Result<Vec<u8>, CampaignStoreError> {
    let parsed =
        crate::json_bridge::from_serde(value).map_err(|error| CampaignStoreError::Json {
            path: campaigns_root(repo),
            message: error.to_string(),
        })?;
    canonical_bytes(&parsed).map_err(|error| CampaignStoreError::Json {
        path: campaigns_root(repo),
        message: error.to_string(),
    })
}

/// Read a strict JSON record through its generated contract type.
///
/// # Errors
/// Refuses duplicate JSON keys and any invalid generated type field.
pub fn read_typed<T: DeserializeOwned>(path: &Path) -> Result<T, CampaignStoreError> {
    typed_from_bytes(&read_bounded(path)?, path)
}

/// Parse strict canonical JSON bytes read from `path` into a typed record.
///
/// # Errors
/// Refuses duplicate JSON keys and any invalid generated type field.
pub fn typed_from_bytes<T: DeserializeOwned>(
    bytes: &[u8],
    path: &Path,
) -> Result<T, CampaignStoreError> {
    let strict = parse_strict_json(bytes).map_err(|error| CampaignStoreError::Json {
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
