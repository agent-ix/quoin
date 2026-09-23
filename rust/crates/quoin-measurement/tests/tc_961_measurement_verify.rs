// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-961 (part 1): the independent measurement-verdict checker.
//!
//! The deliberately wrong collections live under
//! `tests/fixtures/verify/wrong/`, one directory per defect, and the checker
//! must reject each one. `tests/fixtures/verify/right/accept/` is the positive
//! control: the same apparatus, consistent, and accepted — without it, a
//! checker that rejected everything would pass every test here.
//!
//! Plans are real assurance documents loaded through
//! `load_measurement_plans`, and collections are read through the store's own
//! `stored_measurement_collection`, exactly as `measurement.verify` reads them.
//!
//! Provenance: PLAT-961

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::float_cmp,
    reason = "FR-108-AC-4 makes exact equality the contract: a recomputed estimate either is the stored value or is not"
)]

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use engineering_assurance::measurement::{Comparator, DecisionRule, Estimator};
use proptest::prelude::*;
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::MemoryMeasurement;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::observation::{
    Dimensions, MeasurementObservation, MeasurementPopulation, MeasurementShape, MeasurementState,
};
use quoin_measurement::types::plan::{MeasurementPlan, StatisticalDesign};
use quoin_measurement::verify::rows::{assess, recompute};
use quoin_measurement::verify::{EstimateBasis, MeasurementVerdict};
use quoin_measurement::{
    Ranked, Reason, Verdict, stored_measurement_collection, verdict_json, verify,
};
use quoin_store::parse_strict_json;
use serde_json::json;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("verify")
}

/// Every fixture plan, loaded the way the store loads them.
fn plans() -> Vec<MeasurementPlan> {
    let mut source = MemoryMeasurement::new();
    for entry in std::fs::read_dir(fixtures().join("plans")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        source = source.with_document(
            format!("spec/assurance/{name}"),
            std::fs::read_to_string(&path).unwrap(),
        );
    }
    load_measurement_plans(&source, PlanLoadOptions::default()).expect("the fixture plans load")
}

fn plan(id: &str) -> MeasurementPlan {
    plans()
        .into_iter()
        .find(|plan| plan.id.as_str() == id)
        .expect("a fixture plan with this id")
}

fn read(path: &Path) -> MeasurementCollection {
    let bytes = std::fs::read(path).unwrap();
    stored_measurement_collection(&parse_strict_json(&bytes).unwrap()).unwrap()
}

/// A case directory's collections, in file-name order.
fn case(relative: &str) -> Vec<MeasurementCollection> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures().join(relative))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    paths.iter().map(|path| read(path)).collect()
}

/// Each collection in its own intake group, in the order given.
fn in_order(collections: &[MeasurementCollection]) -> Vec<Ranked<'_>> {
    (0_u64..)
        .zip(collections)
        .map(|(intake, collection)| Ranked {
            collection,
            intake: Some(intake),
        })
        .collect()
}

fn check(plan_id: &str, relative: &str, claimed: Option<Verdict>) -> MeasurementVerdict {
    let collections = case(relative);
    verify(&plan(plan_id), &in_order(&collections), claimed)
}

/// A measured `gate.pass_rate` observation of `value` over `population`.
fn observation(value: f64, population: Option<MeasurementPopulation>) -> MeasurementObservation {
    let text = |value: &str| {
        quoin_measurement::types::ids::NonEmptyText::parse(
            value,
            MeasurementErrorCode::CollectionInvalid,
            "x",
        )
        .unwrap()
    };
    MeasurementObservation {
        metric: text("gate.pass_rate"),
        plan_id: text("MP-961"),
        definition_version: text("gate.pass-rate-v1"),
        state: MeasurementState::Measured,
        value: Some(value),
        unit: text("fraction"),
        shape: MeasurementShape::Ratio,
        population,
        dimensions: Dimensions::ABSENT,
        reason: None,
    }
}

fn population(examined: f64, matched: f64, complete: Option<bool>) -> MeasurementPopulation {
    MeasurementPopulation {
        examined: Some(examined),
        matched: Some(matched),
        complete,
        ..MeasurementPopulation::EMPTY
    }
}

