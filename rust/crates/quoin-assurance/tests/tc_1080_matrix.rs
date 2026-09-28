// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin_assurance::matrix`: the evidence status mapping, its precedence,
//! the total evidence detail, the grouping and the markdown render (FR-115).
//!
//! Inputs are written in the wire shapes the three producers emit
//! (`coverage_matrix`, the persisted `BindingsFile`, the `AuditReport`), so a
//! field renamed on either side fails to deserialize here rather than reading
//! as an empty column.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_assurance::matrix::{
    EMPTY_REASON, EvidenceStatus, MatrixInput, MatrixOutput, build, render_markdown,
};
use serde_json::{Value, json};

fn criterion(id: &str) -> Value {
    json!({ "id": id, "statement": "s", "method": "Test", "binders": [], "status": "untagged" })
}

fn binding(obligation: &str, suite: &str, commit: &str) -> Value {
    json!({
        "obligation": obligation,
        "statementHashAtBinding": "h",
        "suite": suite,
        "commit": commit,
        "symbols": ["tc_1"],
    })
}

fn finding(obligation: &str, kind: &str) -> Value {
    json!({ "kind": kind, "obligation": obligation, "severity": "high", "summary": format!("{kind} on {obligation}") })
}

fn unevaluated(obligation: &str, check: &str) -> Value {
    json!({ "check": check, "obligation": obligation, "suites": ["unit"], "reason": "r" })
}

/// A request over `ids` in one document, with the given bindings and audit.
fn input(ids: &[&str], bindings: &[Value], audit: &Value) -> MatrixInput {
    let criteria: Vec<Value> = ids.iter().map(|id| criterion(id)).collect();
    serde_json::from_value(json!({
        "coverage": [{ "document": "spec/FR-001.md", "criteria": criteria }],
        "bindings": { "schemaVersion": 1, "bindings": bindings },
        "audit": audit,
    }))
    .unwrap()
}

fn audit(findings: &[Value], healthy: &[&str], checks: &[Value]) -> Value {
    json!({ "findings": findings, "healthy": healthy, "unevaluated": checks })
}

/// Every criterion's evidence status, in output order.
fn statuses(output: &MatrixOutput) -> Vec<(String, EvidenceStatus)> {
    output
        .requirements
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .map(|criterion| (criterion.id.clone(), criterion.evidence_status))
        .collect()
}

/// One status for one criterion carrying `findings`, in a populated store.
fn status_with(findings: &[&str], healthy: bool, checks: &[&str]) -> EvidenceStatus {
    let id = "FR-001-AC-1";
    let findings: Vec<Value> = findings.iter().map(|kind| finding(id, kind)).collect();
    let checks: Vec<Value> = checks.iter().map(|check| unevaluated(id, check)).collect();
    let healthy: &[&str] = if healthy { &[id] } else { &[] };
    let output = build(&input(
        &[id],
        &[binding(id, "unit", "a")],
        &audit(&findings, healthy, &checks),
    ))
    .unwrap();
    statuses(&output)[0].1
}

/// An empty binding graph reads `no run evidence` for every criterion,
/// whatever the audit says — and the detail is still computed.
///
/// Trace: FR-115-AC-3, TC-1966
#[test]
fn tc_1080_001_an_empty_store_is_no_run_evidence_and_still_carries_detail() {
    let output = build(&input(
        &["FR-001-AC-1", "FR-001-AC-2"],
        &[],
        &audit(
            &[finding("FR-001-AC-1", "undischarged")],
            &["FR-001-AC-2"],
            &[],
        ),
    ))
    .unwrap();
    assert_eq!(
        statuses(&output),
        [
            ("FR-001-AC-1".to_owned(), EvidenceStatus::NoRunEvidence),
            ("FR-001-AC-2".to_owned(), EvidenceStatus::NoRunEvidence),
        ]
    );
    let detail = &output.requirements[0].criteria[0].evidence_detail;
    assert_eq!(
        detail.findings.len(),
        1,
        "the override must not suppress detail"
    );
    assert_eq!(detail.findings[0].kind, "undischarged");
    let payload = serde_json::to_value(&output).unwrap();
    assert_eq!(
        payload["requirements"][0]["criteria"][0]["evidence_status"],
        "no run evidence"
    );
}

