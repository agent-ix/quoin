// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-958: a `ratchet` or `target` plan's `objective` (part 1), and a `gate`
//! plan's `decision_rule` (part 2), are applied to its newest value in
//! `quoin report`.
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
    BestPrior, GateOutcome, InconclusiveReason, RatchetOutcome, StageVerdict, TargetOutcome,
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

/// A measured observation of `value` under `definition`, over `population`,
/// or over ten examined items when none is given.
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
    observation["population"] = population.unwrap_or_else(|| json!({ "examined": 10 }));
    observation
}

/// `observation` for the slice `family=<family>`.
fn sliced(value: f64, family: &str) -> Value {
    let mut observation = observation(value, "v1", None);
    observation["dimensions"] = json!({ "family": family });
    observation
}

/// `observation` with no `population` member at all.
fn unstated(value: f64) -> Value {
    let mut observation = observation(value, "v1", None);
    observation
        .as_object_mut()
        .expect("an object")
        .remove("population");
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
    admitted_many(plans, index, &[observation])
}

/// As [`admitted`], with several observations.
fn admitted_many(
    plans: &[MeasurementPlan],
    index: u8,
    observations: &[Value],
) -> MeasurementCollection {
    validate::measurement_collection(
        &from_serde(&collection_json(
            &format!("run-{index}"),
            &format!("2026-09-{:02}T00:00:00.000Z", index + 1),
            observations,
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
            distance: 0.0,
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
            distance: 0.0,
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
            distance: 0.0,
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

/// A plan with no `objective` — a `ratchet` or a `gate` alike — or one at
/// `trend`, a stage this requirement never decides regardless of its
/// `objective`, carries no verdict and renders exactly as before.
///
/// Trace: FR-107-AC-6
/// Provenance: PLAT-958
#[test]
fn tc_958_009_a_plan_without_an_objective_or_at_another_stage_is_unchanged() {
    // A `gate` plan requires `protected_apparatus` and `negative_controls`
    // (PLAT-975, FR-110-AC-1) whether or not it states an `objective`; this
    // test is not about apparatus, so it carries the minimal lists that
    // satisfy plan load and nothing else exercises them.
    let gate_no_objective = "protected_apparatus:\n  - answers.json\nnegative_controls:\n  - kind: apparatus-edit\n    \
         description: test-only\n";
    for document in [
        plan_document("ratchet", "v1", ""),
        plan_document("gate", "v1", gate_no_objective),
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
            "progress": "not_reached",
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
        objective("zero", Some("-1")),
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

/// The verdict of the one row whose dimensions are `family=<family>`.
fn slice_verdict(report: &MeasurementReport, family: &str) -> Option<StageVerdict> {
    report
        .current
        .iter()
        .find(|row| {
            row.observation.as_ref().is_some_and(|observation| {
                observation.dimensions.entries().get("family")
                    == Some(&quoin_store::JsonValue::string(family.to_owned()))
            })
        })
        .and_then(|row| row.stage_verdict.clone())
}

fn held(plans: &[MeasurementPlan], current: f64, best_prior: BestPrior) -> StageVerdict {
    StageVerdict::Ratchet {
        objective: plans[0].objective.expect("an objective"),
        outcome: RatchetOutcome::Held {
            current,
            best_prior,
        },
    }
}

/// A ratchet holds each slice against that slice's own history: a better
/// earlier value in another slice, under another plan id, or under another
/// metric never becomes the best.
///
/// Trace: FR-107-AC-2
/// Provenance: PLAT-958
#[test]
fn tc_958_013_the_best_prior_is_taken_from_the_same_slice_plan_and_metric_only() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    let first = admitted_many(&plans, 0, &[sliced(0.5, "a"), sliced(0.95, "b")]);
    let newest = admitted_many(&plans, 3, &[sliced(0.6, "a"), sliced(0.96, "b")]);
    let report_ab = report(&plans, &[first.clone(), newest.clone()]);
    assert_eq!(
        slice_verdict(&report_ab, "a"),
        Some(held(&plans, 0.6, best(0.5, "run-0")))
    );
    assert_eq!(
        slice_verdict(&report_ab, "b"),
        Some(held(&plans, 0.96, best(0.95, "run-0")))
    );

    // A better earlier value of slice `a` under another plan id. Intake cannot
    // admit that, so the admitted record is edited after the fact.
    let mut other_plan = admitted_many(&plans, 1, &[sliced(0.99, "a")]);
    other_plan.observations[0].plan_id = quoin_measurement::types::ids::NonEmptyText::parse(
        "MP-OTHER",
        MeasurementErrorCode::PlanInvalid,
        "planId",
    )
    .expect("a plan id");
    // A better earlier value of slice `a` under another metric.
    let mut other_metric = admitted_many(&plans, 2, &[sliced(0.99, "a")]);
    other_metric.observations[0].metric = quoin_measurement::types::ids::NonEmptyText::parse(
        "quality.other",
        MeasurementErrorCode::PlanInvalid,
        "metric",
    )
    .expect("a metric");
    let report_all = report(&plans, &[first, other_plan, other_metric, newest]);
    assert_eq!(
        slice_verdict(&report_all, "a"),
        Some(held(&plans, 0.6, best(0.5, "run-0")))
    );
}

/// Equal best values name the earliest collection that reached them.
///
/// Trace: FR-107-AC-2
/// Provenance: PLAT-958
#[test]
fn tc_958_014_a_tie_for_the_best_prior_names_the_earliest_collection() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    assert_eq!(
        ratchet_over(&plans, &[0.8, 0.8, 0.7, 0.9]),
        RatchetOutcome::Held {
            current: 0.9,
            best_prior: best(0.8, "run-0")
        }
    );
}

/// A newest value naming another plan is `inconclusive` (`plan_mismatch`).
///
/// Trace: FR-107-AC-3
/// Provenance: PLAT-958
#[test]
fn tc_958_015_a_newest_value_under_another_plan_id_is_inconclusive() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    let first = admitted(&plans, 0, observation(0.5, "v1", None));
    let mut newest = admitted(&plans, 1, observation(0.9, "v1", None));
    newest.observations[0].plan_id = quoin_measurement::types::ids::NonEmptyText::parse(
        "MP-OTHER",
        MeasurementErrorCode::PlanInvalid,
        "planId",
    )
    .expect("a plan id");
    assert_eq!(
        report(&plans, &[first, newest]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::PlanMismatch)
    );
}

/// A slice with a usable earlier value that the newest collection omits is
/// reported `inconclusive` (`no_current_value`) and named as an attention
/// item, so a slice cannot regress by being dropped.
///
/// Trace: FR-107-AC-3, FR-107-AC-5
/// Provenance: PLAT-958
#[test]
fn tc_958_016_a_slice_the_newest_collection_drops_is_reported_inconclusive() {
    let plans = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    let first = admitted_many(&plans, 0, &[sliced(0.5, "a"), sliced(0.6, "b")]);
    let newest = admitted_many(&plans, 1, &[sliced(0.7, "a")]);
    let report = report(&plans, &[first, newest]);
    assert_eq!(report.vanished_slices.len(), 1);
    assert_eq!(
        report.vanished_slices[0]
            .stage_verdict
            .inconclusive_reason(),
        Some(InconclusiveReason::NoCurrentValue)
    );

    let text = render_measurement_report(&report).expect("the report renders");
    assert!(
        text.contains(
            "| quality.score [family=a] | MP-958 (spec/assurance/MP-958.md) | ratchet | higher \
             | held | current 0.7; best prior 0.5 (run-0) |\n\
             | quality.score [family=b] | MP-958 (spec/assurance/MP-958.md) | ratchet | higher \
             | inconclusive | no_current_value: no measured value in the newest collection |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "- quality.score [family=b]: measured by an earlier collection but absent from the \
             newest; plan MP-958 cannot hold its ratchet."
        ),
        "{text}"
    );
    let parsed: Value =
        serde_json::from_str(&render_measurement_report_json(&report).expect("the JSON renders"))
            .expect("JSON");
    assert_eq!(
        parsed["vanishedSlices"],
        json!([{
            "metric": "quality.score",
            "planId": "MP-958",
            "planPath": "spec/assurance/MP-958.md",
            "dimensions": { "family": "b" },
            "stageVerdict": {
                "stage": "ratchet",
                "objective": { "direction": "higher" },
                "verdict": "inconclusive",
                "reason": "no_current_value",
                "current": null,
                "bestPrior": null,
            },
        }])
    );

    // Nothing dropped: no such member, and no such attention item.
    let steady = steady_report(&plans);
    assert!(steady.vanished_slices.is_empty());
    let json_text = render_measurement_report_json(&steady).expect("the JSON renders");
    assert!(!json_text.contains("vanishedSlices"), "{json_text}");
}

/// Two collections that both measure slice `a`.
fn steady_report(plans: &[MeasurementPlan]) -> MeasurementReport {
    let first = admitted_many(plans, 0, &[sliced(0.5, "a")]);
    let newest = admitted_many(plans, 1, &[sliced(0.7, "a")]);
    report(plans, &[first, newest])
}

/// An observation stating no `population` is `inconclusive`
/// (`population_unstated`) for a ratchet and a target alike, and an earlier
/// one never sets a ratchet's best.
///
/// Trace: FR-107-AC-3, FR-107-AC-4
/// Provenance: PLAT-958
#[test]
fn tc_958_017_an_unstated_population_is_inconclusive_for_ratchet_and_target() {
    let ratchet = plans(&plan_document("ratchet", "v1", &objective("higher", None)));
    let first = admitted(&ratchet, 0, observation(0.5, "v1", None));
    let newest = admitted(&ratchet, 1, unstated(0.9));
    assert_eq!(
        report(&ratchet, &[first.clone(), newest]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::PopulationUnstated)
    );
    let better_unstated = admitted(&ratchet, 1, unstated(0.99));
    let newest = admitted(&ratchet, 2, observation(0.6, "v1", None));
    assert_eq!(
        report(&ratchet, &[first, better_unstated, newest]).current[0].stage_verdict,
        Some(held(&ratchet, 0.6, best(0.5, "run-0")))
    );

    let target = plans(&plan_document(
        "target",
        "v1",
        &objective("higher", Some("0.5")),
    ));
    assert_eq!(
        report(&target, &[admitted(&target, 0, unstated(0.9))]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::PopulationUnstated)
    );
}

/// A `target`-direction target is reached only on exact IEEE equality, with
/// no tolerance, and its distance is `0` there.
///
/// Trace: FR-107-AC-4
/// Provenance: PLAT-958
#[test]
fn tc_958_018_a_target_direction_is_reached_only_on_exact_equality() {
    let exact = plans(&plan_document(
        "target",
        "v1",
        &objective("target", Some("10")),
    ));
    assert_eq!(
        target_over(&exact, &[10.0]),
        TargetOutcome::Measured {
            current: 10.0,
            bound: 10.0,
            distance: 0.0,
            reached: true
        }
    );
    assert_eq!(
        target_over(&exact, &[9.5]),
        TargetOutcome::Measured {
            current: 9.5,
            bound: 10.0,
            distance: 0.5,
            reached: false
        }
    );
}

// --- PLAT-958 part 2: `gate` ---------------------------------------------

/// A `gate` plan for `quality.score` under `definition`, with `objective`
/// direction `direction` and `statistical_design.decision_rule` set to
/// `rule` (already YAML-indented under `decision_rule:`). `protected_apparatus`
/// and `negative_controls` are the minimal lists FR-110-AC-1 requires of
/// every `gate` plan; this file is not about apparatus, so nothing else
/// exercises them.
fn gate_plan_document(direction: &str, definition: &str, rule: &str) -> String {
    format!(
        "---\n\
         id: MP-958\n\
         title: Example gate\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: gate\n\
         metric: quality.score\n\
         definition_version: {definition}\n\
         objective:\n  direction: {direction}\n\
         protected_apparatus:\n  - answers.json\n\
         negative_controls:\n  - kind: apparatus-edit\n    description: test-only\n\
         statistical_design:\n  decision_rule:\n{rule}\
         ---\n\
         \n\
         # Example gate\n"
    )
}

/// Collection `index` of `observations` for a gate plan, as intake stores
/// it: admissible under `plans`, and carrying the protected-apparatus record
/// intake's writer adds (FR-110-AC-2) — the answer key at a digest of
/// `digit` repeated — or no record at all for `None`.
fn stored_gate(
    plans: &[MeasurementPlan],
    index: u8,
    observations: &[Value],
    digit: Option<char>,
) -> MeasurementCollection {
    admitted_many(plans, index, observations);
    let mut value = collection_json(
        &format!("run-{index}"),
        &format!("2026-09-{:02}T00:00:00.000Z", index + 1),
        observations,
    );
    if let Some(digit) = digit {
        value["verificationStack"]["protectedApparatus"] = json!({
            "MP-958": { "answers.json": format!("sha256:{}", digit.to_string().repeat(64)) }
        });
    }
    quoin_measurement::stored_measurement_collection(&from_serde(&value).expect("the bridge"))
        .expect("the stored record reads")
}

/// The one row's gate outcome over `collections`.
fn gate_of(plans: &[MeasurementPlan], collections: &[MeasurementCollection]) -> GateOutcome {
    let mut report = report(plans, collections);
    assert_eq!(report.current.len(), 1);
    match report.current.remove(0).stage_verdict {
        Some(StageVerdict::Gate { outcome, .. }) => outcome,
        other => panic!("expected a gate verdict, found {other:?}"),
    }
}

/// The gate outcome over collections of `values`, measured in order under
/// one unchanged protected apparatus.
fn gate_over(plans: &[MeasurementPlan], values: &[f64]) -> GateOutcome {
    let collections: Vec<MeasurementCollection> = values
        .iter()
        .zip(0_u8..)
        .map(|(value, index)| {
            stored_gate(plans, index, &[observation(*value, "v1", None)], Some('0'))
        })
        .collect();
    gate_of(plans, &collections)
}

/// A `gate` plan's verdict comes only from `decision_rule`, never from
/// `objective.bound`: a `threshold` rule passes when the newest value holds
/// and fails otherwise, in both directions, and the same value passes or
/// fails depending only on the rule's own comparator and threshold.
///
/// Trace: FR-107-AC-7
/// Provenance: PLAT-958
#[test]
fn tc_958_019_a_threshold_gate_passes_or_fails_by_the_rule_alone_in_both_directions() {
    let higher = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: ge\n    threshold: 0.8\n",
    ));
    assert_eq!(
        gate_over(&higher, &[0.9]),
        GateOutcome::Pass {
            current: 0.9,
            baseline: None
        }
    );
    assert_eq!(
        gate_over(&higher, &[0.5]),
        GateOutcome::Fail {
            current: 0.5,
            baseline: None
        }
    );
    // Exactly the threshold passes: `ge`.
    assert_eq!(
        gate_over(&higher, &[0.8]),
        GateOutcome::Pass {
            current: 0.8,
            baseline: None
        }
    );

    // The same rule under an objective whose `bound` disagrees with it in
    // both directions: a value short of the bound still passes, and a value
    // past the bound still fails. The bound is never read.
    let bounded = |bound: &str| {
        plans(
            &gate_plan_document("higher", "v1", "    comparator: ge\n    threshold: 0.8\n")
                .replace(
                    "direction: higher\n",
                    &format!("direction: higher\n  bound: {bound}\n"),
                ),
        )
    };
    let unreachable = bounded("0.99");
    assert_eq!(unreachable[0].objective.and_then(|o| o.bound()), Some(0.99));
    assert_eq!(
        gate_over(&unreachable, &[0.9]),
        GateOutcome::Pass {
            current: 0.9,
            baseline: None
        }
    );
    let already_met = bounded("0.1");
    assert_eq!(
        gate_over(&already_met, &[0.5]),
        GateOutcome::Fail {
            current: 0.5,
            baseline: None
        }
    );

    let lower = plans(&gate_plan_document(
        "lower",
        "v1",
        "    comparator: le\n    threshold: 0.2\n",
    ));
    assert_eq!(
        gate_over(&lower, &[0.1]),
        GateOutcome::Pass {
            current: 0.1,
            baseline: None
        }
    );
    assert_eq!(
        gate_over(&lower, &[0.5]),
        GateOutcome::Fail {
            current: 0.5,
            baseline: None
        }
    );
}

/// A `baseline` rule reads its reference from the same usable-evidence pool
/// a ratchet draws from: `prior-collection` is the nearest earlier usable
/// value, and `best-seen` is the maximum for `gt`/`ge` and the minimum for
/// `lt`/`le`/`eq`, over the plan's own slice.
///
/// Trace: FR-107-AC-7
/// Provenance: PLAT-958
#[test]
fn tc_958_020_a_baseline_gate_reads_prior_collection_or_best_seen() {
    let prior = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: gt\n    baseline: prior-collection\n",
    ));
    // 0.6 > prior (0.5): pass.
    assert_eq!(
        gate_over(&prior, &[0.5, 0.6]),
        GateOutcome::Pass {
            current: 0.6,
            baseline: Some(0.5)
        }
    );
    // 0.4 > prior (0.6): fail — `prior-collection` is the nearest earlier
    // value, not the best of every earlier value.
    assert_eq!(
        gate_over(&prior, &[0.5, 0.6, 0.4]),
        GateOutcome::Fail {
            current: 0.4,
            baseline: Some(0.6)
        }
    );

    let best_seen = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: ge\n    baseline: best-seen\n",
    ));
    // best-seen over [0.5, 0.6] is 0.6; 0.55 does not reach it.
    assert_eq!(
        gate_over(&best_seen, &[0.5, 0.6, 0.55]),
        GateOutcome::Fail {
            current: 0.55,
            baseline: Some(0.6)
        }
    );
    assert_eq!(
        gate_over(&best_seen, &[0.5, 0.6, 0.6]),
        GateOutcome::Pass {
            current: 0.6,
            baseline: Some(0.6)
        }
    );

    let best_seen_lower = plans(&gate_plan_document(
        "lower",
        "v1",
        "    comparator: le\n    baseline: best-seen\n",
    ));
    // best-seen over [5.0, 3.0] under `le` is the minimum, 3.0.
    assert_eq!(
        gate_over(&best_seen_lower, &[5.0, 3.0, 4.0]),
        GateOutcome::Fail {
            current: 4.0,
            baseline: Some(3.0)
        }
    );
    assert_eq!(
        gate_over(&best_seen_lower, &[5.0, 3.0, 2.0]),
        GateOutcome::Pass {
            current: 2.0,
            baseline: Some(3.0)
        }
    );
}

