// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! The crate boundary error (quoin FR-070; EPIC #373 FR-096).
//!
//! One `thiserror` enum, one stable code per variant, and a typed variant per
//! condition a caller must distinguish. Everything here is a failure to *read*
//! an input — a manifest that will not parse, a vendored schema that will not
//! compile. A module that parses but violates the contract is not an error: it
//! is a [`crate::SemanticDiagnostic`], because the caller reports it rather
//! than aborting on it.

use std::path::{Path, PathBuf};

/// Stable code for one [`SemanticError`] condition.
///
/// Codes are **never renamed or reused**: they are what a caller matches on and
/// what a consumer of the eventual `quoin-core` boundary sees on stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SemanticErrorCode {
    /// `QSEM-001` — a manifest file could not be read from disk.
    ManifestUnreadable,
    /// `QSEM-002` — a manifest file's bytes are not YAML.
    ManifestNotYaml,
    /// `QSEM-003` — a manifest parsed, but its document is not a mapping.
    ManifestNotAMapping,
    /// `QSEM-004` — a vendored schema file could not be read from disk.
    VendoredSchemaUnreadable,
    /// `QSEM-005` — a vendored schema file's bytes are not JSON.
    VendoredSchemaNotJson,
    /// `QSEM-006` — a vendored schema is JSON but does not compile.
    VendoredSchemaInvalid,
    /// `QSEM-007` — a vendored schema does not carry the sub-schema asked for.
    VendoredSchemaIncomplete,
    /// `QSEM-008` — a corpus root could not be walked.
    CorpusUnreadable,
}

impl SemanticErrorCode {
    /// The stable wire code, e.g. `QSEM-001`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ManifestUnreadable => "QSEM-001",
            Self::ManifestNotYaml => "QSEM-002",
            Self::ManifestNotAMapping => "QSEM-003",
            Self::VendoredSchemaUnreadable => "QSEM-004",
            Self::VendoredSchemaNotJson => "QSEM-005",
            Self::VendoredSchemaInvalid => "QSEM-006",
            Self::VendoredSchemaIncomplete => "QSEM-007",
            Self::CorpusUnreadable => "QSEM-008",
        }
    }

    /// Every code, so a catalogue check can enumerate them.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::ManifestUnreadable,
            Self::ManifestNotYaml,
            Self::ManifestNotAMapping,
            Self::VendoredSchemaUnreadable,
            Self::VendoredSchemaNotJson,
            Self::VendoredSchemaInvalid,
            Self::VendoredSchemaIncomplete,
            Self::CorpusUnreadable,
        ]
    }

    /// Parse a wire code back to its variant.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for SemanticErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every way reading the semantic contract can fail.
///
/// Each variant names one condition a caller must distinguish, and carries the
/// path it concerns rather than a pre-formatted sentence.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SemanticError {
    /// A manifest file could not be read.
    #[error("QSEM-001: manifest {path} could not be read: {source}")]
    ManifestUnreadable {
        /// The manifest that could not be read.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// A manifest file's bytes are not YAML.
    #[error("QSEM-002: manifest {path} is not YAML: {source}")]
    ManifestNotYaml {
        /// The manifest that would not parse.
        path: PathBuf,
        /// The underlying parse failure.
        source: serde_norway::Error,
    },

    /// A manifest parsed but is not a mapping, so it has no `semantic` key.
    #[error("QSEM-003: manifest {path} is not a mapping")]
    ManifestNotAMapping {
        /// The manifest whose document is a scalar or a sequence.
        path: PathBuf,
    },

    /// A vendored schema file could not be read.
    #[error("QSEM-004: vendored schema {path} could not be read: {source}")]
    VendoredSchemaUnreadable {
        /// The schema file that could not be read.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// A vendored schema file's bytes are not JSON.
    #[error("QSEM-005: vendored schema {path} is not JSON: {source}")]
    VendoredSchemaNotJson {
        /// The schema file that would not parse.
        path: PathBuf,
        /// The underlying parse failure.
        source: serde_json::Error,
    },

    /// A vendored schema is JSON but the validator refused to compile it.
    #[error("QSEM-006: vendored schema {path} does not compile: {detail}")]
    VendoredSchemaInvalid {
        /// The schema file that would not compile.
        path: PathBuf,
        /// The validator's own account of the refusal.
        detail: String,
    },

    /// A vendored schema does not carry the sub-schema this crate reads from it.
    #[error("QSEM-007: vendored schema {path} has no {pointer}")]
    VendoredSchemaIncomplete {
        /// The schema file that is missing a member.
        path: PathBuf,
        /// The JSON pointer that resolved to nothing.
        pointer: &'static str,
    },

    /// A corpus root could not be walked.
    #[error("QSEM-008: corpus root {path} could not be read: {source}")]
    CorpusUnreadable {
        /// The directory that could not be listed.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },
}

impl SemanticError {
    /// The stable code for this condition.
    #[must_use]
    pub fn code(&self) -> SemanticErrorCode {
        match self {
            Self::ManifestUnreadable { .. } => SemanticErrorCode::ManifestUnreadable,
            Self::ManifestNotYaml { .. } => SemanticErrorCode::ManifestNotYaml,
            Self::ManifestNotAMapping { .. } => SemanticErrorCode::ManifestNotAMapping,
            Self::VendoredSchemaUnreadable { .. } => SemanticErrorCode::VendoredSchemaUnreadable,
            Self::VendoredSchemaNotJson { .. } => SemanticErrorCode::VendoredSchemaNotJson,
            Self::VendoredSchemaInvalid { .. } => SemanticErrorCode::VendoredSchemaInvalid,
            Self::VendoredSchemaIncomplete { .. } => SemanticErrorCode::VendoredSchemaIncomplete,
            Self::CorpusUnreadable { .. } => SemanticErrorCode::CorpusUnreadable,
        }
    }

    /// The path the failure concerns.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::ManifestUnreadable { path, .. }
            | Self::ManifestNotYaml { path, .. }
            | Self::ManifestNotAMapping { path }
            | Self::VendoredSchemaUnreadable { path, .. }
            | Self::VendoredSchemaNotJson { path, .. }
            | Self::VendoredSchemaInvalid { path, .. }
            | Self::VendoredSchemaIncomplete { path, .. }
            | Self::CorpusUnreadable { path, .. } => path,
        }
    }
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Trace: FR-096
    #[test]
    fn tc_378_001_every_code_round_trips() {
        for code in SemanticErrorCode::all() {
            assert_eq!(SemanticErrorCode::from_code(code.as_str()), Some(*code));
        }
    }

    /// Trace: FR-096
    #[test]
    fn tc_378_002_codes_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for code in SemanticErrorCode::all() {
            assert!(seen.insert(code.as_str()), "duplicate code {code}");
        }
        assert_eq!(seen.len(), SemanticErrorCode::all().len());
    }

    /// Trace: FR-096
    #[test]
    fn tc_378_003_unknown_code_is_none() {
        assert_eq!(SemanticErrorCode::from_code("QSEM-999"), None);
    }
}
