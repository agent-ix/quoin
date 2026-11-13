// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The crate's failure envelope.

/// A stable, `Copy` classification of an operational failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// A file or directory could not be read.
    Io,
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
    pub(crate) fn io(path: &std::path::Path, error: &std::io::Error) -> Self {
        Self {
            code: ErrorCode::Io,
            message: format!("{}: {error}", path.display()),
        }
    }

    pub(crate) fn render(error: &serde_json::Error) -> Self {
        Self {
            code: ErrorCode::Render,
            message: error.to_string(),
        }
    }
}