/// A `gate` verdict is `inconclusive`, never `pass`, when the plan states no
/// `decision_rule`, when a `baseline` rule finds no earlier usable value
/// (the first collection a gate ever sees), and when the rule's baseline is
/// `constant-predictor`, which needs per-item rows no collection here
/// carries.
///
/// Trace: FR-107-AC-8
/// Provenance: PLAT-958
#[test]
fn tc_958_021_a_gate_is_inconclusive_with_no_rule_no_prior_or_an_unsupported_baseline() {
    // No `statistical_design` at all: `no_decision_rule`.
    let no_rule = plans(&plan_document("gate", "v1", &{
        let mut objective = objective("higher", None);
        objective.push_str(
            "protected_apparatus:\n  - answers.json\nnegative_controls:\n  - kind: \
             apparatus-edit\n    description: test-only\n",
        );
        objective
    }));
    assert_eq!(
        gate_over(&no_rule, &[0.9]),
        GateOutcome::Inconclusive(InconclusiveReason::NoDecisionRule)
    );

    // A `baseline` rule on the first collection a gate ever sees: `no_prior`.
    let prior = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: gt\n    baseline: prior-collection\n",
    ));
    assert_eq!(
        gate_over(&prior, &[0.9]),
        GateOutcome::Inconclusive(InconclusiveReason::NoPrior)
    );

    // `constant-predictor` needs per-item rows this crate never stores.
    let constant_predictor = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: gt\n    baseline: constant-predictor\n",
    ));
    assert_eq!(
        gate_over(&constant_predictor, &[0.5, 0.9]),
        GateOutcome::Inconclusive(InconclusiveReason::ConstantPredictorUnsupported)
    );
}