fn design(estimator: Estimator) -> StatisticalDesign {
    StatisticalDesign {
        minimum_population: None,
        repetitions: None,
        estimator: Some(estimator),
        decision_rule: Some(DecisionRule::against_threshold(Comparator::Ge, 0.8).unwrap()),
    }
}

/// The accepted collection's one observation, with `edit` applied, wrapped
/// in a copy of that collection.
fn accepted_with(edit: impl FnOnce(&mut MeasurementObservation)) -> MeasurementCollection {
    let mut collection = case("right/accept").remove(0);
    edit(&mut collection.observations[0]);
    collection
}

/// Trace: FR-108-AC-1
/// Provenance: PLAT-961
#[test]
fn tc_961_001_estimator_and_decision_rule_are_read_into_eas_types() {
    let gate = plan("MP-961");
    let design = gate.statistical_design.unwrap();
    assert_eq!(design.estimator, Some(Estimator::Proportion));
    assert_eq!(
        design.decision_rule,
        Some(DecisionRule::against_threshold(Comparator::Ge, 0.8).unwrap())
    );
    assert_eq!(design.minimum_population, NonZeroU32::new(10));
}

fn plan_document(design: &str, objective: &str) -> String {
    format!(
        "---\nid: MP-9\ntitle: A plan\ntype: MeasurementPlan\nstatus: active\nstage: gate\n\
         metric: m\ndefinition_version: v1\n{objective}statistical_design:\n{design}---\n\n# A plan\n"
    )
}

/// Trace: FR-108-AC-1
/// Provenance: PLAT-961
#[test]
fn tc_961_002_a_malformed_estimator_or_rule_refuses_the_plan_load() {
    let higher = "objective:\n  direction: higher\n";
    for (design, objective, names) in [
        (
            "  estimator: fraction correct\n",
            "",
            "statistical_design.estimator",
        ),
        (
            "  decision_rule: pass at 0.9\n",
            "",
            "statistical_design.decision_rule",
        ),
        (
            "  decision_rule:\n    comparator: ge\n    threshold: 0.9\n    baseline: best-seen\n",
            "",
            "statistical_design.decision_rule",
        ),
        (
            "  decision_rule:\n    comparator: ge\n    threshold: 0.9\n    margin: 0.1\n",
            "",
            "statistical_design.decision_rule",
        ),
        (
            "  estimator: count\n  decision_rule:\n    comparator: gt\n    baseline: constant-predictor\n",
            "",
            "statistical_design.decision_rule",
        ),
        (
            "  decision_rule:\n    comparator: le\n    threshold: 0.9\n",
            higher,
            "statistical_design.decision_rule",
        ),
    ] {
        let source = MemoryMeasurement::new()
            .with_document("spec/assurance/MP-9.md", plan_document(design, objective));
        let error = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid, "{design}");
        assert!(error.to_string().contains(names), "{error}");
    }
}

/// Trace: FR-108-AC-2, FR-108-AC-4
/// Provenance: PLAT-961
#[test]
fn tc_961_003_a_consistent_collection_is_accepted_and_its_estimate_recomputed() {
    let verdict = check("MP-961", "right/accept", Some(Verdict::Accept));
    assert_eq!(verdict.verdict, Verdict::Accept);
    assert!(verdict.reasons.is_empty());
    assert_eq!(verdict.candidate.as_deref(), Some("verify-accept"));
    assert_eq!(verdict.decisions.len(), 1);
    assert_eq!(verdict.decisions[0].estimate.value, 0.9);
    assert_eq!(
        verdict.decisions[0].estimate.basis,
        EstimateBasis::Recomputed
    );
    assert_eq!(verdict.decisions[0].holds, Some(true));
    assert_eq!(verdict.counts.collections_considered, 1);
    assert_eq!(verdict.counts.observations_recomputed, 1);
    assert_eq!(verdict.counts.observations_asserted, 0);
    assert_eq!(verdict.counts.regressed_runs, 0);
}

