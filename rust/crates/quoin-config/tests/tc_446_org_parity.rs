// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The filesystem-free org resolution decides what the filesystem one decides.
//!
//! `quoin-core`'s library half may not touch a disk
//! (`quoin-core/tests/tc_library_containment.rs`), so `config.resolve_org`
//! calls [`resolve_org_from_documents`] over bytes the TypeScript caller read.
//! That leaves two implementations of one rule, and a port whose two halves
//! disagree is worse than no port: the CLI would answer one way and the
//! boundary another, with nothing failing.
//!
//! Every case below builds a **real** tree, runs [`resolve_org`] over it, then
//! reads the same three documents off that same tree and runs
//! [`resolve_org_from_documents`] over them, and asserts the two answers are
//! identical — org and provenance both. The fixtures are the tree, not a
//! hand-written pair of inputs, so neither half can be fed a document the other
//! never saw.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};

use quoin_config::org::resolve_git_dir;
use quoin_config::paths::{Environment, FixedEnvironment, RuntimeContext};
use quoin_config::{
    ConfigService, IncidentLog, OrgDocumentReport, OrgOptions, OrgSource, QuoinConfig, ResolvedOrg,
    resolve_org, resolve_org_from_documents,
};

/// A repository with a user config root, a project config root and a git dir.
struct Tree {
    _root: tempfile::TempDir,
    repo: PathBuf,
    env: FixedEnvironment,
    ctx: RuntimeContext,
}

fn tree() -> Tree {
    let root = tempfile::tempdir().expect("temp dir");
    let repo = root.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    let xdg = root.path().join("xdg");
    Tree {
        env: FixedEnvironment::new().with_var("XDG_CONFIG_HOME", xdg.to_string_lossy()),
        ctx: RuntimeContext::new().with_project_config_root(repo.join(".ix")),
        repo,
        _root: root,
    }
}

impl Tree {
    fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env = self.env.with_var(key, value);
        self
    }

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn user_config(self, body: &str) -> Self {
        let service = ConfigService::<QuoinConfig>::for_plugin(&self.env, &self.ctx);
        let path = service.file_path().to_path_buf();
        drop(service);
        Self::write(&path, body);
        self
    }

    fn project_config(self, body: &str) -> Self {
        let service = ConfigService::<QuoinConfig>::for_plugin(&self.env, &self.ctx);
        let path = service
            .project_file_path()
            .expect("the fixture declares a project config root")
            .to_path_buf();
        drop(service);
        Self::write(&path, body);
        self
    }

    fn origin(self, url: &str) -> Self {
        let path = self.repo.join(".git").join("config");
        Self::write(&path, &format!("[remote \"origin\"]\n\turl = {url}\n"));
        self
    }

    /// What the CLI answers: the reads are inside.
    fn through_the_filesystem(&self, flag: Option<&str>) -> ResolvedOrg {
        let mut log = IncidentLog::new();
        resolve_org(
            &self.repo,
            &OrgOptions { flag },
            &self.env,
            &self.ctx,
            &mut log,
        )
    }

    /// What the boundary answers: the reads are out here, exactly as
    /// `src/core/org.ts` does them before calling `quoin-core`.
    fn through_documents(&self, flag: Option<&str>) -> (ResolvedOrg, OrgDocumentReport) {
        let service = ConfigService::<QuoinConfig>::for_plugin(&self.env, &self.ctx);
        let user = fs::read_to_string(service.file_path()).ok();
        let project = service
            .project_file_path()
            .and_then(|path| fs::read_to_string(path).ok());
        let git =
            resolve_git_dir(&self.repo).and_then(|dir| fs::read_to_string(dir.join("config")).ok());
        resolve_org_from_documents(
            &OrgOptions { flag },
            &self.env,
            user.as_deref(),
            project.as_deref(),
            git.as_deref(),
        )
    }

    /// Both halves, over the same tree, and they must agree.
    fn agreed(&self, flag: Option<&str>) -> ResolvedOrg {
        let filesystem = self.through_the_filesystem(flag);
        let (documents, _) = self.through_documents(flag);
        assert_eq!(
            filesystem, documents,
            "the filesystem path and the document path disagree; one of them is \
             now a second, private rule"
        );
        filesystem
    }
}

fn org_of(resolved: &ResolvedOrg) -> Option<&str> {
    resolved.org.as_ref().map(quoin_config::OrgName::as_str)
}

