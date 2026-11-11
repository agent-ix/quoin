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
const FAILED: &str = include_str!("fixtures/cargo-deny-failed-real.jsonl");
const BANS_ONLY: &str = include_str!("fixtures/cargo-deny-bans-real.jsonl");
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
        r#"{"type":"summary","fields":{"advisories":{"errors":1,"warnings":0}}}"#,
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

/// A failed run printed under `--log-level off` is only a summary counting an
/// error. It is real output, and it must not read as a clean scan.
#[trace("FR-034-AC-25", "TC-2706")]
#[test]
fn tc_2706_a_summary_counting_more_than_the_diagnostics_read_is_refused() {
    let error = parse_cargo_deny(FAILED).unwrap_err().to_string();
    assert!(error.contains("counts 3 error and warning"), "{error}");
    // The same count with its diagnostic present is a finding, not a refusal.
    let raw = concat!(
        r#"{"type":"diagnostic","fields":{"code":"rejected","severity":"error","message":"m"}}"#,
        "\n",
        r#"{"type":"summary","fields":{"licenses":{"errors":1,"warnings":0}}}"#,
    );
    assert_eq!(parse_cargo_deny(raw).unwrap().findings.len(), 1);
}

/// The summary must name a check, is recorded as the ruleset, and is exactly
/// one and last. A diagnostic with no id is kept, not dropped.
#[trace("FR-034-AC-26", "TC-2707")]
#[test]
fn tc_2707_the_summary_names_the_checks_that_ran_and_closes_the_output() {
    assert_eq!(
        parse_cargo_deny(BANS_ONLY).unwrap().ruleset.as_deref(),
        Some("bans")
    );
    assert_eq!(
        parse_cargo_deny(CLEAN).unwrap().ruleset.as_deref(),
        Some("advisories,bans,licenses,sources")
    );
    assert!(
        parse_cargo_deny(r#"{"type":"summary","fields":{}}"#)
            .unwrap_err()
            .to_string()
            .contains("names no check")
    );
    let twice = format!("{BANS_ONLY}\n{BANS_ONLY}");
    assert!(parse_cargo_deny(&twice).is_err());
    let mut trailing = BANNED.to_owned();
    trailing.push_str(BANNED.lines().next().unwrap());
    assert!(parse_cargo_deny(&trailing).is_err());
    let nameless = concat!(
        r#"{"type":"diagnostic","fields":{"severity":"warning","message":"m"}}"#,
        "\n",
        r#"{"type":"summary","fields":{"bans":{"errors":0,"warnings":1}}}"#,
    );
    assert_eq!(
        parse_cargo_deny(nameless).unwrap().findings[0].rule_id,
        "unclassified"
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