/// Trace: FR-108-AC-5
/// Provenance: PLAT-961
#[test]
fn tc_961_004_a_wrong_claimed_verdict_is_rejected() {
    let verdict = check(
        "MP-961",
        "wrong/wrong-claimed-verdict",
        Some(Verdict::Accept),
    );
    assert_eq!(verdict.verdict, Verdict::Reject);
    assert_eq!(
        verdict.reasons,
        [Reason::RuleNotMet, Reason::ClaimedVerdictDisagrees]
    );
    // The same data with no claim is still rejected by the rule, and an
    // accepted collection claimed as rejected is rejected for the claim alone.
    let unclaimed = check("MP-961", "wrong/wrong-claimed-verdict", None);
    assert_eq!(unclaimed.reasons, [Reason::RuleNotMet]);
    let understated = check("MP-961", "right/accept", Some(Verdict::Reject));
    assert_eq!(understated.verdict, Verdict::Reject);
    assert_eq!(understated.reasons, [Reason::ClaimedVerdictDisagrees]);
}

/// Trace: FR-108-AC-3, FR-108-AC-4
/// Provenance: PLAT-961
#[test]
fn tc_961_005_a_hand_edited_baseline_is_rejected() {
    let verdict = check("MP-961-prior", "wrong/hand-edited-baseline", None);
    assert_eq!(verdict.verdict, Verdict::Reject);
    assert!(verdict.reasons.contains(&Reason::ValueDisagreesWithRows));
    let finding = verdict
        .findings
        .iter()
        .find(|finding| finding.reason == Reason::ValueDisagreesWithRows)
        .unwrap();
    assert_eq!(
        finding.collection_id.as_deref(),
        Some("verify-edited-prior")
    );
    // The edited 0.5 never became the baseline the 0.8 candidate passed.
    assert_ne!(verdict.decisions[0].baseline, Some(0.5));
    assert_ne!(verdict.decisions[0].holds, Some(true));
}

/// Trace: FR-108-AC-3
/// Provenance: PLAT-961
#[test]
fn tc_961_006_a_selectively_reported_run_is_rejected_and_the_regression_counted() {
    let verdict = check("MP-961", "wrong/selectively-reported-run", None);
    assert_eq!(verdict.verdict, Verdict::Reject);
    assert_eq!(verdict.reasons, [Reason::RerunUntilPass]);
    assert_eq!(verdict.candidate.as_deref(), Some("verify-rerun-pass"));
    assert_eq!(verdict.decisions[0].holds, Some(true));
    assert_eq!(verdict.counts.collections_considered, 2);
    assert_eq!(verdict.counts.regressed_runs, 1);
    assert_eq!(verdict.regressed_runs, ["verify-rerun-regressed"]);
    // A regression under a different source revision is history, counted
    // but not held against a later pass.
    let mut collections = case("wrong/selectively-reported-run");
    collections[1].source_revision = quoin_measurement::types::ids::NonEmptyText::parse(
        "b".repeat(40).as_str(),
        MeasurementErrorCode::CollectionInvalid,
        "sourceRevision",
    )
    .unwrap();
    let fixed = verify(&plan("MP-961"), &in_order(&collections), None);
    assert_eq!(fixed.verdict, Verdict::Accept);
    assert_eq!(fixed.counts.regressed_runs, 1);
}

/// Trace: FR-108-AC-4
/// Provenance: PLAT-961
#[test]
fn tc_961_007_a_short_population_is_rejected() {
    let verdict = check("MP-961", "wrong/short-population", None);
    assert_eq!(verdict.verdict, Verdict::Reject);
    assert_eq!(verdict.reasons, [Reason::PopulationBelowMinimum]);
}

/// Trace: FR-108-AC-4
/// Provenance: PLAT-961
#[test]
fn tc_961_008_a_tampered_observation_is_rejected() {
    let verdict = check(
        "MP-961",
        "wrong/tampered-observation",
        Some(Verdict::Accept),
    );
    assert_eq!(verdict.verdict, Verdict::Reject);
    assert!(verdict.reasons.contains(&Reason::ValueDisagreesWithRows));
    // A tampered observation was still recomputed; it is not asserted.
    assert_eq!(verdict.counts.observations_recomputed, 1);
    assert_eq!(verdict.counts.observations_asserted, 0);
}

