// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1046: the baked version must be refreshed by commits made in a linked
//! worktree, where `.git` is a text file that never changes.

#![allow(clippy::expect_used, reason = "test fixtures may panic")]

#[path = "../build_git.rs"]
mod build_git;

use ix_trace_rs::trace;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

#[trace("FR-002")]
#[test]
fn tc_1046_linked_worktree_watches_its_own_head_and_the_shared_branch_ref() {
    let root = tempfile::tempdir().expect("tempdir");
    let main = root.path().join("main");
    std::fs::create_dir(&main).expect("mkdir");
    git(&main, &["init", "-q", "-b", "trunk"]);
    git(&main, &["commit", "-q", "--allow-empty", "-m", "one"]);
    let linked = root.path().join("linked");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            linked.to_str().expect("utf8"),
        ],
    );
    assert!(
        linked.join(".git").is_file(),
        "fixture must be a linked worktree"
    );

    let watched = build_git::watched_paths(&linked);
    let canonical = |path: &Path| path.canonicalize().expect("watched path exists or parent");
    let head = watched
        .iter()
        .find(|path| path.ends_with("worktrees/linked/HEAD"))
        .expect("worktree-private HEAD is watched");
    assert!(head.is_file());
    let branch = watched
        .iter()
        .find(|path| path.ends_with("refs/heads/side"))
        .expect("shared branch ref is watched");
    assert_eq!(
        canonical(branch.parent().expect("parent")),
        canonical(&main.join(".git/refs/heads"))
    );
    assert!(
        !watched
            .iter()
            .any(|path| path.ends_with(".git") && path.is_file())
    );
}

#[trace("FR-002")]
#[test]
fn tc_1046_main_checkout_still_watches_head_and_branch() {
    let root = tempfile::tempdir().expect("tempdir");
    git(root.path(), &["init", "-q", "-b", "trunk"]);
    git(root.path(), &["commit", "-q", "--allow-empty", "-m", "one"]);
    let watched = build_git::watched_paths(root.path());
    assert!(watched.iter().any(|path| path.ends_with(".git/HEAD")));
    assert!(
        watched
            .iter()
            .any(|path| path.ends_with("refs/heads/trunk"))
    );
}
