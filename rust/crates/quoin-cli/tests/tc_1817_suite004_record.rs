// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! SUITE-004 retains tool identity, exact inputs and unavailable-tool state.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test assertions use panics as the failure report"
)]

use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::fs;
use std::process::Command;

const MODULE: &str = r"name: wave2-suite004-fixture
version: 0.1.0
description: Fixture for the retained SUITE-004 run test
artifact_types:
  - name: FR
    grammar_ref: iso-spec-core
    defaults:
      id_pattern: FR-{next:03d}
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
";

const SPEC: &str = "---\nid: FR-001\ntype: FR\ntitle: Fixture\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|---|---|---|\n| FR-001-AC-1 | The retained tool evidence is complete. | Test |\n";

#[trace("FR-030-AC-42", "FR-030-AC-28")]
#[test]
fn tc_1817_suite004_retains_tool_inputs_and_unavailable_state() {
    let repo = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(repo.path().join("manifest.yaml"), MODULE).unwrap();
    fs::create_dir_all(repo.path().join("spec")).unwrap();
    fs::write(repo.path().join("spec/FR-001-example.md"), SPEC).unwrap();
    let input = home.path().join("results.json");
    let results = json!({
        "entries": [
            {
                "symbol": "osv-authentic-report",
                "outcome": "pass",
                "traceIds": ["FR-001-AC-1"],
                "config": {
                    "tool": "osv-scanner",
                    "tool_version": "osv-scanner 2.1.0",
                    "input": "reports/osv-report.json",
                    "osv_mirror_date": "2026-11-23",
                    "execution_state": "executed"
                }
            },
            {
                "symbol": "engineering-assurance-cli",
                "outcome": "error",
                "traceIds": ["FR-001-AC-1"],
                "config": {
                    "tool": "engineering-assurance",
                    "tool_version": "unavailable",
                    "input": "throwaway signed repositories",
                    "execution_state": "not-executed",
                    "reason": "required executable is unavailable"
                }
            }
        ]
    });
    fs::write(&input, serde_json::to_vec_pretty(&results).unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence",
            "record",
            "--repo",
            repo.path().to_str().unwrap(),
            "--module",
            ".",
            "--suite",
            "SUITE-004",
            "--kind",
            "Integration",
            "--adapter",
            "entries",
            "--tool",
            "quoin-real-tool-test 1.0.0",
            "--results",
            input.to_str().unwrap(),
            "--json",
        ])
        .current_dir(repo.path())
        .env("IX_HOME", home.path())
        .env("GIT_CEILING_DIRECTORIES", repo.path().parent().unwrap())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let retained_path = repo.path().join(response["run_path"].as_str().unwrap());
    let retained: Value = serde_json::from_slice(&fs::read(retained_path).unwrap()).unwrap();

    assert_eq!(retained["suite"], "SUITE-004");
    assert_eq!(retained["tool"], "quoin-real-tool-test 1.0.0");
    let timestamp = retained["timestamp"].as_str().unwrap();
    assert!(
        quoin_evidence::parse_timestamp(timestamp).is_some(),
        "generated timestamp must be canonical: {timestamp}"
    );
    let entries = retained["entries"].as_array().unwrap();
    let osv = entries
        .iter()
        .find(|entry| entry["symbol"] == "osv-authentic-report")
        .unwrap();
    assert_eq!(osv["outcome"], "pass");
    assert_eq!(osv["config"]["tool_version"], "osv-scanner 2.1.0");
    assert_eq!(osv["config"]["input"], "reports/osv-report.json");
    assert_eq!(osv["config"]["osv_mirror_date"], "2026-11-23");

    let unavailable = entries
        .iter()
        .find(|entry| entry["symbol"] == "engineering-assurance-cli")
        .unwrap();
    assert_eq!(unavailable["outcome"], "error");
    assert_eq!(unavailable["config"]["tool_version"], "unavailable");
    assert_eq!(unavailable["config"]["execution_state"], "not-executed");
}
