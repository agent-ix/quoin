// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Command-level validation-report acceptance tests.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test assertions report command-contract failures"
)]

use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn write_run(root: &Path, status: &str, include_missing_tool: bool) -> std::path::PathBuf {
    let path = root.join("quoin-run.json");
    let documents = vec![
        json!({
            "path": "IU-001.md",
            "raw": "---\nid: IU-001\ntype: IntendedUse\ntool: quoin\n---\nPurpose: Inspect Quoin output.\n",
        }),
        json!({
            "path": "FRR-001.md",
            "raw": "---\nid: FRR-001\ntype: FeatureRiskRating\nscope: supplier\nrating: high_process_risk\nrated_by: quality\nintended_use: IU-001\nrelationships:\n  - target: ix://agent-ix/quoin/FR-001\n    type: rates_feature\n---\n",
        }),
    ];
    let intended_use = if include_missing_tool {
        vec![
            documents[0].clone(),
            documents[1].clone(),
            json!({
                "path": "IU-002.md",
                "raw": "---\nid: IU-002\ntype: IntendedUse\ntool: quire-cli\n---\nPurpose: Inspect Quire CLI output.\n",
            }),
        ]
    } else {
        documents
    };
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({
            "schemaVersion": 1,
            "tool": "quoin",
            "toolVersion": "quoin test",
            "os": "linux",
            "architecture": "x86_64",
            "suite": "SELFTEST",
            "timestamp": "2026-11-25T12:00:00Z",
            "results": [{
                "id": "fixture-case",
                "status": status,
                "criteria": ["FR-001-AC-1"],
                "detail": if status == "pass" { "" } else { "fixture failure" },
                "stderr": "",
            }],
            "validationInputs": {"provided": true, "documents": intended_use},
            "anomalySet": {
                "provided": true,
                "producerVersion": "quoin test",
                "documents": [],
            },
            "externalTools": [],
            "retainedLocation": "spec/evidence/runs/SELFTEST/run.json",
        }))
        .expect("run record serializes"),
    )
    .expect("run record written");
    path
}

fn invoke(result: &Path, output: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["validation", "report", "--results"])
        .arg(result)
        .arg("--out")
        .arg(output)
        .output()
        .expect("validation report command starts")
}

#[trace("FR-027-AC-13", "FR-217-AC-1", "FR-220-AC-2")]
#[test]
fn command_reports_each_external_tool_version_with_its_configuration_index_reference() {
    let temp = tempfile::tempdir().expect("temporary fixture directory");
    let result = write_run(temp.path(), "pass", false);
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: Value = serde_json::from_slice(
        &std::fs::read(repository.join("spec/validation/external-tools.v1.json"))
            .expect("authoritative external-tools index reads"),
    )
    .expect("authoritative external-tools index parses");
    let entries = index["entries"].as_array().expect("index entries");
    let tools = entries
        .iter()
        .map(|entry| {
            let id = entry["id"].as_str().expect("tool id");
            json!({
                "tool_id": id,
                "executable": entry["executable"],
                "version_invocation": entry["version_invocation"],
                "observed_version_output": format!("{id} fixture version 1.2.3"),
                "observed_version_stream": "stdout",
                "index_path": "spec/validation/external-tools.v1.json",
                "index_format_version": "1",
            })
        })
        .collect::<Vec<_>>();
    let mut record: Value =
        serde_json::from_slice(&std::fs::read(&result).expect("run record reads"))
            .expect("run record parses");
    record["externalTools"] = json!(tools);
    std::fs::write(
        &result,
        serde_json::to_vec(&record).expect("run record serializes"),
    )
    .expect("external tool evidence writes");

    let output_directory = temp.path().join("report");
    let output = invoke(&result, &output_directory);
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(
        &std::fs::read(output_directory.join("validation-report.json"))
            .expect("JSON report is written"),
    )
    .expect("report JSON parses");
    assert_eq!(report["status"], "complete");
    assert_eq!(
        report["externalToolsIndex"]["path"],
        "spec/validation/external-tools.v1.json"
    );
    assert_eq!(report["externalToolsIndex"]["formatVersion"], "1");
    let reported_tools = report["externalTools"].as_array().expect("tool rows");
    assert_eq!(reported_tools.len(), entries.len());
    for entry in entries {
        let id = entry["id"].as_str().expect("tool id");
        let row = reported_tools
            .iter()
            .find(|row| row["toolId"] == id)
            .expect("every configured tool has a report row");
        assert_eq!(row["status"], "observed");
        assert_eq!(row["executable"], entry["executable"]);
        assert_eq!(row["versionInvocation"], entry["version_invocation"]);
        assert_eq!(
            row["observedVersionOutput"],
            format!("{id} fixture version 1.2.3")
        );
        assert_eq!(row["indexPath"], "spec/validation/external-tools.v1.json");
        assert_eq!(row["indexFormatVersion"], "1");
    }

    let mut incorrect_reference = record;
    incorrect_reference["externalTools"][0]["index_path"] = json!("spec/validation/wrong.json");
    std::fs::write(
        &result,
        serde_json::to_vec(&incorrect_reference).expect("mutated run record serializes"),
    )
    .expect("incorrect-reference case writes");
    let negative_output = invoke(&result, &temp.path().join("negative-report"));
    assert_eq!(negative_output.status.code(), Some(1));
    let negative_report: Value = serde_json::from_slice(
        &std::fs::read(temp.path().join("negative-report/validation-report.json"))
            .expect("negative JSON report is written"),
    )
    .expect("negative report parses");
    assert_eq!(negative_report["status"], "incomplete");
    let invalid_tool_id = tools[0]["tool_id"].as_str().expect("tool id");
    let invalid_tool = negative_report["externalTools"]
        .as_array()
        .expect("negative tool rows")
        .iter()
        .find(|row| row["toolId"] == invalid_tool_id)
        .expect("invalid tool remains in the report");
    assert_eq!(invalid_tool["status"], "not provided");
}