/// Each finding kind lands in the bucket the requirement assigns it, alone.
///
/// Trace: FR-115-AC-4, TC-1967
#[test]
fn tc_1080_002_every_finding_kind_maps_to_its_status() {
    for kind in [
        "suspect-link",
        "mocked-confirmation",
        "insufficient-independence",
        "vacuous-evidence",
    ] {
        assert_eq!(
            status_with(&[kind], false, &[]),
            EvidenceStatus::Suspect,
            "{kind}"
        );
    }
    assert_eq!(
        status_with(&["stale-evidence"], false, &[]),
        EvidenceStatus::Stale
    );
    for kind in [
        "undischarged",
        "unknown-method",
        "method-conformance",
        "insufficient-multiplicity",
        "insufficient-mutation-score",
        "unmeasured-mutation-score",
        "combinatorial-gap",
        "a-kind-the-auditor-adds-later",
    ] {
        assert_eq!(
            status_with(&[kind], false, &[]),
            EvidenceStatus::Undischarged,
            "{kind}"
        );
    }
}

/// Precedence: bound > suspect > stale > undischarged, and an unevaluated
/// entry or a total absence is undischarged.
///
/// Trace: FR-115-AC-4, TC-1967
#[test]
fn tc_1080_003_precedence_is_bound_then_suspect_then_stale_then_undischarged() {
    assert_eq!(status_with(&[], true, &[]), EvidenceStatus::Bound);
    // Vacuity behind a stale run renders suspect, not the milder stale.
    assert_eq!(
        status_with(&["stale-evidence", "vacuous-evidence"], false, &[]),
        EvidenceStatus::Suspect
    );
    assert_eq!(
        status_with(&["suspect-link", "undischarged"], false, &[]),
        EvidenceStatus::Suspect
    );
    assert_eq!(
        status_with(&["unknown-method", "stale-evidence"], false, &[]),
        EvidenceStatus::Stale
    );
    assert_eq!(
        status_with(&["stale-evidence"], false, &["mocked-confirmation"]),
        EvidenceStatus::Stale
    );
    assert_eq!(
        status_with(&[], false, &["mocked-confirmation"]),
        EvidenceStatus::Undischarged
    );
    assert_eq!(status_with(&[], false, &[]), EvidenceStatus::Undischarged);

    // No binding for this criterion while the store holds one for another.
    let output = build(&input(
        &["FR-001-AC-1", "FR-001-AC-2"],
        &[binding("FR-001-AC-1", "unit", "a")],
        &audit(&[], &["FR-001-AC-1"], &[]),
    ))
    .unwrap();
    assert_eq!(statuses(&output)[1].1, EvidenceStatus::Undischarged);
}

/// An id both healthy and in a finding, or in an unevaluated check, refuses
/// the whole build and names the first such id.
///
/// Trace: FR-115-AC-5, FR-115-CON-4, TC-1968
#[test]
fn tc_1080_004_a_contradictory_audit_is_refused_not_resolved() {
    let with_finding = build(&input(
        &["FR-001-AC-1"],
        &[binding("FR-001-AC-1", "unit", "a")],
        &audit(
            &[
                finding("FR-001-AC-2", "suspect-link"),
                finding("FR-001-AC-1", "suspect-link"),
            ],
            &["FR-001-AC-1", "FR-001-AC-2"],
            &[],
        ),
    ))
    .unwrap_err();
    assert_eq!(with_finding.obligation, "FR-001-AC-1");

    let with_check = build(&input(
        &["FR-001-AC-1"],
        &[],
        &audit(
            &[],
            &["FR-009-AC-1"],
            &[unevaluated("FR-009-AC-1", "mocked-confirmation")],
        ),
    ))
    .unwrap_err();
    assert_eq!(
        with_check.obligation, "FR-009-AC-1",
        "an id outside the matrix and an empty store still refuse"
    );
}