/// Trace: FR-025-AC-1, FR-025-AC-2, FR-027-AC-1, FR-027-AC-2, FR-027-AC-3,
/// FR-027-AC-4, FR-027-AC-9, FR-096
/// Provenance: quoin#446
#[test]
fn tc_446_026_the_document_path_answers_what_the_filesystem_path_answers() {
    // The flag beats everything present underneath it.
    let flagged = tree()
        .with_env("QUOIN_ORG", "from-env")
        .user_config("org: from-user\n")
        .origin("git@github.com:from-git/repo.git");
    let resolved = flagged.agreed(Some("from-flag"));
    assert_eq!(org_of(&resolved), Some("from-flag"));
    assert_eq!(resolved.source, OrgSource::Flag);

    // The environment beats the file, and SAYS it was the environment.
    let enved = tree()
        .with_env("QUOIN_ORG", "from-env")
        .user_config("org: from-user\n")
        .origin("git@github.com:from-git/repo.git");
    let resolved = enved.agreed(None);
    assert_eq!(org_of(&resolved), Some("from-env"));
    assert_eq!(resolved.source, OrgSource::Env);

    // The file beats the remote.
    let filed = tree()
        .user_config("org: from-user\n")
        .origin("git@github.com:from-git/repo.git");
    let resolved = filed.agreed(None);
    assert_eq!(org_of(&resolved), Some("from-user"));
    assert_eq!(resolved.source, OrgSource::Config);

    // The project layer beats the user layer, and the merge is the same merge.
    let layered = tree()
        .user_config("org: from-user\n")
        .project_config("org: from-project\n")
        .origin("git@github.com:from-git/repo.git");
    let resolved = layered.agreed(None);
    assert_eq!(org_of(&resolved), Some("from-project"));
    assert_eq!(resolved.source, OrgSource::Config);

    // The remote is the last resort, over every url shape the parser accepts.
    for url in [
        "git@github.com:from-git/repo.git",
        "https://github.com/from-git/repo.git",
        "ssh://git@github.com/from-git/repo",
    ] {
        let remote = tree().origin(url);
        let resolved = remote.agreed(None);
        assert_eq!(org_of(&resolved), Some("from-git"), "url: {url}");
        assert_eq!(resolved.source, OrgSource::Git, "url: {url}");
    }

    // Nothing anywhere is unresolved on both paths, not an error on either.
    let bare = tree();
    let resolved = bare.agreed(None);
    assert_eq!(org_of(&resolved), None);
    assert_eq!(resolved.source, OrgSource::None);

    // A blank flag falls through rather than winning empty.
    let blank = tree().user_config("org: from-user\n");
    let resolved = blank.agreed(Some("   "));
    assert_eq!(org_of(&resolved), Some("from-user"));
    assert_eq!(resolved.source, OrgSource::Config);
}

/// Trace: FR-027-AC-5, FR-096
/// Provenance: quoin#446
#[test]
fn tc_446_027_a_broken_layer_degrades_identically_and_stops_neither_path() {
    let broken = tree()
        .user_config("org: [not, a, string]\n")
        .origin("git@github.com:from-git/repo.git");

    // The filesystem path records an incident and carries on to the remote.
    let mut log = IncidentLog::new();
    let filesystem = resolve_org(
        &broken.repo,
        &OrgOptions::default(),
        &broken.env,
        &broken.ctx,
        &mut log,
    );
    assert_eq!(org_of(&filesystem), Some("from-git"));
    assert_eq!(filesystem.source, OrgSource::Git);

    // The document path reaches the same answer AND hands the issue back, so
    // the boundary can report the degraded read instead of swallowing it. A
    // degraded read that reported nothing would be the failure FR-027-AC-5
    // exists to prevent, seen from the other side.
    let (documents, report) = broken.through_documents(None);
    assert_eq!(documents, filesystem);
    assert!(
        report.degraded,
        "the broken layer was not reported as a degraded read; the boundary \
         derives the payload's `degraded` field from exactly this flag"
    );
    assert!(
        !report.issues.is_empty(),
        "the broken layer produced no issue for the boundary to report"
    );
}

/// Trace: FR-025-AC-2, FR-096
/// Provenance: quoin#446
#[test]
fn tc_446_028_a_worktree_git_file_resolves_to_the_common_config() {
    // `src/org.ts` followed `.git` as a FILE ("gitdir: …") before reading the
    // remote, which is what a `git worktree` checkout has. The document path
    // does that read itself, so the locator it uses is pinned here: pointed at
    // a worktree, it must land on the config that actually holds `origin`.
    let fixture = tree();
    let common = fixture.repo.parent().unwrap().join("common.git");
    fs::create_dir_all(&common).unwrap();
    fs::write(
        common.join("config"),
        "[remote \"origin\"]\n\turl = git@github.com:from-worktree/repo.git\n",
    )
    .unwrap();

    let linked = fixture.repo.parent().unwrap().join("linked");
    fs::create_dir_all(&linked).unwrap();
    let linked_git = common.join("worktrees").join("linked");
    fs::create_dir_all(&linked_git).unwrap();
    fs::write(linked_git.join("commondir"), "../..\n").unwrap();
    fs::write(
        linked.join(".git"),
        format!("gitdir: {}\n", linked_git.display()),
    )
    .unwrap();

    let dir = resolve_git_dir(&linked).expect("the worktree pointer resolves");
    let git = fs::read_to_string(dir.join("config")).expect("the common config is readable");
    let (resolved, report) = resolve_org_from_documents(
        &OrgOptions::default(),
        &FixedEnvironment::new() as &dyn Environment,
        None,
        None,
        Some(&git),
    );
    assert_eq!(org_of(&resolved), Some("from-worktree"));
    assert_eq!(resolved.source, OrgSource::Git);
    assert!(!report.degraded, "no config layer was supplied: {report:?}");
}
