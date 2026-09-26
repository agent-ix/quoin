// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! EA-26: an observation's `interval` decides a plan's rule at its
//! unfavourable bound, in `quoin measurement verify` (FR-108-AC-11..AC-14)
//! and in a `gate` plan's report verdict (FR-107-AC-10, FR-107-AC-11).
//!
//! Plans are real assurance documents loaded through `load_measurement_plans`
//! (so `decision_rule.interval_level` and `margin_mode` are read by the real
//! parser into engineering-assurance's `DecisionRule`); collections are the
//! `verify` fixtures' accepted collection, edited per case. Every run states
//! the same apparatus, so a regressed run followed by a pass is a rerun.
//!
//! Provenance: EA-26

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::float_cmp,
    reason = "the decision's bound and level are stated values, compared exactly"
)]

use std::path::{Path, PathBuf};

use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::interval::{Bound, Decided};
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::report::{GateOutcome, InconclusiveReason, StageVerdict, stage_verdict};
use quoin_measurement::source::MemoryMeasurement;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::verify::{EstimateBasis, MeasurementVerdict};
use quoin_measurement::{
    OrderSource, Ranked, Reason, TamperFacts, Verdict, stored_measurement_collection, verdict_json,
    verify,
};
use quoin_store::parse_strict_json;
use serde_json::{Value, json};

const FIXTURES: &str = "tests/fixtures/verify";

/// A gate plan for `gate.pass_rate` whose `decision_rule` is `rule` (each
/// line already indented as YAML needs) under an objective of `direction`.
fn plan_document(rule: &str, direction: &str) -> String {
    format!(
        "---\n\
         id: MP-961\n\
         title: Interval gate\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: gate\n\
         metric: gate.pass_rate\n\
         definition_version: gate.pass-rate-v1\n\
         ground_truth_kind: mechanical\n\
         objective:\n\
         \x20\x20direction: {direction}\n\
         statistical_design:\n\
         \x20\x20population: every case\n\
         \x20\x20sampling: exhaustive\n\
         \x20\x20repetitions: 1\n\
         \x20\x20estimator: proportion\n\
         \x20\x20error_model: none\n\
         \x20\x20uncertainty: stated by the producer\n\
         \x20\x20decision_rule:\n\
         {rule}\
         protected_apparatus:\n\
         \x20\x20- spec/assurance/MP-961-gate.md\n\
         negative_controls:\n\
         \x20\x20- kind: apparatus-edit\n\
         \x20\x20\x20\x20description: the plan document is digested with every collection\n\
         ---\n\
         \n\
         # Interval gate\n"
    )
}

fn load(
    rule: &str,
    direction: &str,
) -> Result<MeasurementPlan, quoin_measurement::MeasurementError> {
    let source = MemoryMeasurement::new()
        .with_document("spec/assurance/MP-961.md", plan_document(rule, direction));
    load_measurement_plans(&source, PlanLoadOptions::default()).map(|mut plans| plans.remove(0))
}

/// `ge 0.8` judged at `interval_level` 0.95.
fn threshold_plan() -> MeasurementPlan {
    load(
        "\x20\x20\x20\x20comparator: ge\n\x20\x20\x20\x20threshold: 0.8\n\x20\x20\x20\x20interval_level: 0.95\n",
        "higher",
    )
    .expect("the plan loads")
}

/// `ge` the prior collection's estimate, judged at `interval_level` 0.95.
fn baseline_plan() -> MeasurementPlan {
    load(
        "\x20\x20\x20\x20comparator: ge\n\x20\x20\x20\x20baseline: prior-collection\n\x20\x20\x20\x20interval_level: 0.95\n",
        "higher",
    )
    .expect("the plan loads")
}

fn text(value: &str) -> NonEmptyText {
    NonEmptyText::parse(value, MeasurementErrorCode::CollectionInvalid, "x").unwrap()
}

/// The `verify` fixtures' accepted collection: one `gate.pass_rate`
/// observation, 9 of 10, value 0.9, the apparatus recorded.
fn accepted() -> MeasurementCollection {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES)
        .join("right/accept/1.json");
    let bytes = std::fs::read(path).unwrap();
    stored_measurement_collection(&parse_strict_json(&bytes).unwrap()).unwrap()
}

