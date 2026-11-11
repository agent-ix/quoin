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

#[trace("FR-133-AC-7", "TC-2606")]
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
            "--recorded-at",
            "2026-08-02T00:00:00Z",
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
