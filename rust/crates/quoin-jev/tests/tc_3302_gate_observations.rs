// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-131-AC-14: the observation of a Jev gate (MP-229, MP-230) states the bars
//! the run computed as its examined ids and the bars that held as its matched
//! ids, and intake admits it only when it states exactly the plan's bars.
//!
//! The bar results here are the shape a gate run produces, not a measurement:
//! no live Jev call is made, and no verdict is claimed.
//!
//! Provenance: PLAT-1403

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report"
)]

mod support;

use ix_trace_rs::trace;
use quoin_measurement::error::MeasurementErrorCode;
use serde_json::json;
use support::gate::{Bar, admit, collection, plan};

fn bars(plan_ids: &[&'static str], unmet: &[&str]) -> Vec<Bar> {
    plan_ids
        .iter()
        .map(|plan_id| Bar {
            plan_id,
            met: !unmet.contains(plan_id),
        })
        .collect()
}

const EARS: [&str; 4] = ["MP-222", "MP-223", "MP-224", "MP-231"];
const GAP: [&str; 3] = ["MP-222", "MP-223", "MP-224"];

/// A run that evaluated every bar is admitted, with the bars it met as matched.
#[trace("FR-131-AC-14", "TC-3302")]
#[test]
fn tc_3302_a_gate_run_that_evaluated_every_bar_is_admitted() {
    for (gate, ids) in [("MP-229", &EARS[..]), ("MP-230", &GAP[..])] {
        let plan = plan(gate);
        let all_met = collection(&plan, "shape", &bars(ids, &[]));
        admit(&plan, &all_met).unwrap();
        let population = &all_met["observations"][0]["population"];
        assert_eq!(population["examined_ids"], json!(ids));
        assert_eq!(population["matched_ids"], json!(ids));

        let one_unmet = collection(&plan, "shape", &bars(ids, &["MP-223"]));
        admit(&plan, &one_unmet).unwrap();
        let population = &one_unmet["observations"][0]["population"];
        assert_eq!(population["examined"], ids.len());
        assert_eq!(population["matched"], ids.len() - 1);
        assert!(
            !population["matched_ids"]
                .as_array()
                .unwrap()
                .contains(&json!("MP-223"))
        );
    }
}

/// A run that left a bar out, or evaluated one the plan does not name, is
/// refused at intake.
#[trace("FR-131-AC-14", "TC-3302")]
#[test]
fn tc_3302_a_gate_run_that_skipped_or_invented_a_bar_is_refused() {
    let plan = plan("MP-229");
    let skipped = collection(&plan, "shape", &bars(&EARS[..3], &[]));
    assert_eq!(
        admit(&plan, &skipped).unwrap_err().code(),
        MeasurementErrorCode::PopulationPartial
    );
    let mut invented = bars(&EARS, &[]);
    invented.push(Bar {
        plan_id: "MP-226",
        met: true,
    });
    let invented = collection(&plan, "shape", &invented);
    assert_eq!(
        admit(&plan, &invented).unwrap_err().code(),
        MeasurementErrorCode::PopulationOutsideSet
    );
}
