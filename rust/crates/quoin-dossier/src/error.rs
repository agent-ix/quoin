// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The error envelope of `quoin dossier` (FR-191).
//!
//! Each code maps to exactly one exit status, so the status a caller sees is
//! derived from the code rather than chosen at the throw site. Codes are the
//! API: never rename one, never reuse one.

use std::collections::BTreeMap;

use ix_cli_kit::exit::Outcome;
use serde::Serialize;

/// A stable, machine-matchable reason a dossier run did not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DossierErrorCode {
    /// A `--doc` value names no registered document kind. Exit 2.
    UnknownKind,
    /// `--version` is missing or blank. Exit 2.
    VersionRequired,
    /// A selected document does not support the requested operation. Exit 2.
    OperationUnsupported,
    /// A selected document needs `--as-of` and none was given. Exit 2.
    AsOfRequired,
    /// `--as-of` is not an RFC 3339 instant. Exit 2.
    AsOfInvalid,
    /// The output directory cannot be created. Exit 2.
    OutUnwritable,
    /// A document file could not be written. Exit 2.
    WriteFailed,
    /// A generator's source failed. Exit 3.
    SourceFailed,
    /// `quire export` failed or could not start. Exit 4.
    QuireFailed,
    /// Two generators claim one kind or one id. Exit 2: exit 4 is reserved
    /// for `quire export` (FR-191).
    RegistryConflict,
    /// No document is registered to select, so there is nothing to write or
    /// export. Exit 2.
    NoDocuments,
    /// `export` found a selected file missing or stale and did not start
    /// `quire export`. Exit 1.
    ExportStale,
}

impl DossierErrorCode {
    /// The wire spelling. This string is the contract; do not change one.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownKind => "DOSSIER_UNKNOWN_KIND",
            Self::VersionRequired => "DOSSIER_VERSION_REQUIRED",
            Self::OperationUnsupported => "DOSSIER_OPERATION_UNSUPPORTED",
            Self::AsOfRequired => "DOSSIER_AS_OF_REQUIRED",
            Self::AsOfInvalid => "DOSSIER_AS_OF_INVALID",
            Self::OutUnwritable => "DOSSIER_OUT_UNWRITABLE",
            Self::WriteFailed => "DOSSIER_WRITE_FAILED",
            Self::SourceFailed => "DOSSIER_SOURCE_FAILED",
            Self::QuireFailed => "DOSSIER_QUIRE_FAILED",
            Self::RegistryConflict => "DOSSIER_REGISTRY_CONFLICT",
            Self::NoDocuments => "DOSSIER_NO_DOCUMENTS",
            Self::ExportStale => "DOSSIER_EXPORT_STALE",
        }
    }

    /// Every code this build can emit.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::UnknownKind,
            Self::VersionRequired,
            Self::OperationUnsupported,
            Self::AsOfRequired,
            Self::AsOfInvalid,
            Self::OutUnwritable,
            Self::WriteFailed,
            Self::SourceFailed,
            Self::QuireFailed,
            Self::RegistryConflict,
            Self::NoDocuments,
            Self::ExportStale,
        ]
    }

    /// The code spelled `code`, if this build emits one.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }

    /// The only mapping from a code to an exit status (FR-191 exit table).
    #[must_use]
    pub const fn outcome(self) -> Outcome {
        match self {
            Self::UnknownKind
            | Self::VersionRequired
            | Self::OperationUnsupported
            | Self::AsOfRequired
            | Self::AsOfInvalid
            | Self::OutUnwritable
            | Self::WriteFailed
            | Self::RegistryConflict
            | Self::NoDocuments => Outcome::Refused,
            Self::SourceFailed => Outcome::Invalid,
            Self::QuireFailed => Outcome::Internal,
            Self::ExportStale => Outcome::Partial,
        }
    }
}

/// A generator's source could not be read (FR-191, exit 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceFailure {
    /// The named source, for example `requirements`.
    pub source: String,
    /// What the source reported.
    pub detail: String,
}

