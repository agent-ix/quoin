// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence record-customer-artifact`, run as the shipped binary (FR-030-AC-36).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use ix_trace_rs::trace;

const ARTIFACT: &str = r#"{"artifactId":"Complaint-001","title":"Complaint record","location":"/nonexistent/complaint.pdf","suppliedBy":"Quality manager","suppliedAt":"2026-11-20T09:30:00Z"}"#;

fn quoin(repo: &Path, home: &Path, artifact: &str, stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence",
            "record-customer-artifact",
            "--repo",
            repo.to_str().unwrap(),
            "--artifact",
            artifact,
        ])
        .env("IX_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the native quoin binary runs");
    let mut pipe = child.stdin.take().unwrap();
    if let Some(text) = stdin {
        pipe.write_all(text.as_bytes()).unwrap();
    }
    drop(pipe);
    child.wait_with_output().unwrap()
}

#[trace("FR-030-AC-36")]
#[test]
fn tc_3503_the_command_files_a_document_from_a_path_or_stdin_and_a_replay_exists() {
    let repo = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let document = repo.path().join("artifact.json");
    std::fs::write(&document, ARTIFACT).unwrap();

    let first = quoin(repo.path(), home.path(), document.to_str().unwrap(), None);
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    let stdout = String::from_utf8(first.stdout).unwrap();
    assert!(stdout.contains("Complaint-001 created "), "{stdout}");
    let stored = repo
        .path()
        .join("spec/evidence/customer-artifacts/complaint-001.json");
    let bytes = std::fs::read_to_string(&stored).unwrap();
    assert!(
        bytes.contains("\"artifactId\": \"Complaint-001\""),
        "{bytes}"
    );
    assert!(bytes.contains("\"schemaVersion\": 1"), "{bytes}");

    let replay = quoin(repo.path(), home.path(), "-", Some(ARTIFACT));
    assert_eq!(replay.status.code(), Some(0), "{replay:?}");
    let stdout = String::from_utf8(replay.stdout).unwrap();
    assert!(stdout.contains("Complaint-001 exists "), "{stdout}");
    assert_eq!(std::fs::read_to_string(&stored).unwrap(), bytes);

    let changed = ARTIFACT.replace("Complaint record", "Another title");
    let refused = quoin(repo.path(), home.path(), "-", Some(&changed));
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let stderr = String::from_utf8(refused.stderr).unwrap();
    assert!(stderr.contains("CORE_REFUSED"), "{stderr}");
    assert!(
        stderr.contains("customer-artifacts/complaint-001.json"),
        "{stderr}"
    );
    assert_eq!(std::fs::read_to_string(&stored).unwrap(), bytes);
}