/// One run: `id`, the stored `value` over `matched` of `examined`, and the
/// stated `interval`. Every run shares the fixture's apparatus.
fn run(
    id: &str,
    value: f64,
    (matched, examined): (f64, f64),
    interval: Option<Value>,
) -> MeasurementCollection {
    let mut collection = accepted();
    collection.collection_id = text(id);
    let observation = &mut collection.observations[0];
    observation.value = Some(value);
    let population = observation.population.as_mut().unwrap();
    population.matched = Some(matched);
    population.examined = Some(examined);
    observation.interval = interval.map(|stated| from_serde(&stated).unwrap());
    collection
}

/// A stated interval of `method` around a value, at `level`.
fn stated(lower: f64, upper: f64, level: f64) -> Value {
    json!({ "lower": lower, "upper": upper, "level": level, "method": "wilson" })
}

fn decided(plan: &MeasurementPlan, runs: &[MeasurementCollection]) -> MeasurementVerdict {
    let ranked: Vec<Ranked<'_>> = (0_u64..)
        .zip(runs)
        .map(|(intake, collection)| Ranked::new(collection, Some(intake)))
        .collect();
    verify(
        plan,
        &ranked,
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    )
}

/// Under a plan with `interval_level`, a `gt`/`ge` rule is judged at the
/// interval's lower bound and a `lt`/`le` rule at its upper bound, for a
/// threshold and a baseline rule and for a recomputed `proportion`; a point
/// estimate that passes with a failing unfavourable bound is `reject` with
/// `rule_not_met`, and the `decisions` entry carries the three interval
/// members.
///
/// Trace: FR-108-AC-11, TC-1954
/// Provenance: EA-26
#[test]
fn tc_1954_the_rule_is_judged_at_the_unfavourable_bound() {
    let gate = threshold_plan();

    // `ge 0.8`: the point estimate 0.9 passes; a lower bound of 0.75 does not.
    let failing = decided(
        &gate,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.75, 0.99, 0.95)))],
    );
    assert_eq!(failing.verdict, Verdict::Reject);
    assert_eq!(failing.reasons, [Reason::RuleNotMet]);
    let decision = &failing.decisions[0];
    assert_eq!(decision.estimate.basis, EstimateBasis::Recomputed);
    assert_eq!(decision.holds, Some(false));
    assert_eq!(
        decision.interval,
        Some(Decided {
            holds: false,
            bound: Bound::Lower,
            bound_value: 0.75,
            level: 0.95
        })
    );
    let wire = verdict_json(&failing).unwrap();
    assert_eq!(wire["decisions"][0]["intervalBound"], json!("lower"));
    assert_eq!(wire["decisions"][0]["intervalBoundValue"], json!(0.75));
    assert_eq!(wire["decisions"][0]["intervalLevel"], json!(0.95));

    let passing = decided(
        &gate,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.95)))],
    );
    assert_eq!(passing.verdict, Verdict::Accept);
    assert_eq!(passing.decisions[0].interval.unwrap().bound_value, 0.85);
    // A wider (higher-level) interval is only more conservative, and admitted.
    let higher_level = decided(
        &gate,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.99)))],
    );
    assert_eq!(higher_level.verdict, Verdict::Accept);
    assert_eq!(higher_level.decisions[0].interval.unwrap().level, 0.99);

    // `le 0.95` under an objective of `lower`: judged at the upper bound.
    let cap = load(
        "\x20\x20\x20\x20comparator: le\n\x20\x20\x20\x20threshold: 0.95\n\x20\x20\x20\x20interval_level: 0.95\n",
        "lower",
    )
    .expect("the plan loads");
    let over = decided(
        &cap,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.8, 0.99, 0.95)))],
    );
    assert_eq!(over.reasons, [Reason::RuleNotMet]);
    assert_eq!(over.decisions[0].interval.unwrap().bound, Bound::Upper);
    assert_eq!(over.decisions[0].interval.unwrap().bound_value, 0.99);
    let under = decided(
        &cap,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.8, 0.93, 0.95)))],
    );
    assert_eq!(under.verdict, Verdict::Accept);

    // A baseline rule: the baseline is the earlier run's POINT estimate 0.9.
    let baseline = baseline_plan();
    let held = decided(
        &baseline,
        &[
            run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.95, 0.95))),
            run("b", 0.9, (9.0, 10.0), Some(stated(0.9, 0.95, 0.95))),
        ],
    );
    assert_eq!(held.verdict, Verdict::Accept, "{:?}", held.findings);
    assert_eq!(held.decisions[0].baseline, Some(0.9));
    let slipped = decided(
        &baseline,
        &[
            run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.95, 0.95))),
            run("b", 0.9, (9.0, 10.0), Some(stated(0.7, 0.95, 0.95))),
        ],
    );
    assert_eq!(slipped.reasons, [Reason::RuleNotMet]);
    assert_eq!(slipped.decisions[0].baseline, Some(0.9));
    assert_eq!(slipped.decisions[0].interval.unwrap().bound_value, 0.7);
}

