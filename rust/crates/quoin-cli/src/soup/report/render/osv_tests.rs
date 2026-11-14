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

use super::super::{Report, Row};
use super::{markdown, scan_text, tool_lines};
use crate::soup::advisory::{
    Attached, Disposition, RecordJoin, Resolution, ScanOutcome, ScanState, Tool, Unresolved,
};
use crate::soup::policy::{LicenceOutcome, LicenceState, PolicyStatus, RecordPolicy};
use crate::soup::report::Listed;
use crate::soup::support::{Evaluation, Resolved};

#[trace("FR-144-AC-16")]
#[test]
fn synthetic_npm_and_pypi_advisories_render_on_their_rows_with_resolution_state() {
    let npm_purl = "pkg:npm/example@1.0.0";
    let pypi_purl = "pkg:pypi/example@2.0.0";
    let resolved = Attached {
        tool: Tool::OsvScanner,
        rule_id: "OSV-NPM".to_owned(),
        severity: Some("HIGH".to_owned()),
        message: Some("synthetic npm advisory".to_owned()),
        package: npm_purl.to_owned(),
        scan_timestamp: "2026-08-01T00:00:00.000Z".to_owned(),
        resolution: Resolution::Resolved(Disposition {
            decision: quoin_assurance::DispositionDecision::AcceptedRisk,
            rationale: "synthetic approved rationale".to_owned(),
            approval_ref: "synthetic-approval".to_owned(),
            attested_by: "security-reviewer".to_owned(),
        }),
    };
    let unresolved = Attached {
        tool: Tool::OsvScanner,
        rule_id: "OSV-PYPI".to_owned(),
        severity: Some("MODERATE".to_owned()),
        message: Some("synthetic PyPI advisory".to_owned()),
        package: pypi_purl.to_owned(),
        scan_timestamp: "2026-08-01T00:00:00.000Z".to_owned(),
        resolution: Resolution::Unresolved(Unresolved::NoDispositionClause),
    };
    let report = Report {
        as_of: "2026-08-01".to_owned(),
        rows: vec![
            synthetic_row(npm_purl, resolved),
            synthetic_row(pypi_purl, unresolved),
        ],
        scans: vec![ScanState {
            tool: Tool::OsvScanner,
            ruleset: None,
            outcome: ScanOutcome::Read {
                suite: "synthetic-suite".to_owned(),
                timestamp: "2026-08-01T00:00:00.000Z".to_owned(),
                advisories: 2,
                set_aside: 0,
            },
            unread: Vec::new(),
        }],
        unmatched: Vec::new(),
        licence: LicenceState {
            outcome: LicenceOutcome::NotRun,
            unread: Vec::new(),
        },
        deny_policy_read: None,
        unmatched_policy: Vec::new(),
        unreadable_scans: Vec::new(),
    };

    let rendered = markdown(&report);
    let npm_row = rendered
        .split(&format!("## {npm_purl}"))
        .nth(1)
        .unwrap()
        .split("\n## ")
        .next()
        .unwrap();
    let pypi_row = rendered
        .split(&format!("## {pypi_purl}"))
        .nth(1)
        .unwrap()
        .split("\n## ")
        .next()
        .unwrap();

    assert!(npm_row.contains("osv-scanner OSV-NPM"), "{npm_row}");
    assert!(
        npm_row.contains("resolved: accepted_risk by security-reviewer"),
        "{npm_row}"
    );
    assert!(!npm_row.contains("OSV-PYPI"), "{npm_row}");
    assert!(pypi_row.contains("osv-scanner OSV-PYPI"), "{pypi_row}");
    assert!(
        pypi_row.contains("unresolved: no disposition clause"),
        "{pypi_row}"
    );
    assert!(!pypi_row.contains("OSV-NPM"), "{pypi_row}");
}

fn synthetic_row(purl: &str, advisory: Attached) -> Row {
    Row {
        join: RecordJoin {
            path: PathBuf::from("SOUP.md"),
            id: None,
            purl: purl.to_owned(),
            clause: None,
            advisories: vec![advisory],
            free_of: Vec::new(),
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
    }
}

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
