// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-1401: a `proportion` plan declares a `population_set` or a
//! `population_exempt` reason, or its metric cannot read as a pass.
//!
//! Plans are real assurance documents loaded through `load_measurement_plans`.
//!
//! Provenance: PLAT-1401

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use ix_trace_rs::trace;
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::report::build_measurement_report_from;
use quoin_measurement::report::verdict::{InconclusiveReason, StageVerdict};
use quoin_measurement::source::MemoryMeasurement;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::observation::{
    Dimensions, MeasurementObservation, MeasurementPopulation, MeasurementShape, MeasurementState,
};
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::validate;
use quoin_measurement::verify::Reason;
use quoin_measurement::verify::rows::assess;
use serde_json::json;

/// A ratchet plan with the given estimator, and `population` spliced into
/// its `statistical_design` (already indented as YAML needs).
fn plan_document(estimator: &str, population: &str) -> String {
    format!(
        "---\n\
         id: MP-958\n\
         title: Example plan\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: ratchet\n\
         metric: quality.score\n\
         definition_version: v1\n\
         objective:\n\x20\x20direction: higher\n\
         statistical_design:\n\
         \x20\x20population: every case\n\
         \x20\x20sampling: exhaustive\n\
         \x20\x20repetitions: 1\n\
         \x20\x20estimator: {estimator}\n\
         \x20\x20error_model: binomial\n\
         \x20\x20uncertainty: none\n\
         \x20\x20decision_rule:\n\
         \x20\x20\x20\x20comparator: ge\n\
         \x20\x20\x20\x20threshold: 0.5\n\
         {population}\
         ---\n\
         \n\
         # Example plan\n"
    )
}

fn plans_from(document: &str) -> Result<Vec<MeasurementPlan>, quoin_measurement::MeasurementError> {
    let source = MemoryMeasurement::new().with_document("spec/assurance/MP-958.md", document);
    load_measurement_plans(&source, PlanLoadOptions::default())
}

const SET: &str = "\x20\x20population_set:\n\x20\x20\x20\x20ids: [a, b]\n";
const EXEMPT: &str = "\x20\x20population_exempt: repeated passes of a live service\n";

fn collection(
    plans: &[MeasurementPlan],
    index: u8,
    population: &serde_json::Value,
) -> MeasurementCollection {
    let candidate = json!({
        "schemaVersion": 2,
        "collectionId": format!("run-{index}"),
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "timestamp": format!("2026-09-{:02}T00:00:00.000Z", index + 1),
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"],
            "artifacts": ["config"],
        },
        "observations": [{
            "metric": "quality.score",
            "planId": "MP-958",
            "definitionVersion": "v1",
            "state": "measured",
            "value": 1.0,
            "unit": "fraction",
            "shape": "ratio",
            "population": population,
        }],
        "rawEvidence": { "payload": [1, 2] },
    });
    validate::measurement_collection(&from_serde(&candidate).unwrap(), plans).unwrap()
}

/// The newest value's inconclusive reason in the report row, if any.
fn reason_of(
    plans: &[MeasurementPlan],
    population: &serde_json::Value,
) -> Option<InconclusiveReason> {
    let collections = [
        collection(plans, 0, population),
        collection(plans, 1, population),
    ];
    let report = build_measurement_report_from(
        Path::new("/repo"),
        plans,
        &collections,
        &[],
        &[],
        &quoin_measurement::IntakeOrder::none(),
    )
    .unwrap();
    report.current[0]
        .stage_verdict
        .as_ref()
        .and_then(StageVerdict::inconclusive_reason)
}

/// `population_exempt` is read when present; anything but a non-empty string
/// refuses the plan load.
///
/// Provenance: PLAT-1401
#[trace("FR-131-AC-7", "TC-3000")]
#[test]
fn tc_3000_an_exemption_is_a_non_empty_reason() {
    let plans = plans_from(&plan_document("proportion", EXEMPT)).unwrap();
    assert_eq!(
        plans[0]
            .statistical_design
            .as_ref()
            .and_then(|design| design.population_exempt.as_deref()),
        Some("repeated passes of a live service")
    );
    for stated in ["\"\"", "\"   \"", "3", "[a]"] {
        let refusal = plans_from(&plan_document(
            "proportion",
            &format!("\x20\x20population_exempt: {stated}\n"),
        ))
        .expect_err(stated);
        assert_eq!(
            refusal.code(),
            MeasurementErrorCode::PlanInvalid,
            "{stated}"
        );
        assert!(refusal.subject().contains("population_exempt"), "{refusal}");
    }
}

