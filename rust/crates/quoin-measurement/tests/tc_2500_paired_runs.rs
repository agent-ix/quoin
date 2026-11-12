// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-132: a metric that states two runs agree is computed from a paired
//! comparison of two collections under the plan's stated relation.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::observation::MeasurementState;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::{PairedRunOutcome, compare_paired_runs, validate};
use serde_json::{Value, json};

fn plan_text(paired_run: &str, objective: &str) -> String {
    format!(
        "---\nid: MP-001\ntitle: Determinism\ntype: MeasurementPlan\nstatus: active\nowner: test\n\
         stage: branch-comparison\nmetric: quality.example\ndefinition_version: quality.example-v1\n\
         {objective}{paired_run}---\n\n# Determinism\n"
    )
}

fn load(text: &str) -> Result<Vec<MeasurementPlan>, quoin_measurement::MeasurementError> {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("creatable");
    std::fs::write(assurance.join("MP-001.md"), text).expect("writable");
    load_measurement_plans(
        &DiskMeasurement::new(temporary.path()),
        PlanLoadOptions::default(),
    )
}

fn plans(paired_run: &str, objective: &str) -> Vec<MeasurementPlan> {
    load(&plan_text(paired_run, objective)).expect("the plan loads")
}

/// A run with one observation per `(root, value)`.
fn run(id: &str, values: &[(&str, f64)]) -> MeasurementCollection {
    let observations: Vec<Value> = values
        .iter()
        .map(|(root, value)| {
            json!({
                "metric": "quality.example",
                "planId": "MP-001",
                "definitionVersion": "quality.example-v1",
                "state": "measured",
                "value": value,
                "unit": "count",
                "shape": "count",
                "dimensions": { "root": root },
                "population": { "examined": 1, "complete": true },
            })
        })
        .collect();
    let document = json!({
        "schemaVersion": 2,
        "collectionId": id,
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1",
        "timestamp": "2026-08-26T00:00:00.000Z",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"],
            "artifacts": ["config"],
        },
        "observations": observations,
        "rawEvidence": { "payload": [1] },
    });
    let stored = from_serde(&document).expect("bridge");
    validate::stored_measurement_collection(&stored).expect("admissible")
}

const EQUAL_2: &str = "paired_run:\n  relation: equal\n  minimum_pairs: 2\n";

/// Identical runs observe zero differing slices; one differing slice observes one.
#[trace("FR-132-AC-1")]
#[test]
fn tc_2500_equal_counts_the_slices_that_differ() {
    let plans = plans(EQUAL_2, "");
    let first = run("a", &[("r1", 3.0), ("r2", 4.0)]);
    let same = run("b", &[("r1", 3.0), ("r2", 4.0)]);
    let observed = compare_paired_runs(&first, &same, &plans).expect("compares");
    assert_eq!(observed.len(), 1);
    assert_eq!(
        observed[0].outcome,
        PairedRunOutcome::Observed { differing: 0 }
    );
    assert_eq!(observed[0].comparisons, 2);

    let moved = run("c", &[("r1", 3.0), ("r2", 5.0)]);
    let observed = compare_paired_runs(&first, &moved, &plans).expect("compares");
    assert_eq!(
        observed[0].outcome,
        PairedRunOutcome::Observed { differing: 1 }
    );
}

/// `within_delta` tolerates movement up to the delta, in either direction, inclusive.
#[trace("FR-132-AC-2")]
#[test]
fn tc_2501_within_delta_is_a_symmetric_inclusive_band() {
    let plans = plans(
        "paired_run:\n  relation: within_delta\n  minimum_pairs: 1\n  delta: 0.5\n",
        "",
    );
    let first = run("a", &[("r1", 10.0), ("r2", 10.0), ("r3", 10.0)]);
    let second = run("b", &[("r1", 10.5), ("r2", 9.4), ("r3", 10.0)]);
    let observed = compare_paired_runs(&first, &second, &plans).expect("compares");
    assert_eq!(
        observed[0].outcome,
        PairedRunOutcome::Observed { differing: 1 }
    );
}

