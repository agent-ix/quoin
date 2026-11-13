// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The crate's failure envelope.

use std::path::Path;

/// A stable, `Copy` classification of an operational failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// The scope or the modules that declare the record types could not be read.
    Read,
    /// A record breaks the contract its type declares.
    Record,
    /// The report could not be serialized.
    Render,
}

/// An operational failure, as opposed to a finding about a record.
#[derive(Debug, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct AdviceError {
    /// What kind of failure this is.
    pub code: ErrorCode,
    /// The human-readable detail; never branched on.
    pub message: String,
}

impl AdviceError {
    pub(crate) fn read(error: &quoin_quire::Error) -> Self {
        Self {
            code: ErrorCode::Read,
            message: error.to_string(),
        }
    }

    pub(crate) fn record(path: &Path, id: &str, reason: &str) -> Self {
        Self {
            code: ErrorCode::Record,
            message: format!(
                "{} ({id}) breaks its type's contract: {reason}",
                path.display()
            ),
        }
    }

    pub(crate) fn render(error: &serde_json::Error) -> Self {
        Self {
            code: ErrorCode::Render,
            message: error.to_string(),
        }
    }
}
