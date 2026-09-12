// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Validated identities (quoin#379).
//!
//! Every one of these was a bare `string` across the subprocess boundary, and
//! the boundary is where they were confusable: `runQuire(["coverage",
//! "--scope", scope, "--module", module])` is four strings in a row, and
//! swapping two of them is a runtime diagnostic at best. Here the constructor
//! is the check and the type is the proof, so a scope cannot be passed as a
//! module root.
//!
//! Path-shaped ids carry a **named trust boundary**: `ScopeRoot::open` and
//! `ModuleRoot::open` touch the filesystem, canonicalize, and refuse what is
//! not a directory. There is no infallible constructor from a caller-supplied
//! string — that is the point.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The repository root a run is scoped to.
///
/// Canonicalized on construction, because `coverage` and `validate` once
/// resolved *different* roots for one repository — one canonicalized, the
/// other did not (quire-rs#113).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeRoot(PathBuf);

impl ScopeRoot {
    /// Resolve and canonicalize a scope.
    ///
    /// # Errors
    /// [`Error::ScopeNotADirectory`] when the path is not a directory, and
    /// [`Error::Io`] when it cannot be canonicalized.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_dir() {
            return Err(Error::ScopeNotADirectory {
                path: path.to_path_buf(),
            });
        }
        let canonical = path.canonicalize().map_err(|error| Error::Io {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
        Ok(Self(canonical))
    }

    /// The canonical path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// The document root, `<scope>/spec`.
    ///
    /// The directory name is a constant here and in the code-walk exclusion so
    /// the two cannot drift (quire-rs#113); `spec/` is convention, not
    /// configuration.
    ///
    /// # Errors
    /// [`Error::DocumentRootMissing`] when `<scope>/spec` is not a directory.
    pub fn document_root(&self) -> Result<PathBuf> {
        let root = self.0.join(DOCUMENT_ROOT_DIR);
        if !root.is_dir() {
            return Err(Error::DocumentRootMissing { path: root });
        }
        // A resolvable directory that cannot be canonicalized is left as-is
        // rather than failing: the walk can still read it.
        Ok(root.canonicalize().unwrap_or(root))
    }

    /// Render a path as scope-relative, refusing one that escapes the scope.
    ///
    /// rust-review §10: a path that reaches a report has been checked against
    /// traversal, not merely string-trimmed.
    ///
    /// # Errors
    /// [`Error::PathEscapesRoot`] when the path resolves outside the scope.
    pub fn relativize(&self, path: impl AsRef<Path>) -> Result<DocumentPath> {
        let path = path.as_ref();
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.0.join(path)
        };
        // `canonicalize` needs the file to exist; a document being classified
        // always does. When it does not, fall back to lexical containment so a
        // caller naming a not-yet-written path still gets a refusal rather
        // than a silently escaping relative path.
        let resolved = absolute.canonicalize().unwrap_or(absolute);
        let relative = resolved
            .strip_prefix(&self.0)
            .map_err(|_| Error::PathEscapesRoot {
                path: resolved.clone(),
                root: self.0.clone(),
            })?;
        Ok(DocumentPath(relative.to_string_lossy().into_owned()))
    }
}

/// The name of the document root under a scope.
pub const DOCUMENT_ROOT_DIR: &str = "spec";

/// A module directory supplying archetypes, a traceability model, or clause
/// sets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleRoot(PathBuf);

impl ModuleRoot {
    /// Resolve and canonicalize a module root.
    ///
    /// # Errors
    /// [`Error::ModuleLoad`] when the path is not a directory, and
    /// [`Error::Io`] when it cannot be canonicalized.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_dir() {
            return Err(Error::ModuleLoad {
                path: path.to_path_buf(),
                reason: "not a directory".to_string(),
            });
        }
        let canonical = path.canonicalize().map_err(|error| Error::Io {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
        Ok(Self(canonical))
    }

    /// The canonical path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// A scope-relative document location, exactly as it appears in a report.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentPath(String);

impl DocumentPath {
    /// The scope-relative path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DocumentPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A repository identity copied into an assurance source premise.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepositoryId(String);

impl RepositoryId {
    /// Accept a non-empty identity.
    ///
    /// # Errors
    /// [`Error::RepositoryEmpty`] for an empty or whitespace-only value.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(Error::RepositoryEmpty);
        }
        Ok(Self(value))
    }

    /// The identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A full lowercase Git object id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RevisionId(String);

impl RevisionId {
    /// Accept 40 lowercase hexadecimal digits.
    ///
    /// Checked here as well as in the engine: the engine's refusal is the
    /// fail-closed backstop, and this one lets a caller reject a bad argument
    /// before a whole corpus walk has been paid for.
    ///
    /// # Errors
    /// [`Error::RevisionMalformed`] for anything else.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let well_formed = value.len() == 40
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if !well_formed {
            return Err(Error::RevisionMalformed { revision: value });
        }
        Ok(Self(value))
    }

    /// The revision.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The exact identity of a module-supplied clause set (quire-rs FR-067).
///
/// Three `String` arguments in a row is precisely the call site this type
/// exists to make un-swappable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClauseSetRef {
    /// The authority that publishes the set.
    pub authority: ClauseSetAuthority,
    /// The set's id under that authority.
    pub id: ClauseSetId,
    /// The exact version. Never a range: a clause set is evaluated at one
    /// version or not at all.
    pub version: ClauseSetVersion,
}

macro_rules! nonempty_newtype {
    ($(#[$meta:meta])* $name:ident, $field:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Accept a non-empty value.
            ///
            /// # Errors
            /// [`Error::ContextEntryMalformed`] for an empty or
            /// whitespace-only value, naming the field.
            pub fn new(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(Error::ContextEntryMalformed {
                        entry: format!("{}=<empty>", $field),
                    });
                }
                Ok(Self(value))
            }

            /// The value.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

nonempty_newtype!(
    /// A clause-set authority.
    ClauseSetAuthority,
    "authority"
);
nonempty_newtype!(
    /// A clause-set id.
    ClauseSetId,
    "set"
);
nonempty_newtype!(
    /// An exact clause-set version.
    ClauseSetVersion,
    "version"
);

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// Trace: FR-096
    #[test]
    fn tc_379_010_a_revision_must_be_forty_lowercase_hex() {
        assert!(RevisionId::new("a".repeat(40)).is_ok());
        for bad in [
            "a".repeat(39),
            "a".repeat(41),
            "A".repeat(40),
            "g".repeat(40),
            String::new(),
        ] {
            let error = RevisionId::new(bad.clone()).expect_err("must refuse");
            assert_eq!(error.code(), crate::error::ErrorCode::RevisionMalformed);
        }
    }

    /// Trace: FR-096
    #[test]
    fn tc_379_011_an_empty_repository_identity_is_refused() {
        assert!(RepositoryId::new("agent-ix/quoin").is_ok());
        assert_eq!(
            RepositoryId::new("   ").expect_err("must refuse").code(),
            crate::error::ErrorCode::RepositoryEmpty
        );
    }

    /// Trace: FR-096
    #[test]
    fn tc_379_012_a_missing_scope_is_refused_rather_than_walked() {
        let error = ScopeRoot::open("/definitely/not/here").expect_err("must refuse");
        assert_eq!(error.code(), crate::error::ErrorCode::ScopeNotADirectory);
    }
}
