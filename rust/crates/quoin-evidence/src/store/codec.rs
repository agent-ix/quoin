// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one bridge between this crate's record types and `quoin-store`'s bytes.
//!
//! Every record leaves this crate as canonical JSON produced by
//! [`quoin_store::canonical_json_bytes`], and the route there is deliberate:
//! `serde_json` writes the record's own field order, the text is re-read by
//! [`quoin_store::parse_strict_json_str`], and `quoin-store` sorts the keys and
//! formats the numbers. FR-100-CON-4 forbids a second canonicalization, so this
//! module contains no formatting of its own — only the hand-off.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::EvidenceError;

/// Canonical bytes for one record, exactly as the store holds them.
///
/// # Errors
///
/// [`EvidenceError::Canonicalization`] when the value has no canonical
/// spelling — a non-finite number is the only way a record reaches that.
pub(crate) fn canonical_bytes_of<T: Serialize>(
    what: &'static str,
    value: &T,
) -> Result<Vec<u8>, EvidenceError> {
    let text = serde_json::to_string(value).map_err(|error| EvidenceError::Canonicalization {
        what,
        detail: error.to_string(),
    })?;
    let parsed = quoin_store::parse_strict_json_str(&text).map_err(|error| {
        EvidenceError::Canonicalization {
            what,
            detail: error.to_string(),
        }
    })?;
    quoin_store::canonical_json_bytes(&parsed).map_err(|error| EvidenceError::Canonicalization {
        what,
        detail: error.to_string(),
    })
}

/// Parse one store file's text into a record.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`], carrying the retained `StoreReadError`
/// sentence: these files are checked into git, so a merge conflict is the usual
/// cause and the message says so.
pub(crate) fn decode<T: DeserializeOwned>(path: &str, text: &str) -> Result<T, EvidenceError> {
    serde_json::from_str(text).map_err(|error| EvidenceError::StoreRead {
        path: path.to_owned(),
        detail: error.to_string(),
    })
}
