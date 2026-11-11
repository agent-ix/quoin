// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The cargo-deny adapter (FR-034), read against output captured from
//! `cargo deny --format json check` 0.19.8 over a scratch crate and checked in
//! unedited. A fixture written to match the reader only proves the reader
//! parses itself.
//!
//! `cargo-deny-real.jsonl` is a run with one `[bans] deny` hit; the other is
//! the same crate with that rule removed, a run that finished and found
//! nothing.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::adapters::{AdapterSelection, parse_cargo_deny, select_finding_adapter};

const BANNED: &str = include_str!("fixtures/cargo-deny-real.jsonl");
const CLEAN: &str = include_str!("fixtures/cargo-deny-clean-real.jsonl");

/// A diagnostic becomes a finding carrying the check's code, the tool's own
/// severity word, its message and the first crate of the dependency graph.
#[trace("FR-034-AC-21", "TC-2700")]
#[test]
fn tc_2700_a_cargo_deny_diagnostic_is_a_finding() {
    let result = parse_cargo_deny(BANNED).unwrap();
    assert_eq!(result.tool.as_deref(), Some("cargo-deny"));
    assert_eq!(result.findings.len(), 1);
    let finding = &result.findings[0];
    assert_eq!(finding.rule_id, "banned");
    assert_eq!(finding.severity.as_deref(), Some("error"));
    assert_eq!(finding.path.as_deref(), Some("itoa@1.0.19"));
    assert!(
        finding
            .message
            .as_deref()
            .is_some_and(|m| m.contains("explicitly banned")),
        "{finding:?}"
    );
}

/// An advisory is identified by its RUSTSEC id rather than the generic code.
#[trace("FR-034-AC-21", "TC-2700")]
#[test]
fn tc_2700_an_advisory_diagnostic_is_identified_by_its_advisory_id() {
    let raw = concat!(
        r#"{"type":"diagnostic","fields":{"code":"vulnerability","severity":"error","message":"m","advisory":{"id":"RUSTSEC-2000-0001"},"graphs":[{"Krate":{"name":"p","version":"1.0.0"}}]}}"#,
        "\n",
        r#"{"type":"summary","fields":{}}"#,
    );
    let result = parse_cargo_deny(raw).unwrap();
    assert_eq!(result.findings[0].rule_id, "RUSTSEC-2000-0001");
    assert_eq!(result.findings[0].path.as_deref(), Some("p@1.0.0"));
}

/// A finished run with no diagnostic is a scan that happened and found
/// nothing, and cargo-deny reports no rule count, so none is invented.
#[trace("FR-034-AC-22", "TC-2701")]
#[test]
fn tc_2701_a_finished_run_with_no_diagnostic_is_a_clean_scan() {
    let result = parse_cargo_deny(CLEAN).unwrap();
    assert_eq!(result.findings.len(), 0);
    assert_eq!(result.tool.as_deref(), Some("cargo-deny"));
    assert_eq!(result.rules_evaluated, None);
}

/// Output without the closing summary is a run that did not finish, and text
/// that is not JSON is named as such; neither reads as a clean scan.
#[trace("FR-034-AC-23", "TC-2702")]
#[test]
fn tc_2702_output_with_no_summary_or_not_json_is_refused() {
    let unfinished = BANNED.lines().next().unwrap();
    assert!(
        parse_cargo_deny(unfinished)
            .unwrap_err()
            .to_string()
            .contains("no `summary` object")
    );
    assert!(parse_cargo_deny("").is_err());
    assert!(
        parse_cargo_deny("error: not json\n")
            .unwrap_err()
            .to_string()
            .contains("line 1 is not JSON")
    );
}

/// `--tool cargo-deny` selects the cargo-deny reader and `--tool cargo-audit`
/// still selects cargo-audit's.
#[trace("FR-034-AC-24", "TC-2703")]
#[test]
fn tc_2703_the_tool_selects_the_cargo_deny_adapter() {
    let by_tool = |tool| {
        select_finding_adapter(AdapterSelection {
            adapter: None,
            tool: Some(tool),
        })
        .map(|adapter| adapter.name)
    };
    assert_eq!(by_tool("cargo-deny 0.19.8"), Some("cargo-deny"));
    assert_eq!(by_tool("cargo deny"), Some("cargo-deny"));
    assert_eq!(by_tool("cargo-audit 0.21"), Some("cargo-audit"));
}
