// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

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

#[trace("FR-217-AC-1", "FR-217-AC-3", "FR-217-AC-4", "FR-218-AC-1", "FR-218-AC-6")]
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
    assert!(String::from_utf8(first_markdown).expect("Markdown is UTF-8").contains("## Sign-off"));
    let serialized = serde_json::to_string(&report).expect("report serializes");
    assert!(!serialized.contains("digest"));
    assert!(!serialized.contains("commitId"));
    assert!(!serialized.to_ascii_lowercase().contains("accepted"));
}

#[trace("FR-217-AC-2", "FR-218-AC-2", "FR-218-AC-3")]
#[test]
fn incomplete_command_still_writes_the_report_and_names_each_gap() {
    let temp = tempfile::tempdir().expect("temporary fixture directory");
    let result = write_run(temp.path(), "fail", true);
    let output_directory = temp.path().join("report");
    let output = invoke(&result, &output_directory);
    assert_eq!(output.status.code(), Some(1));
    assert!(output_directory.join("validation-report.md").is_file());
    let report: Value = serde_json::from_slice(
        &std::fs::read(output_directory.join("validation-report.json")).expect("JSON report exists"),
    )
    .expect("JSON report parses");
    assert_eq!(report["status"], "incomplete");
    let reasons = report["incompleteReasons"].as_array().expect("reasons listed");
    let reasons = reasons.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert!(reasons.iter().any(|reason| reason.contains("uncovered FR FR-001")));
    assert!(reasons.iter().any(|reason| reason.contains("failed case fixture-case")));
    assert!(reasons.iter().any(|reason| reason.contains("result missing for tool quire-cli")));
}
