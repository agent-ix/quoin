// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Synthetic join cases and an opt-in replay of an external authentic report.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::PathBuf;

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

#[trace("FR-142-AC-23", "FR-149-AC-1")]
#[test]
fn minimal_debian_contract_fixture_keeps_reported_purl_and_id_through_the_join() {
    // The purl and vulnerability identifiers come from the TC-9005 OSV-Scanner
    // 2.7.1 output. This hand-authored envelope is a minimal synthetic contract
    // fixture, not a captured report.
    let purl = "pkg:deb/debian/libc6@2.36-9%2Bdeb12u4?arch=amd64&distro=debian-12";
    let raw = r#"{"results":[{"source":{"path":"synthetic-contract-fixture"},"packages":[{"package":{"purl":"pkg:deb/debian/libc6@2.36-9%2Bdeb12u4?arch=amd64&distro=debian-12"},"vulnerabilities":[{"id":"DSA-5497-2","upstream":["CVE-2023-4863","DEBIAN-CVE-2023-4863"]}],"groups":[]}]}]}"#;
    let scan = scan(
        "osv-scanner 2.7.1",
        "osv-suite",
        "2026-08-01T00:00:00.000Z",
        raw,
    );

    assert_eq!(scan.findings.len(), 1);
    assert_eq!(scan.findings[0].path.as_deref(), Some(purl));
    assert_eq!(scan.findings[0].rule_id, "DSA-5497-2");

    let joined = join(&[record(purl, None)], &[scan], &discharge());
    assert_eq!(joined.records[0].purl, purl);
    assert_eq!(joined.records[0].advisories.len(), 1);
    assert_eq!(joined.records[0].advisories[0].package, purl);
    assert_eq!(joined.records[0].advisories[0].rule_id, "DSA-5497-2");
    assert_eq!(joined.records[0].advisories[0].tool, Tool::OsvScanner);
}

/// Replay one caller-supplied authentic report without copying or normalizing
/// its bytes into this repository. See `tests/fixtures/evidence/README.md`.
#[trace("FR-142-AC-19", "FR-149-AC-10")]
#[ignore = "requires an external authentic report at QUOIN_TEST_OSV_REPORT"]
#[test]
fn tc_osv_scanner_authentic_ecosystem_report_is_parsed_and_joined() {
    let path = std::env::var_os("QUOIN_TEST_OSV_REPORT")
        .map(PathBuf::from)
        .expect("set QUOIN_TEST_OSV_REPORT to an absolute OSV-Scanner report path");
    assert!(
        path.is_absolute(),
        "report path must be absolute: {}",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("caller-supplied report must be readable");
    let raw = std::str::from_utf8(&bytes).expect("OSV-Scanner report must be UTF-8");
    let scan = scan(
        "osv-scanner 2.7.1",
        "osv-authentic-report",
        "2026-11-13T00:00:00.000Z",
        raw,
    );
    let expected = [
        ("pkg:npm/lodash@4.17.20", "GHSA-29mw-wpgm-hmr9"),
        ("pkg:pypi/django@2.2.24", "PYSEC-2021-439"),
        ("pkg:golang/github.com/gin-gonic/gin@v1.7.0", "GO-2021-0052"),
    ];
    for (purl, id) in expected {
        assert!(
            scan.findings
                .iter()
                .any(|finding| finding.path.as_deref() == Some(purl) && finding.rule_id == id),
            "authentic report did not yield {id} at {purl}"
        );
    }

    let records: Vec<_> = expected
        .iter()
        .map(|(purl, _)| record(purl, None))
        .collect();
    let joined = join(&records, &[scan], &discharge());
    assert!(joined.unmatched.is_empty(), "{:?}", joined.unmatched);
    for (purl, id) in expected {
        let attached: Vec<_> = joined
            .records
            .iter()
            .flat_map(|record| &record.advisories)
            .filter(|advisory| advisory.rule_id == id)
            .collect();
        assert_eq!(attached.len(), 1, "{id}");
        assert_eq!(attached[0].package, purl);
        assert_eq!(attached[0].tool, Tool::OsvScanner);
    }
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
