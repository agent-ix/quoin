// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Locate the Git files whose change moves `git describe`, shared by `build.rs`
//! and its regression test.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Ask Git for the on-disk location of `name` as seen from `dir`.
///
/// `--git-path` resolves per-worktree files (`HEAD`) into the worktree's own
/// git directory and shared files (`refs`, `packed-refs`) into the common one,
/// which `.git` itself cannot do because in a linked worktree it is a text
/// file whose mtime never changes.
fn git_path(dir: &Path, name: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--git-path", name])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let text = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(text.trim());
    Some(if path.is_absolute() {
        path
    } else {
        dir.join(path)
    })
}

/// The files a commit, checkout or tag moves, resolved for any checkout layout.
pub(crate) fn watched_paths(dir: &Path) -> Vec<PathBuf> {
    let mut names = vec![
        "HEAD".to_owned(),
        "packed-refs".to_owned(),
        "refs/tags".to_owned(),
    ];
    let branch = Command::new("git")
        .current_dir(dir)
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok());
    if let Some(branch) = branch {
        names.push(branch.trim().to_owned());
    }
    names
        .iter()
        .filter_map(|name| git_path(dir, name))
        .collect()
}
