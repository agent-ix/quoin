// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `matrix.build` cause validation and preservation (FR-115-AC-17..25).
//!
//! The auditor sets an optional typed `cause` on `stale-evidence` findings
//! (FR-032-AC-18..21), and `matrix.build` copies it verbatim, validates it, and
//! orders multiple stale findings by it (FR-115-AC-17..25).
//!
//! Provenance: PLAT-1760

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_assurance::matrix::{EvidenceStatus, InvalidStaleCause, MatrixInput, MatrixRefusal, build};
use serde_json::{Value, json};

fn criterion(id: &str) -> Value {
    json!({ "id": id, "method": "Test", "binders": [], "status": "untagged" })
}

fn binding(obligation: &str, suite: &str) -> Value {
    json!({
        "obligation": obligation,
        "suite": suite,
        "symbols": ["tc_1"],
    })
}

fn audit(healthy: &[&str], findings: &[Value], unevaluated: &[Value]) -> Value {
    json!({
        "healthy": healthy,
        "findings": findings,
        "unevaluated": unevaluated,
    })
}

fn input(ids: &[&str], bindings: &[Value], audit: &Value) -> MatrixInput {
    let criteria: Vec<Value> = ids.iter().map(|id| criterion(id)).collect();
    serde_json::from_value(json!({
        "coverage": [{ "document": "spec/FR-001.md", "criteria": criteria }],
        "bindings": { "schemaVersion": 1, "bindings": bindings },
        "audit": audit,
    }))
    .unwrap()
}

/// An audit whose `stale-evidence` finding carries `cause` `missing-run` yields
/// an `evidence_detail.findings` item with `cause` `missing-run` and
/// `evidence_status` `stale` (FR-115-AC-17).
#[trace("FR-115-AC-17")]
#[test]
fn tc_1760_missing_run_cause_preserved() {
    let finding = json!({
        "kind": "stale-evidence",
        "obligation": "FR-001-AC-1",
        "severity": "high",
        "summary": "no recorded run",
        "cause": "missing-run",
    });
    let matrix_input = input(
        &["FR-001-AC-1"],
        &[binding("FR-001-AC-1", "unit")],
        &audit(&[], &[finding], &[]),
    );

    let matrix = build(&matrix_input).expect("valid input succeeds");

    let criterion = &matrix.requirements[0].criteria[0];
    assert_eq!(criterion.evidence_status, EvidenceStatus::Stale);
    let finding = &criterion.evidence_detail.findings[0];
    assert_eq!(finding.kind, "stale-evidence");
    // Serialize and check the JSON has the cause field
    let finding_json = serde_json::to_value(finding).expect("serializes");
    assert_eq!(
        finding_json["cause"], "missing-run",
        "cause must be preserved: {finding_json}"
    );
}

/// An audit whose `stale-evidence` finding carries `cause` `failed-run` yields
/// an `evidence_detail.findings` item with `cause` `failed-run` and
/// `evidence_status` `stale` (FR-115-AC-18).
#[trace("FR-115-AC-18")]
#[test]
fn tc_1760_failed_run_cause_preserved() {
    let finding = json!({
        "kind": "stale-evidence",
        "obligation": "FR-001-AC-2",
        "severity": "high",
        "summary": "failing run",
        "cause": "failed-run",
    });
    let matrix_input = input(
        &["FR-001-AC-2"],
        &[binding("FR-001-AC-2", "unit")],
        &audit(&[], &[finding], &[]),
    );

    let matrix = build(&matrix_input).expect("valid input succeeds");

    let criterion = &matrix.requirements[0].criteria[0];
    assert_eq!(criterion.evidence_status, EvidenceStatus::Stale);
    let finding = &criterion.evidence_detail.findings[0];
    let finding_json = serde_json::to_value(finding).expect("serializes");
    assert_eq!(
        finding_json["cause"], "failed-run",
        "cause must be preserved: {finding_json}"
    );
}

/// A criterion in `audit.healthy`, with its mock inspection recorded, is `bound`
/// and carries no `stale-evidence` finding and no `cause` (FR-115-AC-19).
#[trace("FR-115-AC-19")]
#[test]
fn tc_1760_bound_has_no_cause() {
    let matrix_input = input(
        &["FR-001-AC-3"],
        &[binding("FR-001-AC-3", "unit")],
        &audit(&["FR-001-AC-3"], &[], &[]),
    );

    let matrix = build(&matrix_input).expect("valid input succeeds");

    let criterion = &matrix.requirements[0].criteria[0];
    assert_eq!(criterion.evidence_status, EvidenceStatus::Bound);
    assert!(criterion.evidence_detail.findings.is_empty());
}

