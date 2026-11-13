// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "shared by several test crates, each using a part; a panic IS the failure report"
)]

//! Helpers shared by the anomaly tests: the `spec-artifacts-process` fixtures,
//! read in place, and throwaway git repositories whose commit sequences the
//! tests author.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// The resolved checkout, canonicalised: `quire` rejects `..` in `--module`.
pub fn process_root() -> PathBuf {
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    std::fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "the spec-artifacts-process checkout is not at {}: {error}. These tests read \
             the documents that repository ships in place and keep no copy: check out \
             agent-ix/spec-artifacts-process next to this repository, or set \
             SPEC_ARTIFACTS_PROCESS_DIR to its checkout",
            root.display()
        )
    })
}

pub fn fixtures() -> PathBuf {
    let dir = process_root().join("tests/fixtures/anomaly");
    assert!(
        dir.is_dir(),
        "the spec-artifacts-process anomaly fixtures are not at {}. These tests read \
         the documents that repository ships in place and keep no copy: check out \
         agent-ix/spec-artifacts-process (at or after PLAT-1415, #140) next to this \
         repository, or set SPEC_ARTIFACTS_PROCESS_DIR to its checkout",
        dir.display()
    );
    dir
}

pub fn fixture(group: &str, name: &str) -> String {
    std::fs::read_to_string(fixtures().join(group).join(format!("{name}.md"))).unwrap()
}

pub fn with_status(text: &str, status: &str) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with("status: ") {
                format!("status: {status}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

pub struct Repo {
    pub dir: TempDir,
    tick: u32,
}

impl Repo {
    pub fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            tick: 0,
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.name", "Fixture Author"]);
        repo.git(&["config", "user.email", "fixture@example.invalid"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    pub fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    pub fn commit(&mut self, name: &str, text: &str) -> String {
        self.write(name, text);
        self.git(&["add", "-A"]);
        self.tick += 1;
        let date = format!(
            "2026-01-01T{:02}:{:02}:00+00:00",
            self.tick / 60,
            self.tick % 60
        );
        let status = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["commit", "-q", "-m", "step"])
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .status()
            .unwrap();
        assert!(status.success());
        let head = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(head.stdout).unwrap().trim().to_owned()
    }
}

pub fn scope_with(anomalies: &[(&str, &str, &str)]) -> TempDir {
    let scope = tempfile::tempdir().unwrap();
    let corpus = fixtures().join("corpus/spec");
    for entry in walk(&corpus) {
        let target = scope
            .path()
            .join("spec")
            .join(entry.strip_prefix(&corpus).unwrap());
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(&entry, target).unwrap();
    }
    for (id, group, name) in anomalies {
        let text = fixture(group, name).replace("id: AN-001", &format!("id: {id}"));
        let anomalies = scope.path().join("spec/anomalies");
        std::fs::create_dir_all(&anomalies).unwrap();
        std::fs::write(anomalies.join(format!("{id}.md")), text).unwrap();
    }
    scope
}

pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}
