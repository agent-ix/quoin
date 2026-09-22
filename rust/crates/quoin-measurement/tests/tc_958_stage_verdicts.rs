// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-958 (part 1): a `ratchet` or `target` plan's `objective` is applied to
//! its newest value in `quoin report`.
//!
//! Every plan here is a real assurance document loaded through
//! `load_measurement_plans`, so the `objective` parse is exercised along with
//! the verdict, and every collection goes through intake
//! (`validate::measurement_collection`) before the report sees it.
//!
//! Provenance: PLAT-958

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
use quoin_measurement::portfolio::{
    build_portfolio_report, render_portfolio_report, render_portfolio_report_json,
};
use quoin_measurement::report::verdict::{
    BestPrior, InconclusiveReason, RatchetOutcome, StageVerdict, TargetOutcome,
};
use quoin_measurement::report::{
    MeasurementReport, build_measurement_report, build_measurement_report_from,
    render_measurement_report, render_measurement_report_json,
};
use quoin_measurement::source::{DiskMeasurement, MemoryMeasurement};
use quoin_measurement::store::write_measurement_collection;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::validate;
use serde_json::{Value, json};

/// An active plan for `quality.score` at `stage`, under `definition`, with
/// `objective` spliced into its frontmatter verbatim (already YAML-indented).
fn plan_document(stage: &str, definition: &str, objective: &str) -> String {
    format!(
        "---\n\
         id: MP-958\n\
         title: Example plan\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: {stage}\n\
         metric: quality.score\n\
         definition_version: {definition}\n\
         {objective}\
         ---\n\
         \n\
         # Example plan\n"
    )
}

/// `objective:` with `direction`, and `bound` when given.
fn objective(direction: &str, bound: Option<&str>) -> String {
    let bound = bound.map_or_else(String::new, |bound| format!("  bound: {bound}\n"));
    format!("objective:\n  direction: {direction}\n{bound}")
}

fn plans(document: &str) -> Vec<MeasurementPlan> {
    let source = MemoryMeasurement::new().with_document("spec/assurance/MP-958.md", document);
    load_measurement_plans(&source, PlanLoadOptions::default()).expect("the plan loads")
}

/// A measured observation of `value` under `definition`, with `population`
/// stated when given.
fn observation(value: f64, definition: &str, population: Option<Value>) -> Value {
    let mut observation = json!({
        "metric": "quality.score",
        "planId": "MP-958",
        "definitionVersion": definition,
        "state": "measured",
        "value": value,
        "unit": "fraction",
        "shape": "scalar",
    });
    if let Some(population) = population {
        observation["population"] = population;
    }
    observation
}

