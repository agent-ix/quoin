// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin discharge --repo --rmm` against a real repository (FR-135).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::Path;
use std::process::{Command, Output};

const BINDING: &str = r#"{
  "schemaVersion": "clause-binding-v1",
  "clauseSet": {"authority": "quire", "id": "spec", "version": "1"},
  "context": {},
  "clauses": [
    {"clauseId": "C-1", "force": "mandatory", "outcome": "binding", "reasons": [], "expectedOutputs": []},
    {"clauseId": "C-2", "force": "mandatory", "outcome": "binding", "reasons": [], "expectedOutputs": []}
  ]
}"#;

const FACTS: &str = r#"[{
  "kind": "direct", "clauseId": "C-1", "evidenceRefs": ["evidence://run/one"],
  "attestation": {"attestedBy": "r", "authority": "lead",
    "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"}
}]"#;

fn quoin(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home)
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[trace("FR-135-AC-7")]
#[test]
fn tc_3206_the_matrix_command_renders_recorded_facts_and_refuses_misuse() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let binding = dir.path().join("binding.json");
    let facts = dir.path().join("facts.json");
    std::fs::write(&binding, BINDING).unwrap();
    std::fs::write(&facts, FACTS).unwrap();
    let (binding, facts, repo) = (
        binding.to_str().unwrap(),
        facts.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    );
    let recorded = quoin(
        home.path(),
        &[
            "evidence",
            "record-discharge",
            "--repo",
            repo,
            "--binding",
            binding,
            "--facts",
            facts,
        ],
    );
    assert!(recorded.status.success(), "{recorded:?}");

    let base = [
        "discharge",
        "--binding",
        binding,
        "--as-of",
        "2026-08-15T00:00:00Z",
    ];
    let rendered = quoin(
        home.path(),
        &[&base[..], &["--repo", repo, "--rmm"]].concat(),
    );
    assert!(rendered.status.success(), "{rendered:?}");
    let report = text(&rendered);
    assert!(
        report.contains("| C-1 | Applicable | Compliant | evidence://run/one | - | - |"),
        "{report}"
    );
    assert!(
        report.contains("| C-2 | Applicable | Not compliant - undischarged: no discharge fact |"),
        "{report}"
    );

    let json = quoin(
        home.path(),
        &[&base[..], &["--repo", repo, "--rmm", "--json"]].concat(),
    );
    assert!(json.status.success(), "{json:?}");
    let matrix: serde_json::Value = serde_json::from_str(&text(&json)).unwrap();
    assert_eq!(matrix["rows"][1]["compliance"], "undischarged");

    let no_repo = quoin(
        home.path(),
        &[&base[..], &["--facts", facts, "--rmm"]].concat(),
    );
    assert_eq!(no_repo.status.code(), Some(1), "{no_repo:?}");
    let message = String::from_utf8_lossy(&no_repo.stderr);
    assert!(message.contains("--rmm needs --repo"), "{message}");
    let with_inward = quoin(
        home.path(),
        &[&base[..], &["--repo", repo, "--rmm", "--inward"]].concat(),
    );
    assert_eq!(with_inward.status.code(), Some(1), "{with_inward:?}");
    let message = String::from_utf8_lossy(&with_inward.stderr);
    assert!(
        message.contains("--inward and --rmm cannot be combined"),
        "{message}"
    );
}
