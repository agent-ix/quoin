// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-1016: constant-predictor item observations through the real intake
//! path, not only an in-memory collection.
//!
//! `quoin-jev`'s producer writes each graded item under
//! `{metric}.constant-predictor-item`, a metric no plan governs by name. Intake
//! refuses any observation whose metric has no plan, so without these checks
//! the item rows could never be recorded and the baseline could never be
//! computed from a stored collection. Every plan here is a real assurance
//! document loaded through `load_measurement_plans`.
//!
//! Provenance: PLAT-1016

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::Path;

use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::{
    MeasurementCollection, MeasurementError, OrderSource, Ranked, Reason, TamperFacts, Verdict,
    compare_measurement_collections, validate, verify, write_measurement_collection,
};
use serde_json::{Value, json};

const METRIC: &str = "quality.gate";
const ITEM_METRIC: &str = "quality.gate.constant-predictor-item";

/// An active gate plan for `metric`, requiring at least ten examined items
/// and beating the constant predictor by five points.
fn plan_document(metric: &str) -> String {
    format!(
        "---\n\
         id: MP-900\n\
         title: Example gate\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: gate\n\
         metric: {metric}\n\
         definition_version: quality.gate-v1\n\
         protected_apparatus:\n\
         \x20\x20- spec/assurance/MP-900.md\n\
         negative_controls:\n\
         \x20\x20- kind: apparatus-edit\n\
         \x20\x20\x20\x20description: the plan document defines the measurement\n\
         ground_truth_kind: human-labelled\n\
         statistical_design:\n\
         \x20\x20population: every labelled case\n\
         \x20\x20minimum_population: 10\n\
         \x20\x20sampling: exhaustive\n\
         \x20\x20repetitions: 1\n\
         \x20\x20estimator: proportion\n\
         \x20\x20population_exempt: fixture cases carry no ids\n\
         \x20\x20error_model: binomial\n\
         \x20\x20uncertainty: wilson interval\n\
         \x20\x20decision_rule:\n\
         \x20\x20\x20\x20comparator: gt\n\
         \x20\x20\x20\x20baseline: constant-predictor\n\
         \x20\x20\x20\x20margin: 0.05\n\
         \x20\x20answer_families:\n\
         \x20\x20\x20\x20- weakness_kind\n\
         \x20\x20\x20\x20- coverage\n\
         ---\n\
         \n\
         # Example gate\n"
    )
}

/// A repository holding the plan document for `metric`.
fn repository(metric: &str) -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(assurance.join("MP-900.md"), plan_document(metric)).expect("writable");
    temporary
}

fn load(metric: &str) -> Result<Vec<MeasurementPlan>, MeasurementError> {
    load_plans(repository(metric).path())
}

fn load_plans(root: &Path) -> Result<Vec<MeasurementPlan>, MeasurementError> {
    load_measurement_plans(&DiskMeasurement::new(root), PlanLoadOptions::default())
}

/// Write `candidate` through the real `write_measurement_collection`,
/// declaring the protected plan document by name.
fn write(root: &Path, candidate: &Value) -> Result<std::path::PathBuf, MeasurementError> {
    let path = "spec/assurance/MP-900.md";
    let mut candidate = candidate.clone();
    candidate["verificationStack"]["artifacts"]
        .as_array_mut()
        .expect("the artifact list")
        .push(json!(path));
    write_measurement_collection(root, &from_serde(&candidate).expect("crosses"))
}

/// The governed aggregate: `matched` of 15.
fn aggregate(matched: u32) -> Value {
    json!({
        "metric": METRIC,
        "planId": "MP-900",
        "definitionVersion": "quality.gate-v1",
        "state": "measured",
        "value": f64::from(matched) / 15.0,
        "unit": "fraction",
        "shape": "ratio",
        "population": { "examined": 15, "matched": matched, "complete": true, "repetitions": 1 },
    })
}

/// One item row, as `quoin-jev`'s producer writes it.
fn item(plan_id: &str, item_id: &str, family: &str, expected: &str, contested: &[&str]) -> Value {
    json!({
        "metric": ITEM_METRIC,
        "planId": plan_id,
        "definitionVersion": "quality.gate-v1",
        "state": "measured",
        "value": 1,
        "unit": "fraction",
        "shape": "scalar",
        "dimensions": {
            "item_id": item_id,
            "family": family,
            "expected": expected,
            "contested": contested,
            "actual": expected,
        },
    })
}

/// FR-108's worked example as item rows: 11 `weakness_kind` (best `sound`,
/// 9/11) and 4 `coverage` (best `2`, 3/4) — baseline `0.8`.
fn items(plan_id: &str) -> Vec<Value> {
    let weakness_kind: [(&str, &[&str]); 11] = [
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("gap", &["gap", "sound"]),
        ("gap", &["gap", "sound"]),
        ("gap", &["gap", "sound"]),
        ("weakness", &["weakness"]),
        ("weakness", &["weakness"]),
    ];
    let coverage: [(&str, &[&str]); 4] =
        [("2", &["2"]), ("2", &["2"]), ("2", &["2"]), ("1", &["1"])];
    let mut out = Vec::new();
    for (index, (expected, contested)) in weakness_kind.into_iter().enumerate() {
        out.push(item(
            plan_id,
            &format!("w{index}"),
            "weakness_kind",
            expected,
            contested,
        ));
    }
    for (index, (expected, contested)) in coverage.into_iter().enumerate() {
        out.push(item(
            plan_id,
            &format!("c{index}"),
            "coverage",
            expected,
            contested,
        ));
    }
    out
}

