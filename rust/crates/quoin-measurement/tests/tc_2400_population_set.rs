// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-110: a percentage metric is taken over a population the plan
//! enumerates (`statistical_design.population_set`), so the denominator is a
//! checkable set a producer cannot choose.
//!
//! Plans are real assurance documents loaded from disk; intake runs through
//! `validate::measurement_collection`; verification through `rows::assess`.
//!
//! Provenance: PLAT-110

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::needless_pass_by_value,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::Path;

use engineering_assurance::measurement::Estimator;
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::population_set::{self, SetFault};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::observation::{
    Dimensions, MeasurementObservation, MeasurementPopulation, MeasurementShape, MeasurementState,
};
use quoin_measurement::types::plan::{MeasurementPlan, PopulationSet, StatisticalDesign};
use quoin_measurement::validate;
use quoin_measurement::verify::Reason;
use quoin_measurement::verify::rows::assess;
use serde_json::{Value, json};

/// An active plan for `quality.gate` whose `statistical_design` carries
/// `population` verbatim (each line indented as YAML needs).
fn plan_document(population: &str) -> String {
    format!(
        "---\n\
         id: MP-900\n\
         title: Example gate\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: ratchet\n\
         metric: quality.gate\n\
         definition_version: quality.gate-v1\n\
         protected_apparatus:\n\
         \x20\x20- spec/assurance/MP-900.md\n\
         negative_controls:\n\
         \x20\x20- kind: apparatus-edit\n\
         \x20\x20\x20\x20description: the plan document defines the measurement\n\
         statistical_design:\n\
         \x20\x20population: every declared path\n\
         \x20\x20sampling: exhaustive\n\
         \x20\x20repetitions: 1\n\
         \x20\x20estimator: proportion\n\
         \x20\x20error_model: binomial\n\
         \x20\x20uncertainty: none\n\
         \x20\x20decision_rule:\n\
         \x20\x20\x20\x20comparator: ge\n\
         \x20\x20\x20\x20threshold: 1.0\n\
         {population}\
         ---\n\
         \n\
         # Example gate\n"
    )
}

/// The set `a, b, c`, declared.
const SET_ABC: &str = "\x20\x20population_set:\n\x20\x20\x20\x20ids: [a, b, c]\n";

fn plans_from(document: &str) -> (tempfile::TempDir, Vec<MeasurementPlan>) {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(assurance.join("MP-900.md"), document).expect("the plan is writable");
    let plans = load_measurement_plans(
        &DiskMeasurement::new(Path::new(temporary.path())),
        PlanLoadOptions::default(),
    )
    .expect("the authored plan loads");
    (temporary, plans)
}

/// A measured ratio observation with `population` as given.
fn observation(population: Value) -> Value {
    json!({
        "metric": "quality.gate",
        "planId": "MP-900",
        "definitionVersion": "quality.gate-v1",
        "state": "measured",
        "value": 0.67,
        "unit": "fraction",
        "shape": "ratio",
        "population": population,
    })
}

fn abc(matched: &[&str]) -> Value {
    json!({
        "examined": 3, "matched": matched.len(),
        "examined_ids": ["a", "b", "c"], "matched_ids": matched,
    })
}

fn intake(
    plans: &[MeasurementPlan],
    observation: Value,
) -> Result<String, quoin_measurement::MeasurementError> {
    let candidate = json!({
        "schemaVersion": 2,
        "collectionId": "run-110",
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "timestamp": "2026-09-22T00:00:00.000Z",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"],
            "artifacts": ["config", "spec/assurance/MP-900.md"],
        },
        "observations": [observation],
        "rawEvidence": { "payload": [1, 2] },
    });
    validate::measurement_collection(&from_serde(&candidate).expect("the bridge"), plans)
        .map(|admitted| admitted.collection_id.as_str().to_owned())
}

fn refused(population: Value) -> quoin_measurement::MeasurementError {
    let (_dir, plans) = plans_from(&plan_document(SET_ABC));
    intake(&plans, observation(population)).expect_err("the population is refused")
}

/// A declared set is read into the plan, in sorted order; a plan declaring
/// none reads as before.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-1", "TC-2400")]
#[test]
fn tc_2400_a_declared_population_set_is_read_into_the_plan() {
    let (_dir, plans) = plans_from(&plan_document(
        "\x20\x20population_set:\n\x20\x20\x20\x20ids: [c, a, b]\n",
    ));
    let set = plans[0]
        .statistical_design
        .as_ref()
        .and_then(|design| design.population_set.as_ref())
        .expect("the set is read");
    assert_eq!(set.iter().collect::<Vec<_>>(), ["a", "b", "c"]);
    let (_dir, plans) = plans_from(&plan_document(""));
    assert!(
        plans[0]
            .statistical_design
            .as_ref()
            .unwrap()
            .population_set
            .is_none()
    );
}

