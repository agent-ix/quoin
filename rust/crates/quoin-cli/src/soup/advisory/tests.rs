// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Tests of the advisory join (FR-142).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_assurance::{
    BuildDischargeRequest, DischargeReport, DispositionDecision, build_discharge_report,
};
use quoin_evidence::adapters::{AdapterSelection, parse_cargo_deny, select_finding_adapter};
use quoin_evidence::types::FindingRecord;
use quoin_evidence::{STORE_SCHEMA_VERSION, SuiteId};
use serde_json::{Value, json};

use super::{
    Join, RecordJoin, Resolution, ScanOutcome, Tool, Unmatched, UnmatchedReason, Unresolved, join,
};
use crate::soup::plan::Existing;

pub(in crate::soup) const BEFORE: &str = "2026-07-31T00:00:00.000Z";
pub(in crate::soup) const AFTER: &str = "2026-08-02T00:00:00.000Z";

fn evidence_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/evidence")
}

/// The real `cargo audit --json` output, as a document to extend.
fn audit_document() -> Value {
    let raw = std::fs::read_to_string(evidence_fixtures().join("cargo-audit-real.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

pub(in crate::soup) fn audit_entry(id: &str, name: &str, version: &str) -> Value {
    json!({
        "advisory": { "id": id, "title": format!("{id} title") },
        "package": { "name": name, "version": version }
    })
}

/// The real fixture with its warnings replaced and `vulnerabilities`
/// listing `vulns`; `warnings` is `(kind, entry)` pairs.
pub(in crate::soup) fn audit_text(vulns: &[Value], warnings: &[(&str, Value)]) -> String {
    let mut document = audit_document();
    document["vulnerabilities"] = json!({
        "found": !vulns.is_empty(), "count": vulns.len(), "list": vulns
    });
    let mut by_kind = serde_json::Map::new();
    for (kind, entry) in warnings {
        by_kind
            .entry((*kind).to_owned())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(entry.clone());
    }
    document["warnings"] = Value::Object(by_kind);
    document.to_string()
}

/// One `cargo deny --format json check` diagnostic in the shape of the
/// real `cargo-deny-real.jsonl` fixture.
pub(in crate::soup) fn deny_diagnostic(
    key: Option<(&str, &str)>,
    name: &str,
    version: &str,
    severity: &str,
) -> Value {
    let mut fields = json!({
        "graphs": [{ "Krate": { "name": name, "version": version } }],
        "message": "a diagnostic",
        "severity": severity
    });
    match key {
        Some(("advisory", id)) => fields["advisory"] = json!({ "id": id }),
        Some((_, code)) => fields["code"] = json!(code),
        None => {}
    }
    json!({ "fields": fields, "type": "diagnostic" })
}

pub(in crate::soup) fn deny_text(diagnostics: &[Value]) -> String {
    let mut lines: Vec<String> = diagnostics.iter().map(Value::to_string).collect();
    lines.push(
        json!({ "fields": { "advisories": { "errors": 0, "warnings": 0 } }, "type": "summary" })
            .to_string(),
    );
    lines.join("\n")
}

/// A retained scan, made the way recording makes one: through the adapter
/// the registry selects for `tool`.
pub(in crate::soup) fn scan(tool: &str, suite: &str, timestamp: &str, raw: &str) -> FindingRecord {
    let adapter = select_finding_adapter(AdapterSelection {
        adapter: None,
        tool: Some(tool),
    })
    .unwrap();
    let parsed = adapter.run(raw, None).unwrap();
    FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: tool.to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        ruleset: parsed.ruleset,
        rules_evaluated: parsed.rules_evaluated,
        findings: parsed.findings,
        evaluated: parsed.evaluated,
    }
}

pub(in crate::soup) fn record(purl: &str, clause: Option<&str>) -> Existing {
    Existing {
        path: PathBuf::from(format!("{purl}.md")),
        purl: purl.to_owned(),
        id: None,
        disposition_clause: clause.map(str::to_owned),
        front: serde_json::Map::new(),
    }
}

pub(in crate::soup) fn attestation(at: &str, until: &str) -> Value {
    json!({
        "attestedBy": "reviewer-1", "authority": "quality-lead",
        "attestedAt": at, "expiresAt": until
    })
}

pub(in crate::soup) fn disposition(clause: &str, at: &str, until: &str) -> Value {
    json!({
        "kind": "disposition", "clauseId": clause, "decision": "accepted_risk",
        "owner": "risk-owner-1",
        "rationale": "no reachable path", "approvalRef": "approval://CAB-7",
        "attestation": attestation(at, until)
    })
}

/// A discharge report over one clause set at 2026-08-15: `C-DISP` holds a
/// current disposition attested 2026-08-01, `C-OLD` an expired one,
/// `C-DIRECT` a direct fact, `C-NONE` no fact, `C-NB` does not bind and
/// `C-UNRES` has undecided applicability.
pub(in crate::soup) fn discharge() -> DischargeReport {
    discharge_with(&disposition(
        "C-DISP",
        "2026-08-01T00:00:00.000Z",
        "2026-09-01T00:00:00.000Z",
    ))
}

pub(in crate::soup) fn discharge_with(fact: &Value) -> DischargeReport {
    let binding_clause = |id: &str, outcome: &str, reasons: Value| {
        json!({ "clauseId": id, "force": "mandatory", "outcome": outcome,
                "reasons": reasons, "expectedOutputs": [] })
    };
    let request: BuildDischargeRequest = serde_json::from_value(json!({
        "binding": {
            "schemaVersion": "clause-binding-v1",
            "clauseSet": { "authority": "example.invalid", "id": "soup", "version": "1.0.0" },
            "context": { "product": "widget" },
            "clauses": [
                binding_clause("C-DISP", "binding", json!([])),
                binding_clause("C-OLD", "binding", json!([])),
                binding_clause("C-DIRECT", "binding", json!([])),
                binding_clause("C-NONE", "binding", json!([])),
                binding_clause("C-NB", "not_binding", json!([])),
                binding_clause("C-UNRES", "unresolved", json!([{
                    "code": "unknown-context", "message": "deployment is not declared"
                }]))
            ]
        },
        "facts": [
            fact,
            disposition("C-OLD", "2026-01-01T00:00:00.000Z", "2026-02-01T00:00:00.000Z"),
            {
                "kind": "direct", "clauseId": "C-DIRECT",
                "evidenceRefs": ["evidence://run/one"],
                "attestation": attestation("2026-08-01T00:00:00.000Z", "2026-09-01T00:00:00.000Z")
            }
        ],
        "asOf": "2026-08-15T00:00:00.000Z"
    }))
    .unwrap();
    build_discharge_report(&request).unwrap()
}

fn anyhow_audit(timestamp: &str) -> FindingRecord {
    scan(
        "cargo-audit",
        "audit",
        timestamp,
        &audit_text(
            &[],
            &[(
                "unsound",
                audit_entry("RUSTSEC-2026-0190", "anyhow", "1.0.102"),
            )],
        ),
    )
}

fn only_record<'a>(joined: &'a Join, purl: &str) -> &'a RecordJoin {
    joined.records.iter().find(|r| r.purl == purl).unwrap()
}

