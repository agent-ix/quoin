// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-008: a corrupt manifest aborts catalog assembly and a path without a
//! manifest is skipped, probed with `quoin catalog list --json`.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;
use std::path::Path;

use super::{Probed, quoin, text};

/// The module every probe lists after the candidate.
const COMPANION: &str = "nfr008-companion";

/// Manifests that are not one valid YAML document, by kind.
const CORRUPT: [(&str, &[u8]); 7] = [
    ("unterminated-flow-sequence", b"name: [unterminated\n"),
    ("bad-indentation", b"name: x\n  bad indent: y\n"),
    ("invalid-utf8", &[0xff, 0xfe, 0x00, 0xc3, 0x28]),
    ("unknown-alias", b"key: *undefined\n"),
    ("two-documents", b"---\na: 1\n---\nb: 2\n"),
    ("unterminated-quoted-scalar", b"\"unterminated string\n"),
    ("unterminated-flow-mapping", b"{a: 1\n"),
];

/// Candidate paths that hold no manifest, by kind.
const MISSING: [&str; 4] = [
    "nonexistent-path",
    "empty-directory",
    "directory-without-manifest",
    "regular-file",
];

fn companion(root: &Path) -> std::path::PathBuf {
    let module = root.join(COMPANION);
    std::fs::create_dir_all(&module).expect("the companion is creatable");
    std::fs::write(
        module.join("manifest.yaml"),
        format!("name: {COMPANION}\nartifact_types:\n  - name: NfrProbe\n"),
    )
    .expect("the companion manifest is writable");
    module
}

/// `quoin catalog list --json` over `candidate` followed by the companion.
fn list(home: &Path, candidate: &Path, companion: &Path) -> std::process::Output {
    let paths = std::env::join_paths([candidate, companion]).expect("paths join");
    quoin(home, &["catalog", "list", "--json"], |process| {
        process.env("QUOIN_MODULE_PATHS", paths);
    })
}

/// NFR-008-M-1: each corrupt kind, matched when assembly exits non-zero
/// without printing a catalog.
pub(crate) fn corrupt_manifests(home: &Path) -> Probed {
    let mut examined = BTreeSet::new();
    let mut matched = BTreeSet::new();
    for (kind, bytes) in CORRUPT {
        let scratch = tempfile::tempdir().expect("scratch is creatable");
        let candidate = scratch.path().join("candidate");
        std::fs::create_dir_all(&candidate).expect("the candidate is creatable");
        std::fs::write(candidate.join("manifest.yaml"), bytes).expect("the manifest is writable");
        let companion = companion(scratch.path());
        let output = list(home, &candidate, &companion);
        if !output.status.success() && output.stdout.is_empty() {
            matched.insert(kind.to_owned());
        }
        examined.insert(kind.to_owned());
    }
    Probed { examined, matched }
}

/// NFR-008-M-2: each manifest-less kind, matched when assembly succeeds and
/// still lists the module after it.
pub(crate) fn missing_manifests(home: &Path) -> Probed {
    let mut examined = BTreeSet::new();
    let mut matched = BTreeSet::new();
    for kind in MISSING {
        let scratch = tempfile::tempdir().expect("scratch is creatable");
        let candidate = scratch.path().join("candidate");
        match kind {
            "empty-directory" => std::fs::create_dir_all(&candidate),
            "directory-without-manifest" => std::fs::create_dir_all(&candidate)
                .and_then(|()| std::fs::write(candidate.join("README.md"), "not a module\n")),
            "regular-file" => std::fs::write(&candidate, "not a module\n"),
            _ => Ok(()),
        }
        .expect("the candidate is built");
        let companion = companion(scratch.path());
        let output = list(home, &candidate, &companion);
        if output.status.success() && text(&output.stdout).contains(COMPANION) {
            matched.insert(kind.to_owned());
        }
        examined.insert(kind.to_owned());
    }
    Probed { examined, matched }
}