/// Trace: FR-108-AC-4
/// Provenance: PLAT-961
#[test]
fn tc_961_009_an_empty_incomplete_or_unstated_population_is_inconclusive_never_accepted() {
    let gate = plan("MP-961");
    for (edit, reason) in [
        (
            Box::new(|o: &mut MeasurementObservation| o.population = None)
                as Box<dyn Fn(&mut MeasurementObservation)>,
            Reason::PopulationUnstated,
        ),
        (
            Box::new(|o: &mut MeasurementObservation| {
                o.population.as_mut().unwrap().complete = None;
            }),
            Reason::PopulationUnstated,
        ),
        (
            Box::new(|o: &mut MeasurementObservation| {
                o.population.as_mut().unwrap().complete = Some(false);
            }),
            Reason::PopulationIncomplete,
        ),
        (
            Box::new(|o: &mut MeasurementObservation| {
                o.population = Some(population(0.0, 0.0, Some(true)));
                o.value = Some(0.0);
            }),
            Reason::PopulationEmpty,
        ),
        (
            Box::new(|o: &mut MeasurementObservation| {
                o.state = MeasurementState::NotComputed;
                o.value = None;
            }),
            Reason::NoValue,
        ),
    ] {
        let collection = accepted_with(edit);
        let verdict = verify(&gate, &in_order(std::slice::from_ref(&collection)), None);
        assert_eq!(verdict.verdict, Verdict::Inconclusive, "{reason:?}");
        assert_eq!(verdict.reasons, [reason]);
    }
}

/// Trace: FR-108-AC-5
/// Provenance: PLAT-961
#[test]
fn tc_961_010_a_plan_without_a_rule_or_estimator_or_any_run_is_inconclusive() {
    let accepted = case("right/accept");
    let mut gate = plan("MP-961");
    let mut no_rule = gate.clone();
    no_rule.statistical_design.as_mut().unwrap().decision_rule = None;
    assert_eq!(
        verify(&no_rule, &in_order(&accepted), None).reasons,
        [Reason::NoDecisionRule]
    );
    let mut no_estimator = gate.clone();
    no_estimator.statistical_design.as_mut().unwrap().estimator = None;
    assert_eq!(
        verify(&no_estimator, &in_order(&accepted), None).reasons,
        [Reason::NoEstimator]
    );
    let nothing = verify(&gate, &[], None);
    assert_eq!(nothing.verdict, Verdict::Inconclusive);
    assert_eq!(nothing.reasons, [Reason::NoCollections]);
    // A run under another definition version is not a run of this plan.
    gate.definition_version = quoin_measurement::types::ids::NonEmptyText::parse(
        "gate.pass-rate-v2",
        MeasurementErrorCode::PlanInvalid,
        "definition_version",
    )
    .unwrap();
    assert_eq!(
        verify(&gate, &in_order(&accepted), None).reasons,
        [Reason::NoCollections]
    );
}

/// Trace: FR-108-AC-3
/// Provenance: PLAT-961
#[test]
fn tc_961_011_a_constant_predictor_baseline_is_inconclusive_without_per_item_rows() {
    let mut gate = plan("MP-961");
    gate.statistical_design.as_mut().unwrap().decision_rule = Some(
        DecisionRule::against_baseline(
            Comparator::Gt,
            engineering_assurance::measurement::Baseline::ConstantPredictor,
            Some(0.05),
        )
        .unwrap(),
    );
    let accepted = case("right/accept");
    let verdict = verify(&gate, &in_order(&accepted), None);
    assert_eq!(verdict.verdict, Verdict::Inconclusive);
    assert_eq!(verdict.reasons, [Reason::ConstantPredictorRowsAbsent]);
}

