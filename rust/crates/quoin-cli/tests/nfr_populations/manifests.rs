// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-008: a corrupt manifest aborts catalog assembly naming its file and a
//! path without a manifest is skipped, probed with `quoin catalog list --json`.
//!
//! The home is seeded from the retained catalog fixture, whose registry has
//! every default module settled, so no probe reaches the network. A control
//! run lists the valid companion alone first; a probe that fails because the
//! environment is broken therefore cannot read as a refusal.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{Probed, quoin, text};

/// The module every probe lists after the candidate.
const COMPANION: &str = "nfr008-companion";

/// Manifests that are not one valid YAML mapping, by kind.
const CORRUPT: [(&str, &[u8]); 12] = [
    ("unterminated-flow-sequence", b"name: [unterminated\n"),
    ("bad-indentation", b"name: x\n  bad indent: y\n"),
    ("invalid-utf8", &[0xff, 0xfe, 0x00, 0xc3, 0x28]),
    ("unknown-alias", b"key: *undefined\n"),
    ("two-documents", b"---\na: 1\n---\nb: 2\n"),
    ("unterminated-quoted-scalar", b"\"unterminated string\n"),
    ("unterminated-flow-mapping", b"{a: 1\n"),
    ("empty-file", b""),
    ("null-document", b"~\n"),
    ("bare-string", b"just a string\n"),
    ("top-level-sequence", b"- a\n- b\n"),
    ("tab-indented-sequence", b"\t- bad: tab\n"),
];

/// Candidate paths that hold no manifest, by kind.
const MISSING: [&str; 4] = [
    "nonexistent-path",
    "empty-directory",
    "directory-without-manifest",
    "regular-file",
];

/// A seeded, offline home and the probes' valid companion module.
pub(crate) struct Harness {
    scratch: tempfile::TempDir,
    home: PathBuf,
    companion: PathBuf,
}

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).expect("the destination is creatable");
    for entry in std::fs::read_dir(source).expect("the fixture is readable") {
        let entry = entry.expect("the entry is readable");
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("the type is readable").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("the file copies");
        }
    }
}

impl Harness {
    /// Seed the home, write the companion, and require the control to list it.
    pub(crate) fn new() -> Self {
        let scratch = tempfile::tempdir().expect("scratch is creatable");
        let home = scratch.path().join("ix-home");
        copy_tree(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/retained-catalog/ix-home"),
            &home,
        );
        let companion = scratch.path().join(COMPANION);
        std::fs::create_dir_all(&companion).expect("the companion is creatable");
        std::fs::write(
            companion.join("manifest.yaml"),
            format!("name: {COMPANION}\nartifact_types:\n  - name: NfrProbe\n"),
        )
        .expect("the companion manifest is writable");
        let harness = Self {
            scratch,
            home,
            companion,
        };
        let control = harness.list(&[&harness.companion]);
        assert!(
            control.status.success() && text(&control.stdout).contains(COMPANION),
            "the control run did not list the companion alone (offline home broken?): {}",
            text(&control.stderr)
        );
        harness
    }

    fn list(&self, candidates: &[&Path]) -> std::process::Output {
        let paths = std::env::join_paths(candidates).expect("paths join");
        quoin(&self.home, &["catalog", "list", "--json"], |process| {
            process.env("QUOIN_MODULE_PATHS", paths);
        })
    }

    fn candidate(&self, kind: &str) -> PathBuf {
        self.scratch.path().join("candidates").join(kind)
    }

    /// `candidate` listed ahead of the companion.
    fn list_before_companion(&self, candidate: &Path) -> std::process::Output {
        self.list(&[candidate, &self.companion])
    }
}

/// NFR-008-M-1: each corrupt kind, matched when assembly exits non-zero,
/// prints no catalog and names the candidate's `manifest.yaml`.
pub(crate) fn corrupt_manifests(harness: &Harness) -> Probed {
    let mut examined = BTreeSet::new();
    let mut matched = BTreeSet::new();
    for (kind, bytes) in CORRUPT {
        let candidate = harness.candidate(kind);
        std::fs::create_dir_all(&candidate).expect("the candidate is creatable");
        let manifest = candidate.join("manifest.yaml");
        std::fs::write(&manifest, bytes).expect("the manifest is writable");
        let output = harness.list_before_companion(&candidate);
        let names_the_file = text(&output.stderr).contains(&manifest.display().to_string());
        if !output.status.success() && output.stdout.is_empty() && names_the_file {
            matched.insert(kind.to_owned());
        }
        examined.insert(kind.to_owned());
    }
    Probed { examined, matched }
}

/// NFR-008-M-2: each manifest-less kind, matched when assembly succeeds and
/// still lists the module after it.
pub(crate) fn missing_manifests(harness: &Harness) -> Probed {
    let mut examined = BTreeSet::new();
    let mut matched = BTreeSet::new();
    for kind in MISSING {
        let candidate = harness.candidate(kind);
        let parent = candidate.parent().expect("a candidate has a parent");
        std::fs::create_dir_all(parent).expect("the parent is creatable");
        match kind {
            "empty-directory" => std::fs::create_dir_all(&candidate),
            "directory-without-manifest" => std::fs::create_dir_all(&candidate)
                .and_then(|()| std::fs::write(candidate.join("README.md"), "not a module\n")),
            "regular-file" => std::fs::write(&candidate, "not a module\n"),
            _ => Ok(()),
        }
        .expect("the candidate is built");
        let output = harness.list_before_companion(&candidate);
        if output.status.success() && text(&output.stdout).contains(COMPANION) {
            matched.insert(kind.to_owned());
        }
        examined.insert(kind.to_owned());
    }
    Probed { examined, matched }
}