/// How `quire export` failed (FR-191, exit 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuireFailure {
    /// quire's exit status; `null` when it never started or was signalled.
    pub exit_code: Option<i32>,
    /// quire's stderr, or the reason it could not start naming the path tried.
    pub stderr: String,
}

/// The envelope every fallible dossier operation returns.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}", code = .code.as_str())]
pub struct DossierError {
    code: DossierErrorCode,
    message: Box<str>,
    context: BTreeMap<String, String>,
    source_failure: Option<SourceFailure>,
}

impl DossierError {
    /// An error with no context.
    #[must_use]
    pub fn new(code: DossierErrorCode, message: impl Into<Box<str>>) -> Self {
        Self {
            code,
            message: message.into(),
            context: BTreeMap::new(),
            source_failure: None,
        }
    }

    /// Add one context entry.
    #[must_use]
    pub fn with_context(mut self, key: &str, value: impl Into<String>) -> Self {
        self.context.insert(key.to_owned(), value.into());
        self
    }

    /// A generator's source failed.
    #[must_use]
    pub fn source_failed(failure: SourceFailure) -> Self {
        let mut error = Self::new(
            DossierErrorCode::SourceFailed,
            format!("source {} failed: {}", failure.source, failure.detail),
        )
        .with_context("source", failure.source.clone());
        error.source_failure = Some(failure);
        error
    }

    /// `quire export` failed or could not start.
    #[must_use]
    pub fn quire_failed(failure: &QuireFailure) -> Self {
        let mut error = Self::new(
            DossierErrorCode::QuireFailed,
            format!("quire export failed: {}", failure.stderr.trim_end()),
        );
        if let Some(code) = failure.exit_code {
            error = error.with_context("exit_code", code.to_string());
        }
        error
    }

    /// `export` stopped before `quire export` because `document` is missing or
    /// differs from its regenerated bytes.
    #[must_use]
    pub fn export_stale(document: &str) -> Self {
        Self::new(
            DossierErrorCode::ExportStale,
            format!(
                "{document} is missing or differs from its regenerated bytes; quire export was not started"
            ),
        )
        .with_context("document", document)
    }

    /// The stable code.
    #[must_use]
    pub const fn code(&self) -> DossierErrorCode {
        self.code
    }

    /// The exit status this error maps to.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        self.code.outcome()
    }

    /// The human message; not part of the contract.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The context entries, in key order.
    #[must_use]
    pub const fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    /// The failed source, for an exit-3 report.
    #[must_use]
    pub const fn source_failure(&self) -> Option<&SourceFailure> {
        self.source_failure.as_ref()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-191")]
    #[test]
    fn every_code_round_trips_and_no_two_share_a_spelling() {
        let mut seen = std::collections::BTreeSet::new();
        for code in DossierErrorCode::all() {
            assert_eq!(DossierErrorCode::from_code(code.as_str()), Some(*code));
            assert!(seen.insert(code.as_str()), "{} is reused", code.as_str());
        }
        assert_eq!(DossierErrorCode::from_code("DOSSIER_NOPE"), None);
    }

    #[trace("FR-191-AC-3", "FR-191-AC-6", "FR-191-AC-11", "FR-191-AC-12")]
    #[test]
    fn codes_map_to_the_exit_table() {
        assert_eq!(DossierErrorCode::UnknownKind.outcome().code(), 2);
        assert_eq!(DossierErrorCode::VersionRequired.outcome().code(), 2);
        assert_eq!(DossierErrorCode::SourceFailed.outcome().code(), 3);
        assert_eq!(DossierErrorCode::QuireFailed.outcome().code(), 4);
        assert_eq!(DossierErrorCode::ExportStale.outcome().code(), 1);
        // Exit 4 is quire export's alone (FR-191).
        assert_eq!(DossierErrorCode::RegistryConflict.outcome().code(), 2);
        assert_eq!(DossierErrorCode::NoDocuments.outcome().code(), 2);
    }
}
