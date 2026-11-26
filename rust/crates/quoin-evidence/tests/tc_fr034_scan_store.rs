// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Scan-only suite enumeration and retention by timestamp.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test assertions use panics as the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::store::{gc, latest_scans, list_recorded_suites, write_scan};
use quoin_evidence::types::{Finding, FindingRecord, STORE_SCHEMA_VERSION};
use quoin_evidence::{MemoryEvidence, SuiteId};

fn scan(suite: &str, timestamp: &str, rule_id: &str) -> FindingRecord {
    FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: "semgrep 1.2.3".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        ruleset: None,
        rules_evaluated: Some(1),
        findings: vec![Finding::new(rule_id)],
        evaluated: None,
    }
}

/// A suite with only a finding-shaped record is returned by suite enumeration.
#[trace("FR-034-AC-18")]
#[test]
fn tc_fr034_scans_only_suites_are_enumerated() {
    let mut store = MemoryEvidence::new();
    write_scan(
        &mut store,
        &scan("scan-only", "2026-11-26T00:00:00Z", "S001"),
    )
    .unwrap();

    let suites = list_recorded_suites(&store).unwrap();

    assert_eq!(
        suites.iter().map(SuiteId::as_str).collect::<Vec<_>>(),
        ["scan-only"]
    );
}

/// GC removes superseded scans and retains the record with the newest
/// timestamp in that suite.
#[trace("FR-034-AC-19")]
#[test]
fn tc_fr034_gc_keeps_the_newest_scan_by_timestamp() {
    let mut store = MemoryEvidence::new();
    let older_path =
        write_scan(&mut store, &scan("scanner", "2026-11-24T00:00:00Z", "OLD")).unwrap();
    let newest_path = write_scan(
        &mut store,
        &scan("scanner", "2026-11-25T00:00:00Z", "NEWEST"),
    )
    .unwrap();

    let deleted = gc(&mut store, false, &mut Vec::new()).unwrap();

    assert!(deleted.contains(&older_path), "deleted paths: {deleted:?}");
    assert!(
        !deleted.contains(&newest_path),
        "deleted paths: {deleted:?}"
    );
    let scans = latest_scans(&store, &mut Vec::new()).unwrap();
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].timestamp, "2026-11-25T00:00:00Z");
    assert_eq!(scans[0].findings[0].rule_id, "NEWEST");
}
