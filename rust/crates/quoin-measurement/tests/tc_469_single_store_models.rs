// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Evidence and measurement records keep their existing shared owners.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "an architecture census failing is a test failure"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn workspace_sources() -> Vec<(String, String)> {
    fn walk(workspace: &Path, directory: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(directory).expect("crate source directory is readable") {
            let path = entry.expect("source directory entry is readable").path();
            if path.is_dir() {
                walk(workspace, &path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let name = path
                    .strip_prefix(workspace)
                    .expect("source is under the Rust workspace")
                    .to_string_lossy()
                    .replace('\\', "/");
                into.push((
                    name,
                    std::fs::read_to_string(&path).expect("source is readable"),
                ));
            }
        }
    }

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let crates = workspace.join("crates");
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(&crates).expect("workspace crates are readable") {
        let crate_root = entry.expect("crate entry is readable").path();
        let source = crate_root.join("src");
        if source.is_dir() {
            walk(&workspace, &source, &mut sources);
        }
    }
    sources.sort_by(|left, right| left.0.cmp(&right.0));
    sources
}

fn declares(line: &str, type_name: &str) -> bool {
    let line = line.trim_start();
    let line = line
        .strip_prefix("pub ")
        .or_else(|| line.strip_prefix("pub(crate) "))
        .unwrap_or(line);
    ["struct ", "enum ", "type "].into_iter().any(|keyword| {
        line.strip_prefix(keyword).is_some_and(|rest| {
            rest.strip_prefix(type_name).is_some_and(|tail| {
                tail.chars()
                    .next()
                    .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_')
            })
        })
    })
}

fn declaration_owners(sources: &[(String, String)], type_name: &str) -> Vec<String> {
    sources
        .iter()
        .filter(|(_, source)| source.lines().any(|line| declares(line, type_name)))
        .map(|(path, _)| path.clone())
        .collect()
}

#[trace("FR-100-CON-1")]
#[test]
fn one_shared_store_and_one_owner_for_evidence_and_measurement_models() {
    let sources = workspace_sources();
    for (type_name, owner) in [
        ("StorePath", "crates/quoin-store/src/store.rs"),
        (
            "StoredAssuranceRecord",
            "crates/quoin-evidence/src/types/assurance.rs",
        ),
        (
            "MeasurementPlan",
            "crates/quoin-measurement/src/types/plan.rs",
        ),
        (
            "MeasurementCollection",
            "crates/quoin-measurement/src/types/collection.rs",
        ),
        (
            "MeasurementObservation",
            "crates/quoin-measurement/src/types/observation.rs",
        ),
    ] {
        assert_eq!(
            declaration_owners(&sources, type_name),
            [owner.to_owned()],
            "{type_name} must keep its single canonical owner"
        );
    }

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    assert!(
        !repo.join("src/evidence").exists() && !repo.join("src/measurement").exists(),
        "the retired evidence and measurement model trees must not return beside Rust"
    );

    let evidence_manifest =
        std::fs::read_to_string(repo.join("rust/crates/quoin-evidence/Cargo.toml"))
            .expect("evidence crate manifest is readable");
    let measurement_manifest =
        std::fs::read_to_string(repo.join("rust/crates/quoin-measurement/Cargo.toml"))
            .expect("measurement crate manifest is readable");
    assert!(
        evidence_manifest.contains("quoin-store = { workspace = true }")
            && measurement_manifest.contains("quoin-store = { workspace = true }")
            && measurement_manifest.contains("quoin-evidence = { workspace = true }"),
        "evidence and measurement must use the shared store and evidence crate"
    );
}
