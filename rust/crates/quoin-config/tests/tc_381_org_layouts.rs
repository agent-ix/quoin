// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Organization resolution over hand-built repository layouts (quoin#381):
//! worktree `.git` pointers, broken pointers, missing metadata, and the
//! flag/environment precedence. Every expectation is written here; nothing is
//! read from a recorded capture.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::Path;

use quoin_config::paths::FixedEnvironment;
use quoin_config::{IncidentLog, OrgOptions, OrgSource, RuntimeContext, resolve_org};

fn git_config(url: &str) -> String {
    format!(
        "[core]\n\trepositoryformatversion = 0\n[remote \"origin\"]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"
    )
}

/// Trace: FR-023-AC-4, FR-025-AC-1
#[test]
fn tc_381_104_flag_and_env_precedence() {
    let root = tempfile::tempdir().expect("temp dir");
    let repo_root = root.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).expect("git dir");
    fs::write(
        repo_root.join(".git").join("config"),
        git_config("git@github.com:from-git/repo.git"),
    )
    .expect("git config");

    // (flag, QUOIN_ORG, expected source, expected org)
    let cases: [(Option<&str>, Option<&str>, &str, &str); 4] = [
        (Some("from-flag"), Some("from-env"), "flag", "from-flag"),
        (None, Some("from-env"), "env", "from-env"),
        (None, None, "git", "from-git"),
        (Some("  "), Some("from-env"), "env", "from-env"),
    ];
    for (flag, env_org, source, org) in cases {
        let mut env = FixedEnvironment::new().with_var(
            "XDG_CONFIG_HOME",
            root.path().join("empty-xdg").to_string_lossy(),
        );
        if let Some(value) = env_org {
            env = env.with_var("QUOIN_ORG", value);
        }
        let mut log = IncidentLog::new();
        let resolved = resolve_org(
            &repo_root,
            &OrgOptions { flag },
            &env,
            &RuntimeContext::new(),
            &mut log,
        );
        assert_eq!(
            resolved.source.as_str(),
            source,
            "flag {flag:?} env {env_org:?}"
        );
        assert_eq!(
            resolved.org.as_ref().map(quoin_config::OrgName::as_str),
            Some(org),
            "flag {flag:?} env {env_org:?}"
        );
    }
}

/// Trace: FR-025-AC-8
#[test]
fn tc_381_105_worktree_gitdir_resolves_the_main_checkouts_org() {
    // Mirror git's layout: the worktree's `.git` is a file naming a gitdir
    // under the main checkout, whose `commondir` points back at the shared
    // `.git` that actually holds the config.
    let root = tempfile::tempdir().expect("temp dir");
    let main = root.path().join("main");
    let common_git = main.join(".git");
    let wt_git_dir = common_git.join("worktrees").join("feature");
    fs::create_dir_all(&wt_git_dir).expect("worktree gitdir");
    fs::write(
        common_git.join("config"),
        git_config("git@github.com:acme/widgets.git"),
    )
    .expect("git config");
    fs::write(wt_git_dir.join("commondir"), "../..\n").expect("commondir");

    let worktree = root.path().join("wt");
    fs::create_dir_all(&worktree).expect("worktree");
    fs::write(
        worktree.join(".git"),
        format!("gitdir: {}\n", wt_git_dir.display()),
    )
    .expect("gitdir pointer");

    let mut log = IncidentLog::new();
    let env = FixedEnvironment::new()
        .with_var("XDG_CONFIG_HOME", root.path().join("xdg").to_string_lossy());
    let resolved = resolve_org(
        &worktree,
        &OrgOptions::default(),
        &env,
        &RuntimeContext::new(),
        &mut log,
    );
    assert_eq!(resolved.source, OrgSource::Git);
    assert_eq!(
        resolved.org.as_ref().map(quoin_config::OrgName::as_str),
        Some("acme")
    );
}

/// Trace: FR-025-AC-8
#[test]
fn tc_381_106_broken_gitdir_pointers_resolve_to_none() {
    let root = tempfile::tempdir().expect("temp dir");
    let env = FixedEnvironment::new()
        .with_var("XDG_CONFIG_HOME", root.path().join("xdg").to_string_lossy());
    for (label, body) in [
        ("points nowhere useful", "gitdir: /nonexistent/path\n"),
        ("no gitdir pointer", "not a gitdir pointer\n"),
        ("empty pointer", "gitdir:   \n"),
    ] {
        let repo = root.path().join(label.replace(' ', "-"));
        fs::create_dir_all(&repo).expect("repo dir");
        fs::write(repo.join(".git"), body).expect("dot git file");
        let mut log = IncidentLog::new();
        let resolved = resolve_org(
            &repo,
            &OrgOptions::default(),
            &env,
            &RuntimeContext::new(),
            &mut log,
        );
        assert_eq!(resolved.source, OrgSource::None, "case {label}");
        assert_eq!(resolved.org, None, "case {label}");
    }
}

/// Trace: FR-025-AC-4
#[test]
fn tc_381_107_missing_repository_metadata_resolves_to_none() {
    let root = tempfile::tempdir().expect("temp dir");
    let repo: &Path = root.path();
    let env = FixedEnvironment::new()
        .with_var("XDG_CONFIG_HOME", root.path().join("xdg").to_string_lossy());
    let mut log = IncidentLog::new();
    let resolved = resolve_org(
        repo,
        &OrgOptions::default(),
        &env,
        &RuntimeContext::new(),
        &mut log,
    );
    assert_eq!(resolved, quoin_config::ResolvedOrg::unresolved());
    assert!(
        quoin_config::UNRESOLVED_ORG_MESSAGE.contains("--org"),
        "the remedy must reach the author"
    );
}
