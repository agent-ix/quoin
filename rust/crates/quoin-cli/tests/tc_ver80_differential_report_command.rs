// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence record` over `filament.differential-report/v2` reports (FR-151,
//! TC-001): what the command prints, and what it refuses before it writes a run.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use filament_analyze::MAX_REPORT_BYTES;
use ix_trace_rs::trace;

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
    fn quoin(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command
            .current_dir(self.repo.path())
            .env("IX_HOME", self.home.path())
            .env(
                "GIT_CEILING_DIRECTORIES",
                self.repo.path().parent().unwrap(),
            );
        command
    }

    fn record(&self, results: &Path, extra: &[&str]) -> Output {
        self.quoin()
            .args(["evidence", "record", "--repo", ".", "--module", "."])
            .args(["--suite", "SUITE-SMT", "--tool", "filament-analyze 0.1.0"])
            .args(["--run-kind", "analysis"])
            .args(["--timestamp", "2026-01-01T00:00:00Z", "--results"])
            .arg(results)
            .args(extra)
            .output()
            .unwrap()
    }

    fn results(&self, bytes: &[u8]) -> std::path::PathBuf {
        let path = self.home.path().join("results.json");
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn matrix(&self, observation: &Path) -> Output {
        self.quoin()
            .args([
                "matrix",
                "--repo",
                ".",
                "--module",
                ".",
                "--json",
                "--run-observation",
            ])
            .arg(observation)
            .output()
            .unwrap()
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

fn evidence_tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut paths = BTreeMap::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                paths.insert(key, None);
                directories.push(path);
            } else {
                assert!(
                    kind.is_file(),
                    "unexpected evidence entry: {}",
                    path.display()
                );
                paths.insert(key, Some(std::fs::read(path).unwrap()));
            }
        }
    }
    paths
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
        text.contains("not transcribed — consistency") && text.contains("reported disagreement"),
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
    let output = fixture
        .quoin()
        .args([
            "evidence", "audit", "--repo", ".", "--module", ".", "--json",
        ])
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

#[trace("FR-153-AC-8")]
#[test]
fn tc_002_matrix_leaves_a_recorded_pass_unevaluated_without_owner_evidence() {
    const RUN: &str = "runs/SUITE-SMT/2026-01-01T00~3A00~3A00Z.json";
    const SELECTED: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const PERFORMED: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let fixture = fixture();
    let spec = SPEC.replace("|  |\n", "| Analysis |\n")
        + "| FR-001-AC-2 | The reader shall report the refused field. | Analysis |\n";
    std::fs::write(fixture.repo.path().join("spec/FR-001-example.md"), spec).unwrap();
    let results = fixture.results(SATISFIED.as_bytes());
    let output = fixture.record(&results, &["--discharges", "FR-001-AC-1", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let recorded: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(recorded["bound"], serde_json::json!(["FR-001-AC-1"]));
    assert!(!undischarged(&audit(&fixture), "FR-001-AC-1"));
    let store = fixture.repo.path().join("spec/evidence");
    let before = evidence_tree(&store);
    assert!(before.contains_key(Path::new(RUN)));
    assert!(before.contains_key(Path::new("bindings.json")));
    assert!(before.contains_key(Path::new(
        "analysis-reports/SUITE-SMT/2026-01-01T00~3A00~3A00Z.json"
    )));

    let observation = fixture.home.path().join("observation.json");
    std::fs::write(
        &observation,
        serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": RUN,
        })
        .to_string(),
    )
    .unwrap();
    let output = fixture.matrix(&observation);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let matrix: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let criteria = matrix["requirements"][0]["criteria"].as_array().unwrap();
    assert_eq!(criteria.len(), 2);
    assert_eq!(criteria[0]["id"], "FR-001-AC-1");
    let analysis = criteria[0]["analysis_evidence"].as_array().unwrap();
    assert!(!analysis.is_empty());
    assert!(
        analysis.iter().all(|item| item["status"] == "unevaluated"),
        "{analysis:?}"
    );
    assert!(analysis.iter().any(|item| {
        let result = &item["result"];
        result["locator"]["record"] == RUN
            && result["detail"].as_str().is_some_and(|detail| {
                detail == "bounded reliance premise: no FR-093 assessment of filament-analyze and the solvers exists for this positive use"
            })
    }), "{analysis:?}");

    assert_eq!(criteria[1]["id"], "FR-001-AC-2");
    let sibling = criteria[1]["analysis_evidence"].as_array().unwrap();
    assert_eq!(sibling.len(), 1, "{sibling:?}");
    assert_eq!(sibling[0]["status"], "absent");

    let mut observation_data = serde_json::json!({
        "protocol": "quoin.analysis-run-observation/v1",
        "record": RUN,
        "selected_head": SELECTED,
        "checkout_sha": PERFORMED
    });
    std::fs::write(&observation, observation_data.to_string()).unwrap();
    let output = fixture.matrix(&observation);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let matrix: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let analysis = matrix["requirements"][0]["criteria"][0]["analysis_evidence"]
        .as_array()
        .unwrap();
    assert!(
        analysis
            .iter()
            .all(|item| item["status"] != "qualified-record")
    );
    let detail = format!(
        "performed revision premise: performed revision {PERFORMED} is not the selected HEAD {SELECTED}"
    );
    assert!(
        analysis.iter().any(|item| {
            item["status"] == "wrong-revision"
                && item["result"]["locator"]["record"] == RUN
                && item["result"]["detail"] == detail
        }),
        "{analysis:?}"
    );

    observation_data["record"] = serde_json::json!("runs/OTHER-SUITE/other.json");
    std::fs::write(&observation, observation_data.to_string()).unwrap();
    let output = fixture.matrix(&observation);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains(
            "run observation names no readable selected record: runs/OTHER-SUITE/other.json"
        ),
        "{}",
        stderr(&output)
    );
    assert_eq!(evidence_tree(&store), before);
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
        stdout(&output).contains("not transcribed — consistency"),
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