fn state(joined: &Join, tool: Tool) -> &ScanOutcome {
    &joined
        .scans
        .iter()
        .find(|s| s.tool == tool)
        .unwrap()
        .outcome
}

/// Distinct advisories and set-aside diagnostics of a scan that was read.
fn read_counts(outcome: &ScanOutcome) -> Option<(usize, usize)> {
    match outcome {
        ScanOutcome::Read {
            advisories,
            set_aside,
            ..
        } => Some((*advisories, *set_aside)),
        _ => None,
    }
}

#[trace("FR-142-AC-1", "FR-142-AC-9")]
#[test]
fn tc_1516_001_a_cargo_audit_advisory_attaches_to_its_crate_only() {
    let records = [
        record("pkg:cargo/anyhow@1.0.102", None),
        record("pkg:cargo/anyhow@1.0.101", None),
        record("pkg:cargo/serde@1.0.228", None),
    ];
    let joined = join(&records, &[anyhow_audit(BEFORE)], &discharge());
    let hit = only_record(&joined, "pkg:cargo/anyhow@1.0.102");
    assert_eq!(hit.advisories.len(), 1);
    assert_eq!(hit.advisories[0].rule_id, "RUSTSEC-2026-0190");
    assert_eq!(hit.advisories[0].package, "anyhow@1.0.102");
    // The scanner's own word, not a class of ours.
    assert_eq!(hit.advisories[0].severity.as_deref(), Some("unsound"));
    assert!(
        only_record(&joined, "pkg:cargo/anyhow@1.0.101")
            .advisories
            .is_empty()
    );
    assert!(
        only_record(&joined, "pkg:cargo/serde@1.0.228")
            .advisories
            .is_empty()
    );
    assert!(joined.unmatched.is_empty());
}