/// `not_worse` reads the plan's objective direction and the order of the runs.
#[trace("FR-132-AC-3")]
#[test]
fn tc_2502_not_worse_follows_the_objective_direction() {
    let paired = "paired_run:\n  relation: not_worse\n  minimum_pairs: 1\n";
    let first = run("a", &[("r1", 5.0), ("r2", 5.0)]);
    let second = run("b", &[("r1", 4.0), ("r2", 4.0)]);
    let differing = |first, second, direction: &str| {
        let plans = plans(paired, &format!("objective:\n  direction: {direction}\n"));
        compare_paired_runs(first, second, &plans).expect("compares")[0]
            .outcome
            .clone()
    };
    let observed = |differing| PairedRunOutcome::Observed { differing };
    assert_eq!(differing(&first, &second, "lower"), observed(0));
    assert_eq!(differing(&first, &second, "higher"), observed(2));
    assert_eq!(differing(&second, &first, "higher"), observed(0));
    assert_eq!(differing(&second, &first, "lower"), observed(2));
}

/// Too few comparisons is inconclusive and carries no count, never zero variance.
#[trace("FR-132-AC-4")]
#[test]
fn tc_2503_fewer_than_the_minimum_is_inconclusive_not_zero() {
    let plans = plans(EQUAL_2, "");
    let one = run("a", &[("r1", 3.0)]);
    let observed = compare_paired_runs(&one, &one, &plans).expect("compares");
    assert_eq!(observed[0].outcome, PairedRunOutcome::Inconclusive);
    assert_eq!(observed[0].comparisons, 1);
    assert!(observed[0].reasons[0].contains("inconclusive"));
}

/// A slice only one run states is a violation of every relation, even when
/// enough shared slices remain to meet the minimum.
#[trace("FR-132-AC-4")]
#[test]
fn tc_2507_a_slice_in_one_run_only_counts_as_differing() {
    let first = run("a", &[("r1", 3.0), ("r2", 4.0), ("r3", 5.0)]);
    let shorter = run("b", &[("r1", 3.0), ("r2", 4.0)]);
    let relations = [
        EQUAL_2,
        "paired_run:\n  relation: within_delta\n  minimum_pairs: 2\n  delta: 1\n",
        "paired_run:\n  relation: not_worse\n  minimum_pairs: 2\n",
    ];
    for relation in relations {
        let plans = plans(relation, "objective:\n  direction: lower\n");
        for (left, right) in [(&first, &shorter), (&shorter, &first)] {
            let observed = compare_paired_runs(left, right, &plans).expect("compares");
            assert_eq!(
                observed[0].outcome,
                PairedRunOutcome::Observed { differing: 1 },
                "{relation}"
            );
            assert_eq!(observed[0].comparisons, 2);
        }
    }
}

/// A slice a run states without a value is neither agreement nor
/// disagreement: the pair is inconclusive, never clean.
#[trace("FR-132-AC-4")]
#[test]
fn tc_2508_a_slice_without_a_value_makes_the_pair_inconclusive() {
    let plans = plans(EQUAL_2, "");
    let first = run("a", &[("r1", 3.0), ("r2", 4.0), ("r3", 5.0)]);
    let mut second = run("b", &[("r1", 3.0), ("r2", 4.0), ("r3", 5.0)]);
    second.observations[2].state = MeasurementState::NotComputed;
    second.observations[2].value = None;
    let observed = compare_paired_runs(&first, &second, &plans).expect("compares");
    assert_eq!(observed[0].outcome, PairedRunOutcome::Inconclusive);
    assert_eq!(observed[0].comparisons, 2);
}

/// The minimum defaults to 1, so a pair that compared nothing is never clean.
#[trace("FR-132-AC-7")]
#[test]
fn tc_2509_minimum_pairs_defaults_to_one() {
    let plans = plans("paired_run:\n  relation: equal\n", "");
    let first = run("a", &[("r1", 3.0)]);
    let other = run("b", &[("r2", 3.0)]);
    let observed = compare_paired_runs(&first, &other, &plans).expect("compares");
    assert_eq!(observed[0].minimum_pairs, 1);
    assert_eq!(observed[0].outcome, PairedRunOutcome::Inconclusive);
    let observed = compare_paired_runs(&first, &first, &plans).expect("compares");
    assert_eq!(
        observed[0].outcome,
        PairedRunOutcome::Observed { differing: 0 }
    );
}