fn collection(id: &str, observations: &[Value]) -> Value {
    json!({
        "schemaVersion": 2,
        "collectionId": id,
        "subject": "fixture",
        "scope": { "cases": 15 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "timestamp": "2026-09-23T00:00:00.000Z",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"],
            "artifacts": ["config"],
        },
        "observations": observations,
        "rawEvidence": { "payload": [1, 2] },
    })
}

fn intake(
    plans: &[MeasurementPlan],
    candidate: &Value,
) -> Result<MeasurementCollection, MeasurementError> {
    validate::measurement_collection(&from_serde(candidate).expect("crosses"), plans)
}

fn with_items(matched: u32, plan_id: &str) -> Vec<Value> {
    let mut observations = vec![aggregate(matched)];
    observations.extend(items(plan_id));
    observations
}

/// Intake admits item rows under the governed metric's own plan, and does
/// not hold a single item row to the plan's `minimum_population` of ten —
/// through the real `write_measurement_collection` as well as the validator.
/// The admitted collection's rule is then decided against the recomputed
/// `0.8` baseline: holds at 13/15, does not at 12/15.
///
/// Provenance: PLAT-1016
#[trace("FR-108-AC-10", "TC-1920")]
#[test]
fn tc_1016_001_intake_admits_item_rows_and_the_admitted_collection_verifies() {
    let repository = repository(METRIC);
    let plans = load_plans(repository.path()).unwrap();
    write(
        repository.path(),
        &collection("run-1016", &with_items(13, "MP-900")),
    )
    .unwrap_or_else(|refusal| panic!("{refusal}: {:?}", refusal.findings()));
    for (matched, holds) in [(13, true), (12, false)] {
        let admitted = intake(
            &plans,
            &collection("run-1016", &with_items(matched, "MP-900")),
        )
        .unwrap_or_else(|refusal| panic!("{refusal}: {:?}", refusal.findings()));
        let result = verify(
            &plans[0],
            &[Ranked::new(&admitted, Some(0))],
            TamperFacts::default(),
            OrderSource::CallerSupplied,
            None,
        );
        assert_eq!(result.decisions.len(), 1, "{result:?}");
        assert_eq!(result.decisions[0].baseline, Some(0.8));
        assert_eq!(result.decisions[0].holds, Some(holds), "{matched}");
        // The apparatus is not resolved in this in-memory verify, so the
        // verdict itself carries `apparatus_unrecorded`; the rule's own
        // outcome is what this test is about.
        assert_eq!(
            result.reasons.contains(&Reason::RuleNotMet),
            !holds,
            "{result:?}"
        );
    }
}

/// An item row is held to its governed plan's identity: one naming another
/// plan id is refused like any other observation would be.
///
/// Provenance: PLAT-1016
#[trace("FR-108-AC-10", "TC-1921")]
#[test]
fn tc_1016_002_an_item_row_naming_another_plan_is_refused() {
    let plans = load(METRIC).unwrap();
    let mut observations = vec![aggregate(13)];
    observations.extend(items("MP-901"));
    let refusal = intake(&plans, &collection("run-1016", &observations)).unwrap_err();
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.starts_with(
                "metric `quality.gate.constant-predictor-item` names plan MP-901; active plan is MP-900 (at observations["
            )),
        "{:?}",
        refusal.findings()
    );
}

/// An item row whose governed metric has no plan is refused as unplanned.
///
/// Provenance: PLAT-1016
#[trace("FR-108-AC-10", "TC-1921")]
#[test]
fn tc_1016_003_an_item_row_of_an_unplanned_metric_is_refused() {
    let plans = load("other.metric").unwrap();
    let refusal = intake(&plans, &collection("run-1016", &items("MP-900"))).unwrap_err();
    assert!(
        refusal.findings().iter().any(|finding| finding
            .starts_with("metric `quality.gate.constant-predictor-item` has no MeasurementPlan")),
        "{:?}",
        refusal.findings()
    );
}

/// A plan cannot claim a metric ending in the reserved item suffix: it would
/// read another plan's item rows as its own run.
///
/// Provenance: PLAT-1016
#[trace("FR-108-AC-10", "TC-1922")]
#[test]
fn tc_1016_004_a_plan_metric_with_the_reserved_suffix_is_refused() {
    let refusal = load(ITEM_METRIC).unwrap_err();
    assert_eq!(refusal.code(), MeasurementErrorCode::PlanInvalid);
    assert!(
        refusal
            .to_string()
            .contains("reserved constant-predictor item suffix"),
        "{refusal}"
    );
}

