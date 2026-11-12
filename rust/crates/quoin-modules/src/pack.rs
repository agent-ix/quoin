// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Packs: the seam that tells an install whether a module is one, and
//! the git work-tree walk that keeps one out of a repository (FR-137, FR-138).
//!
//! This crate decides nothing about rights. A [`PackProbe`] is handed the
//! module directory and answers with the restricted clause sets the engine
//! reports; an empty answer is a module that is not a pack. The probe's
//! production implementation lives in `quoin-core`, over `quoin-quire`.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// One restricted clause set, named without its text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RestrictedSet {
    /// The set's authority.
    pub authority: String,
    /// The set's id.
    pub id: String,
    /// The set's version.
    pub version: String,
}

impl fmt::Display for RestrictedSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}@{}", self.authority, self.id, self.version)
    }
}

/// Reports the restricted clause sets of a module directory.
pub trait PackProbe {
    /// The restricted sets of the module whose `manifest.yaml` is directly in
    /// `module_dir`; empty means the module is not a pack.
    ///
    /// # Errors
    /// The load error, when the module's clause sets do not load. A module that
    /// cannot be read is never reported as one without restricted sets.
    fn restricted_sets(&self, module_dir: &Path) -> Result<Vec<RestrictedSet>, String>;
}

/// A probe that reports no pack, for callers that install no licensed content.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPacks;

impl PackProbe for NoPacks {
    fn restricted_sets(&self, _module_dir: &Path) -> Result<Vec<RestrictedSet>, String> {
        Ok(Vec::new())
    }
}

/// The nearest directory at or above `path` that holds a `.git` entry.
///
/// A directory, a file (linked worktree, submodule) and a stray empty entry all
/// count, and gitignore state and the `GIT_DIR` / `GIT_WORK_TREE` variables are
/// not consulted: the walk reads the filesystem and nothing else. `path` is
/// expected to be canonical.
#[must_use]
pub fn enclosing_work_tree(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|dir| fs::symlink_metadata(dir.join(".git")).is_ok())
        .map(Path::to_path_buf)
}

/// [`enclosing_work_tree`] for a directory that may not exist yet: the walk
/// starts at its nearest existing ancestor, after symlinks are resolved.
#[must_use]
pub fn enclosing_work_tree_of_future_dir(path: &Path) -> Option<PathBuf> {
    let existing = path.ancestors().find_map(|dir| dir.canonicalize().ok())?;
    enclosing_work_tree(&existing)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use ix_trace_rs::trace;

    use super::*;

    /// A scratch directory under a base with no `.git` at or above it: the
    /// default temp directory of a development machine may itself sit inside
    /// a work tree, which would make every "outside any work tree" case false.
    fn scratch(name: &str) -> PathBuf {
        let base = [std::env::temp_dir(), "/var/tmp".into(), "/dev/shm".into()]
            .into_iter()
            .filter_map(|dir| dir.canonicalize().ok())
            .find(|dir| enclosing_work_tree(dir).is_none())
            .expect("a temp base outside every git work tree");
        let dir = base.join(format!("quoin-pack-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[trace("FR-137-AC-4")]
    #[test]
    fn tc_1492_010_a_git_file_or_stray_empty_git_dir_counts_as_a_work_tree() {
        let base = scratch("walk");
        let linked = base.join("linked");
        fs::create_dir_all(linked.join("a/b")).unwrap();
        fs::write(linked.join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert_eq!(enclosing_work_tree(&linked.join("a/b")), Some(linked));

        let stray = base.join("stray");
        fs::create_dir_all(stray.join(".git")).unwrap();
        fs::create_dir_all(stray.join("m")).unwrap();
        assert_eq!(enclosing_work_tree(&stray.join("m")), Some(stray));

        let clean = base.join("clean");
        fs::create_dir_all(&clean).unwrap();
        assert_eq!(enclosing_work_tree(&clean), None);
        fs::remove_dir_all(&base).unwrap();
    }

    #[trace("FR-137-AC-7")]
    #[test]
    fn tc_1492_011_a_directory_not_yet_created_is_judged_by_its_existing_ancestor() {
        let base = scratch("future");
        fs::create_dir_all(base.join(".git")).unwrap();
        assert_eq!(
            enclosing_work_tree_of_future_dir(&base.join("filament/modules")),
            Some(base.clone())
        );
        fs::remove_dir_all(&base).unwrap();
    }
}
