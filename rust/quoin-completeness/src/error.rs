// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! The crate boundary error (FR-037; EPIC #373 FR-096).
//!
//! Deliberately small. Almost everything this crate meets that is wrong with an
//! input is **reported, not raised**: an unreadable module contributes no
//! declaration, a document whose frontmatter will not parse lands in
//! `unreadable`, and a vocabulary that cannot be resolved lands in `unresolved`.
//! That is the design: the check exists to stop silent green, so swallowing a
//! bad input would be the defect. What remains here is the small set of
//! conditions a caller genuinely cannot proceed past.

use std::path::{Path, PathBuf};

/// Stable code for one [`CompletenessError`] condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CompletenessErrorCode {
    /// `QCMP-001` — a bundle root was given that is not a directory.
    BundleRootNotADirectory,
    /// `QCMP-002` — a bundle root exists but could not be walked.
    BundleRootUnreadable,
}

impl CompletenessErrorCode {
    /// The stable wire code, e.g. `QCMP-001`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BundleRootNotADirectory => "QCMP-001",
            Self::BundleRootUnreadable => "QCMP-002",
        }
    }

    /// Every code, so a catalogue check can enumerate them.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::BundleRootNotADirectory, Self::BundleRootUnreadable]
    }

    /// Parse a wire code back to its variant.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for CompletenessErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every way assessing a bundle can fail outright.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompletenessError {
    /// The bundle root exists but is a file.
    #[error("QCMP-001: bundle root {path} is not a directory")]
    BundleRootNotADirectory {
        /// The path that is not a directory.
        path: PathBuf,
    },

    /// The bundle root could not be walked.
    #[error("QCMP-002: bundle root {path} could not be read: {source}")]
    BundleRootUnreadable {
        /// The directory that could not be listed.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },
}

impl CompletenessError {
    /// The stable code for this condition.
    #[must_use]
    pub fn code(&self) -> CompletenessErrorCode {
        match self {
            Self::BundleRootNotADirectory { .. } => CompletenessErrorCode::BundleRootNotADirectory,
            Self::BundleRootUnreadable { .. } => CompletenessErrorCode::BundleRootUnreadable,
        }
    }

    /// The path the failure concerns.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::BundleRootNotADirectory { path } | Self::BundleRootUnreadable { path, .. } => {
                path
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-096
    #[test]
    fn tc_378_200_every_code_round_trips_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for code in CompletenessErrorCode::all() {
            assert_eq!(CompletenessErrorCode::from_code(code.as_str()), Some(*code));
            assert!(seen.insert(code.as_str()), "duplicate code {code}");
        }
        assert_eq!(CompletenessErrorCode::from_code("QCMP-999"), None);
    }
}
