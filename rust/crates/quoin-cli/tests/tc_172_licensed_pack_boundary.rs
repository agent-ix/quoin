// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `spec-security-analysis` skill serves licence-restricted sources only from
//! a locally installed pack, by clause id (FR-172).
//!
//! A live skill run over a synthetic pack is a demonstration scenario
//! (`TC-EV-061`, `TC-EV-062` in `spec/evals.md`) and is not claimed here. These
//! tests check what is deterministic: the synthetic pack installs as a pack, and
//! nothing the skill ships or the fixtures hold carries the pack's clause text or
//! a reference table of a restricted source.

mod pack_support;

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use pack_support::{SENTINEL, quoin, scratch, semantic_root, text};

fn repo_root() -> PathBuf {
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
        .expect("repository root")
}

fn files_under(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_under(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// The synthetic pack `example.invalid/std@1`: its set title and clause text both
/// carry the sentinel.
fn write_std_pack(parent: &Path) -> PathBuf {
    let dir = parent.join("std-pack");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("manifest.yaml"),
        "name: std-pack\nversion: 0.1.0\nclause_sets:\n  - std.yaml\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("std.yaml"),
        format!(
            "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: std\n\
             title: \"{SENTINEL} title\"\nversion: \"1\"\nrights:\n  structure: original\n  \
             text: licensed\nclauses:\n  - id: EX-1\n    force: mandatory\n    text: \"{SENTINEL}\"\n"
        ),
    )
    .unwrap();
    dir
}

#[trace("FR-172-AC-7")]
#[test]
fn tc_172_001_installed_synthetic_pack_text_is_in_no_skill_or_fixture_file() {
    let root = scratch();
    let base = root.path().canonicalize().unwrap();
    let home = base.join("ix-home");
    semantic_root(&home);
    let source = write_std_pack(&base);
    let cwd = base.join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();

    let installed = quoin(
        &home,
        &cwd,
        &["module", "install", &format!("path:{}", source.display())],
        &[],
    );
    assert!(installed.status.success(), "{}", text(&installed));
    let listed = text(&quoin(&home, &cwd, &["module", "list"], &[]));
    assert!(listed.contains("\"pack\": true"), "{listed}");
    assert!(!listed.contains(SENTINEL), "{listed}");

    let mut files = Vec::new();
    for dir in [
        "skills/spec-security-analysis",
        "tests/fixtures/security-analysis",
    ] {
        files_under(&repo_root().join(dir), &mut files);
    }
    assert!(!files.is_empty());
    for file in files {
        let body = String::from_utf8_lossy(&std::fs::read(&file).unwrap()).into_owned();
        assert!(!body.contains(SENTINEL), "{}", file.display());
    }
}

#[trace("FR-172-AC-8")]
#[test]
fn tc_172_002_skill_ships_no_restricted_reference_table_and_reads_ids_from_packs() {
    let skill = repo_root().join("skills/spec-security-analysis");
    for dir in ["soc2", "iso27001"] {
        assert!(
            !skill.join("references").join(dir).exists(),
            "references/{dir} must not ship"
        );
    }
    let mut files = Vec::new();
    files_under(&skill, &mut files);
    for file in files {
        let body = std::fs::read_to_string(&file).unwrap();
        assert!(
            !body.contains("soc2/") && !body.contains("iso27001/"),
            "{} links a restricted reference",
            file.display()
        );
    }
    let text = std::fs::read_to_string(skill.join("SKILL.md")).unwrap();
    assert!(text.contains("Licensed standards come from an installed pack only"));
    assert!(text.contains("`pack=true`"));
}
