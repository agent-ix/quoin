// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run recording requires an immutable semantic-versioned tool identity (FR-030).

#![allow(
    clippy::expect_used,
    reason = "test fixtures deliberately panic to report a failed command contract"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const MODULE: &str = r##"name: quoin-cli-fr030-tool-fixture
version: 0.1.0
description: Minimal module for tool identity recording
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

const SPEC: &str = "---\nid: FR-001\ntype: FR\ntitle: Tool identity fixture\n---\n\n\
## Acceptance Criteria\n\n\
| ID | Criteria | Verification |\n\
|----|----------|--------------|\n\
| FR-001-AC-1 | The test result is retained. | Test |\n";

struct Fixture {
    repo: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let repo = tempfile::tempdir().expect("repository fixture");
        let home = tempfile::tempdir().expect("isolated config root");
        fs::write(repo.path().join("manifest.yaml"), MODULE).expect("module manifest");
        fs::create_dir_all(repo.path().join("spec")).expect("spec directory");
        fs::write(repo.path().join("spec/FR-001.md"), SPEC).expect("spec document");
        Self { repo, home }
    }

    fn record(&self, tool: &str) -> Output {
        let results = self.home.path().join("results.json");
        fs::write(
            &results,
            r#"{"entries":[{"symbol":"tests::tc_one","outcome":"pass","traceIds":["FR-001-AC-1"]}]}"#,
        )
        .expect("adapter results");
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args([
                "evidence",
                "record",
                "--repo",
                ".",
                "--module",
                ".",
                "--suite",
                "SUITE-FR030",
                "--tool",
                tool,
                "--adapter",
                "entries",
                "--results",
            ])
            .arg(&results)
            .args(["--timestamp", "2026-11-13T00:00:00Z", "--json"])
            .current_dir(self.repo.path())
            .env("IX_HOME", self.home.path())
            .env(
                "GIT_CEILING_DIRECTORIES",
                self.repo.path().parent().expect("fixture parent"),
            )
            .output()
            .expect("quoin evidence record runs")
    }

    fn stored_tool(&self, output: &Output) -> String {
        let response: Value = serde_json::from_slice(&output.stdout).expect("JSON response");
        let run_path = Path::new(response["run_path"].as_str().expect("run path"));
        let run: Value = serde_json::from_slice(
            &fs::read(self.repo.path().join(run_path)).expect("stored run exists"),
        )
        .expect("run record is JSON");
        run["tool"].as_str().expect("tool is retained").to_owned()
    }
}

#[trace("FR-030-AC-17")]
#[test]
fn tc_fr030_record_rejects_bare_or_mutable_tool_identities() {
    let fixture = Fixture::new();
    for tool in [
        "vitest",
        "vitest latest",
        "vitest main",
        "vitest 3.2",
        "vitest v3.2.4",
    ] {
        let output = fixture.record(tool);
        assert!(
            !output.status.success(),
            "mutable or inexact identity {tool:?} was accepted: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[trace("FR-030-AC-17")]
#[test]
fn tc_fr030_record_preserves_an_exact_semantic_tool_version() {
    let fixture = Fixture::new();
    let output = fixture.record("vitest 3.2.4");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.stored_tool(&output), "vitest 3.2.4");
}
