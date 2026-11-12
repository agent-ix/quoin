// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The sandbox harness behind FR-139: `module install`, `list`, `inspect` and
//! `remove` over a synthetic pack write only beneath `IX_HOME`.
//!
//! One [`Sandbox`] holds a temp `IX_HOME`, a pack source, and two initialised
//! git repositories (the first is the working directory of every run). An
//! [`Oracle`] records the sandbox before a run and lists every way it differs
//! afterwards. The control test proves the oracle can fail: a run that does
//! write into a repository is detected by the same checks.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod pack_support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use pack_support::{
    SENTINEL, git_init, git_porcelain, quoin, scratch, snapshot, text, write_module,
};

const MODULE: &str = "widget-pack";

struct Sandbox {
    _root: tempfile::TempDir,
    home: PathBuf,
    source: PathBuf,
    repo_cwd: PathBuf,
    repo_other: PathBuf,
}

fn sandbox() -> Sandbox {
    let root = scratch();
    let base = root.path().canonicalize().unwrap();
    let repo_cwd = base.join("repo-cwd");
    let repo_other = base.join("repo-other");
    git_init(&repo_cwd);
    git_init(&repo_other);
    let source = write_module(&base.join("pack-source"), MODULE, "licensed");
    Sandbox {
        home: base.join("ix-home"),
        source,
        repo_cwd,
        repo_other,
        _root: root,
    }
}

impl Sandbox {
    fn root(&self) -> PathBuf {
        self.home.parent().unwrap().to_path_buf()
    }

    /// Run one `quoin module` command with the first repository as cwd.
    fn run(&self, arguments: &[&str]) -> String {
        let output = quoin(&self.home, &self.repo_cwd, arguments, &[]);
        let printed = text(&output);
        assert!(output.status.success(), "quoin {arguments:?}: {printed}");
        printed
    }

    /// Everything in the sandbox outside `IX_HOME`.
    fn outside_home(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut all = snapshot(&self.root());
        all.retain(|path, _| !path.starts_with(&self.home));
        all
    }

    fn installed_dir(&self) -> PathBuf {
        self.home.join("filament/modules").join(MODULE)
    }
}

/// What the sandbox looked like before a run, and the checks after it.
struct Oracle {
    outside: BTreeMap<PathBuf, Vec<u8>>,
    source: BTreeMap<PathBuf, Vec<u8>>,
}

impl Oracle {
    fn record(sandbox: &Sandbox) -> Self {
        Self {
            outside: sandbox.outside_home(),
            source: snapshot(&sandbox.source),
        }
    }

    /// Every way the sandbox now differs from what was recorded; empty means
    /// nothing was written outside `IX_HOME`.
    fn violations(&self, sandbox: &Sandbox) -> Vec<String> {
        let mut found = Vec::new();
        // Snapshots first: `git status` may refresh a repository's index.
        if snapshot(&sandbox.source) != self.source {
            found.push("the pack source directory changed".to_owned());
        }
        if sandbox.outside_home() != self.outside {
            found.push("a file outside IX_HOME changed".to_owned());
        }
        for repo in [&sandbox.repo_cwd, &sandbox.repo_other] {
            let porcelain = git_porcelain(repo);
            if !porcelain.is_empty() {
                found.push(format!(
                    "git status in {} is not empty: {porcelain}",
                    repo.display()
                ));
            }
        }
        found.extend(sentinel_outside_the_module(sandbox));
        found
    }
}

/// Every file under the sandbox, outside the installed module's own directory,
/// that holds the clause-text sentinel. The pack source is the one place that
/// legitimately carries it, so it is excluded as the origin.
fn sentinel_outside_the_module(sandbox: &Sandbox) -> Vec<String> {
    snapshot(&sandbox.root())
        .into_iter()
        .filter(|(path, bytes)| {
            !path.starts_with(sandbox.installed_dir())
                && !path.starts_with(&sandbox.source)
                && String::from_utf8_lossy(bytes).contains(SENTINEL)
        })
        .map(|(path, _)| format!("clause text found in {}", path.display()))
        .collect()
}

#[trace(
    "FR-139-AC-1",
    "FR-139-AC-2",
    "FR-139-AC-3",
    "FR-139-AC-4",
    "FR-139-AC-5"
)]
#[test]
fn tc_1493_001_install_list_inspect_and_remove_write_only_beneath_ix_home() {
    let sandbox = sandbox();
    let oracle = Oracle::record(&sandbox);
    let source_arg = format!("path:{}", sandbox.source.display());

    let installed = sandbox.run(&["module", "install", &source_arg]);
    assert!(!installed.contains(SENTINEL), "{installed}");
    assert!(sandbox.installed_dir().is_dir());

    let listed = sandbox.run(&["module", "list"]);
    assert!(listed.contains("\"pack\": true"), "{listed}");
    sandbox.run(&["module", "inspect"]);
    assert_eq!(oracle.violations(&sandbox), Vec::<String>::new());

    sandbox.run(&["module", "remove", MODULE]);
    assert!(
        !sandbox.installed_dir().exists(),
        "remove deletes the module directory"
    );
    let registry = std::fs::read_to_string(sandbox.home.join("filament/registry.json")).unwrap();
    assert!(!registry.contains(MODULE), "{registry}");
    assert_eq!(oracle.violations(&sandbox), Vec::<String>::new());
}

#[trace("FR-139-AC-6")]
#[test]
fn tc_1493_002_a_run_that_writes_into_a_repository_is_detected() {
    let sandbox = sandbox();
    let oracle = Oracle::record(&sandbox);
    let source_arg = format!("path:{}", sandbox.source.display());
    sandbox.run(&["module", "install", &source_arg]);

    // The control: copy the pack into the working directory's repository and a
    // file naming it into the other, as a defective command would.
    std::fs::copy(
        sandbox.source.join("rules.yaml"),
        sandbox.repo_cwd.join("rules.yaml"),
    )
    .unwrap();
    std::fs::write(sandbox.repo_other.join("note.txt"), "leaked").unwrap();

    let violations = oracle.violations(&sandbox);
    let has = |needle: &str| violations.iter().any(|v| v.contains(needle));
    assert!(has("outside IX_HOME"), "{violations:?}");
    assert!(
        has(&sandbox.repo_cwd.display().to_string()),
        "{violations:?}"
    );
    assert!(
        has(&sandbox.repo_other.display().to_string()),
        "{violations:?}"
    );
    assert!(has("clause text found"), "{violations:?}");
}

#[trace("FR-139-AC-3")]
#[test]
fn tc_1493_003_the_source_tree_checkout_is_left_as_it_was() {
    // The checkout this test runs from is a work tree like any other: whatever
    // its state, the four commands must not change it.
    let tree = Path::new(env!("CARGO_MANIFEST_DIR"));
    let before = git_status(tree);
    let sandbox = sandbox();
    let source_arg = format!("path:{}", sandbox.source.display());
    sandbox.run(&["module", "install", &source_arg]);
    sandbox.run(&["module", "list"]);
    sandbox.run(&["module", "inspect"]);
    sandbox.run(&["module", "remove", MODULE]);
    assert_eq!(git_status(tree), before);
}

fn git_status(dir: &Path) -> String {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}