/// `static_status` and `binders` are copied verbatim and never move with the
/// evidence axis; an absent `method` stays absent rather than `null`.
///
/// Trace: FR-115-AC-7, TC-1970
#[test]
fn tc_1080_005_the_static_axis_is_verbatim_and_method_is_omitted_not_null() {
    let binder = json!({
        "path": "tests/a.rs", "line": 3, "column": 1,
        "qualified_name": "tc_1_a", "kind": "function", "ignored": true,
    });
    let request: MatrixInput = serde_json::from_value(json!({
        "coverage": [{ "document": "spec/FR-001.md", "criteria": [
            { "id": "FR-001-AC-1", "statement": "s", "binders": [binder], "status": "tagged-by-ignored-test" },
        ]}],
        "bindings": { "schemaVersion": 1, "bindings": [binding("FR-001-AC-1", "unit", "a")] },
        "audit": audit(&[], &["FR-001-AC-1"], &[]),
    }))
    .unwrap();
    let payload = serde_json::to_value(build(&request).unwrap()).unwrap();
    let row = &payload["requirements"][0]["criteria"][0];
    assert_eq!(row["static_status"], "tagged-by-ignored-test");
    assert_eq!(row["binders"], json!([binder]));
    assert_eq!(row["evidence_status"], "bound");
    assert!(
        row.as_object().unwrap().get("method").is_none(),
        "an absent method must be omitted, never null: {row}"
    );
}

