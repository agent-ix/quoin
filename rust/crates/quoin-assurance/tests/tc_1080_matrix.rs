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

use ix_trace_rs::trace;
use quoin_assurance::matrix::{
    EMPTY_REASON, EvidenceStatus, MatrixInput, MatrixOutput, build, render_markdown,
};
use serde_json::{Value, json};

fn criterion(id: &str) -> Value {
    json!({ "id": id, "statement": "s", "method": "Test", "binders": [], "status": "untagged" })
}

fn binding(obligation: &str, suite: &str) -> Value {
    json!({
        "obligation": obligation,
        "suite": suite,
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

fn design_input(entries: &[Value]) -> MatrixInput {
    serde_json::from_value(json!({
        "design_coverage": entries,
        "bindings": { "schemaVersion": 1, "bindings": [] },
        "audit": audit(&[], &[], &[]),
    }))
    .unwrap()
}

fn design_criterion(id: &str, elements: Option<Value>) -> Value {
    let mut criterion = criterion(id);
    criterion.as_object_mut().unwrap().remove("statement");
    if let Some(elements) = elements {
        criterion
            .as_object_mut()
            .unwrap()
            .insert("design_elements".to_owned(), elements);
    }
    criterion
}

fn design_entry(
    document: &str,
    id: Option<&str>,
    criteria: Vec<Value>,
    design: Option<Value>,
) -> Value {
    let mut entry = json!({ "document": document, "criteria": criteria });
    if let Some(id) = id {
        entry["id"] = json!(id);
    }
    if let Some(design) = design {
        entry["design"] = design;
    }
    entry
}

fn covered_block(elements: Value) -> Value {
    json!({ "relation": "realized-by", "check": "implemented", "state": "covered", "elements": elements, "dangling": [], "kind_mismatch": [] })
}

fn uncovered_block(check: &str) -> Value {
    json!({ "relation": "realized-by", "check": check, "state": "uncovered", "elements": [], "dangling": [], "kind_mismatch": [] })
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
        &[binding(id, "unit")],
        &audit(&findings, healthy, &checks),
    ))
    .unwrap();
    statuses(&output)[0].1
}

#[trace("FR-115-AC-16", "FR-153-AC-12")]
#[test]
fn analysis_evidence_joins_only_the_exact_target_and_renders_every_item() {
    let target = json!({
        "source": "obligations",
        "document": "spec/FR-001.md",
        "id": "FR-001-AC-1",
        "origin": "authored-row"
    });
    let other_target = json!({
        "source": "obligations",
        "document": "spec/FR-002.md",
        "id": "FR-001-AC-1",
        "origin": "authored-row"
    });
    let record_binder = json!({
        "target": target.clone(),
        "locator": {"record":"runs/SUITE-A/a.json", "run":"SUITE-A@a", "entry":"consistency::x"},
        "method":"Analysis",
        "actor":"test-signer",
        "provenance":["test-provenance"]
    });
    let wrong_target_binder = json!({
        "target": other_target,
        "locator": {"record":"runs/SUITE-B/b.json", "run":"SUITE-B@b", "entry":"consistency::y"},
        "method":"Analysis",
        "actor":"other-signer",
        "provenance":["other-provenance"]
    });
    let result = json!({
        "target": target.clone(),
        "requested_target":"FR-001-AC-1",
        "locator":{"record":"runs/SUITE-C/c.json", "run":"SUITE-C@c", "entry":"consistency::z"},
        "cause":"unevaluated",
        "detail":"actor authority premise: no admission"
    });
    let request_value = json!({
        "coverage": [{
            "document": "spec/FR-001.md",
            "criteria": [{
                "id": "FR-001-AC-1",
                "method": "Analysis",
                "binders": [],
                "status": "tagged",
                "identity": target
            }]
        }],
        "bindings": {"schemaVersion": 1, "bindings": [binding("FR-001-AC-1", "unit")]},
        "audit": audit(&[], &["FR-001-AC-1"], &[]),
        "analysis_evidence": {
            "record_binders": [record_binder, wrong_target_binder],
            "evidence_results": [result]
        }
    });
    let request = serde_json::from_value(request_value).unwrap();

    let output = build(&request).unwrap();
    let criterion = &output.requirements[0].criteria[0];
    let items = criterion.analysis_evidence.as_ref().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["status"], "qualified-record");
    assert_eq!(items[0]["record_binder"]["actor"], "test-signer");
    assert_eq!(items[1]["status"], "unevaluated");
    assert_eq!(
        items[1]["result"]["detail"],
        "actor authority premise: no admission"
    );
    assert_eq!(criterion.evidence_status, EvidenceStatus::Bound);
    let rendered = render_markdown(&output);
    assert!(rendered.contains("test-signer"));
    assert!(rendered.contains("actor authority premise: no admission"));
}

