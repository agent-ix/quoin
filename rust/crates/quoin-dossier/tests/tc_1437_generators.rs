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
use std::process::Command;

use ix_trace_rs::trace;
use quoin_dossier::{DossierErrorCode, Inputs, Report, Status, generate};

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
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: None,
        version: "1.0".to_owned(),
        producer_version: "test-quoin-version".to_owned(),
    }
}

fn run(kind: &str, scope: &Path, out: &Path) -> Report {
    generate(&registry(), &[kind.to_owned()], out, &inputs(scope)).unwrap()
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
    assert_eq!(
        gaps,
        [
            ("criticality_unassigned", "FR-001"),
            ("safety_class_unassigned", "FR-001")
        ]
    );
    assert!(text.contains("Software safety class: unassigned"));
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

/// Run `program` in `dir` and return its stdout; a failure panics with both streams.
fn tool(program: &str, args: &[&str], dir: &Path) -> String {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|error| panic!("{program} does not start: {error}"));
    assert!(
        output.status.success(),
        "{program} {args:?}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Needs `git`, `quire`, `pandoc`, `typst` and `pdftotext`, and the
/// `spec-artifacts-process` module that defines the `DossierDocument`
/// archetype, read in place from `SPEC_ARTIFACTS_PROCESS_DIR` (its
/// `spec_artifacts_process` directory). The SRS and architecture files are
/// written by `generate` over the real fixture, committed unchanged, validated
/// by `quire validate`, exported together to PDF by `quire export`, and every
/// fixture requirement id must be in the PDF text. Nothing of the module is
/// copied here.
///
/// Provenance: PLAT-1437 acceptance 7.
#[trace("FR-192-AC-5", "FR-193-AC-3")]
#[test]
#[ignore = "needs quire, pandoc, typst, pdftotext and SPEC_ARTIFACTS_PROCESS_DIR; run with -- --ignored"]
fn generated_srs_and_architecture_validate_and_export_to_pdf() {
    let module = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR")
        .map(PathBuf::from)
        .expect("set SPEC_ARTIFACTS_PROCESS_DIR to the spec_artifacts_process module directory");
    let module = fs::canonicalize(&module)
        .unwrap_or_else(|error| panic!("the module is not at {}: {error}", module.display()));
    let repo = tempfile::tempdir().unwrap();
    let work = repo.path();
    tool("git", &["init", "-q"], work);
    tool("git", &["config", "user.email", "t@example.com"], work);
    tool("git", &["config", "user.name", "t"], work);

    let out = work.join("dossier");
    let kinds = ["srs".to_owned(), "architecture".to_owned()];
    let inputs = inputs(&fixture());
    let registry = registry();
    generate(&registry, &kinds, &out, &inputs).unwrap();
    tool("git", &["add", "-A"], work);
    tool("git", &["commit", "-q", "-m", "generated dossier"], work);

    let module_arg = module.to_str().unwrap();
    for name in ["DOS-SRS.md", "DOS-ARCH.md"] {
        let path = out.join(name);
        tool(
            "quire",
            &["validate", "--module", module_arg, path.to_str().unwrap()],
            work,
        );
    }

    // The library export takes no module, so the real-tool boundary is
    // `quire export` itself, run inside the committed repository.
    let pdf = work.join("dossier.pdf");
    let srs = out.join("DOS-SRS.md");
    let arch = out.join("DOS-ARCH.md");
    tool(
        "quire",
        &[
            "export",
            srs.to_str().unwrap(),
            arch.to_str().unwrap(),
            "--module",
            module_arg,
            "--bundle-title",
            "Submission Dossier",
            "--bundle-id",
            "DOS",
            "--to",
            "pdf",
            "--output",
            pdf.to_str().unwrap(),
        ],
        work,
    );
    assert!(pdf.exists(), "no PDF written");

    let text = tool("pdftotext", &[pdf.to_str().unwrap(), "-"], work);
    let mut ids = 0;
    for entry in fs::read_dir(fixture().join("spec")).unwrap() {
        let source = fs::read_to_string(entry.unwrap().path()).unwrap();
        for id in source.lines().filter_map(|line| line.strip_prefix("id: ")) {
            assert!(text.contains(id.trim()), "{id} is not in the PDF text");
            ids += 1;
        }
    }
    assert!(ids > 0, "the fixture states no requirement id");
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
        &registry(),
        &["srs".to_owned()],
        out.path(),
        &inputs(scope.path()),
    )
    .unwrap_err();
    assert_eq!(error.code(), DossierErrorCode::SourceFailed);
    assert!(!out.path().join("DOS-SRS.md").exists());
}

fn registry() -> quoin_dossier::Registry {
    quoin_dossier::builtin_registry(quoin_dossier::Sources {
        matrix: std::sync::Arc::new(NoMatrix),
        rmm: |_| panic!("unexpected discharge source"),
        soup: |_| panic!("unexpected SOUP source"),
        policy: |_| panic!("unexpected policy source"),
        threat_model: |_| panic!("unexpected threat-model source"),
        records_of_processing: |_| panic!("unexpected records-of-processing source"),
    })
    .unwrap()
}

struct NoMatrix;

impl quoin_dossier::MatrixSource for NoMatrix {
    fn matrix(
        &self,
        _inputs: &Inputs,
    ) -> Result<quoin_assurance::matrix::MatrixOutput, quoin_dossier::SourceFailure> {
        Err(quoin_dossier::SourceFailure {
            source: "matrix".to_owned(),
            detail: "not used by these documents".to_owned(),
        })
    }
}
