// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-960: a plan's `statistical_design.minimum_population` is enforced at
//! intake, and its `ground_truth_kind` is carried into `quoin report`.
//!
//! The engineering-assurance `MeasurementPlan` schema already let a plan say
//! "a result from fewer than N examined items must not be trusted", but intake
//! read none of it, so a collection examining two items was admitted under a
//! plan that required fifty. Every test here builds its plan as a real
//! assurance document on disk and loads it through `load_measurement_plans`,
//! so the frontmatter parse is exercised along with the check.
//!
//! `statistical_design.repetitions` is parsed but not enforced: no member of
//! a measurement collection records how many repetitions were performed, so
//! there is nothing to compare it against. That gap is PLAT-960's to decide,
//! and no test here pretends otherwise.
//!
//! Provenance: PLAT-960

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::report::{
    build_measurement_report, render_measurement_report, render_measurement_report_json,
};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::store::write_measurement_collection;
use quoin_measurement::types::plan::{GroundTruthKind, MeasurementPlan};
use quoin_measurement::validate;
use serde_json::{Value, json};

/// An active gate plan for `quality.gate`, with `extra` spliced into its
/// frontmatter verbatim (each line already indented as YAML needs).
fn plan_document(extra: &str) -> String {
    format!(
        "---\n\
         id: MP-900\n\
         title: Example gate\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: gate\n\
         metric: quality.gate\n\
         definition_version: quality.gate-v1\n\
         {extra}\
         ---\n\
         \n\
         # Example gate\n"
    )
}

/// The frontmatter lines a plan requiring at least `minimum` examined items
/// carries, with the schema's other required design members beside it.
fn design_with_minimum(minimum: &str) -> String {
    format!(
        "ground_truth_kind: human-labelled\n\
         statistical_design:\n\
         \x20\x20population: every labelled case\n\
         \x20\x20minimum_population: {minimum}\n\
         \x20\x20sampling: exhaustive\n\
         \x20\x20repetitions: 5\n\
         \x20\x20estimator: fraction correct\n\
         \x20\x20error_model: binomial\n\
         \x20\x20uncertainty: wilson interval\n\
         \x20\x20decision_rule: pass at 0.9\n"
    )
}

/// A repository holding one plan document.
fn repository_with(document: &str) -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(assurance.join("MP-900.md"), document).expect("the plan is writable");
    temporary
}

fn authored_plans(root: &Path) -> Vec<MeasurementPlan> {
    load_measurement_plans(&DiskMeasurement::new(root), PlanLoadOptions::default())
        .expect("the authored plan loads")
}

/// One measured observation against `MP-900`, examining `examined` items, or
/// stating no population at all when `examined` is `None`.
fn observation(examined: Option<u32>) -> Value {
    let mut observation = json!({
        "metric": "quality.gate",
        "planId": "MP-900",
        "definitionVersion": "quality.gate-v1",
        "state": "measured",
        "value": 0.9,
        "unit": "fraction",
        "shape": "ratio",
    });
    if let Some(examined) = examined {
        observation["population"] = json!({ "examined": examined, "matched": 1 });
    }
    observation
}

/// A complete, otherwise-admissible schema-v2 collection with `observations`.
fn collection(observations: &[Value]) -> Value {
    json!({
        "schemaVersion": 2,
        "collectionId": "run-960",
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "configDigest": format!("sha256:{}", "a".repeat(64)),
        "timestamp": "2026-09-22T00:00:00.000Z",
        "sourceRevision": "aaaaaaaaaaaaaaaa",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "lockDigest": format!("sha256:{}", "1".repeat(64)),
            "executableDigest": format!("sha256:{}", "2".repeat(64)),
            "buildProfile": "release",
            "toolchains": { "node": "22.15.0", "rust": "1.94.1", "python": "3.10.12" },
            "sources": {
                "fixture": {
                    "revision": "a".repeat(40),
                    "sourceState": "clean",
                    "remote": "https://example.invalid/fixture",
                },
            },
            "capabilities": ["fixture.capability"],
            "artifacts": { "config": format!("sha256:{}", "3".repeat(64)) },
        },
        "observations": observations,
        "rawEvidence": { "payload": [1, 2] },
    })
}

fn intake(
    plans: &[MeasurementPlan],
    candidate: &Value,
) -> Result<String, quoin_measurement::MeasurementError> {
    validate::measurement_collection(
        &from_serde(candidate).expect("the candidate crosses the bridge"),
        plans,
    )
    .map(|admitted| admitted.collection_id.as_str().to_owned())
}

