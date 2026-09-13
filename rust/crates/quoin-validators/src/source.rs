// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Where the gate analysis gets its bytes (quoin#412).
//!
//! The analysis in [`crate::gates`] is the same whether the repository is a
//! directory on disk or a map that arrived over `quoin-core`'s stdin. Only the
//! source of the bytes differs, so only the source of the bytes is abstracted.
//!
//! # Why this seam exists at all
//!
//! `quoin-core`'s library half must be decidable **without a filesystem** —
//! `crates/quoin-core/tests/tc_library_containment.rs` audits every `src/**.rs`
//! but `main.rs` with `engineering_assurance::source_audit` and fails on a host
//! capability. An operation that took a repository path and walked it acquired
//! `Filesystem` and failed that gate. The operation therefore receives a
//! *snapshot* — [`MemoryRepo`] — and the walk stays with the caller that
//! already owns the tree.
//!
//! The seam is a trait rather than two entry points because the alternative is
//! two copies of the analysis, and a port that duplicates the thing it ported
//! has not ported it. The disk implementation ([`crate::repo::DiskRepo`]) is
//! what `inspect_empty_gates` runs on, so the 46-case golden corpus exercises
//! the same analysis the boundary runs.
//!
//! # On-demand reads are part of the contract
//!
//! [`RepoSource::text`] is called only when the analysis actually needs a file.
//! `inspectEmptyGates` called `readFileSync` *inside* `wiring.find(...)`, so a
//! repository whose scripts declare no negative gate claim opened no wiring
//! file at all; an unreadable `Makefile` the oracle never touched must not
//! become a refusal. `tc_377_023` pins both halves, and [`MemoryRepo`] carries
//! the same property: a path the caller could not read is recorded as
//! unreadable and only refuses if the analysis reaches for it.

use std::collections::BTreeMap;
use std::io;

use crate::error::ValidatorError;

/// The repository-relative path naming the root itself.
///
/// Node's `path.relative(repo, repo)` is the empty string, and the walk this
/// mirrors is rooted there, so the empty path is the root's own name rather
/// than a sentinel invented for the wire.
pub const ROOT_PATH: &str = "";

/// Where the gate analysis reads a repository from.
///
/// Implementors supply two things: the ordered list of files the repository
/// holds, and the text at one of them. Everything else — which files are shell
/// scripts, which are wiring, what a gate claim is — is the analysis's, and
/// there is one copy of it.
pub trait RepoSource {
    /// Every file the repository holds, repository-relative and
    /// `/`-separated, in the byte order the walk produces.
    ///
    /// Called once, before any [`RepoSource::text`].
    ///
    /// # Errors
    ///
    /// [`ValidatorError::RepoRootUnreadable`] when the root itself could not be
    /// listed, [`ValidatorError::DirectoryUnreadable`] when a directory inside
    /// it could not be.
    fn paths(&mut self) -> Result<Vec<String>, ValidatorError>;

    /// The text at `path`, read on demand.
    ///
    /// Invalid UTF-8 is replaced rather than refused, matching Node's
    /// `readFileSync(path, "utf8")`: a gate script with one bad byte still has
    /// to be inspected.
    ///
    /// # Errors
    ///
    /// [`ValidatorError::FileUnreadable`] when the text cannot be produced.
    fn text(&mut self, path: &str) -> Result<String, ValidatorError>;
}

/// A repository the caller already read, held in memory.
///
/// This is what `quoin-core`'s `validators.run` builds out of its request. It
/// performs no I/O and names no host capability, which is the whole point.
///
/// Three states are representable, because the caller genuinely observes three:
/// a file it read, a file it found but could not read, and a directory it could
/// not list. The third is separated from the second because the walk aborts on
/// it — a subtree that vanished mid-walk means the answer would be computed
/// over a repository nobody has seen, and an empty result must mean the
/// validator looked and found nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryRepo {
    files: BTreeMap<String, Option<String>>,
    unlistable: Vec<String>,
}

impl MemoryRepo {
    /// An empty snapshot: a repository with no files at all.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a file the caller read.
    #[must_use]
    pub fn with_file(mut self, path: impl Into<String>, text: impl Into<String>) -> Self {
        self.files.insert(path.into(), Some(text.into()));
        self
    }

