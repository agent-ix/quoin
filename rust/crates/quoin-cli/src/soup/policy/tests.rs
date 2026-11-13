// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Tests of the policy-finding join (FR-142, NFR-030).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use ix_trace_rs::trace;
use quoin_evidence::adapters::parse_licence_policy;
use quoin_evidence::types::FindingRecord;
use quoin_evidence::{STORE_SCHEMA_VERSION, SuiteId};

use super::{LicenceOutcome, PolicyJoin, PolicyStatus, PolicyTool, join};
use crate::soup::advisory::tests::{
    AFTER, BEFORE, audit_entry, audit_text, deny_diagnostic, deny_text, discharge, discharge_with,
    disposition, record, scan,
};
use crate::soup::advisory::{self, Resolution, ScanOutcome, Tool, UnmatchedReason, Unresolved};
use crate::soup::plan::Existing;

const SYFT: &str =
    include_str!("../../../../quoin-evidence/tests/fixtures/licence-npm.syft.cdx.json");
const POLICY: &str = "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\nban:\n  - purl: pkg:npm/fx-vale\n";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quoin-evidence/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// A retained `licence-policy` scan, made the way recording makes one.
fn licence_scan(suite: &str, timestamp: &str, sbom: &str, policy: &str) -> FindingRecord {
    let parsed = parse_licence_policy(sbom, policy).unwrap();
    FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: "licence-policy".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        ruleset: parsed.ruleset,
        rules_evaluated: parsed.rules_evaluated,
        findings: parsed.findings,
        evaluated: parsed.evaluated,
    }
}

fn npm_records(clause: Option<&str>) -> Vec<Existing> {
    [
        "ember@0.4.1",
        "lumen@1.2.0",
        "moss@0.9.0",
        "quill@2.0.0",
        "vale@3.1.0",
    ]
    .iter()
    .map(|v| record(&format!("pkg:npm/fx-{v}"), clause))
    .collect()
}

fn of<'a>(joined: &'a PolicyJoin, records: &[Existing], purl: &str) -> &'a super::RecordPolicy {
    let record = records.iter().find(|r| r.purl == purl).unwrap();
    &joined.records[&record.path]
}

fn rules(policy: &super::RecordPolicy) -> Vec<&str> {
    policy.findings.iter().map(|f| f.rule_id.as_str()).collect()
}

/// Each finding of a `licence-policy` scan over the real Syft capture attaches
/// to the record with its own purl, carrying rule id, severity word and message
/// unchanged; the allowed component's record has none.
#[trace("FR-142-AC-29", "FR-144-AC-18")]
#[test]
fn tc_1671_020_each_licence_policy_finding_attaches_to_its_own_record() {
    let records = npm_records(None);
    let scans = [licence_scan("lic", BEFORE, SYFT, POLICY)];
    let joined = join(&records, &scans, &discharge());

    let ember = of(&joined, &records, "pkg:npm/fx-ember@0.4.1");
    assert_eq!(rules(ember), ["licence-denied"]);
    assert_eq!(ember.findings[0].severity.as_deref(), Some("denied"));
    assert_eq!(ember.findings[0].message.as_deref(), Some("GPL-3.0-only"));
    assert_eq!(ember.findings[0].tool, PolicyTool::LicencePolicy);
    assert_eq!(
        rules(of(&joined, &records, "pkg:npm/fx-moss@0.9.0")),
        ["licence-unknown"]
    );
    assert_eq!(
        rules(of(&joined, &records, "pkg:npm/fx-quill@2.0.0")),
        ["licence-not-allowed"]
    );
    assert_eq!(
        rules(of(&joined, &records, "pkg:npm/fx-vale@3.1.0")),
        ["licence-unknown", "package-banned"]
    );
    let lumen = of(&joined, &records, "pkg:npm/fx-lumen@1.2.0");
    assert!(lumen.findings.is_empty());
    assert!(matches!(lumen.status, PolicyStatus::NoFindings { .. }));
    assert!(joined.unmatched.is_empty());
}

