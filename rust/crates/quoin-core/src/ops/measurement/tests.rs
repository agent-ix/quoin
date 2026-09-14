// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What this domain decides WITHOUT a store: which intake a candidate belongs
//! to, and every ceiling.
//!
//! Nothing here opens a filesystem, and that is the point rather than a
//! convenience. Every test below drives an operation to a verdict it reaches
//! *before* `quoin-measurement` is called, which is the only way to demonstrate
//! that the ceilings really are applied first — a test that let the library run
//! would be measuring the library.
//!
//! The routes' behaviour WITH a store is `tests/tc_478_measurement_dispatch.rs`,
//! which drives the real binary against a real one.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use serde_json::json;

use crate::error::CoreErrorCode;

use super::record::Intake;
use super::wire::{
    MAX_COLLECTION_BYTES, MAX_GRAPH_MAPPING_BYTES, MAX_INTERVENTION_RECORD_BYTES,
    MAX_METRIC_NAME_BYTES, MAX_OPERATIONAL_RECORD_BYTES, MAX_RECORD_ID_BYTES, MAX_REVISION_BYTES,
};
use super::{
    build_graph_portfolio, build_series, produce_agent_eval_intervention, record, render_comparison,
};

/// The `record_type` member alone decides which intake accepts a candidate,
/// and an unrecognised or absent one is a collection — the retained
/// `record.ts` ternary, which falls through to `writeMeasurementCollection`.
///
/// Trace: FR-096, FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn the_candidates_own_record_type_names_its_intake() {
    assert_eq!(
        Intake::of(&json!({"record_type": "intervention_experiment"})),
        Intake::Intervention
    );
    assert_eq!(
        Intake::of(&json!({"record_type": "operational_evidence"})),
        Intake::Operational
    );
    assert_eq!(
        Intake::of(&json!({"record_type": "measurement"})),
        Intake::Collection
    );
    assert_eq!(Intake::of(&json!({})), Intake::Collection);
    // Not a string, and not a document: neither is an intake, and neither may
    // be a panic.
    assert_eq!(Intake::of(&json!({"record_type": 7})), Intake::Collection);
    assert_eq!(Intake::of(&json!([])), Intake::Collection);

    assert_eq!(Intake::Collection.limit(), MAX_COLLECTION_BYTES);
    assert_eq!(Intake::Intervention.limit(), MAX_INTERVENTION_RECORD_BYTES);
    assert_eq!(Intake::Operational.limit(), MAX_OPERATIONAL_RECORD_BYTES);
}

/// The whole-request ceiling fires before the store is located, so a request
/// naming a repository that does not exist is still refused for its SIZE.
///
/// The repository below is deliberately absent. If the bound were applied after
/// the store were opened, this test would report an I/O failure instead — which
/// is exactly the defect the ordering exists to prevent.
///
/// Trace: FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn a_record_past_its_intakes_ceiling_is_refused_before_the_store_is_opened() {
    let oversized = json!({
        "repo": "/nonexistent/quoin-478",
        "record": {"record_type": "measurement", "padding": "x".repeat(MAX_COLLECTION_BYTES)},
    });
    let error = record(&oversized).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["op"], "measurement.record");
    assert_eq!(
        error.context["limit_bytes"],
        MAX_COLLECTION_BYTES.to_string()
    );
    assert!(
        error.context["observed_bytes"].parse::<usize>().unwrap() > MAX_COLLECTION_BYTES,
        "observed: {}",
        error.context["observed_bytes"]
    );
}

/// A field ceiling names the field, so the caller knows which member to shrink.
///
/// Trace: FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn a_field_past_its_ceiling_is_refused_and_named() {
    let cases: [(&str, &str, usize, crate::error::CoreError); 4] = [
        (
            "measurement.build_series",
            "metric",
            MAX_METRIC_NAME_BYTES,
            build_series(&json!({
                "repo": "/nonexistent/quoin-478",
                "metric": "m".repeat(MAX_METRIC_NAME_BYTES + 1),
            }))
            .unwrap_err(),
        ),
        (
            "measurement.render_comparison",
            "before_revision",
            MAX_REVISION_BYTES,
            render_comparison(&json!({
                "repo": "/nonexistent/quoin-478",
                "before_revision": "r".repeat(MAX_REVISION_BYTES + 1),
            }))
            .unwrap_err(),
        ),
        (
            "measurement.build_graph_portfolio",
            "graph_exports",
            MAX_GRAPH_MAPPING_BYTES,
            build_graph_portfolio(&json!({
                "locations": ["/nonexistent/quoin-478"],
                "graph_exports": ["a=".to_owned() + &"x".repeat(MAX_GRAPH_MAPPING_BYTES)],
            }))
            .unwrap_err(),
        ),
        (
            "measurement.produce_agent_eval_intervention",
            "record_id",
            MAX_RECORD_ID_BYTES,
            produce_agent_eval_intervention(&json!({
                "repo": "/nonexistent/quoin-478",
                "definition": {"record_id": "i".repeat(MAX_RECORD_ID_BYTES + 1)},
            }))
            .unwrap_err(),
        ),
    ];
    // The anti-vacuity floor: a refactor that stopped constructing cases would
    // otherwise leave this test asserting over an empty population.
    assert_eq!(cases.len(), 4, "every field ceiling has a case here");

    for (op, field, limit, error) in cases {
        assert_eq!(error.code, CoreErrorCode::Refused, "{op}/{field}");
        assert_eq!(error.outcome().code(), 2, "{op}/{field}");
        assert_eq!(error.context["op"], op);
        assert_eq!(error.context["field"], field);
        assert_eq!(
            error.context["limit_bytes"],
            limit.to_string(),
            "{op}/{field}"
        );
    }
}

/// The record-identity ceiling is the STORE's, not a second opinion about it.
///
/// A boundary that refused a longer identity than `quoin-measurement` accepts
/// would reject records the store would take; a shorter one would admit records
/// it will not write.
///
/// Trace: FR-101-AC-7
/// Provenance: quoin#478
#[test]
fn the_identity_ceiling_is_the_stores_own() {
    assert_eq!(MAX_RECORD_ID_BYTES, quoin_measurement::MAX_RECORD_ID_BYTES);
}

/// A request of the wrong shape is the caller's mistake (3), not a refusal (2),
/// and it names the operation that could not read it.
///
/// Trace: FR-101-AC-2
/// Provenance: quoin#478
#[test]
fn a_misshapen_request_is_a_bad_request() {
    let error = build_series(&json!({"repo": 7, "metric": "finding_recall"})).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.outcome().code(), 3);
    assert_eq!(error.context["op"], "measurement.build_series");

    // `deny_unknown_fields`: a member this build does not know is named rather
    // than dropped, because a silently ignored flag is a silently wrong answer.
    let error = build_series(&json!({
        "repo": "/nonexistent/quoin-478",
        "metric": "finding_recall",
        "formaat": "json",
    }))
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert!(
        error.message.contains("formaat"),
        "message: {}",
        error.message
    );
}
