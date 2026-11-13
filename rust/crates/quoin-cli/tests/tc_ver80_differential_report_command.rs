// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence record` over `quire.differential-report/v2` reports (FR-151,
//! TC-001): what the command prints, and what it refuses before it writes a run.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::Path;
use std::process::{Command, Output};

use ix_trace_rs::trace;
use quire_analyze::MAX_REPORT_BYTES;

const DISAGREEMENT: &str =
    include_str!("../../../../tests/fixtures/evidence/differential-report-v2/disagreement.json");
const SATISFIED: &str = include_str!(
    "../../../../tests/fixtures/evidence/differential-report-v2/agreement-satisfied.json"
);

const MODULE: &str = r"name: quoin-cli-differential-fixture
version: 0.1.0
description: Fixture module for the differential-report command tests
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

const SPEC: &str = "---
id: FR-001
type: FR
title: One criterion
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | The reader shall refuse a payload over its ceiling. |  |
";

struct Fixture {
    repo: tempfile::TempDir,
    home: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let fixture = Fixture {
        repo: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    };
    std::fs::write(fixture.repo.path().join("manifest.yaml"), MODULE).unwrap();
    std::fs::create_dir_all(fixture.repo.path().join("spec")).unwrap();
    std::fs::write(fixture.repo.path().join("spec/FR-001-example.md"), SPEC).unwrap();
    fixture
}

impl Fixture {
    fn record(&self, results: &Path, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(["evidence", "record", "--repo", ".", "--module", "."])
            .args(["--suite", "SUITE-SMT", "--tool", "quire-analyze 0.1.0"])
            .args(["--timestamp", "2026-01-01T00:00:00Z", "--results"])
            .arg(results)
            .args(extra)
            .current_dir(self.repo.path())
            .env("IX_HOME", self.home.path())
            .env(
                "GIT_CEILING_DIRECTORIES",
                self.repo.path().parent().unwrap(),
            )
            .output()
            .unwrap()
    }