/// The verdict document for a plan with no `interval_level` is unchanged, its
/// `decisions` entries carrying no interval member, even when the candidate
/// states an `interval`.
///
/// Trace: FR-108-AC-11, TC-1955
/// Provenance: EA-26
#[test]
fn tc_1955_a_plan_without_an_interval_level_is_byte_identical() {
    let plain = load(
        "\x20\x20\x20\x20comparator: ge\n\x20\x20\x20\x20threshold: 0.8\n",
        "higher",
    )
    .expect("the plan loads");
    let without = verdict_json(&decided(&plain, &[run("a", 0.9, (9.0, 10.0), None)])).unwrap();
    // The interval's lower bound would fail the rule; it is never read.
    let with = verdict_json(&decided(
        &plain,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.5, 0.99, 0.95)))],
    ))
    .unwrap();
    assert_eq!(with, without);
    assert_eq!(without["verdict"], json!("accept"));
    let mut members: Vec<&str> = without["decisions"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    members.sort_unstable();
    assert_eq!(
        members,
        [
            "baseline",
            "dimensions",
            "estimate",
            "estimateBasis",
            "holds"
        ]
    );
    // Even a malformed interval is not a finding when the rule states no level.
    let malformed = decided(&plain, &[run("a", 0.9, (9.0, 10.0), Some(json!("wide")))]);
    assert_eq!(malformed.verdict, Verdict::Accept);
}

/// Under an `interval_level` plan every run is judged on its own interval:
/// run 1 failing at its lower bound, the same apparatus re-run, run 2 passing
/// is still `rerun_until_pass`; an earlier run with no interval, a short
/// level or an estimate outside its interval is regressed; changing only an
/// interval's `method` changes nothing.
///
/// Trace: FR-108-AC-11, TC-1956
/// Provenance: EA-26
#[test]
fn tc_1956_every_run_is_judged_on_its_own_interval() {
    let gate = threshold_plan();
    let passing = || run("run-2", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.95)));

    // Control: an earlier run that passes on its interval is no rerun.
    let clean = decided(
        &gate,
        &[
            run("run-1", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.95))),
            passing(),
        ],
    );
    assert_eq!(clean.verdict, Verdict::Accept);
    assert_eq!(clean.counts.regressed_runs, 0);

    // Run 1 passes on its point estimate 0.9 but fails at its lower bound.
    for (why, earlier) in [
        (
            "fails at its lower bound",
            run("run-1", 0.9, (9.0, 10.0), Some(stated(0.7, 0.99, 0.95))),
        ),
        ("states no interval", run("run-1", 0.9, (9.0, 10.0), None)),
        (
            "states a level below the plan's",
            run("run-1", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.9))),
        ),
        (
            // Stored 0.67 sits inside [0.67, 0.9]; the recomputed 200/300 does not.
            "has an estimate outside its interval",
            run("run-1", 0.67, (200.0, 300.0), Some(stated(0.67, 0.9, 0.95))),
        ),
    ] {
        let verdict = decided(&gate, &[earlier, passing()]);
        assert_eq!(verdict.verdict, Verdict::Reject, "{why}");
        assert_eq!(verdict.reasons, [Reason::RerunUntilPass], "{why}");
        assert_eq!(verdict.regressed_runs, ["run-1"], "{why}");
        assert_eq!(verdict.counts.regressed_runs, 1, "{why}");
    }

    // A malformed earlier interval adds its own finding as well.
    let verdict = decided(
        &gate,
        &[
            run("run-1", 0.9, (9.0, 10.0), Some(json!("wide"))),
            passing(),
        ],
    );
    assert_eq!(
        verdict.reasons,
        [Reason::IntervalMalformed, Reason::RerunUntilPass]
    );
    assert_eq!(verdict.regressed_runs, ["run-1"]);
    let finding = verdict
        .findings
        .iter()
        .find(|finding| finding.reason == Reason::IntervalMalformed)
        .unwrap();
    assert_eq!(finding.collection_id.as_deref(), Some("run-1"));

    // `method` is never read.
    let reworded = |method: &str| {
        let mut interval = stated(0.7, 0.99, 0.95);
        interval["method"] = json!(method);
        decided(
            &gate,
            &[run("run-1", 0.9, (9.0, 10.0), Some(interval)), passing()],
        )
    };
    assert_eq!(reworded("wilson"), reworded("a different note"));
}