/// A plan declaring both a set and an exemption is refused at load.
///
/// Provenance: PLAT-1401
#[trace("FR-131-AC-7", "TC-3000")]
#[test]
fn tc_3000_a_set_and_an_exemption_together_refuse_the_plan_load() {
    let refusal = plans_from(&plan_document("proportion", &format!("{SET}{EXEMPT}")))
        .expect_err("both declarations are refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::PlanInvalid);
    assert!(
        refusal
            .subject()
            .contains("both population_set and population_exempt")
    );
}

/// A proportion plan with neither a set nor an exemption is inconclusive in
/// the stage verdict whatever it observed; either declaration, or another
/// estimator, lifts it.
///
/// Provenance: PLAT-1401
#[trace("FR-131-AC-8", "TC-3001")]
#[test]
fn tc_3001_an_undeclared_proportion_plan_is_inconclusive_in_the_report() {
    let population = json!({
        "examined": 2, "matched": 2, "complete": true,
        "examined_ids": ["a", "b"], "matched_ids": ["a", "b"],
    });
    let undeclared = plans_from(&plan_document("proportion", "")).unwrap();
    assert_eq!(
        reason_of(&undeclared, &population),
        Some(InconclusiveReason::PopulationSetUndeclared)
    );
    assert_eq!(
        InconclusiveReason::PopulationSetUndeclared.as_str(),
        "population_set_undeclared"
    );
    for declared in [SET, EXEMPT] {
        let plans = plans_from(&plan_document("proportion", declared)).unwrap();
        assert_ne!(
            reason_of(&plans, &population),
            Some(InconclusiveReason::PopulationSetUndeclared),
            "{declared}"
        );
    }
    let count = plans_from(&plan_document("count", "")).unwrap();
    assert_ne!(
        reason_of(
            &count,
            &json!({ "examined": 2, "matched": 2, "complete": true })
        ),
        Some(InconclusiveReason::PopulationSetUndeclared)
    );
}

/// `quoin measurement verify` assesses an undeclared proportion plan as
/// `population_set_undeclared`, an inconclusive reason, however sound the
/// observation is.
///
/// Provenance: PLAT-1401
#[trace("FR-131-AC-8", "TC-3002")]
#[test]
fn tc_3002_verify_is_inconclusive_for_an_undeclared_proportion_plan() {
    let plans = plans_from(&plan_document("proportion", "")).unwrap();
    let text = |value: &str| {
        NonEmptyText::parse(value, MeasurementErrorCode::CollectionInvalid, "x").unwrap()
    };
    let observation = MeasurementObservation {
        metric: text("quality.score"),
        plan_id: text("MP-958"),
        definition_version: text("v1"),
        state: MeasurementState::Measured,
        value: Some(1.0),
        unit: text("fraction"),
        shape: MeasurementShape::Ratio,
        population: Some(MeasurementPopulation {
            examined: Some(2.0),
            matched: Some(2.0),
            complete: Some(true),
            ..MeasurementPopulation::EMPTY
        }),
        dimensions: Dimensions::ABSENT,
        reason: None,
        interval: None,
    };
    let design = plans[0].statistical_design.clone().unwrap();
    let reason = assess(&design, design.estimator.unwrap(), &observation).unwrap_err();
    assert_eq!(reason, Reason::PopulationSetUndeclared);
    assert_eq!(reason.as_str(), "population_set_undeclared");
    assert_eq!(reason.verdict().as_str(), "inconclusive");

    let exempt = plans_from(&plan_document("proportion", EXEMPT)).unwrap();
    let design = exempt[0].statistical_design.clone().unwrap();
    assert!(assess(&design, design.estimator.unwrap(), &observation).is_ok());
}