/// Not run, then a clean scan that lists the record, then a refused recording
/// that leaves the earlier scan to be read with its timestamp and ruleset.
#[trace("FR-142-AC-30", "NFR-030-AC-5")]
#[test]
fn tc_1671_021_not_run_clean_and_refused_recordings_are_distinct() {
    let records = npm_records(None);
    let none = join(&records, &[], &discharge());
    assert_eq!(none.licence.outcome, LicenceOutcome::NotRun);
    assert!(
        none.records
            .values()
            .all(|r| r.status == PolicyStatus::NotRun)
    );

    let clean_policy = "format: licence-policy-v1\nallow: [MIT, Apache-2.0, GPL-3.0-only, \"MIT OR BSD-3-Clause\"]\n";
    let earlier = licence_scan("lic", BEFORE, SYFT, clean_policy);
    // Moss declares nothing, so the scan is not clean for it; the other four are.
    let read = join(&records, std::slice::from_ref(&earlier), &discharge());
    let lumen = of(&read, &records, "pkg:npm/fx-lumen@1.2.0");
    let PolicyStatus::NoFindings { timestamp, ruleset } = &lumen.status else {
        panic!("{:?}", lumen.status)
    };
    assert_eq!(timestamp, BEFORE);
    assert!(ruleset.as_deref().unwrap().contains("Apache-2.0"));
    assert_ne!(none, read);

    // A refused policy, and a refused SBOM, retain nothing: the earlier scan
    // is still the newest scan, read with its own timestamp and ruleset.
    assert!(parse_licence_policy(SYFT, "allow: [MIT]").is_err());
    assert!(parse_licence_policy("{}", clean_policy).is_err());
    let after_refusal = join(&records, std::slice::from_ref(&earlier), &discharge());
    assert_eq!(after_refusal, read);
}

/// An unknown licence is a finding, unresolved without a disposition, resolved
/// by one attested at or after the scan and `scan newer than attestation` when
/// attested before it. No record is ever stated allowed.
#[trace("FR-142-AC-31", "FR-142-AC-8", "FR-142-AC-24")]
#[test]
fn tc_1671_022_licence_unknown_is_resolved_only_by_a_disposition_after_its_scan() {
    let without = npm_records(None);
    let joined = join(
        &without,
        &[licence_scan("lic", BEFORE, SYFT, POLICY)],
        &discharge(),
    );
    let moss = of(&joined, &without, "pkg:npm/fx-moss@0.9.0");
    assert_eq!(moss.status, PolicyStatus::Findings);
    assert_eq!(
        moss.findings[0].resolution,
        Resolution::Unresolved(Unresolved::NoDispositionClause)
    );

    let with = npm_records(Some("C-DISP"));
    let resolved = join(
        &with,
        &[licence_scan("lic", BEFORE, SYFT, POLICY)],
        &discharge(),
    );
    let Resolution::Resolved(disposition) =
        &of(&resolved, &with, "pkg:npm/fx-moss@0.9.0").findings[0].resolution
    else {
        panic!("not resolved")
    };
    assert_eq!(disposition.attested_by, "reviewer-1");
    assert_eq!(disposition.approval_ref, "approval://CAB-7");

    let later = join(
        &with,
        &[licence_scan("lic", AFTER, SYFT, POLICY)],
        &discharge(),
    );
    assert_eq!(
        of(&later, &with, "pkg:npm/fx-moss@0.9.0").findings[0].resolution,
        Resolution::Unresolved(Unresolved::ScanNewerThanAttestation)
    );
}

/// A finding whose purl matches no record, and one with no purl, are each
/// listed once; attached plus unmatched equals the distinct findings read. A
/// record an unmatched no-purl finding could belong to is not stated clean.
#[trace("FR-142-AC-32", "FR-144-AC-19")]
#[test]
fn tc_1671_023_unmatched_policy_findings_are_listed_and_block_a_clean_statement() {
    let sbom = r#"{"bomFormat":"CycloneDX","components":[
        {"name":"ghost","version":"1","purl":"pkg:npm/ghost@1"},
        {"name":"nopurl","version":"2"},
        {"name":"lumen","version":"1.2.0","purl":"pkg:npm/fx-lumen@1.2.0","licenses":[{"license":{"id":"GPL-3.0-only"}}]}]}"#;
    let records = npm_records(None);
    let joined = join(
        &records,
        &[licence_scan("lic", BEFORE, sbom, POLICY)],
        &discharge(),
    );
    let reasons: Vec<(String, UnmatchedReason)> = joined
        .unmatched
        .iter()
        .map(|u| (u.package.clone().unwrap(), u.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            ("nopurl@2".to_owned(), UnmatchedReason::NoPurl),
            ("pkg:npm/ghost@1".to_owned(), UnmatchedReason::NoRecord),
        ]
    );
    let LicenceOutcome::Read { findings, .. } = joined.licence.outcome else {
        panic!("not read")
    };
    let attached: usize = joined.records.values().map(|r| r.findings.len()).sum();
    assert_eq!(attached + joined.unmatched.len(), findings);
    // Not in `evaluated` anyway, but the no-purl finding also blocks the
    // evaluated record: the three others are not evaluated at all.
    let ember = of(&joined, &records, "pkg:npm/fx-ember@0.4.1");
    assert_eq!(ember.status, PolicyStatus::NotEvaluated);
    assert_eq!(UnmatchedReason::NoPurl.to_string(), "no purl");

    let evaluated_clean = r#"{"bomFormat":"CycloneDX","components":[
        {"name":"ember","version":"0.4.1","purl":"pkg:npm/fx-ember@0.4.1","licenses":[{"license":{"id":"MIT"}}]},
        {"name":"nopurl","version":"2"}]}"#;
    let blocked = join(
        &records,
        &[licence_scan("lic", BEFORE, evaluated_clean, POLICY)],
        &discharge(),
    );
    assert_eq!(
        of(&blocked, &records, "pkg:npm/fx-ember@0.4.1").status,
        PolicyStatus::Blocked
    );
}

