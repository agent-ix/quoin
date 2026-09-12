// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Validated identity newtypes for installed modules (quoin#381).

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ModulesError;

/// The declared name of a spec module.
///
/// A module name is used directly as a directory name under
/// `~/.ix/filament/modules`, so it is validated against path separators and
/// relative-path components at construction. Every place that joins a name onto
/// a directory takes a `ModuleName`, not a `String`, which is what makes that
/// check unskippable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ModuleName(String);

impl ModuleName {
    /// Validate and wrap a module name.
    ///
    /// # Errors
    /// [`ModulesError::InvalidModuleName`] when the name is empty, is `.` or
    /// `..`, contains `/`, `\` or a NUL, or starts with `-`.
    pub fn new(name: impl Into<String>) -> Result<Self, ModulesError> {
        let name = name.into();
        let legal = !name.is_empty()
            && name != "."
            && name != ".."
            && !name.starts_with('-')
            && !name
                .chars()
                .any(|c| c == '/' || c == '\\' || c == '\0' || c.is_control());
        if legal {
            Ok(Self(name))
        } else {
            Err(ModulesError::InvalidModuleName { name })
        }
    }

    /// The name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// This module's directory under `modules_dir`.
    ///
    /// Safe by construction: the name cannot contain a separator or `..`.
    #[must_use]
    pub fn dir_under(&self, modules_dir: &Path) -> PathBuf {
        modules_dir.join(&self.0)
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ModuleName {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// A resolved git commit id, as forty lowercase hex characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct CommitSha(String);

impl CommitSha {
    /// Wrap a full commit id, refusing anything that is not 40 hex digits.
    ///
    /// # Errors
    /// [`ModulesError::InvalidSourceField`] when the value is not a full sha.
    pub fn new(sha: impl Into<String>) -> Result<Self, ModulesError> {
        let sha = sha.into();
        if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(Self(sha.to_ascii_lowercase()))
        } else {
            Err(ModulesError::InvalidSourceField {
                field: "sha",
                rule: crate::error::SourceFieldRule::NonEmpty,
            })
        }
    }

    /// The sha as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommitSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CommitSha {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Registry files written by the TypeScript implementation record
        // whatever `git rev-parse HEAD` printed. Refusing a malformed value
        // here would make an old registry unreadable, so the newtype is
        // permissive on the *read* path and strict on construction.
        Ok(Self(String::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]

    use super::*;

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_200_module_name_refuses_path_traversal() {
        assert!(ModuleName::new("spec-objects-business").is_ok());
        for bad in ["", ".", "..", "a/b", "a\\b", "-x", "a\nb"] {
            assert!(
                ModuleName::new(bad).is_err(),
                "{bad:?} must not be usable as a directory name"
            );
        }
    }

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_201_commit_sha_requires_forty_hex_digits() {
        assert!(CommitSha::new("4e6522fc32f82cac82a3489a4192e17a94f539c3").is_ok());
        assert!(CommitSha::new("4e6522f").is_err());
        assert!(CommitSha::new("z".repeat(40)).is_err());
        assert_eq!(
            CommitSha::new("4E6522FC32F82CAC82A3489A4192E17A94F539C3")
                .expect("uppercase hex is a sha")
                .as_str(),
            "4e6522fc32f82cac82a3489a4192e17a94f539c3"
        );
    }
}