    /// Record a file the caller found but could not read.
    ///
    /// It still counts as present — it is classified as shell or wiring like
    /// any other path — and only refuses if the analysis reaches for its text.
    #[must_use]
    pub fn with_unreadable_file(mut self, path: impl Into<String>) -> Self {
        self.files.insert(path.into(), None);
        self
    }

    /// Record a directory the caller could not list. [`ROOT_PATH`] names the
    /// repository root.
    #[must_use]
    pub fn with_unlistable_directory(mut self, path: impl Into<String>) -> Self {
        self.unlistable.push(path.into());
        self
    }
}

/// The `io::Error` a caller-reported refusal carries.
///
/// The caller's own `errno` did not cross the wire, and inventing one would put
/// a fact in the envelope that nobody observed. The variant and the path are
/// the API; this source is the sentence under them.
fn reported_by_caller() -> io::Error {
    io::Error::other("the caller reported this path as unreadable")
}

impl RepoSource for MemoryRepo {
    fn paths(&mut self) -> Result<Vec<String>, ValidatorError> {
        if let Some(path) = self.unlistable.iter().find(|path| *path == ROOT_PATH) {
            return Err(ValidatorError::RepoRootUnreadable {
                path: path.into(),
                source: reported_by_caller(),
            });
        }
        if let Some(path) = self.unlistable.first() {
            return Err(ValidatorError::DirectoryUnreadable {
                path: path.into(),
                source: reported_by_caller(),
            });
        }
        Ok(self.files.keys().cloned().collect())
    }

    fn text(&mut self, path: &str) -> Result<String, ValidatorError> {
        match self.files.get(path) {
            Some(Some(text)) => Ok(text.clone()),
            // A path the analysis asks for that the snapshot does not hold is
            // the same condition as one it holds as unreadable: the text does
            // not exist and no verdict can be reached without it. Answering
            // "empty file" instead would turn a missing wiring body into a
            // clean verdict, which is the one degradation this crate refuses.
            Some(None) | None => Err(ValidatorError::FileUnreadable {
                path: path.into(),
                source: reported_by_caller(),
            }),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{MemoryRepo, ROOT_PATH, RepoSource};
    use crate::error::ErrorCode;

    /// Paths come back in sorted order regardless of insertion order, because
    /// `wiredBy` is the FIRST wiring file in sort order and a different order
    /// is a different payload.
    #[test]
    fn tc_412_001_paths_are_sorted_not_insertion_ordered() {
        let mut repo = MemoryRepo::new()
            .with_file("scripts/z.sh", "")
            .with_file("Makefile", "")
            .with_file("scripts/a.sh", "");
        assert_eq!(
            repo.paths().unwrap(),
            vec!["Makefile", "scripts/a.sh", "scripts/z.sh"]
        );
    }

    /// An unreadable file is present for classification and refuses only when
    /// its text is asked for.
    #[test]
    fn tc_412_002_an_unreadable_file_is_present_but_has_no_text() {
        let mut repo = MemoryRepo::new().with_unreadable_file("Makefile");
        assert_eq!(repo.paths().unwrap(), vec!["Makefile"]);
        let error = repo.text("Makefile").unwrap_err();
        assert_eq!(error.code(), ErrorCode::FileUnreadable);
        assert_eq!(error.path(), std::path::Path::new("Makefile"));
    }

    /// The root and an interior directory are different refusals, because one
    /// is the caller's mistake and the other is the environment's.
    #[test]
    fn tc_412_003_an_unlistable_root_and_an_unlistable_subtree_differ() {
        let mut root = MemoryRepo::new().with_unlistable_directory(ROOT_PATH);
        assert_eq!(
            root.paths().unwrap_err().code(),
            ErrorCode::RepoRootUnreadable
        );

        let mut subtree = MemoryRepo::new()
            .with_file("Makefile", "")
            .with_unlistable_directory("scripts");
        let error = subtree.paths().unwrap_err();
        assert_eq!(error.code(), ErrorCode::DirectoryUnreadable);
        assert_eq!(error.path(), std::path::Path::new("scripts"));
    }

    /// A path the snapshot never heard of is a refusal, not an empty file.
    #[test]
    fn tc_412_004_an_absent_path_is_a_refusal_not_an_empty_file() {
        let mut repo = MemoryRepo::new();
        assert_eq!(
            repo.text("Makefile").unwrap_err().code(),
            ErrorCode::FileUnreadable
        );
    }
}
