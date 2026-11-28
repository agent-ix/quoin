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
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use filament_analyze::{
    MAX_REPORT_BYTES, RunPackageCommand, parse_run_package_arguments, run_package,
};
use filament_contract_model::{
    CheckedNodeId, CheckedNodeTag, CheckedPackageEvidence, CheckedPackageReadLimits,
    CheckedPackageV2, CheckedPackageV2ReadResult, CompleteLoweringProfileV2, LoweringProfileRef,
};
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

    fn record_with_run_kind(
        &self,
        results: &Path,
        run_kind: Option<&str>,
        formal_rule_selection: Option<&Path>,
    ) -> Output {
        let mut command = self.quoin();
        command.args([
            "evidence",
            "record",
            "--repo",
            ".",
            "--module",
            ".",
            "--suite",
            "SUITE-SMT",
            "--tool",
            "filament-analyze 0.1.0",
            "--timestamp",
            "2026-01-01T00:00:00Z",
            "--results",
        ]);
        command.arg(results);
        if let Some(run_kind) = run_kind {
            command.args(["--run-kind", run_kind]);
        }
        if let Some(selection) = formal_rule_selection {
            command.arg("--formal-rule-selection").arg(selection);
        }
        command.output().unwrap()
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

    fn report_projection_exists(&self) -> bool {
        self.repo
            .path()
            .join("spec/evidence/analysis-reports")
            .exists()
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

#[trace("FR-151-AC-18")]
#[test]
fn tc_001_new_runs_require_a_known_kind_and_legacy_analysis_projection_defaults_to_analysis() {
    let fixture = fixture();
    let results = fixture.results(SATISFIED.as_bytes());

    let missing = fixture.record_with_run_kind(&results, None, None);
    assert_ne!(missing.status.code(), Some(0), "{}", stdout(&missing));
    assert!(!fixture.runs_exist());
    assert!(!fixture.report_projection_exists());

    let unknown = fixture.record_with_run_kind(&results, Some("unknown"), None);
    assert_ne!(unknown.status.code(), Some(0), "{}", stderr(&unknown));
    assert!(!fixture.runs_exist());
    assert!(!fixture.report_projection_exists());

    let formal_selection = fixture.home.path().join("formal-rule-selection.json");
    std::fs::write(&formal_selection, "{}").unwrap();
    let unexpected_selection =
        fixture.record_with_run_kind(&results, Some("analysis"), Some(&formal_selection));
    assert_ne!(unexpected_selection.status.code(), Some(0));
    assert!(!fixture.runs_exist());
    assert!(!fixture.report_projection_exists());

    let spec = SPEC.replace("|  |\n", "| Analysis |\n");
    std::fs::write(fixture.repo.path().join("spec/FR-001-example.md"), spec).unwrap();
    let recorded = fixture.record(&results, &["--discharges", "FR-001-AC-1", "--json"]);
    assert_eq!(recorded.status.code(), Some(0), "{}", stderr(&recorded));
    let recorded: serde_json::Value = serde_json::from_slice(&recorded.stdout).unwrap();
    let projection_path = fixture
        .repo
        .path()
        .join("spec/evidence/analysis-reports/SUITE-SMT/2026-01-01T00~3A00~3A00Z.json");
    let projection: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&projection_path).unwrap()).unwrap();
    assert_eq!(projection["run_kind"], "analysis");

    let observation = fixture.home.path().join("legacy-observation.json");
    let revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let record_path = recorded["run_path"]
        .as_str()
        .unwrap()
        .strip_prefix("./spec/evidence/")
        .unwrap();
    std::fs::write(
        &observation,
        serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": record_path,
            "run_kind": "analysis",
            "selected_head": revision,
            "checkout_sha": revision,
        })
        .to_string(),
    )
    .unwrap();
    let explicit = fixture.matrix(&observation);
    assert_eq!(explicit.status.code(), Some(0), "{}", stderr(&explicit));
    let explicit: serde_json::Value = serde_json::from_slice(&explicit.stdout).unwrap();
    let explicit_evidence = explicit["requirements"][0]["criteria"][0]["analysis_evidence"]
        .as_array()
        .unwrap();
    assert!(!explicit_evidence.is_empty(), "{explicit}");

    let mut projection: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&projection_path).unwrap()).unwrap();
    projection.as_object_mut().unwrap().remove("run_kind");
    std::fs::write(&projection_path, serde_json::to_vec(&projection).unwrap()).unwrap();
    let legacy = fixture.matrix(&observation);
    assert_eq!(legacy.status.code(), Some(0), "{}", stderr(&legacy));
    let legacy: serde_json::Value = serde_json::from_slice(&legacy.stdout).unwrap();
    let legacy_evidence = legacy["requirements"][0]["criteria"][0]["analysis_evidence"]
        .as_array()
        .unwrap();
    assert_eq!(legacy_evidence, explicit_evidence);
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