/// Under an `interval_level` plan: no interval is `inconclusive`
/// `interval_unstated`; a stored malformed interval, in the candidate or an
/// earlier run, is `reject` `interval_malformed`; a short level is `reject`
/// `interval_level_short`; stored 0.67 with `lower` 0.67 and recomputed 2/3 is
/// `inconclusive` `rule_not_evaluable`; a first candidate under a baseline
/// rule with no interval lists both `interval_unstated` and `no_prior`; the
/// point estimate is never used instead.
///
/// Trace: FR-108-AC-12, TC-1957
/// Provenance: EA-26
#[test]
fn tc_1957_a_missing_malformed_or_short_interval_is_never_judged_on_the_point_estimate() {
    let gate = threshold_plan();

    // The point estimate 0.9 passes `ge 0.8`, and is not what decides.
    let unstated = decided(&gate, &[run("a", 0.9, (9.0, 10.0), None)]);
    assert_eq!(unstated.verdict, Verdict::Inconclusive);
    assert_eq!(unstated.reasons, [Reason::IntervalUnstated]);
    assert_eq!(unstated.decisions[0].holds, None);
    assert_eq!(unstated.decisions[0].interval, None);
    let wire = verdict_json(&unstated).unwrap();
    assert!(wire["decisions"][0].get("intervalBound").is_none());

    let malformed = decided(&gate, &[run("a", 0.9, (9.0, 10.0), Some(json!("wide")))]);
    assert_eq!(malformed.verdict, Verdict::Reject);
    assert_eq!(malformed.reasons, [Reason::IntervalMalformed]);
    // A value outside its own bounds is malformed too (FR-044-AC-10).
    let outside = decided(
        &gate,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.91, 0.99, 0.95)))],
    );
    assert_eq!(outside.reasons, [Reason::IntervalMalformed]);

    let short = decided(
        &gate,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.9)))],
    );
    assert_eq!(short.verdict, Verdict::Reject);
    assert_eq!(short.reasons, [Reason::IntervalLevelShort]);

    // A malformed earlier interval rejects, and is named in that run.
    let earlier = decided(
        &gate,
        &[
            run("run-1", 0.9, (9.0, 10.0), Some(json!({ "lower": 0.8 }))),
            run("run-2", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.95))),
        ],
    );
    assert_eq!(earlier.verdict, Verdict::Reject);
    assert!(earlier.reasons.contains(&Reason::IntervalMalformed));

    // The rounded stored value 0.67 is inside [0.67, 0.9]; the recomputed
    // 200/300 is not, so the rule cannot be evaluated.
    let rounded = decided(
        &gate,
        &[run(
            "a",
            0.67,
            (200.0, 300.0),
            Some(stated(0.67, 0.9, 0.95)),
        )],
    );
    assert_eq!(rounded.verdict, Verdict::Inconclusive);
    assert_eq!(rounded.reasons, [Reason::RuleNotEvaluable]);
    assert_eq!(rounded.decisions[0].holds, None);

    // The interval requirement does not wait on the baseline.
    let first = decided(&baseline_plan(), &[run("a", 0.9, (9.0, 10.0), None)]);
    assert_eq!(first.verdict, Verdict::Inconclusive);
    assert_eq!(first.reasons, [Reason::NoPrior, Reason::IntervalUnstated]);
}