#[trace("FR-142-AC-2")]
#[test]
fn tc_1516_002_cargo_deny_advisories_and_audit_warnings_attach() {
    let deny = scan(
        "cargo-deny",
        "deny",
        BEFORE,
        &deny_text(&[deny_diagnostic(
            Some(("advisory", "RUSTSEC-2024-1")),
            "a",
            "1.0.0",
            "error",
        )]),
    );
    let audit = scan(
        "cargo-audit",
        "audit",
        BEFORE,
        &audit_text(
            &[],
            &[("unmaintained", audit_entry("RUSTSEC-2025-2", "b", "2.0.0"))],
        ),
    );
    let records = [
        record("pkg:cargo/a@1.0.0", None),
        record("pkg:cargo/b@2.0.0", None),
    ];
    let joined = join(&records, &[deny, audit], &discharge());
    let a = &only_record(&joined, "pkg:cargo/a@1.0.0").advisories;
    assert_eq!(
        (a.len(), a[0].tool, a[0].severity.as_deref()),
        (1, Tool::CargoDeny, Some("error"))
    );
    let b = &only_record(&joined, "pkg:cargo/b@2.0.0").advisories;
    assert_eq!(
        (b.len(), b[0].tool, b[0].severity.as_deref()),
        (1, Tool::CargoAudit, Some("unmaintained"))
    );
}

#[trace("FR-142-AC-3")]
#[test]
fn tc_1516_003_unmatched_findings_are_listed_once_and_all_are_counted() {
    let audit = scan(
        "cargo-audit",
        "audit",
        BEFORE,
        &audit_text(
            &[
                audit_entry("RUSTSEC-1", "known", "1.0.0"),
                audit_entry("RUSTSEC-2", "stranger", "3.0.0"),
                audit_entry("RUSTSEC-3", "twice", "1.0.0"),
                // The same finding again counts once.
                audit_entry("RUSTSEC-2", "stranger", "3.0.0"),
            ],
            &[(
                "yanked",
                json!({
                    "advisory": { "id": "RUSTSEC-4" },
                    "package": { "name": "unversioned" }
                }),
            )],
        ),
    );
    let records = [
        record("pkg:cargo/known@1.0.0", None),
        record("pkg:cargo/twice@1.0.0", None),
        record("pkg:cargo/twice@1.0.0?arch=x86", None),
        record("pkg:cargo/unversioned@1.0.0", None),
        record("pkg:cargo/bystander@2.0.0", None),
    ];
    let joined = join(&records, &[audit], &discharge());
    let reasons: Vec<(&str, Option<&str>, UnmatchedReason)> = joined
        .unmatched
        .iter()
        .map(|u: &Unmatched| (u.rule_id.as_str(), u.package.as_deref(), u.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            (
                "RUSTSEC-2",
                Some("stranger@3.0.0"),
                UnmatchedReason::NoRecord
            ),
            ("RUSTSEC-3", Some("twice@1.0.0"), UnmatchedReason::Ambiguous),
            ("RUSTSEC-4", Some("unversioned"), UnmatchedReason::NoVersion),
        ]
    );
    assert_eq!(UnmatchedReason::Ambiguous.to_string(), "ambiguous");
    let attached: usize = joined.records.iter().map(|r| r.advisories.len()).sum();
    let (advisories, _) = read_counts(state(&joined, Tool::CargoAudit)).unwrap();
    assert_eq!(attached + joined.unmatched.len(), advisories);
    assert_eq!(advisories, 4);
    // An ambiguous finding could be either record, so neither is free of it.
    assert!(
        only_record(&joined, "pkg:cargo/twice@1.0.0")
            .free_of
            .is_empty()
    );
    // A finding naming its crate without a version could be that crate's record.
    assert!(
        only_record(&joined, "pkg:cargo/unversioned@1.0.0")
            .free_of
            .is_empty()
    );
    // And no other record.
    assert_eq!(
        only_record(&joined, "pkg:cargo/bystander@2.0.0").free_of,
        [Tool::CargoAudit]
    );
}

