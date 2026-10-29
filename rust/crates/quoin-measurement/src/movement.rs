// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Asking git whether named paths differ between two source revisions
//! (FR-110).
//!
//! The comparison and the checker are pure ([`crate::compare::PathMovement`]
//! is the seam); this is the one implementation that runs git. It records and
//! compares no digest: git, which already addresses content, answers whether
//! a path changed.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::compare::PathMovement;

/// [`PathMovement`] over a repository's own git history.
#[derive(Clone, Debug)]
pub struct GitPathMovement {
    repo: PathBuf,
}

impl GitPathMovement {
    /// Ask git inside `repo`.
    #[must_use]
    pub fn new(repo: &Path) -> Self {
        Self {
            repo: repo.to_path_buf(),
        }
    }
}

impl PathMovement for GitPathMovement {
    /// `git diff --name-only` between the revisions, restricted to `paths`.
    ///
    /// A revision is attacker-controlled text, so it is passed after
    /// `--end-of-options` and can never be read as an option; every path is
    /// passed literally after `--`. `None` when git cannot resolve a revision
    /// (a shallow clone, a squash-merged branch's commit, another
    /// repository's) or is not available — never a guess.
    fn changed(
        &self,
        before_revision: &str,
        after_revision: &str,
        paths: &[String],
    ) -> Option<Vec<String>> {
        if before_revision == after_revision || paths.is_empty() {
            return Some(Vec::new());
        }
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args([
                "--literal-pathspecs",
                "diff",
                "--name-only",
                "--no-renames",
                "-z",
                "--end-of-options",
                before_revision,
                after_revision,
                "--",
            ])
            .args(paths)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        Some(
            output
                .stdout
                .split(|byte| *byte == 0)
                .filter(|name| !name.is_empty())
                .map(|name| String::from_utf8_lossy(name).into_owned())
                .collect(),
        )
    }
}