/// A vacuous scan and an unordered one each show no record clean, each with its
/// own reason.
#[trace("FR-142-AC-33", "NFR-030-AC-5", "FR-144-AC-19")]
#[test]
fn tc_1671_024_vacuous_and_unordered_scans_clear_no_record() {
    let records = npm_records(None);
    let vacuous = licence_scan(
        "lic",
        BEFORE,
        r#"{"bomFormat":"CycloneDX","components":[]}"#,
        POLICY,
    );
    let joined = join(&records, &[vacuous], &discharge());
    assert!(matches!(
        joined.licence.outcome,
        LicenceOutcome::Vacuous { .. }
    ));
    assert!(
        joined
            .records
            .values()
            .all(|r| r.status == PolicyStatus::Vacuous)
    );

    let good = licence_scan("lic-a", BEFORE, SYFT, POLICY);
    let broken = licence_scan("lic-b", "not an instant", SYFT, POLICY);
    let joined = join(&records, &[good, broken], &discharge());
    assert!(matches!(
        joined.licence.outcome,
        LicenceOutcome::Unordered { .. }
    ));
    assert!(
        joined
            .records
            .values()
            .all(|r| r.status == PolicyStatus::Unordered)
    );
    assert!(joined.unmatched.is_empty());
}

/// Policy findings are a list of their own: the advisory join counts none of
/// them, and the policy join attaches no advisory.
#[trace("FR-142-AC-34")]
#[test]
fn tc_1671_025_policy_findings_and_advisories_are_never_merged() {
    let records = vec![record("pkg:cargo/a@1.0.0", None)];
    let deny = scan(
        "cargo-deny",
        "deny",
        BEFORE,
        &deny_text(&[
            deny_diagnostic(Some(("advisory", "RUSTSEC-2026-1")), "a", "1.0.0", "error"),
            deny_diagnostic(Some(("code", "rejected")), "a", "1.0.0", "error"),
        ]),
    );
    let scans = [deny];
    let advisories = advisory::join(&records, &scans, &discharge());
    let policies = join(&records, &scans, &discharge());
    assert_eq!(advisories.records[0].advisories.len(), 1);
    assert_eq!(
        advisories.records[0].advisories[0].rule_id,
        "RUSTSEC-2026-1"
    );
    assert_eq!(rules(&policies.records[&records[0].path]), ["rejected"]);
    let state = advisories
        .scans
        .iter()
        .find(|s| s.tool == Tool::CargoDeny)
        .unwrap();
    let ScanOutcome::Read {
        advisories: count,
        set_aside,
        ..
    } = state.outcome
    else {
        panic!("not read")
    };
    assert_eq!((count, set_aside), (1, 0));
    assert_eq!(policies.deny_read, Some(1));
}

/// A record promoted after the scan is absent from `evaluated` and is stated
/// not evaluated, never as having no policy findings.
#[trace("FR-142-AC-35", "NFR-030-AC-5", "FR-144-AC-19")]
#[test]
fn tc_1671_026_a_record_absent_from_evaluated_is_not_evaluated() {
    let mut records = npm_records(None);
    records.push(record("pkg:npm/later-promoted@9.9.9", None));
    let joined = join(
        &records,
        &[licence_scan("lic", BEFORE, SYFT, POLICY)],
        &discharge(),
    );
    assert_eq!(
        of(&joined, &records, "pkg:npm/later-promoted@9.9.9").status,
        PolicyStatus::NotEvaluated
    );
}