    fn results(&self, bytes: &[u8]) -> std::path::PathBuf {
        let path = self.home.path().join("results.json");
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn runs_exist(&self) -> bool {
        self.repo.path().join("spec/evidence/runs").exists()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[trace("FR-151-AC-3", "FR-069-AC-6")]
#[test]
fn tc_001_the_command_prints_the_unrepresented_result_in_human_output() {
    let fixture = fixture();
    let results = fixture.results(DISAGREEMENT.as_bytes());
    let output = fixture.record(&results, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("recorded SUITE-SMT"), "{text}");
    assert!(
        text.contains("not transcribed — consistency::") && text.contains("reported disagreement"),
        "{text}"
    );
    assert!(
        text.contains("neither a pass nor a fail nor an error"),
        "{text}"
    );

    let json = fixture.record(&results, &["--json"]);
    assert!(
        stdout(&json).contains("\"unrepresented\""),
        "{}",
        stdout(&json)
    );
}

#[trace("FR-151-AC-3")]
#[test]
fn tc_001_a_decided_report_prints_no_unrepresented_line() {
    let fixture = fixture();
    let output = fixture.record(&fixture.results(SATISFIED.as_bytes()), &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        !stdout(&output).contains("not transcribed"),
        "{}",
        stdout(&output)
    );
}

#[trace("FR-151-AC-8", "FR-151-AC-14")]
#[test]
fn tc_001_non_utf8_results_are_refused_and_write_no_run() {
    let fixture = fixture();
    let output = fixture.record(&fixture.results(b"\xff\xfe"), &[]);
    assert_ne!(output.status.code(), Some(0));
    assert!(
        format!("{}{}", stdout(&output), stderr(&output)).contains("not UTF-8"),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(!fixture.runs_exist());
}

fn padded(length: usize) -> Vec<u8> {
    vec![b' '; length]
}

#[trace("FR-151-AC-10", "NFR-032")]
#[test]
fn tc_001_the_command_applies_the_report_limit_not_a_lower_request_limit() {
    let fixture = fixture();
    // Not JSON at all, so a refusal that is not about size is the pass signal.
    let at = fixture.record(&fixture.results(&padded(MAX_REPORT_BYTES)), &[]);
    let at_text = format!("{}{}", stdout(&at), stderr(&at));
    assert_ne!(at.status.code(), Some(0), "{at_text}");
    assert!(
        !at_text.contains("byte limit")
            && !at_text.contains("larger than")
            && !at_text.contains("exceeds the accepted size"),
        "{at_text}"
    );

    let over = fixture.record(&fixture.results(&padded(MAX_REPORT_BYTES + 1)), &[]);
    let over_text = format!("{}{}", stdout(&over), stderr(&over));
    assert!(
        over_text.contains(&format!("over the {MAX_REPORT_BYTES}-byte limit")),
        "{over_text}"
    );
    assert!(!fixture.runs_exist());
}

fn audit(fixture: &Fixture) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence", "audit", "--repo", ".", "--module", ".", "--json",
        ])
        .current_dir(fixture.repo.path())
        .env("IX_HOME", fixture.home.path())
        .env(
            "GIT_CEILING_DIRECTORIES",
            fixture.repo.path().parent().unwrap(),
        )
        .output()
        .unwrap();
    serde_json::from_str(&stdout(&output)).unwrap()
}

fn undischarged(audit: &serde_json::Value, obligation: &str) -> bool {
    audit["findings"].as_array().is_some_and(|findings| {
        findings
            .iter()
            .any(|f| f["kind"] == "undischarged" && f["obligation"] == obligation)
    })
}

#[trace("FR-152-AC-2")]
#[test]
fn tc_002_audit_reports_the_obligation_undischarged_until_it_is_stated() {
    let fixture = fixture();
    let results = fixture.results(SATISFIED.as_bytes());
    let output = fixture.record(&results, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let before = audit(&fixture);
    assert!(undischarged(&before, "FR-001-AC-1"), "{before}");

    let stated = self::fixture();
    let results = stated.results(SATISFIED.as_bytes());
    let output = stated.record(&results, &["--discharges", "FR-001-AC-1"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let after = audit(&stated);
    assert!(!undischarged(&after, "FR-001-AC-1"), "{after}");
}

#[trace("FR-152-AC-6")]
#[test]
fn tc_002_the_comma_separated_form_binds_both_and_reports_the_unknown_id() {
    let fixture = fixture();
    let results = fixture.results(SATISFIED.as_bytes());
    let output = fixture.record(&results, &["--discharges", "FR-001-AC-1,NOPE-1", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let payload: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(payload["bound"], serde_json::json!(["FR-001-AC-1"]));
    assert_eq!(payload["unmatched"], serde_json::json!(["NOPE-1"]));
}

#[trace("FR-152-AC-5")]
#[test]
fn tc_002_a_stated_disagreement_prints_the_unrepresented_result_in_human_output() {
    let fixture = fixture();
    let results = fixture.results(DISAGREEMENT.as_bytes());
    let output = fixture.record(&results, &["--discharges", "FR-001-AC-1"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("not transcribed — consistency::"),
        "{}",
        stdout(&output)
    );
}

#[trace("FR-152-AC-7")]
#[test]
fn tc_002_a_refused_report_with_stated_ids_exits_non_zero_and_writes_no_run() {
    let fixture = fixture();
    for bytes in [
        padded(MAX_REPORT_BYTES + 1),
        b"\xff\xfe".to_vec(),
        SATISFIED
            .replace("differential-report/v2", "differential-report/v1")
            .into_bytes(),
    ] {
        let output = fixture.record(&fixture.results(&bytes), &["--discharges", "FR-001-AC-1"]);
        assert_ne!(output.status.code(), Some(0));
        assert!(!fixture.runs_exist());
    }
}