/// Trace: FR-108-AC-2
/// Provenance: PLAT-961
#[test]
fn tc_961_012_intake_order_decides_the_candidate_and_a_tie_is_unattested() {
    // Backdate the regressed run's stated timestamp to after the pass: the
    // stated time would make it the candidate, and intake order does not.
    let mut collections = case("wrong/selectively-reported-run");
    collections[0].timestamp = quoin_measurement::types::ids::NonEmptyText::parse(
        "2030-01-01T00:00:00Z",
        MeasurementErrorCode::CollectionInvalid,
        "timestamp",
    )
    .unwrap();
    let verdict = verify(&plan("MP-961"), &in_order(&collections), None);
    assert_eq!(verdict.candidate.as_deref(), Some("verify-rerun-pass"));
    // Two runs with no attested position, or one shared position, tie.
    for intake in [None, Some(7)] {
        let tied: Vec<Ranked<'_>> = collections
            .iter()
            .map(|collection| Ranked { collection, intake })
            .collect();
        let verdict = verify(&plan("MP-961"), &tied, None);
        assert!(
            verdict.reasons.contains(&Reason::OrderUnattested),
            "{intake:?}"
        );
        assert_ne!(verdict.verdict, Verdict::Accept);
    }
    // A lone run needs no order.
    let accepted = case("right/accept");
    let lone = [Ranked {
        collection: &accepted[0],
        intake: None,
    }];
    let verdict = verify(&plan("MP-961"), &lone, None);
    assert_eq!(verdict.verdict, Verdict::Accept);
    assert_eq!(verdict.counts.order_unattested, 1);
}

/// Trace: FR-108-AC-3
/// Provenance: PLAT-961
#[test]
fn tc_961_013_baselines_come_from_recomputed_rows_through_eas_rule() {
    let prior = plan("MP-961-prior");
    // Each run at its own source revision: a fix, not a rerun.
    let honest = |value: f64, matched: f64, id: &str| {
        let text = |value: &str| {
            quoin_measurement::types::ids::NonEmptyText::parse(
                value,
                MeasurementErrorCode::CollectionInvalid,
                "x",
            )
            .unwrap()
        };
        let mut collection = case("wrong/hand-edited-baseline").remove(0);
        collection.collection_id = text(id);
        collection.source_revision = text(&id.repeat(40));
        collection.observations[0].value = Some(value);
        collection.observations[0].population = Some(population(10.0, matched, Some(true)));
        collection
    };
    // prior-collection: the latest earlier run, not the best one.
    let runs = [
        honest(0.9, 9.0, "a"),
        honest(0.6, 6.0, "b"),
        honest(0.7, 7.0, "c"),
    ];
    let verdict = verify(&prior, &in_order(&runs), None);
    assert_eq!(verdict.verdict, Verdict::Accept);
    assert_eq!(verdict.decisions[0].baseline, Some(0.6));
    assert_eq!(verdict.regressed_runs, ["b"]);
    // best-seen with a margin: 0.7 must be at least best (0.9) + 0.05.
    let mut best = prior.clone();
    best.statistical_design.as_mut().unwrap().decision_rule = Some(
        DecisionRule::against_baseline(
            Comparator::Ge,
            engineering_assurance::measurement::Baseline::BestSeen,
            Some(0.05),
        )
        .unwrap(),
    );
    let verdict = verify(&best, &in_order(&runs), None);
    assert_eq!(verdict.decisions[0].baseline, Some(0.9));
    assert_eq!(verdict.reasons, [Reason::RuleNotMet]);
    // The first run has nothing to compare against.
    let first = verify(&best, &in_order(&runs[..1]), None);
    assert_eq!(first.reasons, [Reason::NoPrior]);
}

/// Trace: FR-108-AC-4
/// Provenance: PLAT-961
#[test]
fn tc_961_014_an_estimate_no_row_data_can_check_is_counted_as_asserted() {
    let mut gate = plan("MP-961");
    gate.statistical_design.as_mut().unwrap().estimator = Some(Estimator::Mean);
    let accepted = case("right/accept");
    let verdict = verify(&gate, &in_order(&accepted), None);
    assert_eq!(verdict.verdict, Verdict::Accept);
    assert_eq!(verdict.decisions[0].estimate.basis, EstimateBasis::Asserted);
    assert_eq!(verdict.counts.observations_asserted, 1);
    assert_eq!(verdict.counts.observations_recomputed, 0);
}