/// A passing but uninspected suite (undischarged through unevaluated mock check)
/// has no `stale-evidence` finding and no `cause` field (FR-115-AC-20).
#[trace("FR-115-AC-20")]
#[test]
fn tc_1760_uninspected_suite_has_no_cause() {
    let unevaluated = json!({
        "check": "mocked-confirmation",
        "obligation": "FR-001-AC-3b",
        "suites": ["unit"],
        "reason": "no current mock inspection",
    });
    let matrix_input = input(
        &["FR-001-AC-3b"],
        &[binding("FR-001-AC-3b", "unit")],
        &audit(&[], &[], &[unevaluated]),
    );

    let matrix = build(&matrix_input).expect("valid input succeeds");

    let criterion = &matrix.requirements[0].criteria[0];
    assert_eq!(criterion.evidence_status, EvidenceStatus::Undischarged);
    assert!(
        criterion.evidence_detail.findings.is_empty(),
        "no stale-evidence finding for uninspected suite"
    );
    // Verify no finding has a cause
    for finding in &criterion.evidence_detail.findings {
        let finding_json = serde_json::to_value(finding).expect("serializes");
        assert!(
            finding_json.get("cause").is_none(),
            "uninspected suite finding must have no cause: {finding_json}"
        );
    }
}

/// An `evidence_detail.findings` item whose `kind` is not `stale-evidence` has
/// no `cause` key in the JSON output, not `null` and not empty (FR-115-AC-21).
#[trace("FR-115-AC-21")]
#[test]
fn tc_1760_non_stale_has_no_cause_key() {
    let finding = json!({
        "kind": "undischarged",
        "obligation": "FR-001-AC-4",
        "severity": "medium",
        "summary": "no evidence",
    });
    let matrix_input = input(
        &["FR-001-AC-4"],
        &[],
        &audit(&[], &[finding], &[]),
    );

    let matrix = build(&matrix_input).expect("valid input succeeds");

    let criterion = &matrix.requirements[0].criteria[0];
    assert_eq!(criterion.evidence_status, EvidenceStatus::Undischarged);
    let finding = &criterion.evidence_detail.findings[0];
    let finding_json = serde_json::to_value(finding).expect("serializes");
    assert!(
        finding_json.get("cause").is_none(),
        "the cause key must not appear: {finding_json}"
    );
}

/// An `audit` finding of kind `stale-evidence` with `cause` outside
/// `missing-run`/`failed-run` is refused with `Refused` and `context.reason`
/// `invalid-stale-cause` naming the obligation (FR-115-AC-22).
#[trace("FR-115-AC-22")]
#[test]
fn tc_1760_invalid_cause_on_stale_refused() {
    let finding = json!({
        "kind": "stale-evidence",
        "obligation": "FR-001-AC-5",
        "severity": "high",
        "summary": "test",
        "cause": "invalid-value",
    });
    let matrix_input = input(
        &["FR-001-AC-5"],
        &[binding("FR-001-AC-5", "unit")],
        &audit(&[], &[finding], &[]),
    );

    // The input parses successfully (unknown cause deserializes to Invalid)
    // but matrix.build refuses the Invalid sentinel
    let result = build(&matrix_input);
    assert!(result.is_err(), "invalid cause value must be refused");
    let err = result.unwrap_err();
    match err {
        MatrixRefusal::InvalidStaleCause(invalid) => {
            assert_eq!(
                invalid.obligation, "FR-001-AC-5",
                "error must name the offending obligation"
            );
        }
        _ => panic!("expected InvalidStaleCause, got {:?}", err),
    }
}

/// An `audit` finding of a kind other than `stale-evidence` that carries a
/// `cause` is refused with `InvalidStaleCause` naming the obligation (FR-115-AC-23).
///
/// The core-level mapping to `Refused` with `context.reason` `invalid-stale-cause`
/// is verified through the quoin-core crate's own tests.
#[trace("FR-115-AC-23")]
#[test]
fn tc_1760_cause_on_non_stale_refused() {
    let finding = json!({
        "kind": "undischarged",
        "obligation": "FR-001-AC-6",
        "severity": "medium",
        "summary": "no evidence",
        "cause": "missing-run",
    });
    let matrix_input = input(
        &["FR-001-AC-6"],
        &[],
        &audit(&[], &[finding], &[]),
    );

    let result = build(&matrix_input);
    assert!(result.is_err(), "a cause on a non-stale finding must be refused");
    let err = result.unwrap_err();
    match err {
        MatrixRefusal::InvalidStaleCause(invalid) => {
            assert_eq!(
                invalid.obligation, "FR-001-AC-6",
                "error must name the offending obligation"
            );
        }
        _ => panic!("expected InvalidStaleCause, got {:?}", err),
    }
}

