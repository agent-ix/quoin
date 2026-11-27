// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-033 run adapters reach the evidence store through `quoin evidence record`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixtures may panic"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const MODULE: &str = r##"name: quoin-cli-fr033-record-fixture
version: 0.1.0
description: Synthetic module for adapter record tests
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

const SPEC: &str = "---\nid: FR-001\ntype: FR\ntitle: Adapter fixture\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | It works. | Test |\n";

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

    fn record(&self, adapter: Option<&str>, tool: &str, results: &str, suite: &str) -> Output {
        let results_path = self.home.path().join("results.txt");
        fs::write(&results_path, results).expect("adapter results");
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command.args([
            "evidence", "record", "--repo", ".", "--module", ".", "--suite", suite, "--tool", tool,
        ]);
        if let Some(adapter) = adapter {
            command.args(["--adapter", adapter]);
        }
        command
            .arg("--results")
            .arg(&results_path)
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

    fn stored_run(&self, output: &Output) -> Value {
        let payload: Value = serde_json::from_slice(&output.stdout).expect("JSON response");
        let run_path = Path::new(payload["run_path"].as_str().expect("run_path"));
        serde_json::from_slice(
            &fs::read(self.repo.path().join(run_path)).expect("run record persists"),
        )
        .expect("stored run is JSON")
    }
}

#[trace("FR-033-AC-11")]
#[test]
fn tc_1915_junit_record_keeps_the_extracted_symbol_and_trace_id() {
    let fixture = Fixture::new();
    let output = fixture.record(
        Some("junit"),
        "junit 1.0.0",
        r#"<testsuite><testcase classname="tests.adapter" name="test_acceptance"><properties><property name="trace" value="FR-001-AC-1"/></properties></testcase></testsuite>"#,
        "SUITE-JUNIT",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = fixture.stored_run(&output);
    assert_eq!(
        run["entries"][0]["symbol"],
        "tests::adapter::test_acceptance"
    );
    assert_eq!(
        run["entries"][0]["traceIds"],
        serde_json::json!(["FR-001-AC-1"])
    );
}

#[trace("FR-033-AC-12")]
#[test]
fn tc_1915_cargo_mutants_record_keeps_the_per_function_score() {
    let fixture = Fixture::new();
    let results = r#"{"outcomes":[{"scenario":{"Mutant":{"file":"src/lib.rs","function":{"function_name":"target"}}},"summary":"CaughtMutant"},{"scenario":{"Mutant":{"file":"src/lib.rs","function":{"function_name":"target"}}},"summary":"MissedMutant"}]}"#;
    let output = fixture.record(
        Some("cargo-mutants"),
        "cargo-mutants 25.1.0",
        results,
        "SUITE-MUTANTS",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = fixture.stored_run(&output);
    assert_eq!(run["entries"][0]["symbol"], "src/lib.rs::target");
    assert_eq!(run["entries"][0]["score"], 0.5);
}

#[trace("FR-033-AC-13")]
#[test]
fn tc_1915_tool_name_selects_the_run_adapter_when_adapter_flag_is_absent() {
    let fixture = Fixture::new();
    let results = r#"{"outcomes":[{"scenario":{"Mutant":{"file":"src/lib.rs","function":{"function_name":"target"}}},"summary":"CaughtMutant"},{"scenario":{"Mutant":{"file":"src/lib.rs","function":{"function_name":"target"}}},"summary":"MissedMutant"}]}"#;
    let output = fixture.record(None, "cargo-mutants 25.1.0", results, "SUITE-TOOL-SELECTED");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = fixture.stored_run(&output);
    assert_eq!(run["suite"], "SUITE-TOOL-SELECTED");
    assert_eq!(run["entries"][0]["symbol"], "src/lib.rs::target");
    assert_eq!(run["entries"][0]["score"], 0.5);
}

#[trace("FR-030-AC-17")]
#[test]
fn tc_1915_record_requires_an_exact_semantic_tool_version() {
    let fixture = Fixture::new();
    let output = fixture.record(
        Some("entries"),
        "quoin 1.0.0",
        r#"{"entries":[]}"#,
        "SUITE-VERSIONED",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.stored_run(&output)["tool"], "quoin 1.0.0");

    for invalid in ["quoin", "quoin latest", "quoin 1.0"] {
        let fixture = Fixture::new();
        let output = fixture.record(
            Some("entries"),
            invalid,
            r#"{"entries":[]}"#,
            "SUITE-INVALID",
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "accepted invalid --tool identity {invalid:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("exact semantic version"),
            "missing version diagnostic: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !fixture.repo.path().join("spec/evidence").exists(),
            "invalid tool identity wrote evidence"
        );
    }
}