proptest! {
    /// Trace: FR-108-AC-4
    /// Provenance: PLAT-961
    #[test]
    fn tc_961_015_proportion_and_count_are_recomputed_from_matched_and_examined(
        examined in 1_u32..1_000_000,
        share in 0.0_f64..=1.0,
    ) {
        let examined = f64::from(examined);
        let matched = (examined * share).floor();
        let proportion = recompute(Estimator::Proportion, Some(matched), examined).unwrap();
        prop_assert!((0.0..=1.0).contains(&proportion));
        prop_assert_eq!(proportion, matched / examined);
        prop_assert_eq!(recompute(Estimator::Count, Some(matched), examined), Some(matched));
        for estimator in [Estimator::Mean, Estimator::Median, Estimator::Ratio] {
            prop_assert_eq!(recompute(estimator, Some(matched), examined), None);
        }
        // A stored value equal to the recomputation is accepted as recomputed.
        let honest = observation(proportion, Some(population(examined, matched, Some(true))));
        let estimate = assess(&design(Estimator::Proportion), Estimator::Proportion, &honest).unwrap();
        prop_assert_eq!(estimate.value, proportion);
        prop_assert_eq!(estimate.basis, EstimateBasis::Recomputed);
    }

    /// Trace: FR-108-AC-4
    /// Provenance: PLAT-961
    #[test]
    fn tc_961_016_any_stored_value_other_than_the_recomputation_is_refused(
        examined in 1_u32..10_000,
        share in 0.0_f64..=1.0,
        stored in -2.0_f64..2.0,
    ) {
        let examined = f64::from(examined);
        let matched = (examined * share).floor();
        prop_assume!(stored != matched / examined);
        let tampered = observation(stored, Some(population(examined, matched, Some(true))));
        prop_assert_eq!(
            assess(&design(Estimator::Proportion), Estimator::Proportion, &tampered),
            Err(Reason::ValueDisagreesWithRows)
        );
        // Whatever the value, an incomplete population is never an estimate.
        let incomplete = observation(stored, Some(population(examined, matched, Some(false))));
        prop_assert_eq!(
            assess(&design(Estimator::Proportion), Estimator::Proportion, &incomplete),
            Err(Reason::PopulationIncomplete)
        );
    }
}

/// Trace: FR-108-AC-5, FR-108-AC-6
/// Provenance: PLAT-961
#[test]
fn tc_961_017_the_verdict_document_has_the_documented_wire_shape() {
    let verdict = check(
        "MP-961",
        "wrong/selectively-reported-run",
        Some(Verdict::Reject),
    );
    let expected = json!({
        "schema": "quoin.measurement-verdict.v1",
        "planId": "MP-961",
        "definitionVersion": "gate.pass-rate-v1",
        "verdict": "reject",
        "reasons": ["rerun_until_pass"],
        "claimed": "reject",
        "candidate": "verify-rerun-pass",
        "decisions": [{
            "dimensions": {},
            "estimate": 0.9,
            "estimateBasis": "recomputed",
            "baseline": null,
            "holds": true,
        }],
        "findings": [{
            "reason": "rerun_until_pass",
            "collectionId": "verify-rerun-regressed",
            "dimensions": null,
        }],
        "regressedRuns": ["verify-rerun-regressed"],
        "counts": {
            "collectionsConsidered": 2,
            "regressedRuns": 1,
            "observationsRecomputed": 2,
            "observationsAsserted": 0,
            "orderAttested": 2,
            "orderUnattested": 0,
        },
    });
    assert_eq!(verdict_json(&verdict).unwrap(), expected);
    // Every reason and verdict spelling round-trips, and no two share one.
    for reason in Reason::ALL {
        assert_eq!(Reason::from_wire(reason.as_str()), Some(reason));
    }
    let mut spellings: Vec<&str> = Reason::ALL.iter().map(|reason| reason.as_str()).collect();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), Reason::ALL.len());
    for verdict in Verdict::ALL {
        assert_eq!(Verdict::from_wire(verdict.as_str()), Some(verdict));
    }
}