#[trace(
    "FR-217-AC-1",
    "FR-217-AC-3",
    "FR-217-AC-4",
    "FR-218-AC-1",
    "FR-218-AC-6"
)]
#[test]
fn command_writes_deterministic_markdown_and_json_for_complete_validation() {
    let temp = tempfile::tempdir().expect("temporary fixture directory");
    let result = write_run(temp.path(), "pass", false);
    let first_directory = temp.path().join("first");
    let first = invoke(&result, &first_directory);
    assert!(
        first.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let second_directory = temp.path().join("second");
    let second = invoke(&result, &second_directory);
    assert!(second.status.success());
    let first_markdown = std::fs::read(first_directory.join("validation-report.md"))
        .expect("Markdown report is written");
    let first_json = std::fs::read(first_directory.join("validation-report.json"))
        .expect("JSON report is written");
    assert_eq!(
        first_markdown,
        std::fs::read(second_directory.join("validation-report.md")).expect("repeat markdown")
    );
    assert_eq!(
        first_json,
        std::fs::read(second_directory.join("validation-report.json")).expect("repeat JSON")
    );
    let report: Value = serde_json::from_slice(&first_json).expect("JSON report parses");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["coverage"][0]["rating"], "high_process_risk");
    assert_eq!(report["ratedBy"], "quality");
    assert!(
        String::from_utf8(first_markdown)
            .expect("Markdown is UTF-8")
            .contains("## Sign-off")
    );
    let serialized = serde_json::to_string(&report).expect("report serializes");
    assert!(!serialized.contains("digest"));
    assert!(!serialized.contains("commitId"));
    assert!(!serialized.to_ascii_lowercase().contains("accepted"));
}

#[trace("FR-217-AC-2", "FR-218-AC-2")]
#[test]
fn incomplete_command_still_writes_the_report_and_names_each_gap() {
    let temp = tempfile::tempdir().expect("temporary fixture directory");
    let result = write_run(temp.path(), "fail", true);
    let output_directory = temp.path().join("report");
    let output = invoke(&result, &output_directory);
    assert_eq!(output.status.code(), Some(1));
    assert!(output_directory.join("validation-report.md").is_file());
    let report: Value = serde_json::from_slice(
        &std::fs::read(output_directory.join("validation-report.json"))
            .expect("JSON report exists"),
    )
    .expect("JSON report parses");
    assert_eq!(report["status"], "incomplete");
    let reasons = report["incompleteReasons"]
        .as_array()
        .expect("reasons listed");
    let reasons = reasons.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert_eq!(report["results"][0]["result"], "fail");
    assert_eq!(report["coverage"][0]["result"], "uncovered");
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("result missing for tool quire-cli"))
    );
}

