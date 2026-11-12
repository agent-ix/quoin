// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-132-AC-15 and AC-16: `report` attests the intake order as `verify` does, so a
//! paired-run plan is decided by both over the same pair (PLAT-1457).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use ix_trace_rs::trace;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::report::build::build_measurement_report_from;
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::{
    IntakeOrder, OrderSource, PairedRunObservation, PairedRunOutcome, Reason, TamperFacts, Verdict,
    validate, verify,
};
use serde_json::{Value, json};

const EQUAL: &str = "paired_run:\n  relation: equal\n  minimum_pairs: 1\n";
fn plans_staged(stage: &str, extra: &str) -> Vec<MeasurementPlan> {
    let text = format!(
        "---\nid: MP-001\ntitle: Determinism\ntype: MeasurementPlan\nstatus: active\nowner: test\n\
         stage: {stage}\nmetric: quality.example\ndefinition_version: quality.example-v1\n\
         {extra}---\n\n# Determinism\n"
    );
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("creatable");
    std::fs::write(assurance.join("MP-001.md"), text).expect("writable");
    load_measurement_plans(
        &DiskMeasurement::new(temporary.path()),
        PlanLoadOptions::default(),
    )
    .expect("the plan loads")
}

fn plans(extra: &str) -> Vec<MeasurementPlan> {
    plans_staged("branch-comparison", extra)
}

/// A run of `plan` (id, definition version) stated at `timestamp`.
fn run_as(
    id: &str,
    plan: (&str, &str),
    timestamp: &str,
    values: &[(&str, f64)],
) -> MeasurementCollection {
    let observations: Vec<Value> = values
        .iter()
        .map(|(root, value)| {
            json!({
                "metric": "quality.example", "planId": plan.0,
                "definitionVersion": plan.1, "state": "measured",
                "value": value, "unit": "count", "shape": "count",
                "dimensions": { "root": root },
                "population": { "examined": 1, "complete": true },
            })
        })
        .collect();
    let document = json!({
        "schemaVersion": 2, "collectionId": id, "subject": "fixture",
        "scope": { "cases": 1 }, "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1", "timestamp": timestamp,
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"], "artifacts": ["config"],
        },
        "observations": observations, "rawEvidence": { "payload": [1] },
    });
    let stored = from_serde(&document).expect("bridge");
    validate::stored_measurement_collection(&stored).expect("admissible")
}

const NOT_WORSE: &str =
    "objective:\n  direction: lower\npaired_run:\n  relation: not_worse\n  minimum_pairs: 1\n";
const OWN: (&str, &str) = ("MP-001", "quality.example-v1");

fn order(source: OrderSource, groups: &[&[&str]]) -> IntakeOrder {
    IntakeOrder {
        source,
        groups: groups
            .iter()
            .map(|group| group.iter().map(|id| (*id).to_owned()).collect())
            .collect(),
    }
}

/// What `verify` and the report each say about the same collections in the
/// same order.
struct Both {
    reasons: Vec<Reason>,
    verdict: Verdict,
    count: Option<f64>,
    pair: PairedRunObservation,
}

impl Both {
    fn unattested(&self) -> bool {
        self.reasons.contains(&Reason::OrderUnattested)
    }
}

fn both(plans: &[MeasurementPlan], runs: &[MeasurementCollection], order: &IntakeOrder) -> Both {
    let verdict = verify(
        &plans[0],
        &order.rank(runs),
        TamperFacts::default(),
        order.source,
        None,
    );
    let report = build_measurement_report_from(Path::new("/repo"), plans, runs, &[], &[], order)
        .expect("builds");
    Both {
        reasons: verdict.reasons.clone(),
        verdict: verdict.verdict,
        count: verdict
            .decisions
            .iter()
            .find(|decision| decision.paired)
            .map(|decision| decision.estimate.value),
        pair: report.paired_runs[0].clone(),
    }
}

/// `report` decides what `verify` decides: the same count when the order is
/// attested, and no count when it is not.
fn agree(both: &Both) {
    if both.unattested() {
        assert_eq!(both.pair.outcome, PairedRunOutcome::Inconclusive);
        assert!(
            both.pair
                .reasons
                .iter()
                .any(|r| r.starts_with("order_unattested"))
        );
        return;
    }
    let PairedRunOutcome::Observed { differing } = both.pair.outcome else {
        panic!("report did not decide: {:?}", both.pair);
    };
    assert_eq!(
        both.count,
        Some(f64::from(u32::try_from(differing).unwrap()))
    );
    assert_eq!(both.verdict == Verdict::Accept, differing == 0);
}

fn abc(c_stamp: &str, c_value: f64) -> [MeasurementCollection; 3] {
    [
        run_as("a", OWN, "2026-08-02T00:00:00.000Z", &[("r1", 9.0)]),
        run_as("b", OWN, "2026-08-03T00:00:00.000Z", &[("r1", 3.0)]),
        run_as("c", OWN, c_stamp, &[("r1", c_value)]),
    ]
}

