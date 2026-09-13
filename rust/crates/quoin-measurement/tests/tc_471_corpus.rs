// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The retained interventions, and the producer that wrote one of them.
//!
//! # Why bytes and not values
//!
//! Same reason as `tc_468_corpus`: a port that reads the evidence store but
//! cannot write back the same bytes has changed the store without saying so.
//! The check is byte equality through [`quoin_store::canonical_json_bytes`]
//! over every file retained under `spec/evidence/interventions/`, and the
//! count is asserted so a walk that stopped finding files cannot report
//! success by measuring nothing.
//!
//! # Why the producer runs end to end
//!
//! The retained record was produced by `agent-eval-intervention.ts` from the
//! three files in `spec/evidence/agent-evals/`, all of which are retained. So
//! the port's producer can be measured against something it did not write:
//! run it on the retained definition, in a copy of the retained tree, and the
//! bytes it publishes must be the bytes already in the store.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_measurement::intervention::agent_eval::{
    AgentEvalInterventionDefinition, produce_agent_eval_intervention,
};
use quoin_measurement::{DiskMeasurement, read_intervention_records};
use quoin_store::{canonical_json_bytes, parse_strict_json};

/// How many intervention records `spec/evidence/interventions/` retains.
///
/// Retained evidence is append-only, so this number may grow and may never
/// shrink. A change here is a claim that the store gained a record.
const RETAINED_INTERVENTIONS: usize = 1;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn retained() -> Vec<(String, Vec<u8>)> {
    let root = repo().join("spec").join("evidence").join("interventions");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root).expect("the interventions directory is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_some_and(|end| end == "json") {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            out.push((name, std::fs::read(&path).expect("a readable record")));
        }
    }
    out.sort();
    out
}

/// Trace: FR-100-AC-1, FR-100-AC-4
/// Provenance: quoin#471
///
/// Every retained record parses, re-canonicalises to the bytes on disk, and is
/// accepted by the vendored schema through [`read_intervention_records`].
#[test]
fn tc_471_every_retained_intervention_round_trips_byte_identically() {
    let files = retained();
    assert!(
        !files.is_empty(),
        "the intervention corpus is empty; this test would measure nothing"
    );
    assert_eq!(files.len(), RETAINED_INTERVENTIONS);
    for (name, bytes) in &files {
        let value = parse_strict_json(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let written =
            canonical_json_bytes(&value).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            written, *bytes,
            "{name} does not round-trip through canonical_json_bytes"
        );
    }

    let source = DiskMeasurement::new(repo());
    let records = read_intervention_records(&source).expect("the retained records are readable");
    assert_eq!(records.len(), files.len());
}

/// Copy a directory tree, so the producer writes into a tree it may modify.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a destination directory");
    for entry in std::fs::read_dir(from).expect("a readable source directory") {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a copied file");
        }
    }
}

/// Trace: FR-100-AC-6
/// Provenance: quoin#471
///
/// The producer, measured against a record it did not write: the retained
/// definition and the two retained reports must reproduce the retained record,
/// byte for byte, at the path the record already occupies.
#[test]
fn tc_471_the_producer_reproduces_the_retained_record() {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let repo_copy = temporary.path();
    copy_tree(&repo().join("spec"), &repo_copy.join("spec"));
    let published = repo_copy
        .join("spec")
        .join("evidence")
        .join("interventions")
        .join("quoin-270-cli-eval-sentinel-contract.json");
    let expected = std::fs::read(&published).expect("the retained record was copied");
    std::fs::remove_file(&published).expect("the retained record is removable");

    let raw = std::fs::read_to_string(
        repo_copy
            .join("spec")
            .join("evidence")
            .join("agent-evals")
            .join("quoin-270-sentinel-definition.json"),
    )
    .expect("the retained definition is readable");
    let definition: AgentEvalInterventionDefinition =
        serde_json::from_str(&raw).expect("the retained definition deserialises");

    let produced = produce_agent_eval_intervention(repo_copy, &definition)
        .expect("the retained definition produces a record");
    assert_eq!(
        std::fs::read(&produced.path).expect("the produced record is readable"),
        expected,
        "the producer did not reproduce the retained record"
    );
    // The bytes match; the *name* does not. The retained record predates the
    // `p-`/`b-` namespacing of `intervention.ts:39-45`, so the writer both
    // implementations share would today publish it beside itself rather than
    // over itself. That is a defect in the retained store, not in this port —
    // quoin#486 — and it is held here so a rename closes the test too.
    assert_eq!(
        produced.path,
        published.with_file_name("p-quoin-270-cli-eval-sentinel-contract.json")
    );
    assert!(
        !published.exists(),
        "the retained record's own name is now what the writer chooses; \
         drop the quoin#486 expectation above"
    );
}