/// An empty or incomplete newest population is `inconclusive`, however good
/// the value, exactly as for a ratchet and a target (AC-2's usable-evidence
/// rule applies to a gate too) — never a green `pass`.
///
/// Trace: FR-107-AC-8
/// Provenance: PLAT-958
#[test]
fn tc_958_022_an_incomplete_or_empty_population_is_inconclusive_for_a_gate() {
    let threshold = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: ge\n    threshold: 0.1\n",
    ));
    let empty = admitted(
        &threshold,
        0,
        observation(0.99, "v1", Some(json!({ "examined": 0 }))),
    );
    assert_eq!(
        report(&threshold, &[empty]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::EmptyPopulation)
    );
    let incomplete = admitted(
        &threshold,
        0,
        observation(
            0.99,
            "v1",
            Some(json!({ "examined": 10, "complete": false })),
        ),
    );
    assert_eq!(
        report(&threshold, &[incomplete]).current[0]
            .stage_verdict
            .as_ref()
            .and_then(StageVerdict::inconclusive_reason),
        Some(InconclusiveReason::IncompletePopulation)
    );

    // A `no collection at all` row: `no_current_value`, never `pass`.
    assert_eq!(
        gate_over(&threshold, &[]),
        GateOutcome::Inconclusive(InconclusiveReason::NoCurrentValue)
    );
}