/// `compare` compares the governed slice and leaves the item rows out, even
/// when the graded tool's per-item answers moved between the two runs.
///
/// Provenance: PLAT-1016
#[trace("FR-108-AC-10", "TC-1923")]
#[test]
fn tc_1016_005_compare_leaves_item_rows_out() {
    let plans = load(METRIC).unwrap();
    let before = intake(&plans, &collection("run-a", &with_items(12, "MP-900"))).unwrap();
    let mut after_observations = with_items(13, "MP-900");
    after_observations[1]["dimensions"]["actual"] = json!("gap");
    let after = intake(&plans, &collection("run-b", &after_observations)).unwrap();
    let comparisons = compare_measurement_collections(&before, &after, &[]).unwrap();
    let metrics: Vec<&str> = comparisons.iter().map(|row| row.metric.as_str()).collect();
    assert_eq!(metrics, [METRIC]);
}

/// A candidate whose item rows fall short of its own `examined` — a
/// producer that lost items — is `inconclusive` with
/// `constant_predictor_rows_mismatch`, never a baseline computed from the
/// survivors.
///
/// Provenance: PLAT-1016
#[trace("FR-108-AC-10", "TC-1924")]
#[test]
fn tc_1016_006_lost_item_rows_are_inconclusive_not_a_smaller_baseline() {
    let plans = load(METRIC).unwrap();
    let mut observations = with_items(13, "MP-900");
    // Drop the last `coverage` row: 14 items against `examined: 15`.
    observations.truncate(15);
    let admitted = intake(&plans, &collection("run-1016", &observations)).unwrap();
    let result = verify(
        &plans[0],
        &[Ranked::new(&admitted, Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    );
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert!(
        result
            .reasons
            .contains(&Reason::ConstantPredictorRowsMismatch),
        "{result:?}"
    );
    assert_eq!(result.decisions[0].baseline, None);
    assert_eq!(result.decisions[0].holds, None);
}

/// Verify `observations` against the plan, as intake stores them.
fn verdict_of(observations: &[Value]) -> quoin_measurement::verify::MeasurementVerdict {
    let plans = load(METRIC).unwrap();
    let admitted = intake(&plans, &collection("run-1020", observations)).unwrap();
    verify(
        &plans[0],
        &[Ranked::new(&admitted, Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    )
}

/// The checker holds the producer's item-row families to the plan's declared
/// `answer_families`: a producer that merged the two declared families into
/// one, or split one, is `inconclusive` with `constant_predictor_family_unexpected`,
/// and one that lost a declared family entirely with
/// `constant_predictor_family_missing`; no baseline is computed in either.
///
/// Provenance: PLAT-1020
#[trace("FR-108-AC-24", "TC-2300")]
#[test]
fn tc_1020_001_item_row_families_that_differ_from_the_declared_list_are_inconclusive() {
    let mut merged = with_items(13, "MP-900");
    for row in &mut merged[1..] {
        row["dimensions"]["family"] = json!("everything");
    }
    let result = verdict_of(&merged);
    assert_eq!(result.verdict, Verdict::Inconclusive);
    assert!(
        result
            .reasons
            .contains(&Reason::ConstantPredictorFamilyUnexpected),
        "{result:?}"
    );
    assert_eq!(result.decisions[0].baseline, None);

    let mut split = with_items(13, "MP-900");
    split[1]["dimensions"]["family"] = json!("weakness_kind_odd");
    let result = verdict_of(&split);
    assert!(
        result
            .reasons
            .contains(&Reason::ConstantPredictorFamilyUnexpected),
        "{result:?}"
    );

    let mut dropped = with_items(13, "MP-900");
    dropped.truncate(12);
    dropped[0] = aggregate(8);
    dropped[0]["population"]["examined"] = json!(11);
    dropped[0]["value"] = json!(8.0 / 11.0);
    let result = verdict_of(&dropped);
    assert!(
        result
            .reasons
            .contains(&Reason::ConstantPredictorFamilyMissing),
        "{result:?}"
    );
    assert_eq!(result.decisions[0].baseline, None);
}

/// A plan that declares `baseline: constant-predictor` must declare
/// `statistical_design.answer_families`; without it the plan load is refused
/// as `QM-PLAN-INVALID` naming the member.
///
/// Provenance: PLAT-1020
#[trace("FR-108-AC-25", "TC-2301")]
#[test]
fn tc_1020_002_answer_families_are_required_with_constant_predictor() {
    let try_load = |document: String| {
        let temporary = tempfile::tempdir().expect("a temporary directory");
        let assurance = temporary.path().join("spec").join("assurance");
        std::fs::create_dir_all(&assurance).expect("creatable");
        std::fs::write(assurance.join("MP-900.md"), document).expect("writable");
        load_plans(temporary.path())
    };
    let document = plan_document(METRIC);

    let without = document.replace(
        "  answer_families:\n    - weakness_kind\n    - coverage\n",
        "",
    );
    assert_ne!(without, document);
    let error = try_load(without).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
    assert!(
        error
            .to_string()
            .contains("statistical_design.answer_families")
    );

    assert!(try_load(document).is_ok());
}
