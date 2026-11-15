// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The SRS and architecture generators run through `generate` over a real
//! module and repository (FR-192, FR-193, PLAT-1437): the fixture the
//! `quire.*` routes of quoin-core use, read in place.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_dossier::{DossierErrorCode, Inputs, Report, Status, builtin_registry, generate};

/// The module that is also the repository.
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("quoin-core")
        .join("tests")
        .join("fixtures")
        .join("quire-repo")
}

fn inputs(scope: &Path) -> Inputs {
    Inputs {
        scope: scope.to_owned(),
        modules: vec![fixture()],
        clause_bindings: Vec::new(),
        pack: None,
        as_of: None,
        version: "1.0".to_owned(),
    }
}

fn run(kind: &str, scope: &Path, out: &Path) -> Report {
    generate(
        &builtin_registry().unwrap(),
        &[kind.to_owned()],
        out,
        &inputs(scope),
    )
    .unwrap()
}

/// Every criterion id and text of the repository appears under its requirement
/// and the missing criticality is a gap, read through the real walker.
#[trace("FR-192-AC-5")]
#[test]
fn the_srs_lists_the_criteria_the_repository_states() {
    let out = tempfile::tempdir().unwrap();
    let report = run("srs", &fixture(), out.path());

    let text = fs::read_to_string(out.path().join("DOS-SRS.md")).unwrap();
    assert!(text.contains("## FR-001: A requirement stating two obligations"));
    assert!(text.contains(
        "| FR-001-AC-1 | Every stored payload shall round-trip through the reader unchanged. |"
    ));
    assert!(text.contains("| FR-001-AC-2 | The reader shall refuse a payload over its ceiling. |"));
    assert!(text.contains("Criticality: unassigned"));

    let document = &report.documents[0];
    assert_eq!(document.status, Status::Incomplete);
    let gaps: Vec<_> = document
        .gaps
        .iter()
        .map(|gap| (gap.gap_kind.as_str(), gap.subject.as_str()))
        .collect();
    assert_eq!(gaps, [("criticality_unassigned", "FR-001")]);
}

/// A module set that declares no `design_relations` is incomplete with that one
/// cause, and the run does not exit 0.
#[trace("FR-193-AC-3")]
#[test]
fn an_architecture_without_design_relations_is_incomplete_and_exits_one() {
    let out = tempfile::tempdir().unwrap();
    let report = run("architecture", &fixture(), out.path());

    let document = &report.documents[0];
    assert_eq!(document.status, Status::Incomplete);
    let gaps: Vec<_> = document
        .gaps
        .iter()
        .map(|gap| gap.gap_kind.as_str())
        .collect();
    assert_eq!(gaps, ["design_relations_unavailable"]);
    assert_eq!(report.outcome().code(), 1);
}

/// Two runs over the same repository write the same bytes.
#[test]
fn generating_twice_gives_byte_identical_files() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for out in [first.path(), second.path()] {
        run("srs", &fixture(), out);
        run("architecture", &fixture(), out);
    }
    for name in ["DOS-SRS.md", "DOS-ARCH.md"] {
        assert_eq!(
            fs::read(first.path().join(name)).unwrap(),
            fs::read(second.path().join(name)).unwrap(),
            "{name} differs between runs"
        );
    }
}

/// A file that opens a frontmatter fence and states no `type` is not silently
/// left out of the SRS: the run fails on its source.
#[test]
fn a_malformed_file_in_scope_fails_the_source_instead_of_vanishing() {
    let scope = tempfile::tempdir().unwrap();
    fs::create_dir_all(scope.path().join("spec")).unwrap();
    fs::write(
        scope.path().join("spec").join("bad.md"),
        "---\ntitle: no type here\n---\n\nbody\n",
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();

    let error = generate(
        &builtin_registry().unwrap(),
        &["srs".to_owned()],
        out.path(),
        &inputs(scope.path()),
    )
    .unwrap_err();
    assert_eq!(error.code(), DossierErrorCode::SourceFailed);
    assert!(!out.path().join("DOS-SRS.md").exists());
}
