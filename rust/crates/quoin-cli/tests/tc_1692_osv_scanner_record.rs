// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! OSV-Scanner evidence recording goes through the finding-shaped scan store.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test assertions use panics as the failure report"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MODULE: &str = r##"name: quoin-cli-osv-record-fixture
version: 0.1.0
description: Fictional module for OSV recording contract tests
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
// Synthetic input: this test checks Quoin's storage family, not scanner output.
const RESULTS: &str = r#"{"results":[{"source":{"path":"synthetic.cdx.json"},"packages":[{"package":{"name":"left-pad","version":"1.3.0","purl":"pkg:npm/left-pad@1.3.0"},"vulnerabilities":[{"id":"GHSA-synthetic"}],"groups":[]}]}]}"#;

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
        let results = home.path().join("osv-results.json");
        std::fs::write(&results, RESULTS).unwrap();
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
                "SUITE-OSV",
                "--tool",
                "osv-scanner 2.7.1",
                "--results",
                self.results.to_str().unwrap(),
                "--timestamp",
                "2026-11-13T00:00:00Z",
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

/// The command persists an OSV finding-shaped record under `scans/`, never `runs/`.
#[trace("FR-149-AC-1")]
#[test]
fn tc_1692_001_osv_scanner_record_persists_only_a_scan_record() {
    let fixture = Fixture::new();
    let output = fixture.record();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["kind"], "scan");
    let scan_path = Path::new(payload["scan_path"].as_str().unwrap());
    let evidence = fixture.repo.path().join("spec/evidence");
    assert!(
        scan_path.starts_with("./spec/evidence/scans/SUITE-OSV/"),
        "scan path {scan_path:?} is outside the scan family"
    );
    let persisted = fixture.repo.path().join(scan_path);
    assert!(persisted.is_file());
    let stored: Value = serde_json::from_slice(&std::fs::read(&persisted).unwrap()).unwrap();
    assert_eq!(stored["tool"], "osv-scanner 2.7.1");
    assert_eq!(stored["findings"][0]["ruleId"], "GHSA-synthetic");
    assert_eq!(stored["findings"][0]["path"], "pkg:npm/left-pad@1.3.0");
    assert!(!evidence.join("runs").exists());
    assert!(!evidence.join("runs/SUITE-OSV").exists());
}
