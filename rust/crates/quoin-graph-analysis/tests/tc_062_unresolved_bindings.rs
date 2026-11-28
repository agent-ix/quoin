// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A binding absent from the accepted export remains visible as a graph gap.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report"
)]

mod support;

use ix_trace_rs::trace;
use quoin_graph_analysis::{GraphAnalysis, analyze_fan_out, render_graph_analysis_json};
use serde_json::{Value, json};

/// A binding missing from the accepted export is named in both report surfaces,
/// while only exported obligations contribute to the suite's live count.
#[trace("FR-062-AC-2")]
#[test]
fn tc_062_unresolved_binding_is_reported_and_excluded_from_live_count() {
    let mut case = support::corpus()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "bindings/all-resolved")
        .expect("the corpus includes the resolved-bindings case")
        .clone();
    case["input"]["bindings"]["bindings"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "obligation": "FR-999-AC-1",
            "suite": "unit",
            "symbols": ["tc_unresolved_binding_is_reported_and_excluded_from_live_count"],
        }));

    let input = support::input(&case);
    let report = render_graph_analysis_json(&GraphAnalysis::from(analyze_fan_out(&input)))
        .expect("the fan-out report is canonical JSON");
    let report: Value = serde_json::from_str(&report).expect("the report parses");

    assert_eq!(report["state"], "incomplete");
    let unit = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["suite"] == "unit")
        .expect("the suite remains visible");
    assert_eq!(unit["unresolvedBindings"], json!(["FR-999-AC-1"]));
    assert_eq!(unit["obligationCount"], 3);
    assert_eq!(unit["obligations"].as_array().unwrap().len(), 3);
    assert!(
        unit["obligations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|obligation| obligation["obligation"] != "FR-999-AC-1")
    );

    assert!(report["gaps"].as_array().unwrap().iter().any(|gap| {
        gap["kind"] == "unresolved-binding" && gap["subject"] == "unit:FR-999-AC-1"
    }));
}