/// A complete, admissible collection `id` at `timestamp`.
fn collection_json(id: &str, timestamp: &str, observations: &[Value]) -> Value {
    json!({
        "schemaVersion": 2,
        "collectionId": id,
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "configDigest": format!("sha256:{}", "a".repeat(64)),
        "timestamp": timestamp,
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

/// Collection `index` (for the id and timestamp), admitted under `plans`.
fn admitted(plans: &[MeasurementPlan], index: u8, observation: Value) -> MeasurementCollection {
    validate::measurement_collection(
        &from_serde(&collection_json(
            &format!("run-{index}"),
            &format!("2026-09-{:02}T00:00:00.000Z", index + 1),
            &[observation],
        ))
        .expect("the candidate crosses the bridge"),
        plans,
    )
    .expect("the collection is admitted")
}

/// The report over `collections`, oldest first, under `plans`.
fn report(plans: &[MeasurementPlan], collections: &[MeasurementCollection]) -> MeasurementReport {
    build_measurement_report_from(Path::new("/repo"), plans, collections, &[], &[])
        .expect("the report builds")
}

/// The one row's verdict, over collections of `values` measured in order.
fn verdict_over(plans: &[MeasurementPlan], values: &[f64]) -> Option<StageVerdict> {
    let collections: Vec<MeasurementCollection> = values
        .iter()
        .zip(0_u8..)
        .map(|(value, index)| admitted(plans, index, observation(*value, "v1", None)))
        .collect();
    let mut report = report(plans, &collections);
    assert_eq!(report.current.len(), 1);
    report.current.remove(0).stage_verdict
}

/// The ratchet outcome over `values`.
fn ratchet_over(plans: &[MeasurementPlan], values: &[f64]) -> RatchetOutcome {
    match verdict_over(plans, values) {
        Some(StageVerdict::Ratchet { outcome, .. }) => outcome,
        other => panic!("expected a ratchet verdict, found {other:?}"),
    }
}

/// The target outcome over `values`.
fn target_over(plans: &[MeasurementPlan], values: &[f64]) -> TargetOutcome {
    match verdict_over(plans, values) {
        Some(StageVerdict::Target { outcome, .. }) => outcome,
        other => panic!("expected a target outcome, found {other:?}"),
    }
}

fn best(value: f64, collection: &str) -> BestPrior {
    BestPrior {
        value,
        collection_id: collection.to_owned(),
    }
}

/// `higher`: the best prior is the maximum; the newest value holds at or
/// above it and regresses below it.
///
/// Trace: FR-107-AC-2
/// Provenance: PLAT-958
#[test]
fn tc_958_001_a_higher_ratchet_holds_at_or_above_the_maximum_and_regresses_below_it() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    assert_eq!(
        ratchet_over(&plans, &[0.5, 0.8, 0.6, 0.9]),
        RatchetOutcome::Held {
            current: 0.9,
            best_prior: best(0.8, "run-1")
        }
    );
    assert_eq!(
        ratchet_over(&plans, &[0.5, 0.8, 0.8]),
        RatchetOutcome::Held {
            current: 0.8,
            best_prior: best(0.8, "run-1")
        }
    );
    assert_eq!(
        ratchet_over(&plans, &[0.5, 0.8, 0.9, 0.7]),
        RatchetOutcome::Regressed {
            current: 0.7,
            best_prior: best(0.9, "run-2")
        }
    );
}

/// `lower`: the best prior is the minimum; the newest value holds at or below
/// it and regresses above it.
///
/// Trace: FR-107-AC-2
/// Provenance: PLAT-958
#[test]
fn tc_958_002_a_lower_ratchet_holds_at_or_below_the_minimum_and_regresses_above_it() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("lower", None)));
    assert_eq!(
        ratchet_over(&plans, &[5.0, 3.0, 4.0, 2.0]),
        RatchetOutcome::Held {
            current: 2.0,
            best_prior: best(3.0, "run-1")
        }
    );
    assert_eq!(
        ratchet_over(&plans, &[5.0, 3.0, 4.0]),
        RatchetOutcome::Regressed {
            current: 4.0,
            best_prior: best(3.0, "run-1")
        }
    );
}

/// `zero`: the best prior is the one closest to zero, on either side.
///
/// Trace: FR-107-AC-2
/// Provenance: PLAT-958
#[test]
fn tc_958_003_a_zero_ratchet_holds_against_the_value_closest_to_zero() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("zero", None)));
    assert_eq!(
        ratchet_over(&plans, &[2.0, -0.5, 1.0, 0.4]),
        RatchetOutcome::Held {
            current: 0.4,
            best_prior: best(-0.5, "run-1")
        }
    );
    assert_eq!(
        ratchet_over(&plans, &[2.0, -0.5, -0.6]),
        RatchetOutcome::Regressed {
            current: -0.6,
            best_prior: best(-0.5, "run-1")
        }
    );
}

