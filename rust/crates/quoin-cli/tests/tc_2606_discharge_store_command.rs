// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence record-discharge` and `quoin discharge --repo --inward`
//! against a real repository (FR-133).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
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

const TEMPORARY_EXCEPTION_FACTS: &str = r#"[{
  "kind": "disposition", "clauseId": "C-1", "decision": "temporary_exception",
  "owner": "risk-owner-1", "rationale": "bounded synthetic exception",
  "approvalRef": "approval://test/one", "decisionExpiresAt": "2026-08-20T00:00:00Z",
  "attestation": {"attestedBy": "reviewer-1", "authority": "quality-lead",
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

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

#[trace("FR-133-AC-7")]
#[test]
fn tc_2606_recorded_facts_are_read_back_by_the_discharge_command() {
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
    assert!(text(&recorded).starts_with("C-1 "), "{}", text(&recorded));

    let inward = quoin(
        home.path(),
        &[
            "discharge",
            "--binding",
            binding,
            "--repo",
            repo,
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--inward",
        ],
    );
    assert!(inward.status.success(), "{inward:?}");
    let report = text(&inward);
    assert!(report.contains("- Binding clauses: 2"), "{report}");
    assert!(report.contains("- Discharged by evidence: 1"), "{report}");
    assert!(report.contains("- `C-2` — mandatory"), "{report}");

    let partition = quoin(
        home.path(),
        &[
            "discharge",
            "--binding",
            binding,
            "--repo",
            repo,
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--json",
        ],
    );
    assert!(partition.status.success(), "{partition:?}");
    let json: serde_json::Value = serde_json::from_str(&text(&partition)).unwrap();
    assert_eq!(json["binding"]["direct"][0]["clauseId"], "C-1");
}

#[trace("FR-133-AC-5")]
#[test]
fn tc_2604_a_doubled_binding_clause_is_refused_not_counted_twice() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut binding: serde_json::Value = serde_json::from_str(BINDING).unwrap();
    let first = binding["clauses"][0].clone();
    binding["clauses"].as_array_mut().unwrap().push(first);
    let path = dir.path().join("binding.json");
    std::fs::write(&path, binding.to_string()).unwrap();
    let output = quoin(
        home.path(),
        &[
            "discharge",
            "--binding",
            path.to_str().unwrap(),
            "--repo",
            dir.path().to_str().unwrap(),
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--inward",
        ],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate clause C-1"), "{stderr}");
}

#[trace("FR-133-AC-2", "FR-174-AC-6", "TC-004")]
#[test]
fn tc_2606_a_stored_temporary_exception_expires_at_its_decision_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let binding = dir.path().join("binding.json");
    let facts = dir.path().join("facts.json");
    std::fs::write(&binding, BINDING).unwrap();
    std::fs::write(&facts, TEMPORARY_EXCEPTION_FACTS).unwrap();
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
    let stored = files_under(dir.path())
        .into_iter()
        .find(|path| path.ends_with("c-1.json"))
        .expect("the temporary exception is persisted for C-1");
    let stored_fact = std::fs::read_to_string(stored).unwrap();
    assert!(
        stored_fact.contains("2026-08-20T00:00:00Z"),
        "{stored_fact}"
    );

    let render = |as_of: &str| {
        quoin(
            home.path(),
            &[
                "discharge",
                "--binding",
                binding,
                "--repo",
                repo,
                "--as-of",
                as_of,
                "--rmm",
            ],
        )
    };
    let before = render("2026-08-19T23:59:59.999Z");
    assert!(before.status.success(), "{before:?}");
    let before = text(&before);
    assert!(
        before.contains("| C-1 | Applicable | Tailored |"),
        "{before}"
    );

    for as_of in ["2026-08-20T00:00:00Z", "2026-08-20T00:00:00.001Z"] {
        let expired = render(as_of);
        assert!(expired.status.success(), "{expired:?}");
        let report = text(&expired);
        assert!(
            report.contains("| C-1 | Applicable | Not compliant - temporary exception expired"),
            "{report}"
        );
    }
}
