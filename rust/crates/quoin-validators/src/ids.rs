// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Identity newtypes for the validator payload (quoin#377).
//!
//! The TypeScript finding is nine `string | number` fields, three of which are
//! identities with different rules: an obligation id, a repository-relative
//! path, and a 1-based line number. Swapping two of them at a call site is a
//! type error here and was a silent payload defect there.

use std::fmt;
use std::num::NonZeroU64;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// A requirement obligation as it was written in the gate comment, e.g.
/// `FR-001-AC-1`.
///
/// The claim regex is case-insensitive, so this deliberately preserves the
/// author's spelling rather than normalising it: the finding must point at what
/// the file actually says.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObligationId(Box<str>);

impl ObligationId {
    /// Wrap an obligation spelling lifted from a gate comment.
    #[must_use]
    pub fn new(value: impl Into<Box<str>>) -> Self {
        Self(value.into())
    }

    /// The obligation as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObligationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A path relative to the repository root, always written with `/` separators.
///
/// Findings are compared, sorted, and matched against build wiring by this
/// string, so the separator normalisation is part of the identity and not a
/// display concern — a Windows-shaped `scripts\gate.sh` and a POSIX
/// `scripts/gate.sh` are the same gate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepoPath(Box<str>);

impl RepoPath {
    /// Build a repository-relative path from an already-relative [`Path`].
    ///
    /// Non-UTF-8 components are replaced, matching Node's `readdirSync` +
    /// string-`join` behaviour, which never refuses a name it cannot decode.
    #[must_use]
    pub fn from_relative(relative: &Path) -> Self {
        Self::new(relative.to_string_lossy().as_ref())
    }

    /// Wrap an already repository-relative, `/`-separated path.
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.replace('\\', "/").into_boxed_str())
    }

    /// The path as a `/`-separated string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The final path segment, or the whole path when it has no separator.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }
}

impl fmt::Display for RepoPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A 1-based line number inside a source file.
///
/// `NonZeroU64` rather than `usize`: line 0 does not exist, and the payload
/// crosses a JSON boundary where a 0 would be read as "unknown".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LineNumber(NonZeroU64);

impl LineNumber {
    /// The first line of a file.
    pub const FIRST: Self = Self(NonZeroU64::MIN);

    /// Convert a 0-based enumeration index into a 1-based line number.
    ///
    /// Saturates rather than panicking or wrapping; a file with `u64::MAX`
    /// lines is not reachable, but a silent wrap to 0 would be a payload lie.
    #[must_use]
    pub fn from_zero_based(index: usize) -> Self {
        let one_based = u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1);
        Self(NonZeroU64::new(one_based).unwrap_or(NonZeroU64::MIN))
    }

    /// The 1-based value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for LineNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{LineNumber, ObligationId, RepoPath};
    use std::path::Path;

    /// The 0-based enumeration index the walk produces becomes a 1-based line.
    #[test]
    fn tc_377_004_line_numbers_are_one_based() {
        assert_eq!(LineNumber::from_zero_based(0).get(), 1);
        assert_eq!(LineNumber::from_zero_based(3).get(), 4);
        assert_eq!(LineNumber::from_zero_based(usize::MAX).get(), u64::MAX);
        assert_eq!(LineNumber::FIRST.get(), 1);
    }

    /// Backslashes are normalised at construction, not at display.
    #[test]
    fn tc_377_005_repo_paths_are_portable() {
        let path = RepoPath::from_relative(Path::new("scripts/ci/gate.sh"));
        assert_eq!(path.as_str(), "scripts/ci/gate.sh");
        assert_eq!(path.file_name(), "gate.sh");
        assert_eq!(
            RepoPath::new("scripts\\gate.sh").as_str(),
            "scripts/gate.sh"
        );
        assert_eq!(RepoPath::new("Makefile").file_name(), "Makefile");
    }

    /// Obligation spelling is preserved verbatim.
    #[test]
    fn tc_377_006_obligation_preserves_spelling() {
        assert_eq!(ObligationId::new("fr-003").as_str(), "fr-003");
        assert_eq!(ObligationId::new("FR-001-AC-1").to_string(), "FR-001-AC-1");
    }
}