/// `target`: the best prior is the one closest to the bound.
///
/// Trace: FR-107-AC-2
/// Provenance: PLAT-958
#[test]
fn tc_958_004_a_target_direction_ratchet_holds_against_the_value_closest_to_the_bound() {
    let plans = plans(&plan_document(
        "ratchet",
        "v1",
        &objective("target", Some("10")),
    ));
    assert_eq!(
        ratchet_over(&plans, &[4.0, 12.0, 11.0]),
        RatchetOutcome::Held {
            current: 11.0,
            best_prior: best(12.0, "run-1")
        }
    );
    assert_eq!(
        ratchet_over(&plans, &[4.0, 12.0, 7.0]),
        RatchetOutcome::Regressed {
            current: 7.0,
            best_prior: best(12.0, "run-1")
        }
    );
}

/// The first collection a ratchet sees has nothing to hold against, and is
/// `inconclusive` — never `held`.
///
/// Trace: FR-107-AC-3
/// Provenance: PLAT-958
#[test]
fn tc_958_005_a_ratchets_first_collection_is_inconclusive_not_held() {
    for direction in ["higher", "lower", "zero"] {
        let plans = plans(&plan_document("ratchet", "v1", &objective(direction, None)));
        assert_eq!(
            ratchet_over(&plans, &[0.9]),
            RatchetOutcome::Inconclusive(InconclusiveReason::NoPrior),
            "{direction}"
        );
    }
    // No collection at all: no current value.
    let plans = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    assert_eq!(
        ratchet_over(&plans, &[]),
        RatchetOutcome::Inconclusive(InconclusiveReason::NoCurrentValue)
    );
}

/// An incomplete or empty newest population is `inconclusive`, however good
/// the value; and an incomplete or empty earlier value never sets the best.
///
/// Trace: FR-107-AC-3
/// Provenance: PLAT-958
#[test]
fn tc_958_006_an_incomplete_or_empty_population_is_inconclusive_and_sets_no_best() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    let first = admitted(&plans, 0, observation(0.5, "v1", None));
    let outcome_with = |population: Value| {
        let newest = admitted(&plans, 1, observation(0.99, "v1", Some(population)));
        report(&plans, &[first.clone(), newest]).current[0]
            .stage_verdict
            .clone()
    };
    for (population, reason) in [
        (
            json!({ "examined": 10, "complete": false }),
            InconclusiveReason::IncompletePopulation,
        ),
        (
            json!({ "examined": 0 }),
            InconclusiveReason::EmptyPopulation,
        ),
    ] {
        match outcome_with(population) {
            Some(StageVerdict::Ratchet {
                outcome: RatchetOutcome::Inconclusive(found),
                ..
            }) => assert_eq!(found, reason),
            other => panic!("expected inconclusive {reason:?}, found {other:?}"),
        }
    }

    // A higher but incomplete earlier value is not the best to hold against.
    let incomplete_high = admitted(
        &plans,
        1,
        observation(0.95, "v1", Some(json!({ "complete": false }))),
    );
    let newest = admitted(&plans, 2, observation(0.6, "v1", None));
    assert_eq!(
        report(&plans, &[first, incomplete_high, newest]).current[0].stage_verdict,
        Some(StageVerdict::Ratchet {
            objective: plans[0].objective.expect("an objective"),
            outcome: RatchetOutcome::Held {
                current: 0.6,
                best_prior: best(0.5, "run-0")
            }
        })
    );
}

