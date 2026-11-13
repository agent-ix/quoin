// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Synthetic OSV-Scanner JSON exercises for the FR-149 adapter boundary.
//!
//! These inputs are hand-authored synthetic documents, not scanner captures or
//! evidence that OSV-Scanner matched an advisory.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_evidence::adapters::{AdapterSelection, parse_osv_scanner, select_finding_adapter};
use quoin_evidence::types::Finding;

const CLEAN: &str = r#"{"results":[{"source":{"path":"synthetic.cdx.json"},"packages":[{"package":{"name":"left-pad","version":"1.3.0","ecosystem":"npm","purl":"pkg:npm/left-pad@1.3.0"},"vulnerabilities":[],"groups":[]}]}]}"#;
const VULNERABLE: &str = r#"{"results":[{"source":{"path":"synthetic.cdx.json"},"packages":[{"package":{"name":"left-pad","version":"1.3.0","ecosystem":"npm","purl":"pkg:npm/left-pad@1.3.0"},"vulnerabilities":[{"id":"GHSA-synthetic","summary":"Synthetic finding"}],"groups":[{"ids":["GHSA-synthetic"],"max_severity":"HIGH"}]}]}]}"#;

fn adapter(tool: &str) -> &'static str {
    select_finding_adapter(AdapterSelection {
        adapter: None,
        tool: Some(tool),
    })
    .unwrap()
    .name
}

#[trace("FR-149-AC-1", "FR-149-AC-6")]
#[test]
fn synthetic_vulnerability_is_transcribed_and_tool_selection_is_registered() {
    let result = parse_osv_scanner(VULNERABLE).unwrap();
    assert_eq!(adapter("osv-scanner 2.7.1"), "osv-scanner");
    assert_eq!(
        select_finding_adapter(AdapterSelection {
            adapter: Some("osv-scanner"),
            tool: None,
        })
        .unwrap()
        .name,
        "osv-scanner"
    );
    assert_eq!(adapter("cargo-audit 0.21"), "cargo-audit");
    assert_eq!(adapter("cargo-deny 0.19.8"), "cargo-deny");
    assert_eq!(result.tool.as_deref(), Some("osv-scanner"));
    assert_eq!(
        result.findings,
        [Finding {
            rule_id: "GHSA-synthetic".to_owned(),
            severity: Some("HIGH".to_owned()),
            message: Some("Synthetic finding".to_owned()),
            path: Some("pkg:npm/left-pad@1.3.0".to_owned()),
            line: None,
            trace_ids: None,
        }]
    );
}

#[trace("FR-149-AC-2", "FR-149-AC-8", "FR-149-AC-11")]
#[test]
fn synthetic_clean_report_is_recordable_without_database_or_rule_count_claims() {
    let result = parse_osv_scanner(CLEAN).unwrap();
    assert!(result.findings.is_empty());
    assert_eq!(result.ruleset, None);
    assert_eq!(result.rules_evaluated, None);
}

#[trace("FR-149-AC-3", "FR-149-AC-4")]
#[test]
fn invalid_truncated_empty_and_missing_package_reports_are_refused() {
    for raw in [
        "not JSON",
        r#"{"results":["#,
        "{}",
        r#"{"results":[]}"#,
        r#"{"results":[{"source":{}}]}"#,
        r#"{"results":[{"packages":[]}] }"#,
    ] {
        assert!(parse_osv_scanner(raw).is_err(), "accepted {raw}");
    }
    let error = parse_osv_scanner(r#"{"results":[{"source":{}}]}"#)
        .unwrap_err()
        .to_string();
    assert!(error.contains("packages"), "{error}");
}

#[trace("FR-149-AC-5")]
#[test]
fn synthetic_vulnerability_without_an_id_is_refused() {
    let raw = r#"{"results":[{"packages":[{"package":{"name":"left-pad","version":"1.3.0"},"vulnerabilities":[{"summary":"missing id"}],"groups":[]}]}]}"#;
    let error = parse_osv_scanner(raw).unwrap_err().to_string();
    assert!(error.contains("vulnerability"), "{error}");
    assert!(error.contains("package"), "{error}");
}

#[trace("FR-149-AC-7")]
#[test]
fn synthetic_vulnerability_without_a_purl_uses_name_and_version() {
    let raw = r#"{"results":[{"packages":[{"package":{"name":"left-pad","version":"1.3.0"},"vulnerabilities":[{"id":"GHSA-synthetic"}],"groups":[]}]}]}"#;
    let result = parse_osv_scanner(raw).unwrap();
    assert_eq!(result.findings[0].path.as_deref(), Some("left-pad@1.3.0"));
}

#[trace("FR-149-AC-10")]
#[test]
fn synthetic_debian_os_package_purl_keeps_its_distro_qualifier() {
    let purl = "pkg:deb/debian/libc6@2.36-9%2Bdeb12u4?arch=amd64&distro=debian-12";
    let raw = format!(
        r#"{{"results":[{{"packages":[{{"package":{{"name":"libc6","version":"2.36-9+deb12u4","ecosystem":"Debian","purl":"{purl}"}},"vulnerabilities":[{{"id":"CVE-synthetic"}}],"groups":[]}}]}}]}}"#
    );
    let result = parse_osv_scanner(&raw).unwrap();
    assert_eq!(result.findings[0].path.as_deref(), Some(purl));
}

#[trace("FR-149-AC-12")]
#[test]
fn vulnerable_packages_without_a_valid_identity_are_refused() {
    let raw_reports = [
        r#"{"results":[{"packages":[{"vulnerabilities":[{"id":"CVE-x"}],"groups":[]}]}]}"#,
        r#"{"results":[{"packages":[{"package":{"name":"","version":"1"},"vulnerabilities":[{"id":"CVE-x"}],"groups":[]}]}]}"#,
        r#"{"results":[{"packages":[{"package":{"name":"a","version":""},"vulnerabilities":[{"id":"CVE-x"}],"groups":[]}]}]}"#,
        r#"{"results":[{"packages":[{"package":{"name":"a","version":"1","purl":"pkg:npm/%ZZ@1"},"vulnerabilities":[{"id":"CVE-x"}],"groups":[]}]}]}"#,
    ];
    for raw in raw_reports {
        let error = parse_osv_scanner(raw).unwrap_err().to_string();
        assert!(error.contains("package identity"), "{error}");
    }
}