/// An empty set, an empty or repeated or non-string id, an unknown member and
/// a non-object all refuse the plan load.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-1", "TC-2401")]
#[test]
fn tc_2401_a_malformed_population_set_refuses_the_plan_load() {
    for (declaration, expected) in [
        ("ids: []", "at least one id"),
        ("ids: [a, a]", "`a` is declared twice"),
        ("ids: [a, \"\"]", "must not be empty"),
        ("ids: [a, 3]", "list of strings"),
        ("ids: a", "list of strings"),
        (
            "ids: [a]\n\x20\x20\x20\x20from: code",
            "unknown member `from`",
        ),
    ] {
        let extra = format!("\x20\x20population_set:\n\x20\x20\x20\x20{declaration}\n");
        let temporary = tempfile::tempdir().unwrap();
        let assurance = temporary.path().join("spec").join("assurance");
        std::fs::create_dir_all(&assurance).unwrap();
        std::fs::write(assurance.join("MP-900.md"), plan_document(&extra)).unwrap();
        let refusal = load_measurement_plans(
            &DiskMeasurement::new(temporary.path()),
            PlanLoadOptions::default(),
        )
        .expect_err(&extra);
        assert_eq!(refusal.code(), MeasurementErrorCode::PlanInvalid, "{extra}");
        assert!(
            refusal.subject().contains(expected),
            "{expected}: {refusal}"
        );
    }
    let scalar = plan_document("\x20\x20population_set: all\n");
    let temporary = tempfile::tempdir().unwrap();
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::write(assurance.join("MP-900.md"), scalar).unwrap();
    let refusal = load_measurement_plans(
        &DiskMeasurement::new(temporary.path()),
        PlanLoadOptions::default(),
    )
    .expect_err("a non-object set is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::PlanInvalid);
}

/// The exact set examined, with any subset matched (including none), is
/// admitted.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-2", "TC-2402")]
#[test]
fn tc_2402_the_exact_set_examined_is_admitted() {
    let (_dir, plans) = plans_from(&plan_document(SET_ABC));
    for matched in [&["a", "b", "c"][..], &["a"], &[]] {
        assert_eq!(
            intake(&plans, observation(abc(matched)))
                .unwrap_or_else(|e| panic!("{matched:?}: {e}")),
            "run-110"
        );
    }
}

/// A measured observation stating no id list is refused as unstated, never
/// admitted on its count alone; a `not_computed` one is not held to it.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-3", "TC-2403")]
#[test]
fn tc_2403_unstated_id_lists_are_refused_and_not_computed_is_exempt() {
    for population in [
        json!({ "examined": 3, "matched": 3 }),
        json!({ "examined": 3, "matched": 3, "examined_ids": ["a", "b", "c"] }),
        json!({ "examined": 3, "matched": 3, "matched_ids": ["a", "b", "c"] }),
    ] {
        let refusal = refused(population);
        assert_eq!(refusal.code(), MeasurementErrorCode::PopulationUnstated);
        assert!(
            refusal.findings()[0].contains("population.examined_ids")
                || refusal.findings()[0].contains("population.matched_ids")
        );
    }
    let (_dir, plans) = plans_from(&plan_document(SET_ABC));
    let mut skipped = observation(json!({}));
    skipped["state"] = json!("not_computed");
    skipped["value"] = Value::Null;
    skipped["reason"] = json!("not run");
    assert!(intake(&plans, skipped).is_ok());
}

/// Duplicate ids, non-lists, empty ids and counts disagreeing with the lists
/// are malformed.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-3", "TC-2404")]
#[test]
fn tc_2404_malformed_id_lists_and_disagreeing_counts_are_refused() {
    for population in [
        json!({ "examined_ids": ["a", "a", "b", "c"], "matched_ids": [] }),
        json!({ "examined_ids": "a,b,c", "matched_ids": [] }),
        json!({ "examined_ids": ["a", "b", "c"], "matched_ids": ["a", ""] }),
        json!({ "examined_ids": ["a", "b", "c"], "matched_ids": [], "examined": 4 }),
        json!({ "examined_ids": ["a", "b", "c"], "matched_ids": ["a"], "matched": 3 }),
    ] {
        let refusal = refused(population.clone());
        assert_eq!(
            refusal.code(),
            MeasurementErrorCode::PopulationMalformed,
            "{population}: {refusal}"
        );
    }
}

/// An id outside the declared set is refused, whether examined or matched,
/// and the finding names it.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-4", "TC-2405")]
#[test]
fn tc_2405_ids_outside_the_declared_set_are_refused() {
    let swapped = json!({
        "examined": 3, "matched": 0,
        "examined_ids": ["a", "b", "z"], "matched_ids": [],
    });
    let refusal = refused(swapped);
    assert_eq!(refusal.code(), MeasurementErrorCode::PopulationOutsideSet);
    assert!(refusal.findings()[0].contains("outside plan MP-900's population: z"));
    let matched_outside = json!({
        "examined": 3, "matched": 1,
        "examined_ids": ["a", "b", "c"], "matched_ids": ["q"],
    });
    assert_eq!(
        refused(matched_outside).code(),
        MeasurementErrorCode::PopulationOutsideSet
    );
}

/// A producer examining only the convenient part of the set is refused and
/// the omitted ids are named; a count claiming the whole set does not help.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-4", "TC-2406")]
#[test]
fn tc_2406_a_convenient_subset_is_refused_naming_the_omitted_ids() {
    let subset = json!({
        "examined": 1, "matched": 1,
        "examined_ids": ["a"], "matched_ids": ["a"],
    });
    let refusal = refused(subset);
    assert_eq!(refusal.code(), MeasurementErrorCode::PopulationPartial);
    assert!(refusal.findings()[0].contains("did not examine 2"));
    assert!(refusal.findings()[0].contains("b, c"));
    let empty = json!({ "examined": 0, "matched": 0, "examined_ids": [], "matched_ids": [] });
    assert_eq!(
        refused(empty).code(),
        MeasurementErrorCode::PopulationPartial
    );
    let claimed = json!({
        "examined": 3, "matched": 1,
        "examined_ids": ["a"], "matched_ids": ["a"],
    });
    assert_eq!(
        refused(claimed).code(),
        MeasurementErrorCode::PopulationMalformed
    );
}

fn text(value: &str) -> NonEmptyText {
    NonEmptyText::parse(value, MeasurementErrorCode::CollectionInvalid, "x").unwrap()
}

fn verify_observation(population: Value) -> MeasurementObservation {
    let mut stated = MeasurementPopulation {
        examined: population["examined"].as_f64(),
        matched: population["matched"].as_f64(),
        complete: Some(true),
        ..MeasurementPopulation::EMPTY
    };
    for member in [population_set::EXAMINED_IDS, population_set::MATCHED_IDS] {
        if let Some(ids) = population.get(member) {
            stated
                .unmodelled
                .insert(member.to_owned(), from_serde(ids).unwrap());
        }
    }
    MeasurementObservation {
        metric: text("quality.gate"),
        plan_id: text("MP-900"),
        definition_version: text("quality.gate-v1"),
        state: MeasurementState::Measured,
        value: Some(0.67),
        unit: text("fraction"),
        shape: MeasurementShape::Ratio,
        population: Some(stated),
        dimensions: Dimensions::ABSENT,
        reason: None,
        interval: None,
    }
}

/// `quoin measurement verify` applies the same check: the exact set is
/// assessed, an unstated list is inconclusive and the rest reject.
///
/// Provenance: PLAT-110
#[trace("FR-131-AC-5", "TC-2407")]
#[test]
fn tc_2407_verify_assesses_only_the_declared_population() {
    let set = PopulationSet::new(["a", "b", "c"].map(str::to_owned)).unwrap();
    let design = StatisticalDesign {
        minimum_population: None,
        repetitions: None,
        estimator: Some(Estimator::Proportion),
        decision_rule: None,
        answer_families: None,
        population_set: Some(set),
    };
    let run = |population: Value| {
        assess(
            &design,
            Estimator::Proportion,
            &verify_observation(population),
        )
    };
    assert!(run(abc(&["a", "b"])).is_ok());
    assert_eq!(
        run(json!({ "examined": 3, "matched": 3 })).unwrap_err(),
        Reason::PopulationUnstated
    );
    assert_eq!(
        run(json!({ "examined": 3, "matched": 0, "examined_ids": ["a", "a", "b"], "matched_ids": [] }))
            .unwrap_err(),
        Reason::PopulationMalformed
    );
    assert_eq!(
        run(json!({ "examined": 3, "matched": 0, "examined_ids": ["a", "b", "z"], "matched_ids": [] }))
            .unwrap_err(),
        Reason::PopulationOutsideSet
    );
    assert_eq!(
        run(json!({ "examined": 1, "matched": 1, "examined_ids": ["a"], "matched_ids": ["a"] }))
            .unwrap_err(),
        Reason::PopulationPartial
    );
    let fault = population_set::check(
        design.population_set.as_ref().unwrap(),
        None,
        None,
        None,
        None,
    );
    assert_eq!(fault, Err(SetFault::Unstated(population_set::EXAMINED_IDS)));
}