/// A collection measured under another `definition_version` never sets the
/// best, however good its value; and a newest value under another definition
/// is `inconclusive`.
///
/// Trace: FR-107-AC-2, FR-107-AC-3
/// Provenance: PLAT-958
#[test]
fn tc_958_007_collections_under_another_definition_version_are_ignored_for_best_seen() {
    let current = plans(&plan_document("ratchet", "v2", &objective("higher", None)));
    let retired = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    let old_definition = admitted(&retired, 0, observation(0.99, "v1", None));
    let prior = admitted(&current, 1, observation(0.6, "v2", None));
    let newest = admitted(&current, 2, observation(0.7, "v2", None));
    assert_eq!(
        report(&current, &[old_definition.clone(), prior, newest]).current[0].stage_verdict,
        Some(StageVerdict::Ratchet {
            objective: current[0].objective.expect("an objective"),
            outcome: RatchetOutcome::Held {
                current: 0.7,
                best_prior: best(0.6, "run-1")
            }
        })
    );

    // Only the other definition before it: no prior under this one.
    let newest = admitted(&current, 1, observation(0.7, "v2", None));
    assert_eq!(
        report(&current, &[old_definition.clone(), newest]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::NoPrior)
    );

    // The newest value itself under another definition.
    let first = admitted(&current, 0, observation(0.6, "v2", None));
    let newest_old = admitted(&retired, 1, observation(0.9, "v1", None));
    assert_eq!(
        report(&current, &[first, newest_old]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::DefinitionMismatch)
    );
}

/// A `target` plan reports distance to the bound and whether it has been
/// reached, per direction; an incomplete population is `inconclusive`.
///
/// Trace: FR-107-AC-4
/// Provenance: PLAT-958
#[test]
fn tc_958_008_a_target_plan_reports_distance_to_the_bound_and_whether_it_is_reached() {
    let higher = plans(&plan_document(
        "target",
        "v1",
        &objective("higher", Some("0.75")),
    ));
    assert_eq!(
        target_over(&higher, &[0.5]),
        TargetOutcome::Measured {
            current: 0.5,
            bound: 0.75,
            distance: 0.25,
            reached: false
        }
    );
    assert_eq!(
        target_over(&higher, &[0.5, 1.0]),
        TargetOutcome::Measured {
            current: 1.0,
            bound: 0.75,
            distance: 0.25,
            reached: true
        }
    );

    let lower = plans(&plan_document(
        "target",
        "v1",
        &objective("lower", Some("4")),
    ));
    assert_eq!(
        target_over(&lower, &[6.0]),
        TargetOutcome::Measured {
            current: 6.0,
            bound: 4.0,
            distance: 2.0,
            reached: false
        }
    );
    assert_eq!(
        target_over(&lower, &[3.0]),
        TargetOutcome::Measured {
            current: 3.0,
            bound: 4.0,
            distance: 1.0,
            reached: true
        }
    );

    let zero = plans(&plan_document(
        "target",
        "v1",
        &objective("zero", Some("1")),
    ));
    assert_eq!(
        target_over(&zero, &[-0.5]),
        TargetOutcome::Measured {
            current: -0.5,
            bound: 1.0,
            distance: 0.5,
            reached: true
        }
    );
    assert_eq!(
        target_over(&zero, &[3.0]),
        TargetOutcome::Measured {
            current: 3.0,
            bound: 1.0,
            distance: 2.0,
            reached: false
        }
    );

    // A target plan with no bound has nothing to measure against.
    let unbounded = plans(&plan_document("target", "v1", &objective("higher", None)));
    assert_eq!(
        target_over(&unbounded, &[0.5]),
        TargetOutcome::Inconclusive(InconclusiveReason::NoBound)
    );

    let incomplete = admitted(
        &higher,
        0,
        observation(0.9, "v1", Some(json!({ "complete": false }))),
    );
    assert_eq!(
        report(&higher, &[incomplete]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::IncompletePopulation)
    );
}