/// A measured observation examining fewer items than the plan's
/// `minimum_population` is refused with its own code, and the finding names
/// the metric, the plan, what was examined and what was required.
///
/// Trace: FR-044-AC-7
/// Provenance: PLAT-960
#[test]
fn tc_960_001_a_population_below_the_minimum_is_refused_with_its_own_code() {
    let repository = repository_with(&plan_document(&design_with_minimum("10")));
    let plans = authored_plans(repository.path());

    let refusal = intake(&plans, &collection(&[observation(Some(9))]))
        .expect_err("nine examined under a minimum of ten is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::PopulationBelowMinimum);
    assert_eq!(
        refusal.findings(),
        [
            "QM-POPULATION-BELOW-MINIMUM: metric `quality.gate` examined 9 but plan MP-900 \
             requires a population of at least 10"
        ]
    );
}

/// Reaching the minimum exactly is enough, and exceeding it is too.
///
/// Trace: FR-044-AC-7
/// Provenance: PLAT-960
#[test]
fn tc_960_002_a_population_at_or_above_the_minimum_is_admitted() {
    let repository = repository_with(&plan_document(&design_with_minimum("10")));
    let plans = authored_plans(repository.path());

    for examined in [10, 11, 5000] {
        assert_eq!(
            intake(&plans, &collection(&[observation(Some(examined))]))
                .unwrap_or_else(|refusal| panic!("{examined} examined was refused: {refusal}")),
            "run-960"
        );
    }
}

/// A measured observation that states no `population.examined` cannot show
/// it met the minimum, so it is refused rather than admitted on silence. A
/// `not_computed` observation carries no value to trust and is not checked.
///
/// Trace: FR-044-AC-7
/// Provenance: PLAT-960
#[test]
fn tc_960_003_a_measured_observation_with_no_population_is_refused_under_a_minimum() {
    let repository = repository_with(&plan_document(&design_with_minimum("10")));
    let plans = authored_plans(repository.path());

    let refusal = intake(&plans, &collection(&[observation(None)]))
        .expect_err("an unstated population under a minimum is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::PopulationUnstated);
    assert_eq!(
        refusal.findings(),
        [
            "QM-POPULATION-UNSTATED: metric `quality.gate` states no population.examined; \
             plan MP-900 requires a population of at least 10"
        ]
    );

    let mut not_computed = observation(None);
    not_computed["state"] = json!("not_computed");
    not_computed["value"] = Value::Null;
    not_computed["reason"] = json!("producer crashed");
    assert_eq!(
        intake(&plans, &collection(&[not_computed])).expect("not_computed is not checked"),
        "run-960"
    );
}

/// Population findings accumulate with every other intake finding rather
/// than stopping the check (PLAT-929). A refusal mixing them carries
/// `QM-COLLECTION-INVALID`, and each population finding still names its own
/// code as its first word.
///
/// Trace: FR-044-AC-7
/// Provenance: PLAT-960
#[test]
fn tc_960_004_population_findings_accumulate_with_every_other_finding() {
    let repository = repository_with(&plan_document(&design_with_minimum("10")));
    let plans = authored_plans(repository.path());

    let mut third_variant = observation(None);
    third_variant["dimensions"] = json!({ "variant": "c" });
    let mut second = observation(Some(3));
    second["dimensions"] = json!({ "variant": "b" });
    let mut unplanned = observation(Some(100));
    unplanned["metric"] = json!("quality.unplanned");
    let refusal = intake(
        &plans,
        &collection(&[observation(Some(2)), second, third_variant, unplanned]),
    )
    .expect_err("three population defects and an unplanned metric are refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert_eq!(
        refusal.findings(),
        [
            "QM-POPULATION-BELOW-MINIMUM: metric `quality.gate` examined 2 but plan MP-900 \
             requires a population of at least 10",
            "QM-POPULATION-BELOW-MINIMUM: metric `quality.gate [variant=b]` examined 3 but \
             plan MP-900 requires a population of at least 10",
            "QM-POPULATION-UNSTATED: metric `quality.gate [variant=c]` states no \
             population.examined; plan MP-900 requires a population of at least 10",
            "metric `quality.unplanned` has no MeasurementPlan under spec/assurance or \
             assurance; record refused",
        ]
    );
}

/// A plan that states neither `ground_truth_kind` nor `statistical_design`
/// loads with neither, admits a collection with no population exactly as it
/// did before PLAT-960, and renders in `quoin report` byte-for-byte as it did
/// before: no ground-truth clause in the text, no `groundTruthKind` in the
/// JSON.
///
/// Trace: FR-044-AC-7, FR-044-AC-8
/// Provenance: PLAT-960
#[test]
fn tc_960_005_a_plan_without_the_fields_behaves_exactly_as_before() {
    let repository = repository_with(&plan_document(""));
    let root = repository.path();
    let plans = authored_plans(root);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].ground_truth_kind, None);
    assert_eq!(plans[0].statistical_design, None);

    write_measurement_collection(
        root,
        &from_serde(&collection(&[observation(None)])).expect("the candidate crosses"),
    )
    .expect("no minimum means nothing new to refuse on");

    let report =
        build_measurement_report(&DiskMeasurement::new(root), root).expect("the report builds");
    let rendered = render_measurement_report(&report).expect("the report renders");
    assert!(
        rendered
            .contains("| quality.gate | MP-900 (spec/assurance/MP-900.md) | gate | 0.9 fraction |"),
        "a plan without ground_truth_kind renders its plan cell unchanged:\n{rendered}"
    );
    let json_text = render_measurement_report_json(&report).expect("the JSON report renders");
    assert!(
        !json_text.contains("groundTruthKind"),
        "a plan without ground_truth_kind states no such member: {json_text}"
    );
}

