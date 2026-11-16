// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The verification report through `generate` over a real repository
//! (FR-196, PLAT-1617): the fixture the `quire.*` routes of quoin-core use,
//! read in place, with the matrix a fixed `MatrixOutput`.

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
use std::sync::Arc;

use ix_trace_rs::trace;
use quoin_assurance::matrix::MatrixOutput;
use quoin_dossier::{Inputs, MatrixSource, SourceFailure, Status, builtin_registry, generate};
use serde_json::json;

/// The module that is also the repository.
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("quoin-core")
        .join("tests")
        .join("fixtures")
        .join("quire-repo")
}

fn inputs() -> Inputs {
    Inputs {
        scope: fixture(),
        modules: vec![fixture()],
        clause_bindings: Vec::new(),
        pack: None,
        as_of: None,
        version: "1.0".to_owned(),
    }
}

/// A matrix as `matrix.build` returns it for FR-001's two criteria: one bound
/// by a traced test, one stale because its newest run failed.
struct Fixed(MatrixOutput);

impl MatrixSource for Fixed {
    fn matrix(&self, _inputs: &Inputs) -> Result<MatrixOutput, SourceFailure> {
        Ok(self.0.clone())
    }
}

fn matrix() -> MatrixOutput {
    let detail = |findings: serde_json::Value| json!({ "findings": findings, "bindings": [], "unevaluated": [] });
    let binder = json!({
        "path": "tests/t.rs", "line": 3, "column": 1,
        "qualified_name": "tc_1", "kind": "function",
    });
    serde_json::from_value(json!({ "requirements": [{
        "id": "FR-001",
        "criteria": [
            {
                "id": "FR-001-AC-1", "method": "Test", "static_status": "tagged",
                "binders": [binder], "evidence_status": "bound",
                "evidence_detail": detail(json!([])),
            },
            {
                "id": "FR-001-AC-2", "method": "Test", "static_status": "tagged",
                "binders": [binder], "evidence_status": "stale",
                "evidence_detail": detail(json!([
                    { "kind": "stale-evidence", "summary": "x", "cause": "failed-run" }
                ])),
            },
        ],
    }]}))
    .unwrap()
}

fn registry() -> quoin_dossier::Registry {
    builtin_registry(Arc::new(Fixed(matrix()))).unwrap()
}

fn kinds() -> [String; 1] {
    ["verification-report".to_owned()]
}

#[trace("FR-196-AC-1", "FR-196-AC-3")]
#[test]
fn the_report_titles_rows_from_the_repository_and_flags_the_failed_run() {
    let out = tempfile::tempdir().unwrap();
    let report = generate(&registry(), &kinds(), out.path(), &inputs()).unwrap();

    let text = fs::read_to_string(out.path().join("DOS-VR.md")).unwrap();
    assert!(text.contains("## FR-001: A requirement stating two obligations"));
    assert!(text.contains("Criticality: unassigned"));
    assert!(text.contains("| FR-001-AC-1 | Test | tc_1 (function, tests/t.rs:3) | none | bound |"));
    assert!(text.contains("| stale | 1 |"));

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
            ("evidence_failed", "FR-001-AC-2")
        ]
    );
}

#[trace("FR-196-AC-8")]
#[test]
fn generating_twice_gives_byte_identical_files() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for out in [first.path(), second.path()] {
        generate(&registry(), &kinds(), out, &inputs()).unwrap();
    }
    assert_eq!(
        fs::read(first.path().join("DOS-VR.md")).unwrap(),
        fs::read(second.path().join("DOS-VR.md")).unwrap()
    );
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
/// `spec_artifacts_process` directory). The report is written by `generate`,
/// committed unchanged, validated by `quire validate` and exported to PDF by
/// `quire export`. Nothing of the module is copied here.
///
/// Provenance: PLAT-1617 acceptance 7.
#[trace("FR-196-AC-1")]
#[test]
#[ignore = "needs quire, pandoc, typst, pdftotext and SPEC_ARTIFACTS_PROCESS_DIR; run with -- --ignored"]
fn the_generated_report_validates_and_exports_to_pdf() {
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
    generate(&registry(), &kinds(), &out, &inputs()).unwrap();
    tool("git", &["add", "-A"], work);
    tool("git", &["commit", "-q", "-m", "generated dossier"], work);

    let module_arg = module.to_str().unwrap();
    let report = out.join("DOS-VR.md");
    tool(
        "quire",
        &["validate", "--module", module_arg, report.to_str().unwrap()],
        work,
    );

    let pdf = work.join("dossier.pdf");
    tool(
        "quire",
        &[
            "export",
            report.to_str().unwrap(),
            "--module",
            module_arg,
            "--to",
            "pdf",
            "--output",
            pdf.to_str().unwrap(),
        ],
        work,
    );
    let text = tool("pdftotext", &[pdf.to_str().unwrap(), "-"], work);
    assert!(
        text.contains("FR-001-AC-2"),
        "the report row is not in the PDF"
    );
}
