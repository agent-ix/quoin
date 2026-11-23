// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence record` exits 1 with a `warning:` stderr line, after
//! publishing, and `--json` stdout still lists the unmatched ids.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test assertions use panics as the failure report"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::process::Command;

const MODULE: &str = r##"name: quoin-cli-record-exit-fixture
version: 0.1.0
description: Fictional module for record exit status tests
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
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: "#[trace({ids})]"
"##;
const SPEC: &str = "---\nid: FR-001\ntype: FR\ntitle: One\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | It works. |  |\n";
const RESULTS: &str =
    r#"{"entries":[{"symbol":"a","outcome":"pass","traceIds":["FR-001-AC-1","ZZ-1"]}]}"#;

/// The unmatched id is reported on stderr, stdout stays the JSON payload, and
/// the run record and the matched binding are on disk when the status is 1.
#[trace("FR-030-AC-32")]
#[test]
fn tc_1806_cli_unmatched_id_exits_1_with_a_warning_line() {
    let repo = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("manifest.yaml"), MODULE).unwrap();
    std::fs::create_dir_all(repo.path().join("spec")).unwrap();
    std::fs::write(repo.path().join("spec/FR-001-example.md"), SPEC).unwrap();
    let results = home.path().join("results.json");
    std::fs::write(&results, RESULTS).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence",
            "record",
            "--repo",
            ".",
            "--module",
            ".",
            "--suite",
            "SUITE-1",
            "--tool",
            "custom",
            "--results",
            results.to_str().unwrap(),
            "--timestamp",
            "2026-11-13T00:00:00Z",
            "--json",
        ])
        .current_dir(repo.path())
        .env("IX_HOME", home.path())
        .env("GIT_CEILING_DIRECTORIES", repo.path().parent().unwrap())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "warning: unmatched trace ids: ZZ-1\n"
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["unmatched"], serde_json::json!(["ZZ-1"]));
    assert!(repo.path().join("spec/evidence/bindings.json").is_file());
    assert!(repo.path().join("spec/evidence/runs/SUITE-1").is_dir());
}