/// `ground_truth_kind` is carried beside the plan in both report views: in
/// the text view's Plan cell, and as the plan's `groundTruthKind` in JSON.
///
/// Trace: FR-044-AC-8
/// Provenance: PLAT-960
#[test]
fn tc_960_006_the_report_states_the_plans_ground_truth_kind() {
    let repository = repository_with(&plan_document(&design_with_minimum("10")));
    let root = repository.path();
    write_measurement_collection(
        root,
        &from_serde(&collection(&[observation(Some(12))])).expect("the candidate crosses"),
    )
    .expect("twelve examined meets a minimum of ten");

    let report =
        build_measurement_report(&DiskMeasurement::new(root), root).expect("the report builds");
    let rendered = render_measurement_report(&report).expect("the report renders");
    assert!(
        rendered.contains(
            "| quality.gate | MP-900 (spec/assurance/MP-900.md; ground truth: human-labelled) \
             | gate | 0.9 fraction |"
        ),
        "the Plan cell must state the ground-truth kind:\n{rendered}"
    );

    let parsed: Value = serde_json::from_str(
        &render_measurement_report_json(&report).expect("the JSON report renders"),
    )
    .expect("the JSON view is JSON");
    assert_eq!(
        parsed["plans"][0]["groundTruthKind"],
        json!("human-labelled")
    );
}

/// The design members are parsed into the plan, `repetitions` included, and
/// every `ground_truth_kind` spelling the schema allows is accepted.
///
/// Trace: FR-044-AC-7
/// Provenance: PLAT-960
#[test]
fn tc_960_007_the_design_members_are_parsed_into_the_plan() {
    let repository = repository_with(&plan_document(&design_with_minimum("10")));
    let plan = authored_plans(repository.path()).remove(0);
    let design = plan.statistical_design.expect("the block is parsed");
    assert_eq!(
        design.minimum_population.map(std::num::NonZeroU32::get),
        Some(10)
    );
    assert_eq!(design.repetitions.map(std::num::NonZeroU32::get), Some(5));
    assert_eq!(plan.ground_truth_kind, Some(GroundTruthKind::HumanLabelled));

    for (spelling, kind) in [
        ("human-labelled", GroundTruthKind::HumanLabelled),
        ("agent-labelled", GroundTruthKind::AgentLabelled),
        ("mechanical", GroundTruthKind::Mechanical),
    ] {
        let repository =
            repository_with(&plan_document(&format!("ground_truth_kind: {spelling}\n")));
        let plan = authored_plans(repository.path()).remove(0);
        assert_eq!(plan.ground_truth_kind, Some(kind), "{spelling}");
        assert_eq!(plan.statistical_design, None);
    }
}

/// A design member present with a value the schema does not allow refuses
/// the whole plan load as `QM-PLAN-INVALID`, naming the member; nothing is
/// rounded, coerced or clamped.
///
/// Trace: FR-044-AC-7
/// Provenance: PLAT-960
#[test]
fn tc_960_008_a_malformed_design_member_refuses_the_plan_load() {
    let cases = [
        (
            "statistical_design: none\n",
            "statistical_design must be an object",
        ),
        (
            "statistical_design:\n  minimum_population: 0\n",
            "statistical_design.minimum_population must be a whole number",
        ),
        (
            "statistical_design:\n  minimum_population: 2.5\n",
            "statistical_design.minimum_population must be a whole number",
        ),
        (
            "statistical_design:\n  minimum_population: \"12\"\n",
            "statistical_design.minimum_population must be a whole number",
        ),
        (
            "statistical_design:\n  repetitions: -1\n",
            "statistical_design.repetitions must be a whole number",
        ),
        (
            "statistical_design:\n  repetitions: 4294967296\n",
            "statistical_design.repetitions must be a whole number",
        ),
        (
            "ground_truth_kind: crowd\n",
            "ground_truth_kind must be one of",
        ),
    ];
    for (extra, expected) in cases {
        let repository = repository_with(&plan_document(extra));
        let refusal = load_measurement_plans(
            &DiskMeasurement::new(repository.path()),
            PlanLoadOptions::default(),
        )
        .expect_err(extra);
        assert_eq!(refusal.code(), MeasurementErrorCode::PlanInvalid, "{extra}");
        assert!(
            refusal.subject().contains(expected),
            "{extra:?} must be refused naming `{expected}`, got {refusal}"
        );
    }
}