#[trace("FR-142-AC-3", "FR-142-AC-4")]
#[test]
fn tc_1516_012_a_finding_with_no_path_could_belong_to_every_record() {
    let mut audit = scan("cargo-audit", "audit", BEFORE, &audit_text(&[], &[]));
    audit
        .findings
        .push(quoin_evidence::Finding::new("RUSTSEC-9"));
    let records = [
        record("pkg:cargo/a@1.0.0", None),
        record("pkg:cargo/b@1.0.0", None),
    ];
    let joined = join(&records, &[audit], &discharge());
    assert_eq!(joined.unmatched.len(), 1);
    assert_eq!(joined.unmatched[0].package, None);
    assert_eq!(joined.unmatched[0].reason, UnmatchedReason::NoVersion);
    assert_eq!(UnmatchedReason::NoVersion.to_string(), "no version");
    assert!(joined.records.iter().all(|r| r.free_of.is_empty()));
}

#[trace("FR-142-AC-4", "FR-142-AC-5")]
#[test]
fn tc_1516_004_not_run_is_not_clean() {
    let records = [record("pkg:cargo/a@1.0.0", None)];
    let none = join(&records, &[], &discharge());
    let clean = join(
        &records,
        &[scan("cargo-audit", "audit", BEFORE, &audit_text(&[], &[]))],
        &discharge(),
    );
    assert_eq!(*state(&none, Tool::CargoAudit), ScanOutcome::NotRun);
    assert!(
        clean
            .scans
            .iter()
            .any(|s| s.tool == Tool::CargoAudit && s.outcome.is_clean())
    );
    assert!(!state(&none, Tool::CargoAudit).is_clean());
    assert!(none.records[0].free_of.is_empty());
    assert_eq!(clean.records[0].free_of, [Tool::CargoAudit]);
    assert_ne!(none, clean);

    // A deny log whose summary counts errors its diagnostics do not show is
    // refused at recording (FR-034-AC-25), so no scan of it is retained: the
    // join over the scans then retained shows cargo-deny not run.
    let refused = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quoin-evidence/tests/fixtures/cargo-deny-failed-real.jsonl"),
    )
    .unwrap();
    let mut retained = vec![scan("cargo-audit", "audit", BEFORE, &audit_text(&[], &[]))];
    if let Ok(parsed) = parse_cargo_deny(&refused) {
        retained.push(FindingRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: SuiteId::new("deny"),
            tool: "cargo-deny".to_owned(),
            evidence_kind: None,
            timestamp: BEFORE.to_owned(),
            ruleset: parsed.ruleset,
            rules_evaluated: parsed.rules_evaluated,
            findings: parsed.findings,
            evaluated: parsed.evaluated,
        });
    }
    assert_eq!(retained.len(), 1, "the refused log leaves no record");
    let after_refusal = join(&records, &retained, &discharge());
    assert_eq!(*state(&after_refusal, Tool::CargoDeny), ScanOutcome::NotRun);
    assert!(
        after_refusal
            .records
            .iter()
            .all(|r| !r.free_of.contains(&Tool::CargoDeny))
    );
}

#[trace("FR-142-AC-6")]
#[test]
fn tc_1516_005_a_scan_that_evaluated_no_rules_proves_nothing() {
    let mut vacuous = scan("cargo-audit", "audit", BEFORE, &audit_text(&[], &[]));
    vacuous.rules_evaluated = Some(0);
    let joined = join(
        &[record("pkg:cargo/a@1.0.0", None)],
        &[vacuous],
        &discharge(),
    );
    assert!(matches!(
        state(&joined, Tool::CargoAudit),
        ScanOutcome::Vacuous { .. }
    ));
    assert!(!state(&joined, Tool::CargoAudit).is_clean());
    assert!(joined.records[0].free_of.is_empty());
}

fn resolved_for(clause: &str, scan_at: &str) -> Resolution {
    let records = [record("pkg:cargo/anyhow@1.0.102", Some(clause))];
    let joined = join(&records, &[anyhow_audit(scan_at)], &discharge());
    joined.records[0].advisories[0].resolution.clone()
}

