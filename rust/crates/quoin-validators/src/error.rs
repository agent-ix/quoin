// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The crate's single error boundary (quoin#377, FR-096).
//!
//! The TypeScript this crate ports lets raw `fs` exceptions escape
//! `inspectEmptyGates`, so a caller can only tell "the repository root you
//! handed me is not usable" from "the tree moved while I was reading it" by
//! pattern-matching an `ENOENT` string. That is exactly the discriminant-in-prose
//! shape the Rust boundary exists to remove: **the variant set is the API, the
//! message is not.**
//!
//! There are three conditions a caller must genuinely distinguish, so there are
//! three variants and three stable codes:
//!
//! | Code | Condition | Who is wrong |
//! |---|---|---|
//! | `QV-E001` | the `repo` argument is not a readable directory | the caller |
//! | `QV-E002` | a directory *inside* the tree could not be listed | the environment |
//! | `QV-E003` | a file the walk already found could not be read | the environment |
//!
//! `QV-E001` is separated from `QV-E002` because it is the only one a caller can
//! fix by passing a different argument; the other two mean the filesystem changed
//! or permissions deny a subtree, and a run that hits them has **not** produced a
//! clean verdict. Nothing here degrades to "no findings" — an empty result must
//! mean the validator looked and found nothing.
//!
//! Codes are stable: never renamed, never reused.

use std::io;
use std::path::PathBuf;

/// Stable, `Copy` code for every condition [`ValidatorError`] can report.
///
/// Kept separate from the error itself so a code can be logged, compared, or
/// carried across the `quoin-core` subprocess boundary without the `io::Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// `QV-E001` — the repository root is not a readable directory.
    RepoRootUnreadable,
    /// `QV-E002` — a directory inside the repository could not be listed.
    DirectoryUnreadable,
    /// `QV-E003` — a file inside the repository could not be read.
    FileUnreadable,
}

impl ErrorCode {
    /// Every code, in declaration order. Exhaustive by construction.
    pub const ALL: &'static [Self] = &[
        Self::RepoRootUnreadable,
        Self::DirectoryUnreadable,
        Self::FileUnreadable,
    ];

    /// The wire spelling of the code. Stable across releases.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RepoRootUnreadable => "QV-E001",
            Self::DirectoryUnreadable => "QV-E002",
            Self::FileUnreadable => "QV-E003",
        }
    }

    /// Parse a wire spelling back into a code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every way [`crate::inspect_empty_gates`] can refuse to produce a verdict.
///
/// Each variant carries the offending [`PathBuf`] as a typed field rather than
/// interpolated prose, so a caller can report the path without re-parsing the
/// message.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ValidatorError {
    /// The `repo` argument is not a directory this process can list.
    #[error("QV-E001: repository root {path} is not a readable directory")]
    RepoRootUnreadable {
        /// The root as supplied by the caller.
        path: PathBuf,
        /// The underlying filesystem refusal.
        #[source]
        source: io::Error,
    },

    /// A directory discovered during the walk could not be listed.
    #[error("QV-E002: cannot list directory {path} under the repository root")]
    DirectoryUnreadable {
        /// The directory that failed, absolute or as joined from the root.
        path: PathBuf,
        /// The underlying filesystem refusal.
        #[source]
        source: io::Error,
    },

    /// A file discovered during the walk could not be read.
    #[error("QV-E003: cannot read file {path} under the repository root")]
    FileUnreadable {
        /// The file that failed, absolute or as joined from the root.
        path: PathBuf,
        /// The underlying filesystem refusal.
        #[source]
        source: io::Error,
    },
}

impl ValidatorError {
    /// The stable code for this error.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::RepoRootUnreadable { .. } => ErrorCode::RepoRootUnreadable,
            Self::DirectoryUnreadable { .. } => ErrorCode::DirectoryUnreadable,
            Self::FileUnreadable { .. } => ErrorCode::FileUnreadable,
        }
    }

    /// The path the error is about.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        match self {
            Self::RepoRootUnreadable { path, .. }
            | Self::DirectoryUnreadable { path, .. }
            | Self::FileUnreadable { path, .. } => path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, ValidatorError};
    use std::io;
    use std::path::PathBuf;

    /// Every declared code round-trips through its wire spelling, so `ALL` and
    /// `as_str` cannot drift apart when a variant is added.
    #[test]
    fn tc_377_001_every_code_round_trips() {
        for code in ErrorCode::ALL {
            assert_eq!(ErrorCode::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(ErrorCode::ALL.len(), 3);
        assert_eq!(ErrorCode::from_code("QV-E999"), None);
    }

    /// Codes are distinct; a copy-pasted variant sharing a code fails here.
    #[test]
    fn tc_377_002_codes_are_distinct() {
        let mut seen: Vec<&str> = ErrorCode::ALL.iter().map(|c| c.as_str()).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before);
    }

    /// `code()` and `path()` are derived from the variant, not from the message.
    #[test]
    fn tc_377_003_accessors_follow_the_variant() {
        let err = ValidatorError::FileUnreadable {
            path: PathBuf::from("/repo/scripts/gate.sh"),
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };
        assert_eq!(err.code(), ErrorCode::FileUnreadable);
        assert_eq!(err.path(), PathBuf::from("/repo/scripts/gate.sh"));
        assert!(err.to_string().starts_with("QV-E003: "));
    }
}