/// A gate's verdict renders in the text report's Stage verdicts table with
/// its baseline, in the JSON `stageVerdict`, and a `fail` adds an attention
/// item, alongside a `regressed` ratchet's.
///
/// Trace: FR-107-AC-9
/// Provenance: PLAT-958
#[test]
fn tc_958_023_a_gate_verdict_renders_in_text_and_json_and_a_fail_is_an_attention_item() {
    let plans = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: gt\n    baseline: prior-collection\n",
    ));
    let first = stored_gate(&plans, 0, &[observation(0.6, "v1", None)], Some('0'));
    let newest = stored_gate(&plans, 1, &[observation(0.4, "v1", None)], Some('0'));
    let failing = report(&plans, &[first, newest]);

    let text = render_measurement_report(&failing).expect("the report renders");
    assert!(
        text.contains(
            "| quality.score | MP-958 (spec/assurance/MP-958.md) | gate | higher | fail | \
             current 0.4; baseline 0.6 |"
        ),
        "{text}"
    );
    assert!(
        text.contains("- quality.score: fails its gate at 0.4; plan MP-958 does not pass."),
        "{text}"
    );

    let parsed: Value =
        serde_json::from_str(&render_measurement_report_json(&failing).expect("the JSON renders"))
            .expect("JSON");
    assert_eq!(
        parsed["current"][0]["stageVerdict"],
        json!({
            "stage": "gate",
            "objective": { "direction": "higher" },
            "verdict": "fail",
            "reason": null,
            "current": 0.4,
            "baseline": 0.6,
        })
    );

    // A `pass` adds no attention item.
    let passing = report(
        &plans,
        &[
            stored_gate(&plans, 0, &[observation(0.4, "v1", None)], Some('0')),
            stored_gate(&plans, 1, &[observation(0.6, "v1", None)], Some('0')),
        ],
    );
    assert_eq!(
        passing.current[0]
            .stage_verdict
            .as_ref()
            .map(StageVerdict::as_str),
        Some("pass")
    );
    let text = render_measurement_report(&passing).expect("the report renders");
    assert!(!text.contains("does not pass"), "{text}");
}

