// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Where installed modules and their registry live (quoin#381, FR-019).
//!
//! One home directory, one modules directory, one registry file — the same
//! layout `src/catalog.ts` and quire-rs both read.

use std::path::{Path, PathBuf};

/// A resolved `~/.ix` home.
///
/// A newtype so a home directory and a modules directory cannot be swapped at
/// a call site; they are both `PathBuf` and differ by one segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IxHome(PathBuf);

impl IxHome {
    /// Wrap an explicit home directory.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Resolve `$IX_HOME`, falling back to `<home>/.ix`.
    ///
    /// `home_dir` is passed rather than read, so a caller can stay hermetic.
    #[must_use]
    pub fn resolve(ix_home_var: Option<&str>, home_dir: Option<&Path>) -> Self {
        if let Some(explicit) = ix_home_var.filter(|v| !v.is_empty()) {
            return Self(PathBuf::from(explicit));
        }
        Self(home_dir.map_or_else(|| PathBuf::from(".ix"), |home| home.join(".ix")))
    }

    /// The home directory itself.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// The single directory that holds installed Filament modules; also read by
    /// quire-rs.
    #[must_use]
    pub fn modules_dir(&self) -> PathBuf {
        self.0.join("filament").join("modules")
    }

    /// The install registry file.
    #[must_use]
    pub fn registry_path(&self) -> PathBuf {
        self.0.join("filament").join("registry.json")
    }

    /// The source cache root.
    ///
    /// Deliberately **not** `<home>/cache/ts-plugin-kit`: that tree holds
    /// checked-out non-bare clones the TypeScript implementation manipulates
    /// with `git checkout` and `git sparse-checkout`, and this crate keeps bare
    /// repositories with no worktree. Sharing one directory between the two
    /// layouts during staged coexistence would corrupt whichever ran second.
    #[must_use]
    pub fn cache_root(&self) -> PathBuf {
        self.0.join("cache").join("quoin-modules")
    }
}

/// Every install path for one home, resolved together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPaths {
    /// Where sources are cached.
    pub cache_root: PathBuf,
    /// Where modules are materialized.
    pub target_root: PathBuf,
    /// The registry file.
    pub registry_path: PathBuf,
}

impl InstallPaths {
    /// Derive every install path from a home.
    #[must_use]
    pub fn for_home(home: &IxHome) -> Self {
        Self {
            cache_root: home.cache_root(),
            target_root: home.modules_dir(),
            registry_path: home.registry_path(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_240_home_derives_modules_dir_and_registry() {
        let home = IxHome::new("/h");
        assert_eq!(home.modules_dir(), PathBuf::from("/h/filament/modules"));
        assert_eq!(
            home.registry_path(),
            PathBuf::from("/h/filament/registry.json")
        );
    }

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_241_ix_home_env_var_wins_over_the_home_directory() {
        assert_eq!(
            IxHome::resolve(Some("/explicit"), Some(Path::new("/home/u"))).as_path(),
            Path::new("/explicit")
        );
        assert_eq!(
            IxHome::resolve(Some(""), Some(Path::new("/home/u"))).as_path(),
            Path::new("/home/u/.ix")
        );
        assert_eq!(
            IxHome::resolve(None, Some(Path::new("/home/u"))).as_path(),
            Path::new("/home/u/.ix")
        );
    }
}
