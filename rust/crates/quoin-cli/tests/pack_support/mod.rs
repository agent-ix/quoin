// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shared fixtures for the licensed-pack command tests (FR-137, FR-138, FR-139).
//!
//! Every authority is `example.invalid` and every text is a synthetic sentinel.

#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "each test binary uses a different subset; in a test, a panic IS the failure report"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The clause text every fixture carries; no output may ever contain it.
pub(crate) const SENTINEL: &str = "SENTINEL-sprocket-text-7731";

/// A directory outside every git work tree. The default temp directory of a
/// development machine may itself sit inside one (a stray `.git` in `/tmp`),
/// which would make every "outside any work tree" case false.
pub(crate) fn clean_base() -> PathBuf {
    let outside = |dir: &Path| dir.ancestors().all(|d| !d.join(".git").exists());
    [std::env::temp_dir(), "/var/tmp".into(), "/dev/shm".into()]
        .into_iter()
        .filter_map(|dir| dir.canonicalize().ok())
        .find(|dir| outside(dir))
        .expect("a temp base outside every git work tree")
}

/// A scratch directory under [`clean_base`], removed on drop.
pub(crate) fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(clean_base()).expect("scratch root")
}

/// The clause-set file of a synthetic set whose text rights are `text_rights`.
/// A set with `text: none` carries no text, as the engine requires.
pub(crate) fn clause_set_yaml(id: &str, text_rights: &str) -> String {
    let text = if text_rights == "none" {
        String::new()
    } else {
        format!("    text: \"{SENTINEL}\"\n")
    };
    format!(
        "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: {id}\n\
         title: Synthetic set {id}\nversion: \"2\"\nrights:\n  structure: original\n  \
         text: {text_rights}\nclauses:\n  - id: C-1\n    force: mandatory\n{text}"
    )
}

/// Write a module `name` under `parent` with one clause set `rules` whose text
/// rights are `text_rights`, and return its directory.
pub(crate) fn write_module(parent: &Path, name: &str, text_rights: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).expect("module dir");
    std::fs::write(
        dir.join("manifest.yaml"),
        format!("name: {name}\nversion: 0.1.0\nclause_sets:\n  - rules.yaml\n"),
    )
    .expect("manifest");
    std::fs::write(
        dir.join("rules.yaml"),
        clause_set_yaml("rules", text_rights),
    )
    .expect("clause set");
    dir
}

/// A real, on-disk semantic contract root, so a run writes nothing into the
/// home beyond the registry and the modules directory.
pub(crate) fn semantic_root() -> &'static Path {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let root =
            std::env::temp_dir().join(format!("quoin-cli-packs-semantic-{}", std::process::id()));
        quoin_semantic::materialize_embedded_contract(&root).expect("embedded contract");
        root
    })
}

/// Run `quoin` against `home` with `cwd` as the working directory and `extra`
/// environment variables set.
pub(crate) fn quoin(
    home: &Path,
    cwd: &Path,
    arguments: &[&str],
    extra: &[(&str, &Path)],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command
        .args(arguments)
        .current_dir(cwd)
        .env("IX_HOME", home)
        .env("QUOIN_SEMANTIC_ROOT", semantic_root());
    for (key, value) in extra {
        command.env(key, value);
    }
    command.output().expect("the native quoin binary runs")
}

/// Everything a run printed.
pub(crate) fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `git init` a repository at `dir`.
pub(crate) fn git_init(dir: &Path) {
    std::fs::create_dir_all(dir).expect("repo dir");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git init");
}

/// `git status --porcelain` in `dir`, which must be empty for an untouched tree.
pub(crate) fn git_porcelain(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git status in {}", dir.display());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Every file and directory under `root` with its bytes, for before/after
/// comparison. A missing root snapshots as empty.
pub(crate) fn snapshot(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn walk(dir: &Path, out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                out.insert(path.clone(), Vec::new());
                walk(&path, out);
            } else {
                out.insert(path.clone(), std::fs::read(&path).expect("file"));
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, &mut out);
    out
}