/// No unusable earlier value ever becomes a gate's baseline, and no other
/// slice's does: an incomplete, empty or population-unstated earlier value,
/// or one under another `definition_version`, is skipped however much it
/// would flatter the newest value, and with only such values before it a
/// `baseline` gate is `no_prior` — never `pass`. Each slice is gated against
/// its own history alone.
///
/// Trace: FR-107-AC-7, FR-107-AC-8
/// Provenance: PLAT-958
#[test]
fn tc_958_024_no_unusable_or_foreign_earlier_value_becomes_a_gate_baseline() {
    let rule = "    comparator: gt\n    baseline: prior-collection\n";
    let plans = plans(&gate_plan_document("higher", "v1", rule));
    let gate =
        |index: u8, observation: Value| stored_gate(&plans, index, &[observation], Some('0'));
    let unusable = [
        observation(
            0.1,
            "v1",
            Some(json!({ "examined": 10, "complete": false })),
        ),
        observation(0.1, "v1", Some(json!({ "examined": 0 }))),
        unstated(0.1),
    ];
    // Were any of them the nearest earlier value, 0.5 > 0.1 would pass.
    let mut collections = vec![gate(0, observation(0.8, "v1", None))];
    collections.extend((1_u8..).zip(unusable.clone()).map(|(i, o)| gate(i, o)));
    collections.push(gate(4, observation(0.5, "v1", None)));
    assert_eq!(
        gate_of(&plans, &collections),
        GateOutcome::Fail {
            current: 0.5,
            baseline: Some(0.8)
        }
    );

    // Only unusable values before it: `no_prior`, not a pass.
    let mut collections: Vec<_> = (0_u8..).zip(unusable).map(|(i, o)| gate(i, o)).collect();
    collections.push(gate(3, observation(0.9, "v1", None)));
    assert_eq!(
        gate_of(&plans, &collections),
        GateOutcome::Inconclusive(InconclusiveReason::NoPrior)
    );

    // A newest value with no population is inconclusive whatever its value.
    let collections = [
        gate(0, observation(0.1, "v1", None)),
        gate(1, unstated(0.9)),
    ];
    assert_eq!(
        gate_of(&plans, &collections),
        GateOutcome::Inconclusive(InconclusiveReason::PopulationUnstated)
    );

    // An earlier value under a retired definition is not a baseline.
    let current = plans_v2(rule);
    let retired = stored_gate(&plans, 0, &[observation(0.1, "v1", None)], Some('0'));
    let newest = stored_gate(&current, 1, &[observation(0.9, "v2", None)], Some('0'));
    assert_eq!(
        gate_of(&current, &[retired, newest]),
        GateOutcome::Inconclusive(InconclusiveReason::NoPrior)
    );

    // Each slice against its own history: slice `b`'s low earlier value is
    // never slice `a`'s baseline.
    let best_seen = self::plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: ge\n    baseline: best-seen\n",
    ));
    let first = stored_gate(
        &best_seen,
        0,
        &[sliced(0.9, "a"), sliced(0.1, "b")],
        Some('0'),
    );
    let newest = stored_gate(
        &best_seen,
        1,
        &[sliced(0.5, "a"), sliced(0.5, "b")],
        Some('0'),
    );
    let sliced_report = report(&best_seen, &[first, newest]);
    let objective = best_seen[0].objective.expect("an objective");
    assert_eq!(
        slice_verdict(&sliced_report, "a"),
        Some(StageVerdict::Gate {
            objective,
            outcome: GateOutcome::Fail {
                current: 0.5,
                baseline: Some(0.9)
            }
        })
    );
    assert_eq!(
        slice_verdict(&sliced_report, "b"),
        Some(StageVerdict::Gate {
            objective,
            outcome: GateOutcome::Pass {
                current: 0.5,
                baseline: Some(0.1)
            }
        })
    );
}

