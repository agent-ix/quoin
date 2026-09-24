// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-968 (FR-113): the portfolio's advisory priority ranking.
//!
//! Every fixture here builds a [`PortfolioReport`] directly from the crate's
//! public types rather than through frontmatter and a temp-dir store —
//! [`rank_portfolio`] is pure over an already-built report, so nothing here
//! needs a filesystem. The plan-load and collection-intake paths that produce
//! a real [`PortfolioReport`] are already exercised by `tc_958_stage_verdicts.rs`
//! and `tc_479_fr044_fr045_criteria.rs`; this file is about the ranking
//! formula alone.
//!
//! Provenance: PLAT-968

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::float_cmp,
    reason = "these assert exact literals the formula produces from exact \
              literal inputs (weight defaults, decay with no half-life, and \
              two budgets floored to the same MIN_BUDGET_FLOOR), not a \
              recomputation that could drift by rounding"
)]

use std::collections::BTreeMap;

use engineering_assurance::measurement::{Direction, Objective};
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::portfolio::{
    PortfolioRanking, PortfolioReport, PortfolioRepositoryReport, RANKING_ADVISORY_NOTE,
    RepositoryStatus, Staleness, StoreState, UnrankedReason, rank_portfolio,
    render_portfolio_report, render_portfolio_report_json,
};
use quoin_measurement::report::{CollectionSummary, CurrentRow, MeasurementReport};
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::observation::{
    Dimensions, MeasurementObservation, MeasurementPopulation, MeasurementShape, MeasurementState,
};
use quoin_measurement::types::plan::{
    GroundTruthKind, LifecycleStatus, MeasurementPlan, MeasurementStage,
};
use quoin_store::JsonValue;
use serde_json::Value;

fn text(value: &str) -> NonEmptyText {
    NonEmptyText::parse(value, MeasurementErrorCode::PlanInvalid, "field").unwrap()
}

fn plan(id: &str, objective: Option<Objective>) -> MeasurementPlan {
    MeasurementPlan {
        id: text(id),
        title: text("Example plan"),
        status: LifecycleStatus::Active,
        stage: MeasurementStage::Target,
        metric: text("quality.score"),
        definition_version: text("v1"),
        execution_procedure: None,
        path: format!("spec/assurance/{id}.md"),
        owner: None,
        action: None,
        preregistration: None,
        ground_truth_kind: None,
        statistical_design: None,
        objective,
        protected_apparatus: None,
        negative_controls: None,
    }
}

fn observation(plan_id: &str, value: f64) -> MeasurementObservation {
    MeasurementObservation {
        metric: text("quality.score"),
        plan_id: text(plan_id),
        definition_version: text("v1"),
        state: MeasurementState::Measured,
        value: Some(value),
        unit: text("fraction"),
        shape: MeasurementShape::Scalar,
        population: Some(MeasurementPopulation {
            examined: Some(10.0),
            ..MeasurementPopulation::EMPTY
        }),
        dimensions: Dimensions::ABSENT,
        reason: None,
    }
}

fn collection(timestamp: &str) -> CollectionSummary {
    CollectionSummary {
        collection_id: "run-1".to_owned(),
        timestamp: timestamp.to_owned(),
        tool_identity: "fixture".to_owned(),
        tool_version: "1".to_owned(),
        config_digest: format!("sha256:{}", "a".repeat(64)),
        source_revision: "a".repeat(40),
        corpus_revision: None,
        path: "spec/evidence/measurements/run-1.json".to_owned(),
        unverified_artifacts: Vec::new(),
    }
}

/// One repository, one row: `plan`'s newest value is `value`, measured in a
/// collection timestamped `timestamp`, with no `objective` unless `plan`
/// carries one.
fn repository_with_row(
    name: &str,
    plan: MeasurementPlan,
    value: f64,
    timestamp: &str,
) -> PortfolioRepositoryReport {
    let row = CurrentRow {
        metric: "quality.score".to_owned(),
        plan_id: plan.id.as_str().to_owned(),
        plan_path: plan.path.clone(),
        plan_definition_version: plan.definition_version.as_str().to_owned(),
        stage: plan.stage,
        plan_ground_truth_kind: None::<GroundTruthKind>,
        observation: Some(observation(plan.id.as_str(), value)),
        collection: Some(collection(timestamp)),
        stage_verdict: None,
    };
    PortfolioRepositoryReport {
        name: name.to_owned(),
        root: format!("/repos/{name}"),
        status: RepositoryStatus::Readable,
        store: StoreState::Present,
        profiles: Vec::new(),
        plans: vec![plan],
        measurements: Some(MeasurementReport {
            plans: Vec::new(),
            current: vec![row],
            vanished_slices: Vec::new(),
            corpus_gaps: None,
            interventions: Vec::new(),
            operational: Vec::new(),
        }),
        latest_collection: None,
        comparison: None,
        staleness: Staleness::NotComputed,
    }
}

