// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The public evidence-record command persists clean SARIF scans distinctly.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test assertions use panics as the failure report"
)]

use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MODULE: &str = r##"name: quoin-cli-fr034-sarif-fixture
version: 0.1.0
description: Fixture for FR-034 SARIF recording
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

const SPEC: &str = "---\nid: FR-001\ntype: FR\ntitle: One\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | It works. | Test |\n";
const CLEAN_SARIF: &str = r#"{
  "version": "2.1.0",
  "runs": [{
    "tool": {"driver": {"name": "semgrep", "rules": [{"id": "S001"}]}},
    "results": []
  }]
}"#;

struct Fixture {
    repo: tempfile::TempDir,
    home: tempfile::TempDir,
    results: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let repo = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        std::fs::write(repo.path().join("manifest.yaml"), MODULE).unwrap();
        std::fs::create_dir_all(repo.path().join("spec")).unwrap();
        std::fs::write(repo.path().join("spec/FR-001-example.md"), SPEC).unwrap();
        let results = home.path().join("clean.sarif");
        std::fs::write(&results, CLEAN_SARIF).unwrap();
        Self {
            repo,
            home,
            results,
        }
    }

    fn record(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args([
                "evidence",
                "record",
                "--repo",
                ".",
                "--module",
                ".",
                "--suite",
                "SUITE-SARIF",
                "--tool",
                "semgrep 1.2.3",
                "--adapter",
                "sarif",
                "--results",
                self.results.to_str().unwrap(),
                "--timestamp",
                "2026-11-26T00:00:00Z",
                "--json",
            ])
            .current_dir(self.repo.path())
            .env("IX_HOME", self.home.path())
            .env(
                "GIT_CEILING_DIRECTORIES",
                self.repo.path().parent().unwrap(),
            )
            .output()
            .unwrap()
    }
}

/// The explicit SARIF route records an executed clean scan under scans only,
/// retaining the empty finding list and the rule count from the SARIF driver.
#[trace("FR-034-AC-13", "FR-034-AC-14")]
#[test]
fn tc_fr034_clean_sarif_records_scan_with_rule_count_and_no_run() {
    let fixture = Fixture::new();
    let output = fixture.record();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["kind"], "scan");
    assert_eq!(payload["findings"], json!([]));
    let scan_path = Path::new(payload["scan_path"].as_str().unwrap());
    assert!(
        scan_path.starts_with("./spec/evidence/scans/SUITE-SARIF/"),
        "scan path {scan_path:?} is outside the scans family"
    );
    let stored: Value =
        serde_json::from_slice(&std::fs::read(fixture.repo.path().join(scan_path)).unwrap())
            .unwrap();
    assert_eq!(stored["findings"], json!([]));
    assert_eq!(stored["rulesEvaluated"], 1);
    assert!(!fixture.repo.path().join("spec/evidence/runs").exists());
}