/// An empty binding graph reads `no run evidence` for every criterion,
/// whatever the audit says — and the detail is still computed.
///
#[trace("FR-115-AC-3")]
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
#[trace("FR-115-AC-4")]
#[test]
fn tc_1080_002_every_finding_kind_maps_to_its_status() {
    for kind in [
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
        "surviving-mutant",
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
#[trace("FR-115-AC-4")]
#[test]
fn tc_1080_003_precedence_is_bound_then_suspect_then_stale_then_undischarged() {
    assert_eq!(status_with(&[], true, &[]), EvidenceStatus::Bound);
    // Vacuity behind a stale run renders suspect, not the milder stale.
    assert_eq!(
        status_with(&["stale-evidence", "vacuous-evidence"], false, &[]),
        EvidenceStatus::Suspect
    );
    assert_eq!(
        status_with(&["vacuous-evidence", "undischarged"], false, &[]),
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
        &[binding("FR-001-AC-1", "unit")],
        &audit(&[], &["FR-001-AC-1"], &[]),
    ))
    .unwrap();
    assert_eq!(statuses(&output)[1].1, EvidenceStatus::Undischarged);
}

/// An id both healthy and in a finding, or in an unevaluated check, refuses
/// the whole build and names the first such id.
///
#[trace("FR-115-AC-5", "FR-115-CON-4")]
#[test]
fn tc_1080_004_a_contradictory_audit_is_refused_not_resolved() {
    let with_finding = build(&input(
        &["FR-001-AC-1"],
        &[binding("FR-001-AC-1", "unit")],
        &audit(
            &[
                finding("FR-001-AC-2", "vacuous-evidence"),
                finding("FR-001-AC-1", "vacuous-evidence"),
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
#[trace("FR-115-AC-7")]
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
        "bindings": { "schemaVersion": 1, "bindings": [binding("FR-001-AC-1", "unit")] },
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
#[trace("FR-115-AC-8")]
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
#[trace("FR-115-AC-9")]
#[test]
fn tc_1080_007_evidence_detail_is_total_and_ordered() {
    let id = "FR-001-AC-1";
    let located = json!({
        "kind": "stale-evidence", "obligation": id, "summary": "behind",
        "path": "tests/a.rs", "line": 4, "symbol": "tc_1",
    });
    let output = build(&input(
        &[id, "FR-001-AC-2"],
        &[binding(id, "unit"), binding(id, "bench")],
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
            { "suite": "bench" },
            { "suite": "unit" },
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
#[trace("FR-115-AC-11")]
#[test]
fn tc_1080_008_an_absent_matrix_renders_its_reason() {
    let request: MatrixInput = serde_json::from_value(json!({
        "bindings": { "schemaVersion": 1, "bindings": [] },
        "audit": audit(&[], &[], &[]),
    }))
    .unwrap();
    let output = build(&request).unwrap();
    assert_eq!(output.requirements.len(), 0);
    assert_eq!(output.reason.as_deref(), Some(EMPTY_REASON));
    let rendered = render_markdown(&output);
    assert_eq!(rendered, format!("# Test Matrix\n\n{EMPTY_REASON}\n"));
    assert!(!rendered.contains('|'), "no table for an empty population");
}

/// The markdown render: one `##` section per requirement, the six columns,
/// and a detail cell a `|` inside a summary cannot break.
///
#[trace("FR-115-AC-11")]
#[test]
fn tc_1080_009_markdown_renders_one_table_per_requirement() {
    let output = build(&input(
        &["FR-001-AC-1", "FR-001-AC-2"],
        &[binding("FR-001-AC-1", "unit")],
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
         | FR-001 | FR-001-AC-1 | Test | untagged | stale | stale-evidence: a\\|b; bound by unit |\n\
         | FR-001 | FR-001-AC-2 | Test | untagged | undischarged | — |\n"
    );
}

/// Design blocks and per-criterion references survive the matrix join unchanged.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-1", "FR-165-AC-2")]
#[test]
fn tc_1080_011_design_data_is_carried_verbatim() {
    let blocks = json!([
        uncovered_block("implemented"),
        covered_block(json!([
            { "id": "IMPL-1", "kind": "Component", "path": "src/lib.rs", "verb": "implements", "realized_criteria": [] }
        ]))
    ]);
    let references = json!([{ "id": "IMPL-1" }]);
    let output = build(&design_input(&[design_entry(
        "spec/FR-001.md",
        Some("FR-001"),
        vec![design_criterion("FR-001-AC-1", Some(references.clone()))],
        Some(blocks.clone()),
    )]))
    .unwrap();
    assert_eq!(
        serde_json::to_value(&output).unwrap()["requirements"][0]["design"],
        blocks
    );
    assert_eq!(
        serde_json::to_value(&output).unwrap()["requirements"][0]["criteria"][0]["design_elements"],
        references
    );
}

/// An uncovered block appears in both the design cell and the requirement section.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-3", "FR-165-AC-13")]
#[test]
fn tc_1080_012_gaps_render_with_the_check_token() {
    let output = build(&design_input(&[design_entry(
        "spec/FR-001.md",
        Some("FR-001"),
        vec![design_criterion("FR-001-AC-1", Some(json!([{ "id": "IMPL-1" }])) )],
        Some(json!([
            covered_block(json!([{ "id": "IMPL-1", "kind": "Component", "path": "src/lib.rs", "verb": "implements", "realized_criteria": [] }])),
            uncovered_block("implemented")
        ])),
    )]))
    .unwrap();
    let rendered = render_markdown(&output);
    assert!(
        rendered.contains(
            "| FR-001 | FR-001-AC-1 | IMPL-1 (Component), no design element (implemented) |"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("## FR-001\n\nDesign gap: no design element (implemented)"),
        "{rendered}"
    );
}

/// Missing design stays absent in JSON and renders n/a; an empty reference list is none.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-4")]
#[test]
fn tc_1080_013_absent_and_empty_design_are_distinct() {
    let output = build(&design_input(&[
        design_entry(
            "spec/FR-001.md",
            Some("FR-001"),
            vec![design_criterion("FR-001-AC-1", None)],
            None,
        ),
        design_entry(
            "spec/FR-002.md",
            Some("FR-002"),
            vec![design_criterion("FR-002-AC-1", Some(json!([])))],
            Some(json!([covered_block(json!([]))])),
        ),
    ]))
    .unwrap();
    let value = serde_json::to_value(&output).unwrap();
    assert!(value["requirements"][0].get("design").is_none());
    assert!(
        value["requirements"][0]["criteria"][0]
            .get("design_elements")
            .is_none()
    );
    assert!(render_markdown(&output).contains("| FR-001 | FR-001-AC-1 | n/a |"));
    assert!(render_markdown(&output).contains("| FR-002 | FR-002-AC-1 | (none) |"));
}

/// Criteria-less design entries remain requirements and keep the empty reason semantics.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-5")]
#[test]
fn tc_1080_014_criteria_less_design_entry_is_rendered_without_a_table() {
    let output = build(&design_input(&[design_entry(
        "spec/FR-009.md",
        Some("FR-009"),
        vec![],
        Some(json!([uncovered_block("specified")])),
    )]))
    .unwrap();
    assert_eq!(output.reason, Some(EMPTY_REASON.to_owned()));
    assert_eq!(output.requirements.len(), 1);
    assert!(output.requirements[0].criteria.is_empty());
    let rendered = render_markdown(&output);
    assert!(
        rendered.contains("## FR-009\n\nDesign gap: no design element (specified)"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("| Requirement |"),
        "criteria-less entries have no table"
    );
}

/// Design data does not alter evidence status, and absent design equals the FR-115 input.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-6", "FR-165-AC-8")]
#[test]
fn tc_1080_015_design_axis_is_independent_from_evidence() {
    let id = "FR-001-AC-1";
    let ordinary = build(&input(
        &[id],
        &[binding(id, "unit")],
        &audit(&[finding(id, "stale-evidence")], &[], &[]),
    ))
    .unwrap();
    let designed = build(&design_input(&[design_entry(
        "spec/FR-001.md",
        Some("FR-001"),
        vec![design_criterion(id, Some(json!([])))],
        Some(json!([uncovered_block("implemented")])),
    )]))
    .unwrap();
    assert_eq!(statuses(&ordinary), statuses(&designed));
    assert_eq!(
        designed.requirements[0].criteria[0].evidence_status,
        EvidenceStatus::NoRunEvidence
    );

    let no_design = build(&design_input(&[design_entry(
        "spec/FR-001.md",
        Some("FR-001"),
        vec![design_criterion(id, None)],
        None,
    )]))
    .unwrap();
    let legacy = build(&input(&[id], &[], &audit(&[], &[], &[]))).unwrap();
    assert_eq!(
        serde_json::to_value(no_design).unwrap(),
        serde_json::to_value(legacy).unwrap()
    );
}

/// Rendering places design between criterion and method and retains element kind.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-7", "FR-165-AC-14")]
#[test]
fn tc_1080_016_design_column_renders_kind_and_untyped_id() {
    let output = build(&design_input(&[design_entry(
        "spec/FR-001.md", Some("FR-001"),
        vec![design_criterion("FR-001-AC-1", Some(json!([{ "id": "TYPED" }, { "id": "PLAIN" }])) )],
        Some(json!([covered_block(json!([
            { "id": "TYPED", "kind": "Component", "path": "src/lib.rs", "verb": "implements", "realized_criteria": [] },
            { "id": "PLAIN", "path": "src/main.rs", "verb": "implements", "realized_criteria": [] }
        ]))])),
    )])).unwrap();
    let rendered = render_markdown(&output);
    assert!(
        rendered.contains("| Requirement | Criterion | Design | Method |"),
        "{rendered}"
    );
    assert!(
        rendered.contains("TYPED (Component), PLAIN | Test"),
        "{rendered}"
    );
    let json = serde_json::to_value(&output).unwrap();
    assert_eq!(
        json["requirements"][0]["criteria"][0]["design_elements"][0],
        json!({ "id": "TYPED" })
    );
}

/// Dangling links and kind mismatches render their engine-provided markers.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-15")]
#[test]
fn tc_1080_017_dangling_and_kind_mismatch_render() {
    let output = build(&design_input(&[design_entry(
        "spec/FR-001.md", Some("FR-001"),
        vec![design_criterion("FR-001-AC-1", Some(json!([])))],
        Some(json!([
            { "relation": "r", "check": "c", "state": "covered", "elements": [], "dangling": [{ "reference": "IMPL-X", "verb": "implements" }], "kind_mismatch": [{ "id": "DOC-X", "archetype": "Plan", "verb": "implements", "accepted": ["Component"] }] }
        ])),
    )])).unwrap();
    let rendered = render_markdown(&output);
    assert!(rendered.contains("-> IMPL-X (dangling)"), "{rendered}");
    assert!(
        rendered.contains("kind mismatch: DOC-X (Plan)"),
        "{rendered}"
    );
}

/// Design blocks concatenate in repo and document order, including orphaned entries.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-16")]
#[test]
fn tc_1080_018_design_blocks_group_and_sort_by_repo_then_document() {
    let entries = [
        design_entry(
            "spec/z.md",
            Some("FR-001"),
            vec![design_criterion("FR-001-AC-2", None)],
            Some(json!([uncovered_block("z")])),
        ),
        json!({ "document": "spec/orphan.md", "repo": "a", "id": "FR-001", "criteria": [], "design": [uncovered_block("orphan")] }),
        json!({ "document": "spec/a.md", "repo": "a", "id": "FR-001", "criteria": [{ "id": "FR-001-AC-1", "method": "Test", "binders": [], "status": "untagged", "design_elements": [] }], "design": [uncovered_block("a")] }),
        json!({ "document": "spec/no-repo.md", "id": "FR-001", "criteria": [{ "id": "FR-001-AC-3", "method": "Test", "binders": [], "status": "untagged", "design_elements": [] }], "design": [uncovered_block("no-repo")] }),
    ];
    let output = build(&design_input(&entries)).unwrap();
    let blocks = output.requirements[0].design.as_ref().unwrap();
    let checks: Vec<&str> = blocks
        .iter()
        .map(|block| block["check"].as_str().unwrap())
        .collect();
    assert_eq!(checks, ["no-repo", "a", "orphan", "z"]);
    assert_eq!(output.requirements[0].criteria.len(), 3);
}

/// Design output is deterministic and the build path only copies engine fields.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-9", "FR-165-AC-11", "FR-165-CON-1")]
#[test]
fn tc_1080_019_build_is_deterministic_and_does_not_derive_design_facts() {
    let request = design_input(&[design_entry(
        "spec/FR-001.md",
        Some("FR-001"),
        vec![design_criterion("FR-001-AC-1", Some(json!([])))],
        Some(json!([uncovered_block("implemented")])),
    )]);
    let first = serde_json::to_vec(&build(&request).unwrap()).unwrap();
    let second = serde_json::to_vec(&build(&request).unwrap()).unwrap();
    assert_eq!(first, second);
    let source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/matrix.rs")).unwrap();
    let build_source = source
        .split("pub fn build(")
        .nth(1)
        .unwrap()
        .split("fn add_criterion")
        .next()
        .unwrap();
    assert!(!build_source.contains("realized_criteria"));
    assert!(!build_source.contains("archetype"));
    assert!(!build_source.contains("design_join"));
}

/// `matrix` performs no file, network or subprocess I/O: the shared source
/// auditor finds no host capability in the module, so every input can only
/// have arrived as an argument. `quoin-core`'s `ops/matrix.rs` shell is held to
/// the same rule by `tc_373_the_library_half_names_no_host_capability`.
///
#[trace("FR-115-CON-1")]
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

// ── FR-160: the class column ──────────────────────────────────────────────

/// An audit that ran under a class policy: `classReport` is the switch.
fn classed_audit(classes: &Value, unassigned: &[&str]) -> Value {
    let mut audit = audit(&[], &[], &[]);
    audit["classReport"] = json!({ "classes": classes, "unassigned": unassigned });
    audit
}

/// Without a class policy the matrix has no class column, key or count.
#[trace("FR-160-AC-1")]
#[test]
fn tc_160_matrix_001_no_policy_means_no_class_column_key_or_count() {
    let output = build(&input(
        &["FR-001-AC-1"],
        &[binding("FR-001-AC-1", "unit")],
        &audit(&[], &[], &[]),
    ))
    .unwrap();
    let payload = serde_json::to_value(&output).unwrap();
    assert!(payload.get("unclassified").is_none(), "{payload}");
    assert!(
        payload["requirements"][0]["criteria"][0]
            .get("class")
            .is_none()
    );
    let rendered = render_markdown(&output);
    assert!(!rendered.contains("Class"), "{rendered}");
}

/// Under a policy the class is shown as written, `unassigned` in markdown and
/// absent in JSON for a criterion whose requirement states none.
#[trace("FR-160-AC-6", "FR-160-AC-11")]
#[test]
fn tc_160_matrix_002_the_class_column_shows_the_value_and_unassigned() {
    let output = build(&input(
        &["FR-001-AC-1", "FR-001-AC-2"],
        &[binding("FR-001-AC-1", "unit")],
        &classed_audit(&json!({ "FR-001-AC-1": "class-2" }), &["FR-001-AC-2"]),
    ))
    .unwrap();
    let payload = serde_json::to_value(&output).unwrap();
    let criteria = &payload["requirements"][0]["criteria"];
    assert_eq!(criteria[0]["class"], "class-2");
    assert!(criteria[1].get("class").is_none(), "{criteria}");
    assert_eq!(payload["unclassified"], 1);

    let rendered = render_markdown(&output);
    assert!(rendered.contains("| Class |"), "{rendered}");
    assert!(rendered.contains("| class-2 |"), "{rendered}");
    assert!(rendered.contains("| unassigned |"), "{rendered}");
    assert!(rendered.contains("1 criterion(s) state no software safety class."));
}