fn report(repositories: Vec<PortfolioRepositoryReport>, newest: &str) -> PortfolioReport {
    PortfolioReport {
        newest_collection_timestamp: Some(newest.to_owned()),
        repositories,
    }
}

/// A plan far from its bound outranks one close to its bound, with equal
/// weight.
///
/// Trace: FR-113-AC-1
#[test]
fn tc_968_001_ranks_by_gap_to_bound_when_weights_are_equal() {
    let far = plan(
        "MP-far",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );
    let near = plan(
        "MP-near",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );
    let report = report(
        vec![
            repository_with_row("repo", far, 0.1, "2026-01-01T00:00:00.000Z"),
            repository_with_row("repo2", near, 0.85, "2026-01-01T00:00:00.000Z"),
        ],
        "2026-01-01T00:00:00.000Z",
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert_eq!(ranking.ranked.len(), 2);
    assert_eq!(ranking.ranked[0].plan_id, "MP-far");
    assert_eq!(ranking.ranked[1].plan_id, "MP-near");
    assert!(ranking.ranked[0].score > ranking.ranked[1].score);
}

/// A higher declared weight moves an otherwise-smaller gap ahead of an
/// unweighted, larger one — the "with and without weight" comparison the
/// acceptance criterion asks for.
///
/// Trace: FR-113-AC-1
#[test]
fn tc_968_002_weight_can_reorder_a_smaller_gap_ahead_of_a_larger_one() {
    let small_gap_high_weight = plan(
        "MP-weighted",
        Some(
            Objective::with_steering(Direction::Higher, Some(0.9), Some(20.0), None, None).unwrap(),
        ),
    );
    let large_gap_no_weight = plan(
        "MP-unweighted",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );
    let report = report(
        vec![
            repository_with_row(
                "repo",
                small_gap_high_weight,
                0.85,
                "2026-01-01T00:00:00.000Z",
            ),
            repository_with_row(
                "repo2",
                large_gap_no_weight,
                0.1,
                "2026-01-01T00:00:00.000Z",
            ),
        ],
        "2026-01-01T00:00:00.000Z",
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert_eq!(ranking.ranked[0].plan_id, "MP-weighted");
    assert_eq!(ranking.ranked[0].weight, 20.0);
    assert_eq!(
        ranking.ranked[1].weight, 1.0,
        "unweighted defaults to neutral 1.0"
    );
}

/// A stale gap, discounted by its half-life, sorts behind a fresher, smaller
/// gap it would otherwise outrank.
///
/// Trace: FR-113-AC-2
#[test]
fn tc_968_003_half_life_discount_can_move_a_stale_large_gap_behind_a_fresh_small_one() {
    let stale = plan(
        "MP-stale",
        Some(
            Objective::with_steering(Direction::Higher, Some(0.9), None, Some(10.0), None).unwrap(),
        ),
    );
    let fresh = plan(
        "MP-fresh",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );
    let report = report(
        vec![
            // 40 days behind portfolio-newest, half-life 10 days: decay is
            // 0.5^4 = 0.0625, so its gap of 0.8 scores as 0.05.
            repository_with_row("repo", stale, 0.1, "2025-11-22T00:00:00.000Z"),
            // Measured at portfolio-newest itself: full-strength gap of 0.1.
            repository_with_row("repo2", fresh, 0.8, "2026-01-01T00:00:00.000Z"),
        ],
        "2026-01-01T00:00:00.000Z",
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert_eq!(ranking.ranked[0].plan_id, "MP-fresh");
    assert_eq!(ranking.ranked[1].plan_id, "MP-stale");
    assert!((ranking.ranked[1].decay_factor - 0.0625).abs() < 1e-9);
}

/// No `value_half_life` applies no discount, regardless of age.
///
/// Trace: FR-113-AC-2
#[test]
fn tc_968_004_no_half_life_applies_no_discount() {
    let old_no_half_life = plan(
        "MP-old",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );
    let report = report(
        vec![repository_with_row(
            "repo",
            old_no_half_life,
            0.1,
            "2020-01-01T00:00:00.000Z",
        )],
        "2026-01-01T00:00:00.000Z",
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert_eq!(ranking.ranked[0].decay_factor, 1.0);
}

/// A larger declared budget divides the score down; a near-zero budget is
/// floored rather than sending the score to an absurd extreme.
///
/// Trace: FR-113-AC-3
#[test]
fn tc_968_005_budget_normalizes_the_score_and_a_tiny_budget_is_floored() {
    let cheap = plan(
        "MP-cheap",
        Some(
            Objective::with_steering(Direction::Higher, Some(0.9), None, None, Some(2.0)).unwrap(),
        ),
    );
    let expensive = plan(
        "MP-expensive",
        Some(
            Objective::with_steering(Direction::Higher, Some(0.9), None, None, Some(200.0))
                .unwrap(),
        ),
    );
    let tiny_budget = plan(
        "MP-tiny",
        Some(
            Objective::with_steering(Direction::Higher, Some(0.9), None, None, Some(0.0001))
                .unwrap(),
        ),
    );
    let no_budget = plan(
        "MP-none",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );
    let report = report(
        vec![
            repository_with_row("r1", cheap, 0.5, "2026-01-01T00:00:00.000Z"),
            repository_with_row("r2", expensive, 0.5, "2026-01-01T00:00:00.000Z"),
            repository_with_row("r3", tiny_budget, 0.5, "2026-01-01T00:00:00.000Z"),
            repository_with_row("r4", no_budget, 0.5, "2026-01-01T00:00:00.000Z"),
        ],
        "2026-01-01T00:00:00.000Z",
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    let by_id = |id: &str| {
        ranking
            .ranked
            .iter()
            .find(|entry| entry.plan_id == id)
            .unwrap()
    };
    assert!(by_id("MP-cheap").score > by_id("MP-expensive").score);
    // A tiny budget is floored to the same divisor as no budget at all.
    assert_eq!(by_id("MP-tiny").score, by_id("MP-none").score);
}

/// A plan with no objective is listed as unranked, not scored, and does not
/// appear in the ranked list.
///
/// Trace: FR-113-AC-4
#[test]
fn tc_968_006_no_objective_is_listed_unranked_not_scored() {
    let report = report(
        vec![repository_with_row(
            "repo",
            plan("MP-plain", None),
            0.5,
            "2026-01-01T00:00:00.000Z",
        )],
        "2026-01-01T00:00:00.000Z",
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert!(ranking.ranked.is_empty());
    assert_eq!(ranking.unranked.len(), 1);
    assert_eq!(ranking.unranked[0].plan_id, "MP-plain");
    assert_eq!(ranking.unranked[0].reason, UnrankedReason::NoObjective);
}

/// An objective with no bound is listed as unranked with `no_bound`.
///
/// Trace: FR-113-AC-4
#[test]
fn tc_968_007_objective_with_no_bound_is_unranked() {
    let report = report(
        vec![repository_with_row(
            "repo",
            plan(
                "MP-nobound",
                Some(Objective::new(Direction::Higher, None).unwrap()),
            ),
            0.5,
            "2026-01-01T00:00:00.000Z",
        )],
        "2026-01-01T00:00:00.000Z",
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert!(ranking.ranked.is_empty());
    assert_eq!(ranking.unranked[0].reason, UnrankedReason::NoBound);
}

/// A row with no usable current value is unranked with `no_current_estimate`,
/// never scored as though its gap were zero.
///
/// Trace: FR-113-AC-4
#[test]
fn tc_968_008_no_current_estimate_is_unranked() {
    let mut repository = repository_with_row(
        "repo",
        plan(
            "MP-noval",
            Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
        ),
        0.5,
        "2026-01-01T00:00:00.000Z",
    );
    repository.measurements.as_mut().unwrap().current[0].observation = None;
    let report = report(vec![repository], "2026-01-01T00:00:00.000Z");
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert!(ranking.ranked.is_empty());
    assert_eq!(
        ranking.unranked[0].reason,
        UnrankedReason::NoCurrentEstimate
    );
}

/// An empty portfolio ranks and lists nothing, and both rendered forms still
/// carry the ranking and its advisory sentence.
///
/// Trace: FR-113-AC-5
#[test]
fn tc_968_009_empty_portfolio_ranks_nothing() {
    let empty = PortfolioReport {
        newest_collection_timestamp: None,
        repositories: Vec::new(),
    };
    assert_eq!(
        rank_portfolio(&empty).expect("the ranking builds"),
        PortfolioRanking::default()
    );

    let text = render_portfolio_report(&empty).expect("the portfolio renders");
    assert!(text.contains("## Priority ranking"), "{text}");
    assert!(text.contains(RANKING_ADVISORY_NOTE), "{text}");

    let json_text = render_portfolio_report_json(&empty).expect("the portfolio JSON renders");
    let parsed: Value = serde_json::from_str(&json_text).expect("the JSON view is JSON");
    assert_eq!(
        parsed["ranking"]["advisory"],
        Value::String(RANKING_ADVISORY_NOTE.to_owned())
    );
    assert_eq!(parsed["ranking"]["ranked"], Value::Array(Vec::new()));
    assert_eq!(parsed["ranking"]["unranked"], Value::Array(Vec::new()));
}

/// The rendered text carries the advisory sentence verbatim, a ranked plan's
/// row and an unranked plan's reason; the JSON carries the same information
/// under `ranking.advisory`, `ranking.ranked` and `ranking.unranked`.
///
/// Trace: FR-113-AC-5
#[test]
fn tc_968_010_rendered_text_and_json_both_state_the_ranking_is_advisory() {
    let ranked = plan(
        "MP-ranked",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );
    let unranked = plan("MP-unranked", None);
    let report = report(
        vec![
            repository_with_row("repo", ranked, 0.1, "2026-01-01T00:00:00.000Z"),
            repository_with_row("repo2", unranked, 0.5, "2026-01-01T00:00:00.000Z"),
        ],
        "2026-01-01T00:00:00.000Z",
    );

    let text = render_portfolio_report(&report).expect("the portfolio renders");
    assert!(text.contains("## Priority ranking"), "{text}");
    assert!(text.contains(RANKING_ADVISORY_NOTE), "{text}");
    assert!(text.contains("MP-ranked"), "{text}");
    assert!(text.contains("MP-unranked"), "{text}");
    assert!(text.contains("no_objective"), "{text}");

    let json_text = render_portfolio_report_json(&report).expect("the portfolio JSON renders");
    let parsed: Value = serde_json::from_str(&json_text).expect("the JSON view is JSON");
    assert_eq!(
        parsed["ranking"]["advisory"],
        Value::String(RANKING_ADVISORY_NOTE.to_owned())
    );
    assert_eq!(
        parsed["ranking"]["ranked"][0]["planId"],
        Value::String("MP-ranked".to_owned())
    );
    assert_eq!(
        parsed["ranking"]["unranked"][0]["planId"],
        Value::String("MP-unranked".to_owned())
    );
    assert_eq!(
        parsed["ranking"]["unranked"][0]["reason"],
        Value::String("no_objective".to_owned())
    );
}

/// The gap is read in the objective's direction: a `higher` plan past its
/// bound, a `lower` plan under its bound and a `zero` plan inside its bound
/// have no gap and sort behind a plan that still falls short, however far
/// they cleared the bound; overshooting a `target` still counts as a gap.
///
/// Trace: FR-113-AC-1
#[test]
fn tc_968_011_a_met_bound_has_no_gap_in_the_objectives_direction() {
    let objective = |direction, bound| Some(Objective::new(direction, Some(bound)).unwrap());
    let at = "2026-01-01T00:00:00.000Z";
    let report = report(
        vec![
            repository_with_row(
                "r1",
                plan("MP-higher-met", objective(Direction::Higher, 0.5)),
                5.0,
                at,
            ),
            repository_with_row(
                "r2",
                plan("MP-lower-met", objective(Direction::Lower, 10.0)),
                1.0,
                at,
            ),
            repository_with_row(
                "r3",
                plan("MP-zero-met", objective(Direction::Zero, 2.0)),
                -1.5,
                at,
            ),
            repository_with_row(
                "r4",
                plan("MP-short", objective(Direction::Higher, 0.9)),
                0.8,
                at,
            ),
            repository_with_row(
                "r5",
                plan("MP-target-over", objective(Direction::Target, 0.5)),
                0.75,
                at,
            ),
            repository_with_row(
                "r6",
                plan("MP-zero-short", objective(Direction::Zero, 0.0)),
                -0.3,
                at,
            ),
        ],
        at,
    );
    let ranking = rank_portfolio(&report).expect("the ranking builds");
    let order: Vec<&str> = ranking
        .ranked
        .iter()
        .map(|entry| entry.plan_id.as_str())
        .collect();
    assert_eq!(
        order,
        [
            "MP-zero-short",
            "MP-target-over",
            "MP-short",
            "MP-higher-met",
            "MP-lower-met",
            "MP-zero-met"
        ]
    );
    let gap = |id: &str| {
        ranking
            .ranked
            .iter()
            .find(|entry| entry.plan_id == id)
            .unwrap()
            .gap
    };
    assert_eq!(gap("MP-higher-met"), 0.0);
    assert_eq!(gap("MP-lower-met"), 0.0);
    assert_eq!(gap("MP-zero-met"), 0.0);
    assert_eq!(gap("MP-target-over"), 0.25);
    assert_eq!(gap("MP-zero-short"), 0.3);
    assert!((gap("MP-short") - 0.1).abs() < 1e-9);
}

/// One repository whose one plan, `MP-multi` (higher-is-better, bound
/// `0.9`), has one row per `(quantity, value)` slice; a `None` value leaves
/// that slice with no usable estimate.
fn multi_slice_report(slices: &[(&str, Option<f64>)]) -> PortfolioReport {
    let sliced_plan = plan(
        "MP-multi",
        Some(Objective::new(Direction::Higher, Some(0.9)).unwrap()),
    );

    let sliced_row = |quantity: &str, value: Option<f64>| {
        let mut observation = observation(sliced_plan.id.as_str(), 0.0);
        observation.value = value;
        observation.dimensions = Dimensions::stated(BTreeMap::from([(
            "quantity".to_owned(),
            JsonValue::string(quantity),
        )]));
        CurrentRow {
            metric: "quality.score".to_owned(),
            plan_id: sliced_plan.id.as_str().to_owned(),
            plan_path: sliced_plan.path.clone(),
            plan_definition_version: sliced_plan.definition_version.as_str().to_owned(),
            stage: sliced_plan.stage,
            plan_ground_truth_kind: None::<GroundTruthKind>,
            observation: Some(observation),
            collection: Some(collection("2026-01-01T00:00:00.000Z")),
            stage_verdict: None,
        }
    };

    let rows = slices
        .iter()
        .map(|(quantity, value)| sliced_row(quantity, *value))
        .collect();
    let repository = PortfolioRepositoryReport {
        name: "repo".to_owned(),
        root: "/repos/repo".to_owned(),
        status: RepositoryStatus::Readable,
        store: StoreState::Present,
        profiles: Vec::new(),
        plans: vec![sliced_plan],
        measurements: Some(MeasurementReport {
            plans: Vec::new(),
            current: rows,
            vanished_slices: Vec::new(),
            corpus_gaps: None,
            interventions: Vec::new(),
            operational: Vec::new(),
        }),
        latest_collection: None,
        comparison: None,
        staleness: Staleness::NotComputed,
    };
    report(vec![repository], "2026-01-01T00:00:00.000Z")
}

/// A plan tracking dimension-sliced quantities (PLAT-968's "tracking two
/// related quantities" pattern, e.g. `dimensions.quantity: cost` vs
/// `dimensions.quantity: latency`) produces one ranking row per slice, and
/// both rendered forms must let a reader tell the rows apart — the exact gap
/// PLAT-1019 found: neither row named its slice.
///
/// Two slices are scorable and a third has no usable value, so the label is
/// checked on both `ranked` and `unranked`. Text assertions read only the
/// "Priority ranking" section: the per-repository table above it already
/// prints each row's sliced label, so a whole-document `contains` would pass
/// even if the ranking still printed the bare metric.
///
/// Trace: FR-113-AC-6
/// Provenance: PLAT-1019
#[test]
fn tc_1019_multi_slice_rows_carry_distinct_dimension_labels() {
    let report = multi_slice_report(&[
        ("cost", Some(0.1)),
        ("latency", Some(0.2)),
        ("memory", None),
    ]);

    let ranking = rank_portfolio(&report).expect("the ranking builds");
    assert!(
        ranking
            .ranked
            .iter()
            .all(|entry| entry.plan_id == "MP-multi" && entry.metric == "quality.score"),
        "every row shares plan and bare metric -- only the label tells them apart"
    );
    // Higher-is-better toward 0.9: cost (0.1) is further short than latency
    // (0.2), so it ranks first.
    let ranked: Vec<&str> = ranking
        .ranked
        .iter()
        .map(|entry| entry.label.as_str())
        .collect();
    assert_eq!(
        ranked,
        [
            "quality.score [quantity=cost]",
            "quality.score [quantity=latency]"
        ]
    );
    assert_eq!(ranking.unranked.len(), 1);
    assert_eq!(ranking.unranked[0].label, "quality.score [quantity=memory]");
    assert_eq!(
        ranking.unranked[0].reason,
        UnrankedReason::NoCurrentEstimate
    );

    let text = render_portfolio_report(&report).expect("the portfolio renders");
    let (_, section) = text
        .split_once("## Priority ranking")
        .expect("the ranking section is rendered");
    assert!(
        section.contains(
            "| repo | MP-multi (spec/assurance/MP-multi.md) | quality.score [quantity=cost] |"
        ),
        "{section}"
    );
    assert!(
        section.contains(
            "| repo | MP-multi (spec/assurance/MP-multi.md) | quality.score [quantity=latency] |"
        ),
        "{section}"
    );
    assert!(
        section.contains(
            "- repo — MP-multi (spec/assurance/MP-multi.md) — quality.score [quantity=memory]: \
             no_current_estimate"
        ),
        "{section}"
    );

    let json_text = render_portfolio_report_json(&report).expect("the portfolio JSON renders");
    let parsed: Value = serde_json::from_str(&json_text).expect("the JSON view is JSON");
    let labels_of = |member: &str| -> Vec<String> {
        parsed["ranking"][member]
            .as_array()
            .expect("the member is an array")
            .iter()
            .map(|entry| {
                entry["label"]
                    .as_str()
                    .expect("label is a string")
                    .to_owned()
            })
            .collect()
    };
    assert_eq!(
        labels_of("ranked"),
        [
            "quality.score [quantity=cost]",
            "quality.score [quantity=latency]"
        ]
    );
    assert_eq!(labels_of("unranked"), ["quality.score [quantity=memory]"]);
}

/// A dimension value is authored text: a `|` would split the ranking table's
/// row into extra columns and a CR/LF would end the row or bullet early. The
/// text form escapes them (and `\\`, so the escapes stay unambiguous) while
/// the JSON `label` keeps the authored bytes.
///
/// Trace: FR-113-AC-6
/// Provenance: PLAT-1019
#[test]
fn tc_1019_ranking_text_escapes_label_characters_that_break_markdown() {
    let report = multi_slice_report(&[("type|script", Some(0.1)), ("a\\b\r\nc", None)]);

    let text = render_portfolio_report(&report).expect("the portfolio renders");
    let (_, section) = text
        .split_once("## Priority ranking")
        .expect("the ranking section is rendered");
    assert!(
        section.contains("| quality.score [quantity=type\\|script] |"),
        "{section}"
    );
    assert!(
        section.contains("quality.score [quantity=a\\\\b\\r\\nc]: no_current_estimate"),
        "{section}"
    );

    let json_text = render_portfolio_report_json(&report).expect("the portfolio JSON renders");
    let parsed: Value = serde_json::from_str(&json_text).expect("the JSON view is JSON");
    assert_eq!(
        parsed["ranking"]["ranked"][0]["label"],
        "quality.score [quantity=type|script]"
    );
    assert_eq!(
        parsed["ranking"]["unranked"][0]["label"],
        "quality.score [quantity=a\\b\r\nc]"
    );
}