#[trace("FR-151-AC-1", "FR-184-AC-26")]
#[test]
fn a_runner_report_is_published_and_reaches_the_qualified_record_consumer()
-> Result<(), String> {
    let Some(producer_execute) =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|directory| directory.join("engineering-assurance"))
            .find(|path| path.is_absolute() && path.is_file())
    else {
        return Err("engineering-assurance producer-execute must be installed on PATH".to_owned());
    };
    let package_fixture = include_bytes!("fixtures/formal-rules/checked-package-v2-success.json");
    let mut evidence = CheckedPackageEvidence::default();
    evidence.support_feature("filament.value.complete/v1");
    let checked = match CheckedPackageV2::read(
        package_fixture,
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => return Err(format!("checked package fixture was refused: {other:?}")),
    };
    let selected_claim: CheckedNodeId = serde_json::from_value(serde_json::json!({
        "package": "test/formal-rules",
        "path": ["claim/approved"],
        "ordinal": 0
    }))
    .unwrap();
    let lowered = checked.lower(
        &[selected_claim],
        &CompleteLoweringProfileV2 {
            reference: LoweringProfileRef {
                authority: "filament-analyze".into(),
                identity: "quoin-fr184-ac26".into(),
            },
            required_source_selections: Vec::new(),
            supported_tags: CheckedNodeTag::ALL.iter().copied().collect(),
            require_bounds: true,
            work_limit: u64::MAX,
        },
    );
    let fixture = fixture();
    let spec = SPEC.replace("|  |\n", "| Analysis |\n");
    std::fs::write(fixture.repo.path().join("spec/FR-001-example.md"), spec).unwrap();
    let package_path = fixture.home.path().join("contract-package.json");
    let generated_path = fixture.home.path().join("generated-report.json");
    let report_path = fixture.home.path().join("published-report.json");
    std::fs::write(&package_path, lowered.package.canonical_bytes().unwrap()).unwrap();
    let arguments = [
        "--package",
        package_path.to_str().unwrap(),
        "--output",
        generated_path.to_str().unwrap(),
        "--z3",
        "/usr/bin/false",
        "--cvc5",
        "/usr/bin/false",
        "--producer-execute",
        producer_execute.to_str().unwrap(),
    ];
    let command = parse_run_package_arguments(arguments.into_iter().map(OsString::from))
        .map_err(|error| error.to_string())?;
    let RunPackageCommand::Run(arguments) = command else {
        return Err("package runner unexpectedly returned help".to_owned());
    };
    run_package(&arguments).map_err(|error| error.to_string())?;
    assert!(generated_path.is_file(), "package runner did not write a report");

    let Some(binary) = std::env::var_os("FILAMENT_ANALYZE_BIN")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_file())
    else {
        return Err("FILAMENT_ANALYZE_BIN must name the real executable absolute path".to_owned());
    };
    let published = Command::new(binary)
        .current_dir(fixture.repo.path())
        .args([
            "publish-report",
            "--input",
            generated_path.to_str().unwrap(),
            "--output",
            report_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        published.status.success() || published.status.code() == Some(3),
        "{}",
        stderr(&published)
    );
    assert!(report_path.is_file(), "{}", stderr(&published));
    assert_eq!(
        std::fs::read(&report_path).unwrap(),
        std::fs::read(&generated_path).unwrap()
    );
    let actual_report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report_path).unwrap()).unwrap();
    assert_eq!(actual_report["schema"], "filament.differential-report/v2");
    assert_eq!(actual_report["producer"]["name"], "filament-analyze");

    let recorded = fixture.record(&report_path, &["--discharges", "FR-001-AC-1", "--json"]);
    assert_eq!(recorded.status.code(), Some(0), "{}", stderr(&recorded));
    let record: serde_json::Value = serde_json::from_slice(&recorded.stdout).unwrap();
    let run = assert_retained_run_projection(&fixture, &actual_report, &record);

    assert_real_matrix_consumption(&fixture, &actual_report, run)?;
    Ok(())
}