#[trace("FR-142-AC-7")]
#[test]
fn tc_1516_006_a_current_disposition_at_or_after_the_scan_resolves() {
    let found = match resolved_for("C-DISP", BEFORE) {
        Resolution::Resolved(found) => Some(found),
        Resolution::Unresolved(_) => None,
    }
    .unwrap();
    assert_eq!(found.decision, DispositionDecision::AcceptedRisk);
    assert_eq!(found.rationale, "no reachable path");
    assert_eq!(found.approval_ref, "approval://CAB-7");
    assert_eq!(found.attested_by, "reviewer-1");
    // At the same instant is still resolved.
    assert!(matches!(
        resolved_for("C-DISP", "2026-08-01T00:00:00.000Z"),
        Resolution::Resolved(_)
    ));
}

#[trace("FR-142-AC-8")]
#[test]
fn tc_1516_007_each_way_a_disposition_fails_names_itself() {
    let why = |clause: &str| {
        match resolved_for(clause, BEFORE) {
            Resolution::Unresolved(reason) => Some(reason),
            Resolution::Resolved(_) => None,
        }
        .unwrap()
    };
    assert!(matches!(why("C-OLD"), Unresolved::NotCurrent(text) if text.contains("expired")));
    assert_eq!(why("C-NONE"), Unresolved::NoFact);
    assert_eq!(why("C-DIRECT"), Unresolved::NotApprovedDisposition);
    assert!(matches!(why("C-NB"), Unresolved::ClauseDoesNotBind(_)));
    assert!(matches!(
        why("C-UNRES"),
        Unresolved::ClauseDoesNotBind(text) if text.contains("deployment is not declared")
    ));
    assert_eq!(why("C-ABSENT"), Unresolved::ClauseNotInBindingReport);

    let records = [record("pkg:cargo/anyhow@1.0.102", None)];
    let joined = join(&records, &[anyhow_audit(BEFORE)], &discharge());
    assert_eq!(
        joined.records[0].advisories[0].resolution,
        Resolution::Unresolved(Unresolved::NoDispositionClause)
    );
    assert_eq!(
        Unresolved::NoDispositionClause.to_string(),
        "no disposition clause"
    );
}

#[trace("FR-142-AC-12")]
#[test]
fn tc_1516_008_a_newer_scan_unresolves_every_advisory_until_attested_again() {
    let audit_two = |at: &str| {
        scan(
            "cargo-audit",
            "audit",
            at,
            &audit_text(
                &[
                    audit_entry("RUSTSEC-A", "x", "1.0.0"),
                    audit_entry("RUSTSEC-B", "x", "1.0.0"),
                ],
                &[],
            ),
        )
    };
    let records = [record("pkg:cargo/x@1.0.0", Some("C-DISP"))];
    // Accepted for A against the earlier scan, which reports A alone.
    let earlier = scan(
        "cargo-audit",
        "audit",
        BEFORE,
        &audit_text(&[audit_entry("RUSTSEC-A", "x", "1.0.0")], &[]),
    );
    let accepted = join(&records, std::slice::from_ref(&earlier), &discharge());
    assert_eq!(accepted.records[0].advisories.len(), 1);
    assert!(matches!(
        accepted.records[0].advisories[0].resolution,
        Resolution::Resolved(_)
    ));
    // A newer scan that also reports B unresolves both.
    let newer = join(&records, &[earlier, audit_two(AFTER)], &discharge());
    assert_eq!(newer.records[0].advisories.len(), 2);
    for advisory in &newer.records[0].advisories {
        assert_eq!(
            advisory.resolution,
            Resolution::Unresolved(Unresolved::ScanNewerThanAttestation)
        );
    }
    assert_eq!(
        Unresolved::ScanNewerThanAttestation.to_string(),
        "scan newer than attestation"
    );
    // Attesting again after the scan resolves both.
    let again = discharge_with(&disposition(
        "C-DISP",
        "2026-08-03T00:00:00.000Z",
        "2026-09-01T00:00:00.000Z",
    ));
    let renewed = join(&records, &[audit_two(AFTER)], &again);
    assert!(
        renewed.records[0]
            .advisories
            .iter()
            .all(|a| matches!(a.resolution, Resolution::Resolved(_)))
    );
}