/// A plan whose rule states `interval_level` with `eq`, at 0, 1, above 1,
/// non-numeric or null refuses the plan load as `QM-PLAN-INVALID` naming the
/// member; a valid `interval_level` loads.
///
/// Trace: FR-108-AC-12, TC-1958
/// Provenance: EA-26
#[test]
fn tc_1958_a_bad_interval_level_refuses_the_plan_load() {
    for (why, rule) in [
        (
            "eq",
            "comparator: eq\n    threshold: 0.8\n    interval_level: 0.95\n",
        ),
        (
            "zero",
            "comparator: ge\n    threshold: 0.8\n    interval_level: 0\n",
        ),
        (
            "one",
            "comparator: ge\n    threshold: 0.8\n    interval_level: 1\n",
        ),
        (
            "above one",
            "comparator: ge\n    threshold: 0.8\n    interval_level: 1.5\n",
        ),
        (
            "non-numeric",
            "comparator: ge\n    threshold: 0.8\n    interval_level: high\n",
        ),
        (
            "tilde",
            "comparator: ge\n    threshold: 0.8\n    interval_level: ~\n",
        ),
        (
            "null",
            "comparator: ge\n    threshold: 0.8\n    interval_level: null\n",
        ),
        (
            "bare",
            "comparator: ge\n    threshold: 0.8\n    interval_level:\n",
        ),
    ] {
        let error = load(&format!("    {rule}"), "higher").expect_err(why);
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid, "{why}");
        assert!(
            error
                .to_string()
                .contains("statistical_design.decision_rule"),
            "{why}: {error}"
        );
    }
    let plan = threshold_plan();
    let level = plan
        .statistical_design
        .unwrap()
        .decision_rule
        .unwrap()
        .interval_level()
        .unwrap();
    assert_eq!(level.get(), 0.95);
}

/// A plan with `margin_mode: relative` gives a different verify verdict from
/// the same-numbers absolute margin, and `absolute` or no `margin_mode`
/// decides as before.
///
/// Trace: FR-108-AC-14, TC-1960
/// Provenance: EA-26
#[test]
fn tc_1960_a_relative_margin_is_applied_in_verify() {
    // `ge` the prior estimate less a margin of 0.05. Run 1 is 0.5 and run 2
    // 0.46: absolute, the floor is 0.45 and 0.46 holds; relative, it is
    // 0.5 - 0.05 * 0.5 = 0.475 and 0.46 does not.
    let rule = |mode: &str| {
        format!("    comparator: ge\n    baseline: prior-collection\n    margin: -0.05\n{mode}")
    };
    let runs = [
        run("a", 0.5, (50.0, 100.0), None),
        run("b", 0.46, (46.0, 100.0), None),
    ];
    let judged = |mode: &str| decided(&load(&rule(mode), "higher").expect("the plan loads"), &runs);
    let absolute = judged("");
    assert_eq!(absolute.verdict, Verdict::Accept, "{:?}", absolute.findings);
    assert_eq!(absolute.decisions[0].holds, Some(true));
    assert_eq!(
        judged("    margin_mode: absolute\n").decisions[0].holds,
        Some(true)
    );
    let relative = judged("    margin_mode: relative\n");
    assert_eq!(relative.verdict, Verdict::Reject);
    assert_eq!(relative.reasons, [Reason::RuleNotMet]);
    assert_eq!(relative.decisions[0].holds, Some(false));
}

/// The newest observation of a gate plan, the runs before it, and the plan's
/// stage verdict over them.
fn gate_outcome(plan: &MeasurementPlan, runs: &[MeasurementCollection]) -> GateOutcome {
    let (newest, earlier) = runs.split_last().expect("at least one run");
    match stage_verdict(plan, newest.observations.first(), Some(newest), earlier) {
        Some(StageVerdict::Gate { outcome, .. }) => outcome,
        other => panic!("a gate plan states a gate verdict, not {other:?}"),
    }
}

/// A `gate` plan with `interval_level` is decided by `holds_on_interval` on
/// the newest observation's interval: an unfavourable bound that fails is
/// `fail` while the point estimate passes; no interval is `inconclusive`
/// `interval_unstated`, a short level `interval_level_short`, an estimate
/// outside `rule_not_evaluable`, also under a baseline rule with no prior;
/// the row carries the three interval members, and rows for plans without
/// `interval_level` are byte-identical.
///
/// Trace: FR-107-AC-10, TC-1961
/// Provenance: EA-26
#[test]
fn tc_1961_a_gate_is_decided_on_the_newest_observations_interval() {
    let gate = threshold_plan();

    let fail = gate_outcome(
        &gate,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.75, 0.99, 0.95)))],
    );
    let bound = Decided {
        holds: false,
        bound: Bound::Lower,
        bound_value: 0.75,
        level: 0.95,
    };
    assert_eq!(
        fail,
        GateOutcome::Fail {
            current: 0.9,
            baseline: None,
            interval: Some(bound)
        }
    );
    let pass = gate_outcome(
        &gate,
        &[run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.95)))],
    );
    assert!(matches!(
        pass,
        GateOutcome::Pass {
            interval: Some(Decided {
                bound: Bound::Lower,
                ..
            }),
            ..
        }
    ));

    let inconclusive = |reason| GateOutcome::Inconclusive(reason);
    for (plan, runs, reason) in [
        (
            &gate,
            vec![run("a", 0.9, (9.0, 10.0), None)],
            InconclusiveReason::IntervalUnstated,
        ),
        (
            // Malformed reads as no valid interval; rejection is verify's.
            &gate,
            vec![run("a", 0.9, (9.0, 10.0), Some(json!("wide")))],
            InconclusiveReason::IntervalUnstated,
        ),
        (
            &gate,
            vec![run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.9)))],
            InconclusiveReason::IntervalLevelShort,
        ),
        (
            // A baseline rule with no prior: the missing interval is still
            // what is reported, not `no_prior`.
            &baseline_plan(),
            vec![run("a", 0.9, (9.0, 10.0), None)],
            InconclusiveReason::IntervalUnstated,
        ),
    ] {
        assert_eq!(
            gate_outcome(plan, &runs),
            inconclusive(reason),
            "{reason:?}"
        );
    }
}

