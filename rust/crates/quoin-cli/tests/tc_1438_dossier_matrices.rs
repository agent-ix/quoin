// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Cross-check dossier documents against the matrix and discharge commands
//! over the same real fixture inputs (FR-194, FR-195).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::{Value, json};

mod pack_support;

fn quoin(work: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .current_dir(work)
        .output()
        .unwrap()
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../quoin-core/tests/fixtures/quire-repo")
}

fn payload(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {output:?}"))
}

fn binding(work: &Path) -> PathBuf {
    let path = work.join("binding.json");
    fs::write(&path, json!({
        "schemaVersion":"clause-binding-v1", "clauseSet":{"authority":"example.invalid","id":"fixture","version":"1"},
        "context":{}, "clauses":[
            {"clauseId":"C-2", "force":"mandatory", "outcome":"binding", "reasons":[], "expectedOutputs":[]},
            {"clauseId":"C-1", "force":"permitted", "outcome":"not_binding", "reasons":[], "expectedOutputs":[]}
        ]
    }).to_string()).unwrap();
    path
}

#[trace("FR-195-AC-3")]
#[test]
fn no_binding_is_incomplete_exit_one_without_as_of() {
    let work = tempfile::tempdir().unwrap();
    let output = quoin(
        work.path(),
        &[
            "dossier",
            "generate",
            "--doc",
            "rmm",
            "--version",
            "1",
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let report = payload(&output);
    assert_eq!(report["documents"][0]["status"], "incomplete");
    assert_eq!(
        report["documents"][0]["gaps"][0]["gap_kind"],
        "no_clause_binding"
    );
    let text = fs::read_to_string(work.path().join("dossier/DOS-RMM.md")).unwrap();
    assert!(text.contains("no_clause_binding"));
    assert!(!text.contains("as_of:"));
}

#[trace("FR-195-AC-7")]
#[test]
fn a_binding_or_pack_requires_as_of_before_reading_or_writing() {
    for selector in ["--clause-binding", "--pack"] {
        let work = tempfile::tempdir().unwrap();
        let output = quoin(
            work.path(),
            &[
                "dossier",
                "generate",
                "--doc",
                "rmm",
                "--version",
                "1",
                selector,
                "missing",
                "--json",
            ],
        );
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("--as-of")
        );
        assert!(output.stdout.is_empty());
        assert!(!work.path().join("dossier").exists());
    }
}

#[trace("FR-194-AC-1", "FR-194-AC-5")]
#[test]
fn dossier_rows_equal_the_evidence_backed_matrix_command() {
    let work = tempfile::tempdir().unwrap();
    let source = fixture();
    let repo = source.to_str().unwrap();
    let matrix = quoin(
        work.path(),
        &["matrix", "--repo", repo, "--module", repo, "--json"],
    );
    assert!(matches!(matrix.status.code(), Some(0 | 1)), "{matrix:?}");
    let matrix = payload(&matrix);
    let output = quoin(
        work.path(),
        &[
            "dossier",
            "generate",
            "--doc",
            "trace-matrix",
            "--version",
            "1",
            "--scope",
            repo,
            "--module",
            repo,
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = fs::read_to_string(work.path().join("dossier/DOS-TRACE.md")).unwrap();
    let rows: Vec<_> = text
        .lines()
        .filter(|row| row.starts_with("| FR-"))
        .collect();
    let criteria: Vec<_> = matrix["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|requirement| requirement["criteria"].as_array().unwrap())
        .collect();
    assert!(!criteria.is_empty());
    assert_eq!(rows.len(), criteria.len());
    for criterion in criteria {
        let id = criterion["id"].as_str().unwrap();
        let row = rows
            .iter()
            .find(|row| row.starts_with(&format!("| {id} |")))
            .unwrap();
        assert!(row.ends_with(&format!(
            "| {} | {} |",
            criterion["static_status"].as_str().unwrap(),
            criterion["evidence_status"].as_str().unwrap()
        )));
    }
    assert!(text.contains("Every stored payload shall round-trip through the reader unchanged."));
    assert!(text.contains("Criticality: unassigned"));
}

#[trace("FR-194-AC-3")]
#[test]
fn an_empty_matrix_population_renders_the_source_reason() {
    let work = tempfile::tempdir().unwrap();
    fs::create_dir(work.path().join("spec")).unwrap();
    let module = fixture();
    let repo = work.path().to_str().unwrap();
    let module = module.to_str().unwrap();
    let source = quoin(
        work.path(),
        &["matrix", "--repo", repo, "--module", module, "--json"],
    );
    assert!(source.status.success(), "{source:?}");
    let matrix = payload(&source);
    let reason = matrix["reason"].as_str().unwrap();
    let output = quoin(
        work.path(),
        &[
            "dossier",
            "generate",
            "--doc",
            "trace-matrix",
            "--version",
            "1",
            "--scope",
            repo,
            "--module",
            module,
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(payload(&output)["documents"][0]["status"], "incomplete");
    let text = fs::read_to_string(work.path().join("dossier/DOS-TRACE.md")).unwrap();
    assert!(text.contains(reason));
    assert!(text.contains("no_requirements"));
}

#[trace("FR-195-AC-1", "FR-195-AC-6", "FR-195-AC-8")]
#[test]
fn dossier_rows_equal_the_discharge_command_member_for_member() {
    let work = tempfile::tempdir().unwrap();
    let binding = binding(work.path());
    let path = binding.to_str().unwrap();
    let source = quoin(
        work.path(),
        &[
            "discharge",
            "--binding",
            path,
            "--repo",
            ".",
            "--as-of",
            "2026-11-15T00:00:00Z",
            "--rmm",
            "--json",
        ],
    );
    assert!(source.status.success(), "{source:?}");
    let source = payload(&source);
    let output = quoin(
        work.path(),
        &[
            "dossier",
            "generate",
            "--doc",
            "rmm",
            "--version",
            "1",
            "--clause-binding",
            path,
            "--as-of",
            "2026-11-15T00:00:00Z",
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = fs::read_to_string(work.path().join("dossier/DOS-RMM.md")).unwrap();
    let rows: Vec<_> = text.lines().filter(|row| row.starts_with("| C-")).collect();
    assert_eq!(rows.len(), source["rows"].as_array().unwrap().len());
    for (row, source) in rows.iter().zip(source["rows"].as_array().unwrap()) {
        let cells: Vec<_> = row.trim_matches('|').split('|').map(str::trim).collect();
        for (cell, column) in cells.iter().zip([
            "clauseId",
            "force",
            "applicability",
            "compliance",
            "reason",
            "evidenceRefs",
            "tailoringDescription",
            "tailoringRationale",
        ]) {
            let expected = match &source[column] {
                Value::Null => "(none)".to_owned(),
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            assert_eq!(*cell, expected, "{column}");
        }
    }
    assert!(text.contains("As of: 2026-11-15T00:00:00Z"));
    assert_eq!(
        payload(&output)["documents"][0]["gaps"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[trace("FR-195-AC-2", "FR-195-AC-4")]
#[test]
fn a_restricted_pack_uses_index_order_and_never_renders_clause_text() {
    let work = std::iter::once(std::env::temp_dir())
        .chain(std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
        .chain([PathBuf::from("/var/tmp"), PathBuf::from("/dev/shm")])
        .filter_map(|base| base.canonicalize().ok())
        .filter(|base| {
            base.ancestors()
                .all(|ancestor| !ancestor.join(".git").exists())
        })
        .find_map(|base| tempfile::tempdir_in(base).ok())
        .expect("a writable scratch directory outside every git work tree");
    let home = work.path().join("ix-home");
    let repo = work.path().join("repo");
    pack_support::git_init(&repo);
    let module = pack_support::write_module(work.path(), "dossier-pack", "licensed");
    fs::write(
        module.join("manifest.yaml"),
        "name: dossier-pack\nversion: 0.1.0\nclause_sets:\n  - z.yaml\n  - a.yaml\n",
    )
    .unwrap();
    for id in ["z", "a"] {
        fs::write(
            module.join(format!("{id}.yaml")),
            pack_support::clause_set_yaml(id, "licensed"),
        )
        .unwrap();
    }
    let run = |args: &[&str]| pack_support::quoin(&home, &repo, args, &[]);
    let install = run(&["module", "install", &format!("path:{}", module.display())]);
    assert!(install.status.success(), "{install:?}");
    let source = run(&[
        "discharge",
        "--rmm",
        "--pack",
        "dossier-pack",
        "--repo",
        ".",
        "--as-of",
        "2026-11-15T00:00:00Z",
        "--json",
    ]);
    assert!(source.status.success(), "{source:?}");
    let source = payload(&source);
    let output = run(&[
        "dossier",
        "generate",
        "--doc",
        "rmm",
        "--version",
        "1",
        "--pack",
        "dossier-pack",
        "--as-of",
        "2026-11-15T00:00:00Z",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = fs::read_to_string(repo.join("dossier/DOS-RMM.md")).unwrap();
    assert!(
        !text.contains(pack_support::SENTINEL),
        "licensed text leaked"
    );
    assert!(!text.contains("Synthetic set"), "clause-set title leaked");
    let mut previous = text.find("## Index").unwrap();
    for entry in source["index"].as_array().unwrap() {
        let key = &entry["clauseSet"];
        let heading = format!(
            "## {}/{}@{}",
            key["authority"].as_str().unwrap(),
            key["id"].as_str().unwrap(),
            key["version"].as_str().unwrap()
        );
        let next = text.find(&heading).unwrap();
        assert!(previous < next);
        previous = next;
    }
    assert_eq!(text.matches("| clauseId | force |").count(), 2);
    assert!(text.contains("## Context"));
}

#[trace("FR-194-AC-5", "FR-195-AC-1")]
#[test]
#[ignore = "needs quire, pandoc, typst and SPEC_ARTIFACTS_PROCESS_DIR; run with -- --ignored"]
fn generated_matrix_documents_validate_and_export_to_pdf_and_docx() {
    let module = fs::canonicalize(
        std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").expect("set SPEC_ARTIFACTS_PROCESS_DIR"),
    )
    .unwrap();
    let work = tempfile::tempdir().unwrap();
    let repo = work.path();
    let fixture = fixture();
    let source = fixture.to_str().unwrap();
    let binding = binding(repo);
    let output = quoin(
        repo,
        &[
            "dossier",
            "generate",
            "--doc",
            "trace-matrix",
            "--doc",
            "rmm",
            "--version",
            "1",
            "--scope",
            source,
            "--module",
            source,
            "--clause-binding",
            binding.to_str().unwrap(),
            "--as-of",
            "2026-11-15T00:00:00Z",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@example.invalid"],
        &["config", "user.name", "test"],
        &["add", "."],
        &["commit", "-q", "-m", "generated documents"],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(repo)
                .status()
                .unwrap()
                .success()
        );
    }
    for name in ["DOS-TRACE", "DOS-RMM"] {
        let path = repo.join(format!("dossier/{name}.md"));
        let validation = Command::new("quire")
            .arg("validate")
            .arg(&path)
            .arg("--module")
            .arg(&module)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(validation.status.success(), "{validation:?}");
        for format in ["pdf", "docx"] {
            let target = repo.join(format!("{name}.{format}"));
            let exported = Command::new("quire")
                .arg("export")
                .arg(&path)
                .arg("--module")
                .arg(&module)
                .args(["--to", format, "--output"])
                .arg(&target)
                .current_dir(repo)
                .output()
                .unwrap();
            assert!(exported.status.success(), "{exported:?}");
            assert!(fs::metadata(target).unwrap().len() > 0);
        }
    }
}