/// A source finding and an `unclassified` one are set aside and counted; a
/// licence or ban finding is neither an advisory nor set aside: the policy join
/// reads it (FR-142-AC-28).
#[trace("FR-142-AC-13")]
#[test]
fn tc_1516_009_deny_diagnostics_that_are_not_advisories_are_set_aside() {
    let deny = scan(
        "cargo-deny",
        "deny",
        BEFORE,
        &deny_text(&[
            deny_diagnostic(Some(("code", "rejected")), "a", "1.0.0", "error"),
            deny_diagnostic(Some(("code", "banned")), "a", "1.0.0", "error"),
            deny_diagnostic(Some(("code", "source-not-allowed")), "a", "1.0.0", "error"),
            deny_diagnostic(None, "a", "1.0.0", "warning"),
            deny_diagnostic(
                Some(("advisory", "GHSA-xxxx-yyyy-zzzz")),
                "a",
                "1.0.0",
                "error",
            ),
            deny_diagnostic(Some(("advisory", "CVE-2026-1")), "a", "1.0.0", "error"),
        ]),
    );
    let records = [record("pkg:cargo/a@1.0.0", None)];
    let joined = join(&records, std::slice::from_ref(&deny), &discharge());
    // Two advisories; the source and the unclassified finding are set aside;
    // the `rejected` and `banned` ones are in neither count.
    assert_eq!(read_counts(state(&joined, Tool::CargoDeny)), Some((2, 2)));
    assert_eq!(joined.records[0].advisories.len(), 2);
    assert!(joined.unmatched.is_empty());
    let policies = crate::soup::policy::join(&records, &[deny], &discharge());
    let attached: Vec<&str> = policies.records[&records[0].path]
        .findings
        .iter()
        .map(|f| f.rule_id.as_str())
        .collect();
    assert_eq!(attached, ["banned", "rejected"]);
}

#[trace("FR-142-AC-14", "FR-142-AC-10", "FR-142-CON-1")]
#[test]
fn tc_1516_010_only_the_crate_selects_and_no_record_file_changes() {
    let directory = tempfile::tempdir().unwrap();
    let body = |purl: &str, extra: &str| {
        format!(
            "---\nid: SOUP-1\ntype: soup_record\npurl: \"{purl}\"\nname: anyhow\n\
             disposition_clause: C-DISP\n{extra}---\n# body\n"
        )
    };
    let chosen = directory.path().join("anyhow.md");
    let other = directory.path().join("other.md");
    std::fs::write(
        &chosen,
        body(
            "pkg:cargo/anyhow@1.0.102",
            "finding_refs: [\"RUSTSEC-2026-9999\"]\n",
        ),
    )
    .unwrap();
    // An authored vulnerability object whose `affects` edge names the record.
    let vulnerability = directory.path().join("vulnerability.md");
    std::fs::write(
        &vulnerability,
        "---\nid: VULN-1\ntype: vulnerability\nrelationships:\n  - target: \"ix://agent-ix/x/SOUP-1\"\n    type: \"affects\"\n---\n# VULN-1\n",
    )
    .unwrap();
    std::fs::write(
        &other,
        body(
            "pkg:cargo/serde@1.0.228",
            "finding_refs: [\"RUSTSEC-2026-0190\"]\n",
        ),
    )
    .unwrap();
    let before = (
        std::fs::read(&chosen).unwrap(),
        std::fs::read(&other).unwrap(),
    );
    let records = crate::soup::read_records(directory.path()).ok().unwrap();

    // The same finding twice, as a re-run may report it.
    let mut doubled = anyhow_audit(BEFORE);
    doubled.findings.extend(doubled.findings.clone());
    let joined = join(&records, &[doubled], &discharge());

    assert_eq!(
        only_record(&joined, "pkg:cargo/anyhow@1.0.102")
            .advisories
            .len(),
        1
    );
    assert!(
        only_record(&joined, "pkg:cargo/serde@1.0.228")
            .advisories
            .is_empty()
    );
    assert!(joined.unmatched.is_empty());
    assert_eq!(
        joined.records.len(),
        2,
        "the vulnerability file is not a record"
    );
    assert_eq!(
        before,
        (
            std::fs::read(&chosen).unwrap(),
            std::fs::read(&other).unwrap()
        )
    );
}

#[trace("FR-142-AC-15")]
#[test]
fn tc_1516_011_only_the_scan_with_the_newer_timestamp_is_read() {
    let old = scan(
        "cargo-audit",
        "zzz-newest-name",
        BEFORE,
        &audit_text(&[audit_entry("RUSTSEC-OLD", "a", "1.0.0")], &[]),
    );
    let new = scan(
        "cargo-audit",
        "aaa-oldest-name",
        AFTER,
        &audit_text(&[audit_entry("RUSTSEC-NEW", "a", "1.0.0")], &[]),
    );
    let records = [record("pkg:cargo/a@1.0.0", None)];
    for scans in [[old.clone(), new.clone()], [new, old]] {
        let joined = join(&records, &scans, &discharge());
        let ids: Vec<&str> = joined.records[0]
            .advisories
            .iter()
            .map(|a| a.rule_id.as_str())
            .collect();
        assert_eq!(ids, ["RUSTSEC-NEW"]);
    }
}

