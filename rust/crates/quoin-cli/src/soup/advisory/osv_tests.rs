// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Synthetic join behavior for OSV findings; these are not scanner captures.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;

use super::tests::{discharge, record, scan};
use super::{ScanOutcome, Tool, UnmatchedReason, join};

fn report(path: &str, id: &str) -> String {
    format!(
        r#"{{"results":[{{"source":{{"path":"synthetic.cdx.json"}},"packages":[{{"package":{{"name":"pkg","version":"1.0","purl":"{path}"}},"vulnerabilities":[{{"id":"{id}"}}],"groups":[]}}]}}]}}"#
    )
}

fn scan_osv(path: &str, id: &str) -> quoin_evidence::types::FindingRecord {
    scan(
        "osv-scanner 2.7.1",
        "osv-suite",
        "2026-08-01T00:00:00.000Z",
        &report(path, id),
    )
}

#[trace("FR-142-AC-23")]
#[test]
fn debian_os_package_purl_matches_while_a_spelling_mismatch_is_unmatched() {
    let debian = "pkg:deb/debian/libc6@2.36-9%2Bdeb12u4?arch=amd64&distro=debian-12";
    let matched = record("pkg:deb/debian/libc6@2.36-9+deb12u4?arch=arm64", None);
    let unmatched_record = record("pkg:npm/left-pad@1.3.0", None);
    let scans = [scan_osv(debian, "CVE-synthetic")];
    let joined = join(&[matched, unmatched_record], &scans, &discharge());
    assert_eq!(joined.records[0].advisories.len(), 1);
    assert_eq!(joined.records[0].advisories[0].tool, Tool::OsvScanner);
    assert_eq!(joined.records[0].advisories[0].rule_id, "CVE-synthetic");

    let spelling_mismatch = scan_osv("pkg:npm/left_pad@1.3.0", "GHSA-spelling");
    let unmatched = join(
        &[record("pkg:npm/left-pad@1.3.0", None)],
        &[spelling_mismatch],
        &discharge(),
    );
    assert_eq!(unmatched.unmatched.len(), 1);
    assert_eq!(unmatched.unmatched[0].reason, UnmatchedReason::NoRecord);
    assert_eq!(unmatched.unmatched[0].rule_id, "GHSA-spelling");
}

#[trace("FR-142-AC-20")]
#[test]
fn osv_assesses_deb_and_supported_ecosystems_but_not_lean_or_other_types() {
    let records = [
        record("pkg:npm/name@1.0", None),
        record("pkg:pypi/name@1.0", None),
        record("pkg:golang/example.com/name@1.0", None),
        record("pkg:deb/debian/name@1.0?distro=debian-12", None),
        record("pkg:lake/name@1.0", None),
        record("pkg:generic/name@1.0", None),
        record("pkg:other/name@1.0", None),
        record("pkg:cargo/name@1.0", None),
    ];
    let clean = r#"{"results":[{"packages":[{"package":{"name":"name","version":"1.0","purl":"pkg:npm/name@1.0"},"vulnerabilities":[],"groups":[]}]}]}"#;
    let osv = scan(
        "osv-scanner",
        "osv-suite",
        "2026-08-01T00:00:00.000Z",
        clean,
    );
    let joined = join(&records, &[osv], &discharge());
    let free: Vec<&str> = joined
        .records
        .iter()
        .filter(|record| record.free_of.contains(&Tool::OsvScanner))
        .map(|record| record.purl.as_str())
        .collect();
    assert_eq!(
        free,
        [
            "pkg:deb/debian/name@1.0?distro=debian-12",
            "pkg:golang/example.com/name@1.0",
            "pkg:npm/name@1.0",
            "pkg:pypi/name@1.0",
        ]
    );
    for purl in [
        "pkg:lake/name@1.0",
        "pkg:generic/name@1.0",
        "pkg:other/name@1.0",
        "pkg:cargo/name@1.0",
    ] {
        let record = joined
            .records
            .iter()
            .find(|record| record.purl == purl)
            .unwrap();
        assert!(!record.free_of.contains(&Tool::OsvScanner), "{purl}");
        assert!(!Tool::OsvScanner.assesses(purl), "{purl}");
    }
    assert!(Tool::OsvScanner.assesses("pkg:deb/debian/name@1.0?distro=debian-12"));
}

#[trace("FR-142-AC-26")]
#[test]
fn name_version_fallback_is_unmatched_and_blocks_supported_records_from_clean_claims() {
    let raw = r#"{"results":[{"packages":[{"package":{"name":"name","version":"1.0"},"vulnerabilities":[{"id":"CVE-fallback"}],"groups":[]}]}]}"#;
    let osv = scan("osv-scanner", "osv-suite", "2026-08-01T00:00:00.000Z", raw);
    let records = [
        record("pkg:npm/name@1.0", None),
        record("pkg:deb/debian/name@1.0", None),
        record("pkg:cargo/name@1.0", None),
    ];
    let joined = join(&records, &[osv], &discharge());
    assert_eq!(joined.unmatched.len(), 1);
    assert_eq!(joined.unmatched[0].reason, UnmatchedReason::NoPurl);
    assert_eq!(joined.unmatched[0].package.as_deref(), Some("name@1.0"));
    assert!(
        joined
            .records
            .iter()
            .all(|record| { !record.free_of.contains(&Tool::OsvScanner) })
    );
    let read = joined
        .scans
        .iter()
        .find(|state| state.tool == Tool::OsvScanner)
        .unwrap();
    assert!(matches!(
        read.outcome,
        ScanOutcome::Read { advisories: 1, .. }
    ));
}