/// One disposition covers the findings of scans at or before its attestation: an
/// advisory scanned before it stays resolved while a ban finding from a later
/// `licence-policy` scan is unresolved; attesting again resolves both.
#[trace("FR-142-AC-36", "FR-144-AC-20")]
#[test]
fn tc_1671_027_a_later_policy_scan_is_not_covered_by_an_earlier_attestation() {
    let records = vec![record("pkg:npm/fx-vale@3.1.0", Some("C-DISP"))];
    let advisory_scan = scan(
        "cargo-audit",
        "audit",
        BEFORE,
        &audit_text(
            &[],
            &[(
                "unsound",
                audit_entry("RUSTSEC-2026-0190", "fx-vale", "3.1.0"),
            )],
        ),
    );
    // The advisory is matched by crate name, so give it a cargo record too.
    let records = [
        records,
        vec![record("pkg:cargo/fx-vale@3.1.0", Some("C-DISP"))],
    ]
    .concat();
    let scans = [advisory_scan, licence_scan("lic", AFTER, SYFT, POLICY)];

    let joined = advisory::join(&records, &scans, &discharge());
    assert!(matches!(
        joined
            .records
            .iter()
            .find(|r| r.purl.starts_with("pkg:cargo"))
            .unwrap()
            .advisories[0]
            .resolution,
        Resolution::Resolved(_)
    ));
    let policies = join(&records, &scans, &discharge());
    let vale = of(&policies, &records, "pkg:npm/fx-vale@3.1.0");
    let ban = vale
        .findings
        .iter()
        .find(|f| f.rule_id == "package-banned")
        .unwrap();
    assert_eq!(
        ban.resolution,
        Resolution::Unresolved(Unresolved::ScanNewerThanAttestation)
    );

    let attested_again = discharge_with(&disposition(
        "C-DISP",
        "2026-08-03T00:00:00.000Z",
        "2026-09-01T00:00:00.000Z",
    ));
    let policies = join(&records, &scans, &attested_again);
    let vale = of(&policies, &records, "pkg:npm/fx-vale@3.1.0");
    assert!(
        vale.findings
            .iter()
            .all(|f| matches!(f.resolution, Resolution::Resolved(_)))
    );
}

/// Real cargo-deny output: `rejected`, `unlicensed`, `banned` and
/// `not-allowed` diagnostics attach to their `pkg:cargo/` record as policy
/// findings, resolved by the disposition rule; `duplicate`, `wildcard`,
/// `license-not-encountered`, `unmatched-skip` and the other codes are set aside
/// and counted. The fixtures are cargo-deny 0.19.8's own output, unedited.
#[trace("FR-142-AC-28")]
#[test]
fn tc_1671_028_real_cargo_deny_licence_and_ban_codes_are_policy_findings() {
    let records = vec![
        record("pkg:cargo/itoa@1.0.19", Some("C-DISP")),
        record("pkg:cargo/denyscratch@0.1.0", None),
        record("pkg:cargo/nolic@0.1.0", None),
        record("pkg:cargo/bitflags@1.3.2", None),
    ];
    let cases = [
        (
            "cargo-deny-licences-real.jsonl",
            ["rejected", "unlicensed", "unlicensed"].as_slice(),
            5,
        ),
        ("cargo-deny-real.jsonl", ["banned"].as_slice(), 0),
        (
            "cargo-deny-not-allowed-real.jsonl",
            ["not-allowed", "not-allowed"].as_slice(),
            0,
        ),
        ("cargo-deny-duplicate-real.jsonl", [].as_slice(), 1),
    ];
    for (name, expected, set_aside) in cases {
        let scans = [scan("cargo-deny", "deny", BEFORE, &fixture(name))];
        let policies = join(&records, &scans, &discharge());
        let mut found: Vec<&str> = policies
            .records
            .values()
            .flat_map(|r| r.findings.iter().map(|f| f.rule_id.as_str()))
            .collect();
        found.sort_unstable();
        assert_eq!(found, expected, "{name}");
        assert!(policies.unmatched.is_empty(), "{name}");
        let advisories = advisory::join(&records, &scans, &discharge());
        let state = advisories
            .scans
            .iter()
            .find(|s| s.tool == Tool::CargoDeny)
            .unwrap();
        let ScanOutcome::Read {
            advisories: count,
            set_aside: aside,
            ..
        } = state.outcome
        else {
            panic!("not read")
        };
        assert_eq!((count, aside), (0, set_aside), "{name}");
    }
    // Resolved by the disposition rule: itoa's `rejected` finding, scanned
    // before the attestation, under C-DISP.
    let scans = [scan(
        "cargo-deny",
        "deny",
        BEFORE,
        &fixture("cargo-deny-licences-real.jsonl"),
    )];
    let policies = join(&records, &scans, &discharge());
    let itoa = of(&policies, &records, "pkg:cargo/itoa@1.0.19");
    assert!(matches!(
        itoa.findings[0].resolution,
        Resolution::Resolved(_)
    ));
    assert_eq!(itoa.findings[0].severity.as_deref(), Some("error"));
}