/// More than two collections in a git repo: report names the pair verify
/// names (the latest two by first-add commit), and an earlier differing
/// collection changes neither.
#[trace("FR-132-AC-15", "TC-3600")]
#[test]
fn tc_3600_three_collections_in_git_are_decided_by_both() {
    let runs = abc("2026-08-04T00:00:00.000Z", 3.0);
    let git = order(OrderSource::GitFirstParentAdd, &[&["a"], &["b"], &["c"]]);
    let both = both_of(&runs, &git);
    assert!(!both.unattested(), "{:?}", both.reasons);
    assert_eq!(
        both.pair.outcome,
        PairedRunOutcome::Observed { differing: 0 }
    );
    assert_eq!(both.verdict, Verdict::Accept);
    agree(&both);
    // The newest differs: both reject with the same count.
    let runs = abc("2026-08-04T00:00:00.000Z", 4.0);
    let both = both_of(&runs, &git);
    assert_eq!(
        both.pair.outcome,
        PairedRunOutcome::Observed { differing: 1 }
    );
    assert_eq!(both.verdict, Verdict::Reject);
    agree(&both);
}

fn both_of(runs: &[MeasurementCollection], order: &IntakeOrder) -> Both {
    both(&plans(EQUAL), runs, order)
}

/// Intake a, b, c while c states the earliest timestamp: the stated time
/// contradicts the attested order, so neither names a count.
#[trace("FR-132-AC-16", "TC-3601")]
#[test]
fn tc_3601_a_contradicting_timestamp_is_unattested_in_both() {
    let runs = abc("2026-08-01T00:00:00.000Z", 4.0);
    let git = order(OrderSource::GitFirstParentAdd, &[&["a"], &["b"], &["c"]]);
    let both = both_of(&runs, &git);
    assert!(both.unattested());
    agree(&both);
}

/// No order at all, a shallow clone and a tie are each unattested in both.
#[trace("FR-132-AC-12", "FR-132-AC-16", "TC-3602")]
#[test]
fn tc_3602_a_missing_order_is_unattested_in_both() {
    let runs = abc("2026-08-04T00:00:00.000Z", 3.0);
    for unordered in [
        IntakeOrder::none(),
        order(OrderSource::GitShallow, &[&["a", "b", "c"]]),
        order(OrderSource::GitFirstParentAdd, &[&["a"], &["b", "c"]]),
    ] {
        let both = both_of(&runs, &unordered);
        assert!(both.unattested(), "{:?}", unordered.source);
        agree(&both);
    }
}

/// A shallow clone attests no order for any plan: two collections under
/// `equal` are `order_unattested` in both, with no count in the report.
#[trace("FR-132-AC-12", "FR-132-AC-16", "TC-3602")]
#[test]
fn tc_3602_a_shallow_clone_is_unattested_for_two_collections_in_both() {
    let runs = [
        run_as("a", OWN, "2026-08-02T00:00:00.000Z", &[("r1", 3.0)]),
        run_as("b", OWN, "2026-08-03T00:00:00.000Z", &[("r1", 3.0)]),
    ];
    let shallow = order(OrderSource::GitShallow, &[&["a"], &["b"]]);
    let both = both_of(&runs, &shallow);
    assert!(both.unattested());
    assert_eq!(
        both.reasons
            .iter()
            .filter(|r| **r == Reason::OrderUnattested)
            .count(),
        1
    );
    agree(&both);
}

/// Two collections need no order under `equal`, and are decided with none.
#[trace("FR-132-AC-15", "TC-3603")]
#[test]
fn tc_3603_two_collections_under_equal_need_no_order() {
    let runs = [
        run_as("a", OWN, "2026-08-02T00:00:00.000Z", &[("r1", 3.0)]),
        run_as("b", OWN, "2026-08-03T00:00:00.000Z", &[("r1", 4.0)]),
    ];
    let both = both_of(&runs, &IntakeOrder::none());
    assert!(!both.unattested());
    assert_eq!(
        both.pair.outcome,
        PairedRunOutcome::Observed { differing: 1 }
    );
    agree(&both);
}

/// Under `not_worse` the order ranks the runs: attested, report and verify
/// agree on the count in both directions; unattested, neither decides.
#[trace("FR-132-AC-15", "TC-3604")]
#[test]
fn tc_3604_not_worse_is_decided_only_with_an_attested_order() {
    let plans = plans(NOT_WORSE);
    let runs = [
        run_as("a", OWN, "2026-08-02T00:00:00.000Z", &[("r1", 3.0)]),
        run_as("b", OWN, "2026-08-03T00:00:00.000Z", &[("r1", 4.0)]),
    ];
    let forward = order(OrderSource::GitFirstParentAdd, &[&["a"], &["b"]]);
    let worse = both(&plans, &runs, &forward);
    assert_eq!(
        worse.pair.outcome,
        PairedRunOutcome::Observed { differing: 1 }
    );
    agree(&worse);
    // The same values, intaken in the other order and stated consistently.
    let later = [
        run_as("a", OWN, "2026-08-03T00:00:00.000Z", &[("r1", 3.0)]),
        run_as("b", OWN, "2026-08-02T00:00:00.000Z", &[("r1", 4.0)]),
    ];
    let backward = order(OrderSource::GitFirstParentAdd, &[&["b"], &["a"]]);
    let reversed = both(&plans, &later, &backward);
    assert_eq!(
        reversed.pair.outcome,
        PairedRunOutcome::Observed { differing: 0 }
    );
    agree(&reversed);
    let none = both(&plans, &runs, &IntakeOrder::none());
    assert!(none.unattested());
    agree(&none);
}
