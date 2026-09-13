// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The GitHub-release producer, against the evidence that produced the one
//! retained pair.
//!
//! # The oracle is the committed pair, not a fixture this test wrote
//!
//! `spec/evidence/operational/pairs/` holds one pair, produced by the retained
//! TypeScript from the definition and three exports committed under
//! `spec/evidence/github-actions/`. This test hands the Rust producer those
//! same four files and requires the same bytes and the same file name. A
//! fixture written by this test would prove only that the producer agrees with
//! itself.
//!
//! # `on:` is a string, and that is a claim about the YAML version
//!
//! Every GitHub workflow keys its triggers on `on:`. Under YAML 1.1 — which
//! every libyaml binding implements — that plain scalar resolves to the boolean
//! `true`, and a producer looking up `"on"` would miss on every workflow file
//! there is. [`quoin_yaml`] is YAML 1.2 core, where it stays the string. The
//! producer depends on that, so it is asserted here against the retained export
//! rather than assumed.
//!
//! # The multi-document scan (quoin#466)
//!
//! quoin#466 is open on the possibility that a retained workflow export is a
//! multi-document YAML stream, which [`quoin_yaml::from_str`] reads only the
//! first document of. The scan below measures it. A hit is a real defect and
//! belongs on #466; it is not something for this test to accept quietly.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_measurement::operational::github_release::{
    GitHubReleaseProducerDefinition, produce_github_release_operational,
};
use quoin_measurement::source::SystemClock;

/// How many workflow exports `spec/evidence/github-actions/` retains.
const RETAINED_WORKFLOW_EXPORTS: usize = 1;

/// The pair the retained TypeScript produced from the retained exports.
const RETAINED_PAIR: &str = "c1b30a188d4d03bbe316e0fdb7582eff3fa314c55268a5f71f81f99f8ea2acf8.json";

/// The definition that produced it.
const DEFINITION: &str = "github-actions/quoin-271-release-v0.22.5-definition.json";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn evidence_root() -> PathBuf {
    repo().join("spec").join("evidence")
}

fn workflow_exports() -> Vec<(PathBuf, String)> {
    let root = evidence_root().join("github-actions");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&root).expect("the github-actions directory is readable") {
        let path = entry.expect("a directory entry").path();
        if path
            .extension()
            .is_some_and(|end| end == "yml" || end == "yaml")
        {
            let text = std::fs::read_to_string(&path).expect("a readable export");
            found.push((path, text));
        }
    }
    found.sort();
    found
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a writable destination");
    for entry in std::fs::read_dir(from).expect("a readable source") {
        let path = entry.expect("a directory entry").path();
        let target = to.join(path.file_name().expect("a file name"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("a copied file");
        }
    }
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#472
#[test]
fn tc_472_the_on_key_survives_the_yaml_reader_as_a_string() {
    let exports = workflow_exports();
    assert_eq!(
        exports.len(),
        RETAINED_WORKFLOW_EXPORTS,
        "anti-vacuity floor: {RETAINED_WORKFLOW_EXPORTS} retained workflow export(s) expected, \
         saw {}",
        exports.len()
    );
    for (path, text) in &exports {
        let document: serde_json::Value = quoin_yaml::from_str(text)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let members = document
            .as_object()
            .unwrap_or_else(|| panic!("{} is not a mapping", path.display()));
        assert!(
            members.contains_key("on"),
            "{} lost its `on:` key — a YAML 1.1 reader resolves it to the boolean `true` and \
             this producer would never find a trigger again",
            path.display()
        );
        assert!(
            !members.contains_key("true"),
            "{} carries a `true` key, which is what `on:` becomes under YAML 1.1",
            path.display()
        );
    }
}

/// quoin#466: a retained workflow export is a single YAML document.
///
/// Trace: FR-100-AC-4, NFR-025-AC-3
/// Provenance: quoin#472
#[test]
fn tc_472_no_retained_workflow_export_is_a_multi_document_stream() {
    let exports = workflow_exports();
    assert_eq!(exports.len(), RETAINED_WORKFLOW_EXPORTS);
    for (path, text) in &exports {
        let markers = text.lines().filter(|line| *line == "---").count();
        assert_eq!(
            markers,
            0,
            "{} carries {markers} `---` document marker(s). quoin#466 is open on exactly this: \
             `quoin_yaml::from_str` reads the first document of a stream and the rest would be \
             silently dropped. Report it on #466 rather than relaxing this scan.",
            path.display()
        );
    }
}

/// Trace: FR-100-AC-4, FR-100-AC-2
/// Provenance: quoin#472
#[test]
fn tc_472_the_producer_rebuilds_the_committed_pair_byte_for_byte() {
    let expected_path = evidence_root()
        .join("operational")
        .join("pairs")
        .join(RETAINED_PAIR);
    let expected = std::fs::read(&expected_path).expect("the retained pair is readable");

    // A repository holding the producer's whole input contract and nothing
    // else: the assurance documents the governance check reads, and the four
    // retained GitHub exports. Not the retained pair — the producer has to
    // write that itself.
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path();
    copy_tree(
        &repo().join("spec").join("assurance"),
        &root.join("spec").join("assurance"),
    );
    copy_tree(
        &evidence_root().join("github-actions"),
        &root.join("spec").join("evidence").join("github-actions"),
    );

    let text = std::fs::read_to_string(evidence_root().join(DEFINITION))
        .expect("the retained definition is readable");
    let definition: GitHubReleaseProducerDefinition =
        serde_json::from_str(&text).expect("the retained definition parses");

    let produced = produce_github_release_operational(root, &SystemClock, &definition)
        .expect("the retained evidence satisfies the producer's input contract");

    assert_eq!(
        produced.path.file_name().and_then(|name| name.to_str()),
        Some(RETAINED_PAIR),
        "the pair is named by the digest of the two record identities, and the name moved"
    );
    let written = std::fs::read(&produced.path).expect("the produced pair is readable");
    assert_eq!(
        String::from_utf8(written).unwrap(),
        String::from_utf8(expected).unwrap(),
        "the Rust producer did not reproduce the retained pair"
    );
}