/// A plan with no `objective`, or with one at a stage this PR does not
/// decide, carries no verdict and renders exactly as before.
///
/// Trace: FR-107-AC-6
/// Provenance: PLAT-958
#[test]
fn tc_958_009_a_plan_without_an_objective_or_at_another_stage_is_unchanged() {
    for document in [
        plan_document("ratchet", "v1", ""),
        plan_document("gate", "v1", &objective("higher", Some("0.9"))),
        plan_document("trend", "v1", &objective("lower", None)),
    ] {
        let plans = plans(&document);
        let collections = [
            admitted(&plans, 0, observation(0.9, "v1", None)),
            admitted(&plans, 1, observation(0.1, "v1", None)),
        ];
        let report = report(&plans, &collections);
        assert_eq!(report.current[0].stage_verdict, None, "{document}");
        let text = render_measurement_report(&report).expect("the report renders");
        assert!(!text.contains("Stage verdicts"), "{text}");
        assert!(!text.contains("regressed"), "{text}");
        let json_text = render_measurement_report_json(&report).expect("the JSON renders");
        assert!(!json_text.contains("stageVerdict"), "{json_text}");
    }
    // No objective: the plan wire states no such member either.
    let plans = plans(&plan_document("ratchet", "v1", ""));
    let json_text = render_measurement_report_json(&report(&plans, &[])).expect("the JSON renders");
    assert!(!json_text.contains("objective"), "{json_text}");
}

/// A repository with two collections on disk, under a ratchet plan whose
/// newest value regressed, and a target plan beside it.
fn regressed_repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
    let assurance = root.join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(
        assurance.join("MP-958.md"),
        plan_document("ratchet", "v1", &objective("higher", Some("0.95"))),
    )
    .expect("the plan is writable");
    for (index, value) in [(0_u8, 0.9), (1, 0.8)] {
        write_measurement_collection(
            root,
            &from_serde(&collection_json(
                &format!("run-{index}"),
                &format!("2026-09-{:02}T00:00:00.000Z", index + 1),
                &[observation(value, "v1", None)],
            ))
            .expect("the candidate crosses"),
        )
        .expect("the collection is admitted");
    }
    temporary
}

/// The verdict is rendered in the text report, the JSON report and the
/// portfolio's text and JSON, and a regression is an attention item.
///
/// Trace: FR-107-AC-5
/// Provenance: PLAT-958
#[test]
fn tc_958_010_the_verdict_is_rendered_in_text_json_and_the_portfolio() {
    let repository = regressed_repository();
    let root = repository.path();
    let report =
        build_measurement_report(&DiskMeasurement::new(root), root).expect("the report builds");

    let text = render_measurement_report(&report).expect("the report renders");
    assert!(
        text.contains(
            "## Stage verdicts\n\n\
             | Metric | Plan | Stage | Objective | Verdict | Detail |\n\
             | --- | --- | --- | --- | --- | --- |\n\
             | quality.score | MP-958 (spec/assurance/MP-958.md) | ratchet | higher; bound 0.95 \
             | regressed | current 0.8; best prior 0.9 (run-0) |\n\n\
             ## Current evidence"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "- quality.score: regressed to 0.8 from the best prior 0.9 (run-0); plan MP-958 does \
             not hold its ratchet."
        ),
        "{text}"
    );

    let parsed: Value = serde_json::from_str(
        &render_measurement_report_json(&report).expect("the JSON report renders"),
    )
    .expect("the JSON view is JSON");
    assert_eq!(
        parsed["plans"][0]["objective"],
        json!({ "direction": "higher", "bound": 0.95 })
    );
    assert_eq!(
        parsed["current"][0]["stageVerdict"],
        json!({
            "stage": "ratchet",
            "objective": { "direction": "higher", "bound": 0.95 },
            "verdict": "regressed",
            "reason": null,
            "current": 0.8,
            "bestPrior": { "value": 0.9, "collectionId": "run-0" },
        })
    );

    let portfolio = build_portfolio_report(&[root.to_path_buf()]);
    let text = render_portfolio_report(&portfolio).expect("the portfolio renders");
    assert!(
        text.contains(
            "| quality.score | MP-958 (spec/assurance/MP-958.md) | ratchet | higher; bound 0.95 \
             | regressed | current 0.8; best prior 0.9 (run-0) |"
        ),
        "{text}"
    );
    let parsed: Value = serde_json::from_str(
        &render_portfolio_report_json(&portfolio).expect("the portfolio JSON renders"),
    )
    .expect("the portfolio JSON is JSON");
    assert_eq!(
        parsed["repositories"][0]["measurements"]["current"][0]["stageVerdict"]["verdict"],
        json!("regressed")
    );
}

