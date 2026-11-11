// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-1021: a graded run's record carries the aggregate and the item rows
//! behind it.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_jev::{ConstantPredictorItem, GradedRun, record_input};
use quoin_measurement::error::MeasurementErrorCode;

fn run() -> GradedRun<'static> {
    GradedRun {
        plan_id: "MP-1021",
        definition_version: "graded-run-v1",
        metric: "jev.criterion-strength.agreement-rate",
        collection_id: "run-1",
        subject: "fixture",
        tool_version: "0.0.0",
        timestamp: "2026-11-11T00:00:00.000Z",
        plan_path: "spec/assurance/MP-1021.md",
    }
}

fn item(id: &str, family: &str, expected: &str, actual: &str) -> ConstantPredictorItem {
    ConstantPredictorItem {
        item_id: id.to_owned(),
        family: family.to_owned(),
        expected: expected.to_owned(),
        contested: vec![expected.to_owned()],
        actual: actual.to_owned(),
    }
}

/// Provenance: PLAT-1021
#[trace("FR-134-AC-1", "TC-2800")]
#[test]
fn tc_1021_001_aggregate_and_item_rows_come_from_the_same_items() {
    let items = [
        item("a", "weakness_kind", "sound", "sound"),
        item("b", "weakness_kind", "gap", "sound"),
        item("c", "coverage", "2", "2"),
    ];
    let record = record_input(&run(), &items).unwrap();
    let rows = record["observations"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    let aggregate = &rows[0];
    assert_eq!(aggregate["metric"], "jev.criterion-strength.agreement-rate");
    assert_eq!(aggregate["population"]["examined"], 3);
    assert_eq!(aggregate["population"]["matched"], 2);
    for row in &rows[1..] {
        assert_eq!(
            row["metric"],
            "jev.criterion-strength.agreement-rate.constant-predictor-item"
        );
    }
    assert_eq!(rows[2]["dimensions"]["item_id"], "b");
    assert_eq!(rows[2]["dimensions"]["actual"], "sound");

    let error = record_input(&run(), &[]).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::CollectionInvalid);
}
