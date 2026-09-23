// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-1016: the constant-predictor baseline, producer to checker, end to
//! end.
//!
//! `quoin-jev::constant_predictor::item_observations` (the producer half)
//! writes one observation per graded item; `quoin_measurement::verify` (the
//! checker half, `Baseline::ConstantPredictor`) reads them back and computes
//! MP-222/PLAT-932's formula. Fixture corpus: quoin FR-108's own worked
//! example — two families, 11 `weakness_kind` items (best constant `sound`,
//! 9/11) and 4 `coverage` items (best constant level `2`, 3/4) — so the
//! checker's baseline must land on exactly `(9 + 3) / 15 = 0.8`.
//!
//! Provenance: PLAT-1016, PLAT-985, PLAT-932, MP-222.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::float_cmp,
    reason = "the fixture's counts are exact integers over an exact total; the baseline this \
              test asserts is not a rounded aggregate"
)]

use engineering_assurance::measurement::{Baseline, Comparator, DecisionRule, Estimator};
use quoin_jev::{ConstantPredictorItem, item_observations};
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::observation::{
    Dimensions, MeasurementObservation, MeasurementPopulation, MeasurementShape, MeasurementState,
};
use quoin_measurement::types::plan::{
    LifecycleStatus, MeasurementPlan, MeasurementStage, StatisticalDesign,
};
use quoin_measurement::{OrderSource, Ranked, Reason, TamperFacts, Verdict, verify};

const PLAN_ID: &str = "MP-1016";
const DEFINITION_VERSION: &str = "constant-predictor-v1";
const METRIC: &str = "jev.criterion-strength.agreement-rate";

fn text(value: &str) -> NonEmptyText {
    NonEmptyText::parse(value, MeasurementErrorCode::CollectionInvalid, "x").unwrap()
}

/// The worked-example corpus: 11 `weakness_kind` items where `sound` is the
/// best constant (9 agree), 4 `coverage` items where level `2` is the best
/// constant (3 agree). Baseline: `(9 + 3) / 15`.
fn corpus() -> Vec<ConstantPredictorItem> {
    let mut items = Vec::new();
    // weakness_kind: 9 of 11 items best-answered by "sound" (including
    // contested rows where "sound" is a defensible alternate).
    let weakness_kind_labels = [
        ("sound", vec!["sound"]),
        ("sound", vec!["sound"]),
        ("sound", vec!["sound"]),
        ("sound", vec!["sound"]),
        ("sound", vec!["sound"]),
        ("sound", vec!["sound"]),
        ("gap", vec!["gap", "sound"]),
        ("gap", vec!["gap", "sound"]),
        ("gap", vec!["gap", "sound"]),
        ("weakness", vec!["weakness"]),
        ("weakness", vec!["weakness"]),
    ];
    for (index, (expected, contested)) in weakness_kind_labels.into_iter().enumerate() {
        items.push(ConstantPredictorItem {
            item_id: format!("w{index}"),
            family: "weakness_kind".to_owned(),
            expected: expected.to_owned(),
            contested: contested.into_iter().map(str::to_owned).collect(),
            actual: expected.to_owned(),
        });
    }
    // coverage: 3 of 4 items best-answered by level "2".
    for (index, (expected, contested)) in [
        ("2", vec!["2"]),
        ("2", vec!["2"]),
        ("2", vec!["2"]),
        ("1", vec!["1"]),
    ]
    .into_iter()
    .enumerate()
    {
        items.push(ConstantPredictorItem {
            item_id: format!("c{index}"),
            family: "coverage".to_owned(),
            expected: expected.to_owned(),
            contested: contested.into_iter().map(str::to_owned).collect(),
            actual: expected.to_owned(),
        });
    }
    items
}

/// The plan's own aggregate proportion observation.
fn aggregate(matched: f64, examined: f64) -> MeasurementObservation {
    MeasurementObservation {
        metric: text(METRIC),
        plan_id: text(PLAN_ID),
        definition_version: text(DEFINITION_VERSION),
        state: MeasurementState::Measured,
        value: Some(matched / examined),
        unit: text("fraction"),
        shape: MeasurementShape::Ratio,
        population: Some(MeasurementPopulation {
            examined: Some(examined),
            matched: Some(matched),
            complete: Some(true),
            repetitions: None,
            identity: None,
            unmodelled: std::collections::BTreeMap::new(),
        }),
        dimensions: Dimensions::ABSENT,
        reason: None,
    }
}