/// An inconclusive verdict and a target's progress render with their reason
/// and numbers, and the JSON states every member of its stage.
///
/// Trace: FR-107-AC-5
/// Provenance: PLAT-958
#[test]
fn tc_958_011_inconclusive_and_target_verdicts_render_with_reason_and_numbers() {
    let ratchet = plans(&plan_document("ratchet", "v1", &objective("lower", None)));
    let report_one = report(
        &ratchet,
        &[admitted(&ratchet, 0, observation(3.0, "v1", None))],
    );
    let text = render_measurement_report(&report_one).expect("the report renders");
    assert!(
        text.contains(
            "| quality.score | MP-958 (spec/assurance/MP-958.md) | ratchet | lower | inconclusive \
             | no_prior: no earlier collection measured this plan under its definition version |"
        ),
        "{text}"
    );
    let parsed: Value = serde_json::from_str(
        &render_measurement_report_json(&report_one).expect("the JSON renders"),
    )
    .expect("JSON");
    assert_eq!(
        parsed["current"][0]["stageVerdict"],
        json!({
            "stage": "ratchet",
            "objective": { "direction": "lower" },
            "verdict": "inconclusive",
            "reason": "no_prior",
            "current": null,
            "bestPrior": null,
        })
    );

    let target = plans(&plan_document(
        "target",
        "v1",
        &objective("lower", Some("4")),
    ));
    let report_two = report(
        &target,
        &[admitted(&target, 0, observation(6.0, "v1", None))],
    );
    let text = render_measurement_report(&report_two).expect("the report renders");
    assert!(
        text.contains(
            "| quality.score | MP-958 (spec/assurance/MP-958.md) | target | lower; bound 4 \
             | not_reached | current 6; bound 4; distance 2 |"
        ),
        "{text}"
    );
    let parsed: Value = serde_json::from_str(
        &render_measurement_report_json(&report_two).expect("the JSON renders"),
    )
    .expect("JSON");
    assert_eq!(
        parsed["current"][0]["stageVerdict"],
        json!({
            "stage": "target",
            "objective": { "direction": "lower", "bound": 4 },
            "verdict": "not_reached",
            "reason": null,
            "current": 6,
            "distance": 2,
            "reached": false,
        })
    );
}

/// `objective` is parsed into engineering-assurance's type, and a block EA
/// refuses refuses the plan load as `QM-PLAN-INVALID`, naming the member.
///
/// Trace: FR-107-AC-1
/// Provenance: PLAT-958
#[test]
fn tc_958_012_the_objective_is_parsed_and_a_malformed_one_refuses_the_plan() {
    use engineering_assurance::measurement::Direction;

    let plan = plans(&plan_document(
        "target",
        "v1",
        &objective("target", Some("0.5")),
    ))
    .remove(0);
    let parsed = plan.objective.expect("the objective is parsed");
    assert_eq!(parsed.direction(), Direction::Target);
    assert_eq!(parsed.bound(), Some(0.5));
    assert_eq!(
        plans(&plan_document("ratchet", "v1", ""))
            .remove(0)
            .objective,
        None
    );

    for malformed in [
        objective("sideways", None),
        objective("target", None),
        objective("higher", Some("\"high\"")),
        "objective:\n  direction: higher\n  goal: 1\n".to_owned(),
        "objective: higher\n".to_owned(),
    ] {
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/MP-958.md",
            plan_document("ratchet", "v1", &malformed),
        );
        let error = load_measurement_plans(&source, PlanLoadOptions::default())
            .expect_err("a malformed objective refuses the plan");
        assert_eq!(
            error.code(),
            MeasurementErrorCode::PlanInvalid,
            "{malformed}"
        );
        assert!(
            error.subject().contains("objective is invalid"),
            "{malformed}: {}",
            error.subject()
        );
    }
}