/// The gate's interval check comes before its baseline: a baseline rule is
/// decided on the newest interval against the earlier point estimate, a first
/// collection with a valid interval and no prior is `no_prior`, and a plan
/// without `interval_level` decides on the point estimate with no interval on
/// its outcome.
///
/// Trace: FR-107-AC-10, TC-1961
/// Provenance: EA-26
#[test]
fn tc_1961_a_gate_baseline_rule_reads_the_interval_before_the_baseline() {
    let inconclusive = |reason| GateOutcome::Inconclusive(reason);
    // The interval is judged before the baseline: a first collection under a
    // baseline rule with no prior and a valid interval is `no_prior`.
    assert_eq!(
        gate_outcome(
            &baseline_plan(),
            &[run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.99, 0.95)))]
        ),
        inconclusive(InconclusiveReason::NoPrior)
    );

    // A baseline rule decided on the newest interval against the earlier
    // point estimate, and the row carries the bound.
    let baseline = gate_outcome(
        &baseline_plan(),
        &[
            run("a", 0.9, (9.0, 10.0), Some(stated(0.85, 0.95, 0.95))),
            run("b", 0.9, (9.0, 10.0), Some(stated(0.7, 0.95, 0.95))),
        ],
    );
    assert_eq!(
        baseline,
        GateOutcome::Fail {
            current: 0.9,
            baseline: Some(0.9),
            interval: Some(Decided {
                holds: false,
                bound: Bound::Lower,
                bound_value: 0.7,
                level: 0.95
            })
        }
    );

    // A plan without `interval_level` decides on the point estimate and its
    // outcome carries no interval, whatever the observation states.
    let plain = load(
        "\x20\x20\x20\x20comparator: ge\n\x20\x20\x20\x20threshold: 0.8\n",
        "higher",
    )
    .expect("the plan loads");
    assert_eq!(
        gate_outcome(
            &plain,
            &[run("a", 0.9, (9.0, 10.0), Some(stated(0.5, 0.99, 0.95)))]
        ),
        GateOutcome::Pass {
            current: 0.9,
            baseline: None,
            interval: None
        }
    );
}

/// A `gate` plan whose rule states `margin_mode: relative` is evaluated with
/// the relative reference in the report; `absolute` or an absent mode is
/// unchanged.
///
/// Trace: FR-107-AC-11, TC-1962
/// Provenance: EA-26
#[test]
fn tc_1962_a_relative_margin_is_applied_by_the_gate() {
    let runs = [
        run("a", 0.5, (50.0, 100.0), None),
        run("b", 0.46, (46.0, 100.0), None),
    ];
    let judged = |mode: &str| {
        let plan = load(
            &format!(
                "    comparator: ge\n    baseline: prior-collection\n    margin: -0.05\n{mode}"
            ),
            "higher",
        )
        .expect("the plan loads");
        gate_outcome(&plan, &runs)
    };
    let held = GateOutcome::Pass {
        current: 0.46,
        baseline: Some(0.5),
        interval: None,
    };
    assert_eq!(judged(""), held);
    assert_eq!(judged("    margin_mode: absolute\n"), held);
    assert_eq!(
        judged("    margin_mode: relative\n"),
        GateOutcome::Fail {
            current: 0.46,
            baseline: Some(0.5),
            interval: None
        }
    );
}
