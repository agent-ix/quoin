// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1791: `quoin evidence gc` names the records it keeps because it cannot
//! read them (FR-030-AC-23), against the real binary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use quoin_evidence::store::write_run;
use quoin_evidence::types::{RunRecord, STORE_SCHEMA_VERSION};
use quoin_evidence::{DiskEvidence, EvidenceSource, SuiteId};
use std::process::{Command, Output};

fn run(timestamp: &str) -> RunRecord {
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("SUITE-A"),
        tool: "cargo test".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        entries: Vec::new(),
    }
}

fn gc(repo: &std::path::Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["evidence", "gc", "--repo"])
        .arg(repo)
        .args(extra)
        .output()
        .unwrap()
}

#[trace("FR-030-AC-23")]
#[test]
fn gc_reports_the_unreadable_record_it_keeps_in_text_and_json() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    write_run(&mut store, &run("2026-11-22T00:00:01Z")).unwrap();
    write_run(&mut store, &run("2026-11-22T00:00:02Z")).unwrap();
    store
        .write("runs/SUITE-A/corrupt.json", b"{ not json")
        .unwrap();

    let json = gc(repo.path(), &["--json"]);
    assert_eq!(
        json.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["deleted"], serde_json::json!([]));
    let skipped = report["skipped"].as_array().unwrap();
    assert_eq!(skipped.len(), 1, "{report}");
    assert!(
        skipped[0]
            .as_str()
            .unwrap()
            .ends_with("runs/SUITE-A/corrupt.json")
    );

    let text = gc(repo.path(), &[]);
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("kept unreadable"), "{text}");
    assert!(!text.contains("nothing to collect"), "{text}");

    // Healthy control: with the corrupt file gone, the superseded run is collected.
    store.remove("runs/SUITE-A/corrupt.json").unwrap();
    let healthy = gc(repo.path(), &["--json"]);
    let report: serde_json::Value = serde_json::from_slice(&healthy.stdout).unwrap();
    assert_eq!(report["deleted"].as_array().unwrap().len(), 1, "{report}");
    assert_eq!(report["skipped"], serde_json::json!([]));
}