/// A blocked comparison makes the whole pair incomparable, with no count.
#[trace("FR-132-AC-5")]
#[test]
fn tc_2504_a_blocked_slice_makes_the_pair_incomparable() {
    let plans = plans(EQUAL_2, "");
    let first = run("a", &[("r1", 3.0), ("r2", 4.0)]);
    let second = run("b", &[("r1", 3.0), ("r2", 4.0)]);
    let mut moved_profile = second.clone();
    if let Some(stack) = moved_profile.verification_stack.as_mut() {
        stack.build_profile = Some(quoin_measurement::types::collection::BuildProfile::Debug);
    }
    let observed = compare_paired_runs(&first, &moved_profile, &plans).expect("compares");
    assert_eq!(observed[0].outcome, PairedRunOutcome::Incomparable);
    assert!(
        observed[0]
            .reasons
            .iter()
            .any(|r| r.starts_with("profile_changed"))
    );

    let mut moved_definition = run("c", &[("r1", 3.0), ("r2", 4.0)]);
    for observation in &mut moved_definition.observations {
        observation.definition_version = NonEmptyText::parse(
            "quality.example-v2",
            MeasurementErrorCode::CollectionInvalid,
            "definitionVersion",
        )
        .expect("non-empty");
    }
    let observed = compare_paired_runs(&first, &moved_definition, &plans).expect("compares");
    assert_eq!(observed[0].outcome, PairedRunOutcome::Incomparable);
    assert!(
        observed[0]
            .reasons
            .iter()
            .any(|r| r.starts_with("definition_changed"))
    );

    let mut moved_apparatus = run("d", &[("r1", 3.0), ("r2", 4.0)]);
    if let Some(stack) = moved_apparatus.verification_stack.as_mut() {
        stack.protected_apparatus.insert(
            "MP-001".to_owned(),
            ["harness/answers.json".to_owned()].into(),
        );
    }
    let observed = compare_paired_runs(&first, &moved_apparatus, &plans).expect("compares");
    assert_eq!(observed[0].outcome, PairedRunOutcome::Incomparable);
    assert!(
        observed[0]
            .reasons
            .iter()
            .any(|r| r.starts_with("apparatus_changed"))
    );

    let mut incomplete = run("e", &[("r1", 3.0), ("r2", 4.0)]);
    for observation in &mut incomplete.observations {
        if let Some(population) = observation.population.as_mut() {
            population.complete = Some(false);
        }
    }
    let observed = compare_paired_runs(&first, &incomplete, &plans).expect("compares");
    assert_eq!(observed[0].outcome, PairedRunOutcome::Incomparable);
    assert!(
        observed[0]
            .reasons
            .iter()
            .any(|r| r.starts_with("incomplete_population"))
    );
}

/// A plan without `paired_run` yields nothing.
#[trace("FR-132-AC-6")]
#[test]
fn tc_2505_a_plan_without_paired_run_yields_no_observation() {
    let plans = plans("", "");
    let first = run("a", &[("r1", 3.0), ("r2", 4.0)]);
    assert!(
        compare_paired_runs(&first, &first, &plans)
            .expect("compares")
            .is_empty()
    );
}

/// The plan load refuses every malformed or inconsistent `paired_run` block.
#[trace("FR-132-AC-7")]
#[test]
fn tc_2506_a_malformed_paired_run_refuses_the_plan() {
    let lower = "objective:\n  direction: lower\n";
    for (paired_run, objective) in [
        ("paired_run: equal\n", ""),
        ("paired_run:\n  relation: equal\n  extra: 1\n", ""),
        (
            "paired_run:\n  relation: within_delta\n  minimum_pairs: 1\n  delta: 0\n",
            "",
        ),
        ("paired_run:\n  relation: equal\n  minimum_pairs: 0\n", ""),
        ("paired_run:\n  relation: equal\n  minimum_pairs: 1.5\n", ""),
        (
            "paired_run:\n  relation: equal\n  minimum_pairs: 1\n  delta: 1\n",
            "",
        ),
        ("paired_run:\n  relation: similar\n  minimum_pairs: 1\n", ""),
        (
            "paired_run:\n  relation: within_delta\n  minimum_pairs: 1\n",
            "",
        ),
        (
            "paired_run:\n  relation: within_delta\n  minimum_pairs: 1\n  delta: -1\n",
            "",
        ),
        (
            "paired_run:\n  relation: not_worse\n  minimum_pairs: 1\n",
            "",
        ),
        (
            "paired_run:\n  relation: not_worse\n  minimum_pairs: 1\n",
            "objective:\n  direction: target\n",
        ),
    ] {
        assert!(
            load(&plan_text(paired_run, objective)).is_err(),
            "{paired_run:?} with {objective:?} must be refused"
        );
    }
    assert!(
        load(&plan_text(
            "paired_run:\n  relation: not_worse\n  minimum_pairs: 1\n",
            lower
        ))
        .is_ok()
    );
}
