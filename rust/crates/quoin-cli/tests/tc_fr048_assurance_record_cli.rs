// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The experiment and operational record commands accept JSON from files and
//! stdin and report the immutable publication identity (FR-048).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a panic in a command integration test is the failure report"
)]

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const RECORD_ID: &str = "8f14e45f-eaab-4b3b-a456-426614174000";

fn experiment() -> Value {
    json!({
        "recordId": RECORD_ID,
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
        "producerProvenance": {
            "schemaVersion": "producer-provenance-v1",
            "identity": "quoin",
            "version": "0.1.0",
            "capabilities": ["record"],
            "artifacts": [{ "name": "report.json" }]
        }
    })
}

fn operational() -> Value {
    json!({
        "recordId": RECORD_ID,
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
        "producerProvenance": {
            "schemaVersion": "producer-provenance-v1",
            "identity": "quoin",
            "version": "0.1.0",
            "capabilities": ["record"],
            "artifacts": [{ "name": "report.json" }]
        }
    })
}

fn invoke(
    repo: &Path,
    home: &Path,
    command: &str,
    input: &str,
    stdin: Option<&str>,
    json_output: bool,
) -> Output {
    let mut command_process = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command_process
        .args(["evidence", command, "--repo"])
        .arg(repo)
        .args(["--input", input])
        .env("IX_HOME", home);
    if json_output {
        command_process.arg("--json");
    }
    let mut process = command_process
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("native quoin command starts");
    let mut pipe = process.stdin.take().expect("stdin pipe is available");
    if let Some(body) = stdin {
        pipe.write_all(body.as_bytes()).expect("stdin accepts JSON");
    }
    drop(pipe);
    process
        .wait_with_output()
        .expect("native quoin command exits")
}

fn assert_file_then_stdin_record(
    repo: &Path,
    home: &Path,
    command: &str,
    family: &str,
    document: &Value,
) {
    let bytes = serde_json::to_vec(document).expect("record serializes as JSON");
    let input_file = repo.join(format!("{family}.json"));
    std::fs::write(&input_file, &bytes).expect("input file is written");
    let file_result = invoke(
        repo,
        home,
        command,
        input_file.to_str().expect("UTF-8 input path"),
        None,
        true,
    );
    assert!(
        file_result.status.success(),
        "{}",
        String::from_utf8_lossy(&file_result.stderr)
    );
    let file_result: Value = serde_json::from_slice(&file_result.stdout).expect("JSON response");
    let expected_path = repo
        .join("spec/evidence")
        .join(family)
        .join(format!("{RECORD_ID}.json"))
        .display()
        .to_string();
    assert_eq!(file_result["record"]["recordId"], RECORD_ID);
    assert_eq!(file_result["path"], expected_path);
    assert_eq!(file_result["created"], true);
    assert!(Path::new(&expected_path).is_file());

    let replay = invoke(
        repo,
        home,
        command,
        "-",
        Some(std::str::from_utf8(&bytes).expect("record is UTF-8")),
        false,
    );
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    let expected_text = format!("{RECORD_ID} exists {expected_path}\n");
    assert_eq!(String::from_utf8(replay.stdout).unwrap(), expected_text);
}

#[trace("FR-048-AC-7")]
#[test]
fn assurance_record_commands_accept_files_and_stdin_and_report_id_path_and_creation() {
    let repo = tempfile::tempdir().expect("temporary repository");
    let home = tempfile::tempdir().expect("temporary home");
    assert_file_then_stdin_record(
        repo.path(),
        home.path(),
        "record-experiment",
        "experiments",
        &experiment(),
    );
    assert_file_then_stdin_record(
        repo.path(),
        home.path(),
        "record-operational",
        "operational",
        &operational(),
    );
}