#[trace("FR-218-AC-1", "FR-218-AC-3")]
#[test]
fn complete_report_keeps_failed_and_errored_results_and_uncovered_requirements() {
    let temp = tempfile::tempdir().expect("temporary fixture directory");
    let quoin = write_run(temp.path(), "fail", false);
    let quire_cli = temp.path().join("quire-cli-run.json");
    let quire_cli_record = json!({
        "schemaVersion": 1,
        "tool": "quire-cli",
        "toolVersion": "quire-cli test",
        "os": "linux",
        "architecture": "x86_64",
        "suite": "NORMAL",
        "timestamp": "2026-11-25T12:00:00Z",
        "results": [{
            "id": "case-error",
            "status": "error",
            "criteria": ["FR-002-AC-1"],
            "detail": "fixture execution error",
            "stderr": "fixture stderr"
        }],
        "validationInputs": {"provided": true, "documents": []},
        "anomalySet": {
            "provided": true,
            "producerVersion": "quire-cli test",
            "documents": []
        },
        "externalTools": [],
        "retainedLocation": "spec/evidence/runs/NORMAL/run.json"
    });
    std::fs::write(
        &quire_cli,
        serde_json::to_vec(&quire_cli_record).expect("run serializes"),
    )
    .expect("second run record writes");
    let output_directory = temp.path().join("report");
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["validation", "report", "--results"])
        .arg(&quoin)
        .arg("--results")
        .arg(&quire_cli)
        .arg("--out")
        .arg(&output_directory)
        .output()
        .expect("validation report command starts");
    assert_eq!(
        output.status.code(),
        Some(0),
        "complete evidence exits successfully even when tests fail: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(
        &std::fs::read(output_directory.join("validation-report.json"))
            .expect("JSON report is written"),
    )
    .expect("report JSON parses");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["incompleteReasons"], json!([]));
    let results = report["results"].as_array().expect("result rows");
    assert_eq!(
        results
            .iter()
            .find(|row| row["id"] == "fixture-case")
            .expect("failed result is present")["result"],
        "fail"
    );
    assert_eq!(
        results
            .iter()
            .find(|row| row["id"] == "case-error")
            .expect("errored result is present")["result"],
        "error"
    );
    assert!(
        report["coverage"]
            .as_array()
            .expect("coverage rows")
            .iter()
            .any(|row| row["fr"] == "FR-001" && row["result"] == "uncovered")
    );
    let markdown = std::fs::read_to_string(output_directory.join("validation-report.md"))
        .expect("Markdown report is written");
    assert!(markdown.contains("| fixture-case | quoin | FR-001-AC-1 | fail |"));
    assert!(markdown.contains("| case-error | quire-cli | FR-002-AC-1 | error |"));
}

#[trace("FR-218-AC-1")]
#[test]
fn missing_risk_rating_does_not_make_complete_run_evidence_incomplete() {
    let temp = tempfile::tempdir().expect("temporary fixture directory");
    let result = write_run(temp.path(), "pass", false);
    let mut record: Value =
        serde_json::from_slice(&std::fs::read(&result).expect("run record reads"))
            .expect("run record parses");
    record["validationInputs"]["documents"]
        .as_array_mut()
        .expect("validation documents")
        .retain(|document| {
            document["raw"]
                .as_str()
                .is_some_and(|raw| !raw.contains("FeatureRiskRating"))
        });
    std::fs::write(
        &result,
        serde_json::to_vec(&record).expect("run serializes"),
    )
    .expect("run record writes");
    let output_directory = temp.path().join("report");
    let output = invoke(&result, &output_directory);
    assert_eq!(
        output.status.code(),
        Some(0),
        "risk rating absence does not make run evidence incomplete: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(
        &std::fs::read(output_directory.join("validation-report.json"))
            .expect("JSON report is written"),
    )
    .expect("report JSON parses");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["coverage"][0]["rating_status"], "unrated");
}