/// The matrix `analysis_evidence` rows for `criterion` after `observation`.
fn analysis_rows(
    fixture: &Fixture,
    observation: &serde_json::Value,
    criterion: &str,
) -> Vec<serde_json::Value> {
    let path = fixture.home.path().join("swap-observation.json");
    std::fs::write(&path, observation.to_string()).unwrap();
    let output = fixture.matrix(&path);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let matrix: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    matrix["requirements"][0]["criteria"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == criterion)
        .unwrap()["analysis_evidence"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// Swap-and-refuse through the real command path (FR-116-AC-7, FR-116-AC-8).
///
/// Scope, stated honestly: this fixture cannot start from a qualifying record.
/// A positive decision needs an admitted actor, which the command only derives
/// from a verified signature (`actor_authority`) through a real verifier, and
/// no live positive exists (STD-717). So the baseline is already refused for
/// the missing reliance and actor premises. The test proves what it can fail on:
/// each swap (observed revision, stored projection bytes, recorded criterion)
/// must change the refusal to its own distinct status, which the baseline does
/// not have, and the refusals must not write the store. The positive-to-refused
/// swap is covered at the library level in `analysis_counting.rs`.
#[trace("FR-116-AC-7", "FR-116-AC-8", "FR-153-AC-8")]
#[test]
fn tc_plat_1790_swapped_inputs_refuse_through_the_matrix_command() {
    const RUN: &str = "runs/SUITE-SMT/2026-01-01T00~3A00~3A00Z.json";
    const PROJECTION: &str = "analysis-reports/SUITE-SMT/2026-01-01T00~3A00~3A00Z.json";
    const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let fixture = fixture();
    let spec = SPEC.replace("|  |\n", "| Analysis |\n")
        + "| FR-001-AC-2 | The reader shall report the refused field. | Analysis |\n";
    std::fs::write(fixture.repo.path().join("spec/FR-001-example.md"), spec).unwrap();
    let results = fixture.results(SATISFIED.as_bytes());
    let output = fixture.record(&results, &["--discharges", "FR-001-AC-1", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let store = fixture.repo.path().join("spec/evidence");
    let observation = serde_json::json!({
        "protocol": "quoin.analysis-run-observation/v1",
        "record": RUN,
        "selected_head": HEAD,
        "checkout_sha": HEAD,
    });
    let statuses = |rows: &[serde_json::Value]| -> Vec<String> {
        rows.iter()
            .map(|row| row["status"].as_str().unwrap().to_owned())
            .collect()
    };
    let details = |rows: &[serde_json::Value]| -> Vec<String> {
        rows.iter()
            .filter_map(|row| row["result"]["detail"].as_str().map(ToOwned::to_owned))
            .collect()
    };

    // Baseline: refused for the missing owner premises, with neither the
    // revision nor the conflict status the swaps below must introduce.
    let rows = analysis_rows(&fixture, &observation, "FR-001-AC-1");
    assert!(
        !statuses(&rows).contains(&"qualified-record".to_owned()),
        "{rows:?}"
    );
    assert!(
        details(&rows)
            .iter()
            .any(|d| d.starts_with("bounded reliance premise")),
        "{rows:?}"
    );
    assert!(
        details(&rows)
            .iter()
            .all(|d| !d.starts_with("performed revision premise")),
        "{rows:?}"
    );
    assert!(
        statuses(&rows)
            .iter()
            .all(|s| s != "wrong-revision" && s != "record-conflict"),
        "{rows:?}"
    );

    // Revision swapped: the performed revision differs from the selected HEAD.
    let mut swapped = observation.clone();
    swapped["checkout_sha"] = serde_json::json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let rows = analysis_rows(&fixture, &swapped, "FR-001-AC-1");
    assert!(
        statuses(&rows).contains(&"wrong-revision".to_owned()),
        "{rows:?}"
    );
    assert!(!statuses(&rows).contains(&"qualified-record".to_owned()));

    // Criterion swapped: the report discharges AC-1 only; AC-2 has no record.
    let rows = analysis_rows(&fixture, &observation, "FR-001-AC-2");
    assert_eq!(statuses(&rows), vec!["absent".to_owned()], "{rows:?}");

    // Report bytes swapped: the stored projection now claims a refuted verdict
    // while the run record still says pass.
    let projection_path = store.join(PROJECTION);
    let original = std::fs::read(&projection_path).unwrap();
    let mut projection: serde_json::Value = serde_json::from_slice(&original).unwrap();
    projection["agreed_status"] = serde_json::json!("refuted");
    std::fs::write(&projection_path, projection.to_string()).unwrap();
    // The store as the command will read it, tampering included.
    let tampered = evidence_tree(&store);
    let rows = analysis_rows(&fixture, &observation, "FR-001-AC-1");
    assert!(
        !statuses(&rows).contains(&"qualified-record".to_owned()),
        "{rows:?}"
    );
    assert!(
        statuses(&rows).contains(&"record-conflict".to_owned()),
        "{rows:?}"
    );
    assert!(
        details(&rows).iter().any(|d| d
            == "tool and configuration premise: the supplied projection does not match the retained run"),
        "{rows:?}"
    );
    assert_eq!(
        evidence_tree(&store),
        tampered,
        "the command never writes or repairs the store"
    );
    std::fs::write(&projection_path, original).unwrap();
}