/// The gate plan of [`gate_plan_document`] under `definition_version: v2`.
fn plans_v2(rule: &str) -> Vec<MeasurementPlan> {
    plans(&gate_plan_document("higher", "v2", rule))
}

/// A `baseline` gate never compares across a changed protected apparatus,
/// exactly as a ratchet never holds against one (FR-110-AC-6): an earlier
/// value measured with a different recorded apparatus makes it
/// `apparatus_changed`, and a collection that recorded none makes it
/// `apparatus_unrecorded` — where comparing the numbers alone would pass. A
/// `threshold` gate compares against no earlier value, and is unaffected.
///
/// Trace: FR-107-AC-8, FR-107-CON-3
/// Provenance: PLAT-958, PLAT-975
#[test]
fn tc_958_025_a_baseline_gate_does_not_compare_across_another_apparatus() {
    let plans = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: gt\n    baseline: prior-collection\n",
    ));
    let run = |index: u8, value: f64, digit: Option<char>| {
        stored_gate(&plans, index, &[observation(value, "v1", None)], digit)
    };
    for (earlier, newest, reason) in [
        (Some('0'), Some('1'), InconclusiveReason::ApparatusChanged),
        (None, Some('0'), InconclusiveReason::ApparatusUnrecorded),
        (Some('0'), None, InconclusiveReason::ApparatusUnrecorded),
    ] {
        assert_eq!(
            gate_of(&plans, &[run(0, 0.1, earlier), run(1, 0.5, newest)]),
            GateOutcome::Inconclusive(reason),
            "{earlier:?} -> {newest:?}"
        );
    }
    // The same apparatus throughout: the numbers decide.
    assert_eq!(
        gate_of(&plans, &[run(0, 0.1, Some('0')), run(1, 0.5, Some('0'))]),
        GateOutcome::Pass {
            current: 0.5,
            baseline: Some(0.1)
        }
    );

    let threshold = self::plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: ge\n    threshold: 0.4\n",
    ));
    let changed = [
        stored_gate(&threshold, 0, &[observation(0.1, "v1", None)], Some('0')),
        stored_gate(&threshold, 1, &[observation(0.5, "v1", None)], Some('1')),
    ];
    assert_eq!(
        gate_of(&threshold, &changed),
        GateOutcome::Pass {
            current: 0.5,
            baseline: None
        }
    );
}

/// EA v0.4.0 added `Baseline::ExternalReference`: a per-dimension value the
/// checker resolves from a source outside the plan at evaluation time. The
/// report layer has no such source wired in, so a `gate` reading it is
/// `inconclusive` with its own reason even when a usable earlier value
/// exists — never `no_prior` (a different claim) and never silently treated
/// as `prior-collection` or `best-seen` by a wildcard match arm.
///
/// Trace: FR-107-AC-8
/// Provenance: PLAT-1032
#[test]
fn tc_958_026_an_external_reference_baseline_is_inconclusive_with_its_own_reason() {
    let external_reference = plans(&gate_plan_document(
        "higher",
        "v1",
        "    comparator: gt\n    baseline: external-reference\n",
    ));
    assert_eq!(
        gate_over(&external_reference, &[0.5, 0.9]),
        GateOutcome::Inconclusive(InconclusiveReason::ExternalReferenceUnsupplied)
    );
}
