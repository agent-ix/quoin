// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The assurance-record commands publish immutable records from files and stdin.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report"
)]

use std::io::Write;
use std::process::{Command, Output, Stdio};

use ix_trace_rs::trace;
use serde_json::{Value, json};
use tempfile::TempDir;

fn record_pair() -> (Value, Value) {
    let provenance = json!({
        "schemaVersion": "producer-provenance-v1",
        "identity": "quoin",
        "version": "0.1.0",
        "capabilities": ["record"],
        "artifacts": [{ "name": "report.json" }]
    });
    (
        json!({
            "schemaVersion": "experiment-record-v1",
            "subject": { "kind": "requirement", "id": "FR-001" },
            "recordedAt": "2026-01-01T00:00:00Z",
            "hypothesis": "the adapter reads real output",
            "design": {
                "timeBox": "1d",
                "corpusRefs": ["corpus/a"],
                "comparisonMethod": "A/B",
                "decisionRule": "p<0.05"
            },
            "result": {
                "status": "supported",
                "summary": "it did",
                "evidenceRefs": ["runs/SUITE-A"]
            },
            "producerProvenance": provenance.clone()
        }),
        json!({
            "schemaVersion": "operational-evidence-record-v1",
            "subject": { "kind": "service", "id": "quoin-core" },
            "recordedAt": "2026-01-01T00:00:00Z",
            "window": {
                "startedAt": "2026-01-01T00:00:00Z",
                "endedAt": "2026-01-02T00:00:00Z"
            },
            "environment": "production",
            "observations": [{
                "signal": "latency",
                "value": "12",
                "unit": "ms",
                "interpretation": "within bounds",
                "evidenceRefs": ["metrics/1"]
            }],
            "outcome": "within_bounds",
            "producerProvenance": provenance
        }),
    )
}

fn invoke(repo: &TempDir, command: &str, document: &Value, from_stdin: bool) -> Output {
    let input_path = repo.path().join("input.json");
    if !from_stdin {
        std::fs::write(
            &input_path,
            serde_json::to_vec(document).expect("record serializes"),
        )
        .expect("input file is written");
    }
    let input = if from_stdin {
        "-".to_owned()
    } else {
        input_path.display().to_string()
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence",
            command,
            "--repo",
            repo.path().to_str().expect("repo path is UTF-8"),
            "--input",
            &input,
            "--json",
        ])
        .current_dir(repo.path())
        .stdin(if from_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("quoin starts");
    if from_stdin {
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(&serde_json::to_vec(document).expect("record serializes"))
            .expect("record reaches stdin");
    }
    child.wait_with_output().expect("quoin returns output")
}

fn published(output: Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "record command refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON result is emitted")
}

fn assert_publication(repo: &TempDir, result: &Value, document: &Value, created: bool) {
    assert_eq!(result["created"], created, "{result}");
    let record_id = result["record"]["recordId"]
        .as_str()
        .expect("an immutable record id is returned");
    let path = result["path"]
        .as_str()
        .expect("the stored path is returned");
    assert!(std::path::Path::new(path).is_absolute(), "{path}");
    assert!(std::path::Path::new(path).is_file(), "{path}");
    assert_eq!(
        std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str()),
        Some(format!("{record_id}.json").as_str())
    );
    let stored: Value = serde_json::from_slice(&std::fs::read(path).expect("record is stored"))
        .expect("stored record is JSON");
    assert_eq!(stored["recordId"], record_id);
    assert_eq!(stored["schemaVersion"], document["schemaVersion"]);
    assert!(path.starts_with(repo.path().to_str().expect("repo path is UTF-8")));
}

#[trace("FR-048-AC-7")]
#[test]
fn tc_fr048_record_experiment_and_operational_accept_files_and_stdin() {
    let experiment_repo = tempfile::tempdir().expect("experiment repo exists");
    let (experiment, _) = record_pair();
    let first_experiment = published(invoke(
        &experiment_repo,
        "record-experiment",
        &experiment,
        false,
    ));
    assert_publication(&experiment_repo, &first_experiment, &experiment, true);
    let mut repeated_experiment = experiment.clone();
    repeated_experiment["recordId"] = first_experiment["record"]["recordId"].clone();
    let repeated = published(invoke(
        &experiment_repo,
        "record-experiment",
        &repeated_experiment,
        true,
    ));
    assert_publication(&experiment_repo, &repeated, &repeated_experiment, false);
    assert_eq!(repeated["path"], first_experiment["path"]);
    assert_eq!(
        repeated["record"]["recordId"],
        first_experiment["record"]["recordId"]
    );

    let operational_repo = tempfile::tempdir().expect("operational repo exists");
    let (_, operational) = record_pair();
    let first_operational = published(invoke(
        &operational_repo,
        "record-operational",
        &operational,
        true,
    ));
    assert_publication(&operational_repo, &first_operational, &operational, true);
    let mut repeated_operational = operational.clone();
    repeated_operational["recordId"] = first_operational["record"]["recordId"].clone();
    let repeated = published(invoke(
        &operational_repo,
        "record-operational",
        &repeated_operational,
        false,
    ));
    assert_publication(&operational_repo, &repeated, &repeated_operational, false);
    assert_eq!(repeated["path"], first_operational["path"]);
    assert_eq!(
        repeated["record"]["recordId"],
        first_operational["record"]["recordId"]
    );
}
