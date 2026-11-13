// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Render-level limitation checks for the OSV-Scanner scan summary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;

use std::path::PathBuf;

use super::super::Row;
use super::{scan_text, tool_lines};
use crate::soup::advisory::{RecordJoin, ScanOutcome, ScanState, Tool};
use crate::soup::policy::{PolicyStatus, RecordPolicy};
use crate::soup::report::Listed;
use crate::soup::support::{Evaluation, Resolved};

#[trace("FR-142-AC-22")]
#[test]
fn osv_scan_without_database_identity_states_that_the_database_was_not_recorded() {
    let state = ScanState {
        tool: Tool::OsvScanner,
        ruleset: None,
        outcome: ScanOutcome::Read {
            suite: "synthetic-suite".to_owned(),
            timestamp: "2026-08-01T00:00:00.000Z".to_owned(),
            advisories: 0,
            set_aside: 0,
        },
        unread: Vec::new(),
    };
    let rendered = scan_text(&state);
    assert!(rendered.contains("database: not recorded"), "{rendered}");
    assert!(rendered.contains("0 advisories"), "{rendered}");
    assert!(!rendered.contains("compliant"), "{rendered}");
}

#[trace("FR-142-AC-22", "FR-142-AC-20")]
#[test]
fn a_clean_osv_row_labels_the_advisory_observation_with_the_missing_database() {
    let mut row = Row {
        join: RecordJoin {
            path: PathBuf::from("SOUP.md"),
            id: None,
            purl: "pkg:npm/left-pad@1.3.0".to_owned(),
            clause: None,
            advisories: Vec::new(),
            free_of: vec![Tool::OsvScanner],
        },
        version: None,
        serves: Listed::Absent,
        anomalies: Listed::Absent,
        support: Evaluation {
            support_status: Resolved::Unknown,
            end_of_support: Resolved::Unknown,
            past_end_of_support: false,
        },
        policy: RecordPolicy {
            findings: Vec::new(),
            status: PolicyStatus::NotRun,
        },
    };
    let lines = tool_lines(&row, Tool::OsvScanner, None);
    assert_eq!(
        lines,
        ["osv-scanner: no advisory reported (scan read; database not recorded)"]
    );
    assert!(!lines[0].contains("compliant"));

    row.join.purl = "pkg:lake/sample@1".to_owned();
    row.join.free_of.clear();
    assert_eq!(
        tool_lines(&row, Tool::OsvScanner, None),
        ["advisories not assessed: unsupported purl type"]
    );
}
