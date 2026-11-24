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
use quoin_assurance::matrix::{BindingDetail, MatrixOutput};
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
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
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
    builtin_registry(sources(matrix())).unwrap()
}

#[trace("FR-191", "FR-194-AC-5", "FR-196-AC-1")]
#[test]
fn m1_and_verification_report_keep_bundle_order_and_share_the_matrix_source() {
    let out = tempfile::tempdir().unwrap();
    let kinds = [
        "verification-report",
        "rmm",
        "trace-matrix",
        "architecture",
        "srs",
    ]
    .map(str::to_owned);
    let report = generate(&registry(), &kinds, out.path(), &inputs()).unwrap();
    assert_eq!(
        report
            .documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>(),
        ["DOS-SRS", "DOS-ARCH", "DOS-TRACE", "DOS-RMM", "DOS-VR"]
    );
    for name in ["DOS-TRACE.md", "DOS-VR.md"] {
        let text = fs::read_to_string(out.path().join(name)).unwrap();
        for (id, status) in [("FR-001-AC-1", "bound"), ("FR-001-AC-2", "stale")] {
            let row = text
                .lines()
                .find(|row| row.starts_with(&format!("| {id} |")))
                .unwrap();
            let tail = if name == "DOS-TRACE.md" {
                format!("| {status} | unassigned |")
            } else {
                format!("| {status} |")
            };
            assert!(row.ends_with(&tail), "{name}: {row}");
        }
    }
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
    assert!(text.contains(
        "| FR-001-AC-1 | Test | tc_1 (function, tests/t.rs:3) | none | not recorded | bound |"
    ));
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

#[trace("FR-196-AC-5")]
#[test]
fn a_stale_suite_without_a_traced_or_retained_binder_is_unbacked() {
    let mut source = matrix();
    let criterion = &mut source.requirements[0].criteria[1];
    criterion.binders.clear();
    criterion.static_status = "untagged".to_owned();
    criterion.evidence_detail.bindings = vec![BindingDetail {
        suite: "old-suite".to_owned(),
        run: None,
    }];
    let registry = builtin_registry(sources(source)).unwrap();
    let out = tempfile::tempdir().unwrap();
    let report = generate(&registry, &kinds(), out.path(), &inputs()).unwrap();
    let text = fs::read_to_string(out.path().join("DOS-VR.md")).unwrap();
    assert!(text.contains("suite old-suite"));
    let gaps: Vec<_> = report.documents[0]
        .gaps
        .iter()
        .map(|gap| (gap.gap_kind.as_str(), gap.subject.as_str()))
        .collect();
    assert_eq!(
        gaps,
        [
            ("criticality_unassigned", "FR-001"),
            ("unbacked_criterion", "FR-001-AC-2"),
            ("evidence_failed", "FR-001-AC-2"),
        ]
    );
}

/// The report over `matrix`, with the text of `DOS-VR.md` and its gaps as
/// `(kind, subject, detail)`.
fn report_of(matrix: MatrixOutput) -> (String, Vec<(String, String, String)>) {
    let registry = builtin_registry(sources(matrix)).unwrap();
    let out = tempfile::tempdir().unwrap();
    let report = generate(&registry, &kinds(), out.path(), &inputs()).unwrap();
    let text = fs::read_to_string(out.path().join("DOS-VR.md")).unwrap();
    let gaps = report.documents[0]
        .gaps
        .iter()
        .map(|gap| {
            (
                gap.gap_kind.as_str().to_owned(),
                gap.subject.clone(),
                gap.detail.clone(),
            )
        })
        .collect();
    (text, gaps)
}

/// FR-001-AC-1 of [`matrix`], bound to `bindings` as `matrix.build` reports them.
fn with_bindings(bindings: &serde_json::Value) -> MatrixOutput {
    let mut source = matrix();
    source.requirements[0].criteria[0].evidence_detail.bindings =
        serde_json::from_value(bindings.clone()).unwrap();
    source
}

fn run(timestamp: &str, tool: &str, revision: Option<&str>) -> serde_json::Value {
    let mut run = json!({
        "file": "runs/unit/2026-11-01T00-00-00Z.json", "timestamp": timestamp, "tool": tool,
        "outcomes": { "pass": 1, "fail": 1, "skip": 0, "error": 0 },
    });
    if let Some(revision) = revision {
        run["source_revision"] = json!(revision);
    }
    run
}

fn run_gaps(gaps: &[(String, String, String)]) -> Vec<&(String, String, String)> {
    gaps.iter()
        .filter(|gap| gap.0 == "run_identity_unrecorded")
        .collect()
}

#[trace("FR-196-AC-9")]
#[test]
fn a_bound_suite_shows_the_recorded_run_identity_and_tally_without_a_computed_outcome() {
    // The status says bound while the recorded outcomes include a fail: both
    // are shown unchanged.
    let (text, gaps) = report_of(with_bindings(&json!([
        { "suite": "unit", "run": run("2026-11-01T00:00:00Z", "cargo-test 1.90", None) }
    ])));
    let row = text
        .lines()
        .find(|row| row.starts_with("| FR-001-AC-1 |"))
        .unwrap();
    assert!(row.contains(
        "suite unit: timestamp 2026-11-01T00:00:00Z, tool cargo-test 1.90, file runs/unit/2026-11-01T00-00-00Z.json, outcomes pass 1 fail 1 skip 0 error 0, source revision not recorded"
    ), "{row}");
    assert!(row.ends_with("| bound |"));
    assert!(run_gaps(&gaps).is_empty(), "{gaps:?}");
}

#[trace("FR-196-AC-10")]
#[test]
fn an_empty_tool_or_timestamp_is_one_gap_and_a_recorded_revision_is_verbatim() {
    let (_, gaps) = report_of(with_bindings(&json!([
        { "suite": "unit", "run": run("2026-11-01T00:00:00Z", "", None) }
    ])));
    let found = run_gaps(&gaps);
    assert_eq!(found.len(), 1, "{gaps:?}");
    assert_eq!(found[0].1, "FR-001-AC-1");
    assert!(found[0].2.contains("suite unit") && found[0].2.contains("tool"));
    assert!(!found[0].2.contains("timestamp"));

    let (text, gaps) = report_of(with_bindings(&json!([
        { "suite": "unit", "run": run("", "cargo-test 1.90", Some("9f2c1ab")) }
    ])));
    let found = run_gaps(&gaps);
    assert_eq!(found.len(), 1, "{gaps:?}");
    assert!(found[0].2.contains("timestamp") && !found[0].2.contains("tool"));
    assert!(text.contains("timestamp not recorded, tool cargo-test 1.90"));
    assert!(text.contains("source revision 9f2c1ab"));
}

#[trace("FR-196-AC-11")]
#[test]
fn a_criterion_with_no_selected_run_reads_not_recorded_and_gets_no_run_gap() {
    // No bound suite at all, and an undischarged status.
    let mut source = with_bindings(&json!([]));
    source.requirements[0].criteria[0].evidence_status =
        quoin_assurance::matrix::EvidenceStatus::Undischarged;
    let (text, gaps) = report_of(source);
    let row = text
        .lines()
        .find(|row| row.starts_with("| FR-001-AC-1 |"))
        .unwrap();
    assert!(row.contains("| not recorded | undischarged |"), "{row}");
    assert!(
        gaps.iter()
            .any(|gap| gap.0 == "no_evidence" && gap.1 == "FR-001-AC-1")
    );
    assert!(run_gaps(&gaps).is_empty());

    // A bound suite with no retained run holding a bound entry.
    let (text, gaps) = report_of(with_bindings(&json!([{ "suite": "unit" }])));
    assert!(text.contains("suite unit: not recorded"));
    assert!(run_gaps(&gaps).is_empty());
}

#[trace("FR-196-AC-12")]
#[test]
fn two_bound_suites_show_one_run_identity_each() {
    let mut other = run("2026-11-02T00:00:00Z", "pytest 8", None);
    other["file"] = json!("runs/py/2026-11-02T00-00-00Z.json");
    let (text, _) = report_of(with_bindings(&json!([
        { "suite": "py", "run": other },
        { "suite": "unit", "run": run("2026-11-01T00:00:00Z", "cargo-test 1.90", None) }
    ])));
    let row = text
        .lines()
        .find(|row| row.starts_with("| FR-001-AC-1 |"))
        .unwrap();
    assert!(row.contains("suite py: timestamp 2026-11-02T00:00:00Z, tool pytest 8"));
    assert!(row.contains("suite unit: timestamp 2026-11-01T00:00:00Z, tool cargo-test 1.90"));
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

fn sources(matrix: MatrixOutput) -> quoin_dossier::Sources {
    quoin_dossier::Sources {
        matrix: Arc::new(Fixed(matrix)),
        rmm: |_| panic!("unexpected discharge source"),
        soup: |_| panic!("unexpected SOUP source"),
        policy: |_| panic!("unexpected policy source"),
        threat_model: |_| panic!("unexpected threat-model source"),
        records_of_processing: |_| panic!("unexpected records-of-processing source"),
    }
}