fn assert_real_matrix_consumption(
    fixture: &Fixture,
    actual_report: &serde_json::Value,
    run: &str,
) -> Result<(), String> {
    let observation_path = fixture.home.path().join("run-observation.json");
    let same_revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    std::fs::write(
        &observation_path,
        serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": run,
            "run_kind": "analysis",
            "selected_head": same_revision,
            "checkout_sha": same_revision,
        })
        .to_string(),
    )
    .unwrap();
    let matrix = fixture.matrix(&observation_path);
    assert_eq!(matrix.status.code(), Some(0), "{}", stderr(&matrix));
    let matrix: serde_json::Value = serde_json::from_slice(&matrix.stdout).unwrap();
    let rows = matrix["requirements"][0]["criteria"][0]["analysis_evidence"]
        .as_array()
        .unwrap();
    let expected = match (
        actual_report["differential"]["disposition"].as_str(),
        actual_report["differential"]["agreedStatus"].as_str(),
    ) {
        (Some("agreement"), Some("satisfied")) => "unevaluated",
        (Some("agreement"), Some("refuted")) => "failed",
        (Some("inconclusive" | "unavailable"), _) => "errored",
        (Some("disagreement"), _) => "absent",
        other => return Err(format!("unexpected real analyzer disposition: {other:?}")),
    };
    assert!(rows.iter().any(|row| row["status"] == expected), "{rows:?}");
    assert!(
        rows.iter().all(|row| row["status"] != "qualified-record"),
        "the real report has no admitted FR-116 owner premises: {rows:?}"
    );
    Ok(())
}

fn assert_retained_run_projection(
    fixture: &Fixture,
    actual_report: &serde_json::Value,
    record: &serde_json::Value,
) -> &'static str {
    const RUN: &str = "runs/SUITE-SMT/2026-01-01T00~3A00~3A00Z.json";
    let recorded_run = Path::new(record["run_path"].as_str().unwrap());
    assert!(recorded_run.ends_with(RUN), "{}", recorded_run.display());
    let retained_path = fixture.repo.path().join("spec/evidence").join(RUN);
    let retained: serde_json::Value =
        serde_json::from_slice(&std::fs::read(retained_path).unwrap()).unwrap();
    assert_eq!(retained["suite"], "SUITE-SMT");
    assert_eq!(retained["timestamp"], "2026-01-01T00:00:00Z");
    let projection_path = fixture
        .repo
        .path()
        .join("spec/evidence/analysis-reports")
        .join("SUITE-SMT")
        .join(std::path::Path::new(RUN).file_name().unwrap());
    let projection: serde_json::Value =
        serde_json::from_slice(&std::fs::read(projection_path).unwrap()).unwrap();
    assert_eq!(projection["producer"], actual_report["producer"]);
    for (projected_engine, actual_engine) in projection["engines"]
        .as_array()
        .unwrap()
        .iter()
        .zip(actual_report["engines"].as_array().unwrap())
    {
        assert_eq!(projected_engine["engine"], actual_engine["engine"]);
        assert_eq!(
            projected_engine["configured_executable"],
            actual_engine["configuredExecutable"]
        );
        assert_eq!(projected_engine["status"], actual_engine["status"]);
        assert_eq!(projected_engine["outcome"], actual_engine["outcome"]);
    }
    assert_eq!(
        projection["disposition"],
        actual_report["differential"]["disposition"]
    );

    RUN
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
    assert_ne!(analysis.as_slice(), [] as [serde_json::Value; 0]);
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
    assert_eq!(output.stdout, [] as [u8; 0]);
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
    // The unknown id makes the recording exit 1; the payload is still complete.
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
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