/// An obligation with a `missing-run` and a `failed-run` `stale-evidence`
/// finding lists both in `evidence_detail.findings`, `failed-run` first,
/// whatever order `audit.findings` supplies them, and is `stale` (FR-115-AC-24).
#[trace("FR-115-AC-24")]
#[test]
fn tc_1760_multiple_stale_findings_ordered_by_cause() {
    let finding_missing = json!({
        "kind": "stale-evidence",
        "obligation": "FR-001-AC-7",
        "severity": "high",
        "summary": "no run",
        "cause": "missing-run",
    });
    let finding_failed = json!({
        "kind": "stale-evidence",
        "obligation": "FR-001-AC-7",
        "severity": "high",
        "summary": "failing run",
        "cause": "failed-run",
    });
    // Supply them in reverse order to verify sorting
    let matrix_input = input(
        &["FR-001-AC-7"],
        &[binding("FR-001-AC-7", "unit")],
        &audit(&[], &[finding_missing.clone(), finding_failed.clone()], &[]),
    );

    let matrix = build(&matrix_input).expect("valid input succeeds");

    let criterion = &matrix.requirements[0].criteria[0];
    assert_eq!(criterion.evidence_status, EvidenceStatus::Stale);
    assert_eq!(
        criterion.evidence_detail.findings.len(),
        2,
        "both stale findings are listed"
    );
    // Verify failed-run comes first
    let findings_json = serde_json::to_value(&criterion.evidence_detail.findings).expect("serializes");
    assert_eq!(
        findings_json[0]["cause"], "failed-run",
        "failed-run must come first: {findings_json}"
    );
    assert_eq!(
        findings_json[1]["cause"], "missing-run",
        "missing-run must come second: {findings_json}"
    );
}

/// Two `matrix.build` calls over one fixture that differ only in whether the
/// `stale-evidence` findings carry `cause` produce identical `evidence_status`
/// values, `summary` text, `severity`, finding order, `bindings` lists,
/// `unevaluated` lists, and healthy sets (FR-115-AC-25).
#[trace("FR-115-AC-25")]
#[test]
fn tc_1760_cause_does_not_affect_status_or_other_fields() {
    let finding_with_cause = json!({
        "kind": "stale-evidence",
        "obligation": "FR-001-AC-8",
        "severity": "high",
        "summary": "no run",
        "cause": "missing-run",
    });
    let finding_without_cause = json!({
        "kind": "stale-evidence",
        "obligation": "FR-001-AC-8",
        "severity": "high",
        "summary": "no run",
    });

    let input_with = input(
        &["FR-001-AC-8"],
        &[binding("FR-001-AC-8", "unit")],
        &audit(&[], &[finding_with_cause], &[]),
    );
    let input_without = input(
        &["FR-001-AC-8"],
        &[binding("FR-001-AC-8", "unit")],
        &audit(&[], &[finding_without_cause], &[]),
    );

    let matrix_with = build(&input_with).expect("valid input succeeds");
    let matrix_without = build(&input_without).expect("valid input succeeds");

    let criterion_with = &matrix_with.requirements[0].criteria[0];
    let criterion_without = &matrix_without.requirements[0].criteria[0];

    // Evidence status unchanged
    assert_eq!(
        criterion_with.evidence_status, criterion_without.evidence_status,
        "evidence_status must be identical"
    );

    // Finding details unchanged
    assert_eq!(
        criterion_with.evidence_detail.findings.len(),
        criterion_without.evidence_detail.findings.len(),
        "finding count must be identical"
    );
    assert_eq!(
        criterion_with.evidence_detail.findings[0].summary,
        criterion_without.evidence_detail.findings[0].summary,
        "summary must be identical"
    );
    assert_eq!(
        criterion_with.evidence_detail.findings[0].severity,
        criterion_without.evidence_detail.findings[0].severity,
        "severity must be identical"
    );

    // Other evidence detail unchanged
    assert_eq!(
        criterion_with.evidence_detail.bindings,
        criterion_without.evidence_detail.bindings,
        "bindings must be identical"
    );
    assert_eq!(
        criterion_with.evidence_detail.unevaluated,
        criterion_without.evidence_detail.unevaluated,
        "unevaluated must be identical"
    );

    // Healthy set unchanged
    assert_eq!(
        matrix_with.requirements[0].healthy_count,
        matrix_without.requirements[0].healthy_count,
        "healthy count must be identical"
    );
}