fn collection(id: &str, observations: Vec<MeasurementObservation>) -> MeasurementCollection {
    MeasurementCollection {
        schema_version: quoin_measurement::MEASUREMENT_SCHEMA_VERSION,
        collection_id: text(id),
        subject: text("quoin"),
        scope: quoin_store::JsonValue::Object(quoin_store::JsonObject::new()),
        tool_identity: text("quoin-jev"),
        tool_version: text("0.0.0"),
        config_digest: text("digest-1"),
        timestamp: text("2026-09-23T00:00:00Z"),
        source_revision: text("0000000000000000000000000000000000000000"),
        corpus_revision: None,
        environment: quoin_store::JsonObject::new(),
        verification_stack: None,
        observations,
        raw_evidence: quoin_store::JsonValue::Null,
    }
}

fn plan(decision_rule: DecisionRule) -> MeasurementPlan {
    MeasurementPlan {
        id: text(PLAN_ID),
        title: text("Criterion-strength agreement over constant-predictor baseline"),
        status: LifecycleStatus::Active,
        stage: MeasurementStage::Gate,
        metric: text(METRIC),
        definition_version: text(DEFINITION_VERSION),
        path: "spec/assurance/MP-1016.md".to_owned(),
        owner: None,
        action: None,
        preregistration: None,
        ground_truth_kind: None,
        statistical_design: Some(StatisticalDesign {
            minimum_population: None,
            repetitions: None,
            estimator: Some(Estimator::Proportion),
            decision_rule: Some(decision_rule),
        }),
        objective: None,
        protected_apparatus: None,
        negative_controls: None,
    }
}

/// The producer writes 15 item observations; the checker groups them by
/// family and computes exactly `(9 + 3) / 15 = 0.8`.
#[test]
fn the_producer_output_reproduces_the_worked_example_baseline() {
    let items = item_observations(PLAN_ID, DEFINITION_VERSION, METRIC, &corpus()).unwrap();
    assert_eq!(items.len(), 15);
    for item in &items {
        assert_eq!(
            item.metric.as_str(),
            "jev.criterion-strength.agreement-rate.constant-predictor-item"
        );
    }

    // An observed rate of 13/15 (~0.867) clears baseline (0.8) + margin (0.05).
    let mut observations = vec![aggregate(13.0, 15.0)];
    observations.extend(items);
    let collections = [collection("run-001", observations)];
    let rule =
        DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, Some(0.05))
            .unwrap();
    let result = verify(
        &plan(rule),
        &[Ranked::new(&collections[0], Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    );
    assert_eq!(result.verdict, Verdict::Accept, "{result:?}");
    assert_eq!(result.decisions.len(), 1);
    assert_eq!(result.decisions[0].baseline, Some(0.8));
}

/// The same corpus, but the observed rate (12/15 = 0.8, equal to the
/// baseline itself) does not clear baseline (0.8) plus a five-point margin:
/// `rule_not_met`, not `accept`.
#[test]
fn a_margin_not_cleared_over_the_real_baseline_rejects() {
    let items = item_observations(PLAN_ID, DEFINITION_VERSION, METRIC, &corpus()).unwrap();
    let mut observations = vec![aggregate(12.0, 15.0)];
    observations.extend(items);
    let collections = [collection("run-001", observations)];
    let rule =
        DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, Some(0.05))
            .unwrap();
    let result = verify(
        &plan(rule),
        &[Ranked::new(&collections[0], Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    );
    assert_eq!(result.verdict, Verdict::Reject);
    assert!(result.reasons.contains(&Reason::RuleNotMet));
    assert_eq!(result.decisions[0].baseline, Some(0.8));
}

/// A run whose collection carries the aggregate observation but no per-item
/// observations at all is `constant_predictor_rows_absent`, not a computed
/// baseline of `0`.
#[test]
fn a_run_with_no_retained_items_is_inconclusive_not_a_false_baseline() {
    let collections = [collection("run-001", vec![aggregate(10.0, 15.0)])];
    let rule =
        DecisionRule::against_baseline(Comparator::Gt, Baseline::ConstantPredictor, Some(0.05))
            .unwrap();
    let result = verify(
        &plan(rule),
        &[Ranked::new(&collections[0], Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    );
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert!(
        result
            .reasons
            .contains(&Reason::ConstantPredictorRowsAbsent)
    );
    assert_eq!(result.decisions[0].baseline, None);
}