#[trace("FR-142-AC-16")]
#[test]
fn tc_1516_013_a_purl_path_matches_by_purl_ignoring_qualifiers() {
    let mut audit = anyhow_audit(BEFORE);
    audit.findings[0].path = Some("pkg:cargo/anyhow@1.0.102?arch=x86#sub".to_owned());
    let joined = join(
        &[record("pkg:cargo/anyhow@1.0.102", None)],
        &[audit],
        &discharge(),
    );
    assert_eq!(joined.records[0].advisories.len(), 1);
}

#[trace("FR-142-AC-15")]
#[test]
fn tc_1516_014_two_scans_at_one_instant_read_the_suite_that_sorts_last() {
    let first = scan(
        "cargo-audit",
        "a-suite",
        BEFORE,
        &audit_text(&[audit_entry("RUSTSEC-A", "a", "1.0.0")], &[]),
    );
    // The same instant spelled in another zone.
    let last = scan(
        "cargo-audit",
        "b-suite",
        "2026-07-31T02:00:00.000+02:00",
        &audit_text(&[audit_entry("RUSTSEC-B", "a", "1.0.0")], &[]),
    );
    let records = [record("pkg:cargo/a@1.0.0", None)];
    for scans in [[first.clone(), last.clone()], [last, first]] {
        let joined = join(&records, &scans, &discharge());
        let ids: Vec<&str> = joined.records[0]
            .advisories
            .iter()
            .map(|a| a.rule_id.as_str())
            .collect();
        assert_eq!(ids, ["RUSTSEC-B"]);
        // The scan of the other suite is not read, and the state says so.
        assert_eq!(joined.scans[0].unread, ["a-suite"]);
    }
}

#[trace("FR-142-AC-17")]
#[test]
fn tc_1516_015_a_timestamp_that_is_not_an_instant_reads_no_scan_of_the_tool() {
    let good = scan("cargo-audit", "good", BEFORE, &audit_text(&[], &[]));
    let bad = scan(
        "cargo-audit",
        "bad",
        "2026-11-12 10:00",
        &audit_text(&[audit_entry("RUSTSEC-A", "a", "1.0.0")], &[]),
    );
    let joined = join(
        &[record("pkg:cargo/a@1.0.0", None)],
        &[good, bad],
        &discharge(),
    );
    assert_eq!(
        *state(&joined, Tool::CargoAudit),
        ScanOutcome::Unordered {
            suites: vec!["bad".to_owned()]
        }
    );
    assert!(!state(&joined, Tool::CargoAudit).is_clean());
    assert!(joined.records[0].free_of.is_empty());
    assert!(joined.records[0].advisories.is_empty());
    assert!(joined.unmatched.is_empty());
}

#[trace("FR-142-AC-18")]
#[test]
fn tc_1516_016_one_advisory_from_both_tools_is_attached_per_tool_against_the_newest_scan() {
    let audit = scan(
        "cargo-audit",
        "audit",
        BEFORE,
        &audit_text(&[audit_entry("RUSTSEC-X", "a", "1.0.0")], &[]),
    );
    let deny = scan(
        "cargo-deny",
        "deny",
        AFTER,
        &deny_text(&[deny_diagnostic(
            Some(("advisory", "RUSTSEC-X")),
            "a",
            "1.0.0",
            "error",
        )]),
    );
    let records = [record("pkg:cargo/a@1.0.0", Some("C-DISP"))];
    // Attested 2026-08-01: after the audit scan, before the deny scan.
    let joined = join(&records, &[audit, deny], &discharge());
    let advisories = &joined.records[0].advisories;
    assert_eq!(advisories.len(), 2);
    assert_eq!(
        advisories.iter().map(|a| a.tool).collect::<Vec<_>>(),
        [Tool::CargoAudit, Tool::CargoDeny]
    );
    assert!(
        advisories.iter().all(|a| {
            a.resolution == Resolution::Unresolved(Unresolved::ScanNewerThanAttestation)
        })
    );
}