/// Criteria regroup by requirement id, not by document: one document holding
/// two requirements' criteria yields two entries, in requirement-id byte
/// order, each keeping the matrix's own relative criterion order.
///
/// Trace: FR-115-AC-8, TC-1971
#[test]
fn tc_1080_006_criteria_group_by_requirement_not_by_document() {
    let request: MatrixInput = serde_json::from_value(json!({
        "coverage": [
            { "document": "spec/mixed.md", "criteria": [
                criterion("FR-010-AC-2"), criterion("FR-002-AC-9"), criterion("FR-010-AC-1"),
            ]},
            { "document": "spec/other.md", "criteria": [criterion("FR-002-AC-1")] },
        ],
        "bindings": { "schemaVersion": 1, "bindings": [] },
        "audit": audit(&[], &[], &[]),
    }))
    .unwrap();
    let output = build(&request).unwrap();
    let grouped: Vec<(&str, Vec<&str>)> = output
        .requirements
        .iter()
        .map(|requirement| {
            (
                requirement.id.as_str(),
                requirement
                    .criteria
                    .iter()
                    .map(|criterion| criterion.id.as_str())
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        grouped,
        [
            ("FR-002", vec!["FR-002-AC-9", "FR-002-AC-1"]),
            ("FR-010", vec!["FR-010-AC-2", "FR-010-AC-1"]),
        ]
    );
}

/// The detail is total and ordered: all three lists non-empty for one
/// criterion, all three empty for one reported nowhere.
///
/// Trace: FR-115-AC-9, TC-1972
#[test]
fn tc_1080_007_evidence_detail_is_total_and_ordered() {
    let id = "FR-001-AC-1";
    let located = json!({
        "kind": "stale-evidence", "obligation": id, "summary": "behind",
        "path": "tests/a.rs", "line": 4, "symbol": "tc_1",
    });
    let output = build(&input(
        &[id, "FR-001-AC-2"],
        &[
            binding(id, "unit", "b"),
            binding(id, "bench", "c"),
            binding(id, "unit", "a"),
        ],
        &audit(
            &[
                finding(id, "unknown-method"),
                located,
                finding(id, "combinatorial-gap"),
                finding(id, "vacuous-evidence"),
                finding(id, "insufficient-independence"),
            ],
            &[],
            &[
                unevaluated(id, "z-check"),
                unevaluated(id, "mocked-confirmation"),
            ],
        ),
    ))
    .unwrap();
    let payload = serde_json::to_value(&output).unwrap();
    let full = &payload["requirements"][0]["criteria"][0]["evidence_detail"];
    let kinds: Vec<&str> = full["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        [
            "insufficient-independence",
            "vacuous-evidence",
            "stale-evidence",
            "combinatorial-gap",
            "unknown-method",
        ]
    );
    assert_eq!(
        full["findings"][2],
        json!({ "kind": "stale-evidence", "summary": "behind", "path": "tests/a.rs", "line": 4, "symbol": "tc_1" })
    );
    assert_eq!(
        full["bindings"],
        json!([
            { "suite": "bench", "commit": "c" },
            { "suite": "unit", "commit": "a" },
            { "suite": "unit", "commit": "b" },
        ])
    );
    assert_eq!(
        full["unevaluated"],
        json!(["mocked-confirmation", "z-check"])
    );

    let nowhere = &payload["requirements"][0]["criteria"][1]["evidence_detail"];
    assert_eq!(
        nowhere,
        &json!({ "findings": [], "bindings": [], "unevaluated": [] }),
        "a criterion reported nowhere carries three empty lists, never absent keys"
    );
}

/// An absent coverage matrix renders the reason, never an empty table.
///
/// Trace: FR-115-AC-11, TC-1974
#[test]
fn tc_1080_008_an_absent_matrix_renders_its_reason() {
    let request: MatrixInput = serde_json::from_value(json!({
        "bindings": { "schemaVersion": 1, "bindings": [] },
        "audit": audit(&[], &[], &[]),
    }))
    .unwrap();
    let output = build(&request).unwrap();
    assert!(output.requirements.is_empty());
    assert_eq!(output.reason.as_deref(), Some(EMPTY_REASON));
    let rendered = render_markdown(&output);
    assert_eq!(rendered, format!("# Test Matrix\n\n{EMPTY_REASON}\n"));
    assert!(!rendered.contains('|'), "no table for an empty population");
}

/// The markdown render: one `##` section per requirement, the six columns,
/// and a detail cell a `|` inside a summary cannot break.
///
/// Trace: FR-115-AC-11, TC-1974
#[test]
fn tc_1080_009_markdown_renders_one_table_per_requirement() {
    let output = build(&input(
        &["FR-001-AC-1", "FR-001-AC-2"],
        &[binding("FR-001-AC-1", "unit", "abc")],
        &audit(
            &[json!({ "kind": "stale-evidence", "obligation": "FR-001-AC-1", "summary": "a|b" })],
            &[],
            &[],
        ),
    ))
    .unwrap();
    assert_eq!(
        render_markdown(&output),
        "# Test Matrix\n\
         \n\
         ## FR-001\n\
         \n\
         | Requirement | Criterion | Method | Static Status | Evidence Status | Detail |\n\
         |---|---|---|---|---|---|\n\
         | FR-001 | FR-001-AC-1 | Test | untagged | stale | stale-evidence: a\\|b; bound by unit@abc |\n\
         | FR-001 | FR-001-AC-2 | Test | untagged | undischarged | — |\n"
    );
}

/// `matrix` performs no file, network or subprocess I/O: the shared source
/// auditor finds no host capability in the module, so every input can only
/// have arrived as an argument. `quoin-core`'s `ops/matrix.rs` shell is held to
/// the same rule by `tc_373_the_library_half_names_no_host_capability`.
///
/// Trace: FR-115-CON-1, TC-1979
#[test]
fn tc_1080_010_the_matrix_module_holds_no_host_capability() {
    use engineering_assurance::source_audit::{
        RustSourceAuditRole, RustSourceFindingCategory, audit_rust_source,
    };
    let source = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/src/matrix.rs")).unwrap();
    assert!(
        String::from_utf8_lossy(&source).contains("pub fn build("),
        "the audit must read the module that builds the matrix"
    );
    let findings = audit_rust_source(&source, RustSourceAuditRole::ReusableLibrary).unwrap();
    let capabilities: Vec<String> = findings
        .iter()
        .filter(|finding| finding.category() == RustSourceFindingCategory::ForbiddenCapability)
        .map(|finding| format!("{:?}", finding.capability()))
        .collect();
    assert_eq!(capabilities, Vec::<String>::new());
}
