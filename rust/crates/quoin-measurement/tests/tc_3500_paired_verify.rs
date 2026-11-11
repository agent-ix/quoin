// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-132-AC-8 to AC-13: `verify` and the report evaluate a plan that
//! declares `paired_run` over the plan's latest two collections (PLAT-1404).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use ix_trace_rs::trace;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::report::build_measurement_report_from;
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::types::collection::{BuildProfile, MeasurementCollection};
use quoin_measurement::types::observation::MeasurementState;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::{
    MeasurementVerdict, OrderSource, PairedRunOutcome, Ranked, TamperFacts, Verdict,
    render_measurement_report, render_measurement_report_json, validate, verdict_json, verify,
};
use serde_json::{Value, json};

const EQUAL: &str = "paired_run:\n  relation: equal\n  minimum_pairs: 1\n";
const RULE: &str = "statistical_design:\n  population: every case\n  sampling: every case once\n  \
                    repetitions: 1\n  population_exempt: fixture cases carry no ids\n  \
                    error_model: a case is misjudged\n  uncertainty: none estimated\n  \
                    decision_rule:\n    comparator: ge\n    threshold: 0.8\n";

fn plans_staged(stage: &str, extra: &str) -> Vec<MeasurementPlan> {
    let text = format!(
        "---\nid: MP-001\ntitle: Determinism\ntype: MeasurementPlan\nstatus: active\nowner: test\n\
         stage: {stage}\nmetric: quality.example\ndefinition_version: quality.example-v1\n\
         {extra}---\n\n# Determinism\n"
    );
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("creatable");
    std::fs::write(assurance.join("MP-001.md"), text).expect("writable");
    load_measurement_plans(
        &DiskMeasurement::new(temporary.path()),
        PlanLoadOptions::default(),
    )
    .expect("the plan loads")
}

fn plans(extra: &str) -> Vec<MeasurementPlan> {
    plans_staged("branch-comparison", extra)
}

fn run(id: &str, values: &[(&str, f64)]) -> MeasurementCollection {
    let observations: Vec<Value> = values
        .iter()
        .map(|(root, value)| {
            json!({
                "metric": "quality.example", "planId": "MP-001",
                "definitionVersion": "quality.example-v1", "state": "measured",
                "value": value, "unit": "count", "shape": "count",
                "dimensions": { "root": root },
                "population": { "examined": 1, "complete": true },
            })
        })
        .collect();
    let document = json!({
        "schemaVersion": 2, "collectionId": id, "subject": "fixture",
        "scope": { "cases": 1 }, "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1", "timestamp": "2026-08-26T00:00:00.000Z",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"], "artifacts": ["config"],
        },
        "observations": observations, "rawEvidence": { "payload": [1] },
    });
    let stored = from_serde(&document).expect("bridge");
    validate::stored_measurement_collection(&stored).expect("admissible")
}

/// The verdict over `runs`, oldest first, in a caller-supplied intake order.
fn decide(plans: &[MeasurementPlan], runs: &[&MeasurementCollection]) -> MeasurementVerdict {
    let ranked: Vec<Ranked<'_>> = runs
        .iter()
        .zip(0_u64..)
        .map(|(collection, position)| Ranked::new(collection, Some(position)))
        .collect();
    verify(
        &plans[0],
        &ranked,
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    )
}

fn codes(verdict: &MeasurementVerdict) -> Vec<&'static str> {
    verdict
        .reasons
        .iter()
        .map(|reason| reason.as_str())
        .collect()
}

/// Equal runs accept with a zero count; differing runs reject, counting the slices.
#[trace("FR-132-AC-8", "TC-3500")]
#[test]
fn tc_3500_equal_runs_accept_and_differing_runs_reject() {
    let plans = plans(EQUAL);
    let first = run("a", &[("r1", 3.0), ("r2", 4.0)]);
    let same = run("b", &[("r1", 3.0), ("r2", 4.0)]);
    let clean = decide(&plans, &[&first, &same]);
    assert_eq!(clean.verdict, Verdict::Accept);
    assert!(clean.reasons.is_empty());
    assert_eq!(clean.decisions.len(), 1);
    assert!((clean.decisions[0].estimate.value - 0.0).abs() < f64::EPSILON);
    assert_eq!(clean.decisions[0].holds, Some(true));
    let document = verdict_json(&clean).expect("serialises");
    assert_eq!(document["decisions"][0]["estimate"], json!(0.0));

    let moved = run("c", &[("r1", 3.0), ("r2", 9.0)]);
    let differing = decide(&plans, &[&first, &moved]);
    assert_eq!(differing.verdict, Verdict::Reject);
    assert_eq!(codes(&differing), ["paired_run_differs"]);
    assert!((differing.decisions[0].estimate.value - 1.0).abs() < f64::EPSILON);
    assert_eq!(differing.decisions[0].holds, Some(false));
}

/// `within_delta` admits movement inside the band and rejects beyond it.
#[trace("FR-132-AC-8", "TC-3501")]
#[test]
fn tc_3501_within_delta_admits_the_band() {
    let plans = plans("paired_run:\n  relation: within_delta\n  minimum_pairs: 1\n  delta: 0.5\n");
    let first = run("a", &[("r1", 10.0)]);
    assert_eq!(
        decide(&plans, &[&first, &run("b", &[("r1", 10.5)])]).verdict,
        Verdict::Accept
    );
    assert_eq!(
        decide(&plans, &[&first, &run("c", &[("r1", 10.6)])]).verdict,
        Verdict::Reject
    );
}

/// A paired plan with no rule is not `no_decision_rule`; a half-declared
/// rule keeps its reason; a non-paired plan is unchanged.
#[trace("FR-132-AC-9", "TC-3502")]
#[test]
fn tc_3502_the_relation_is_the_decision_of_a_ruleless_paired_plan() {
    let first = run("a", &[("r1", 3.0)]);
    let second = run("b", &[("r1", 3.0)]);
    let ruleless = decide(&plans(EQUAL), &[&first, &second]);
    assert!(codes(&ruleless).is_empty(), "{:?}", codes(&ruleless));

    let rule_only = decide(&plans(&format!("{EQUAL}{RULE}")), &[&first, &second]);
    assert!(codes(&rule_only).contains(&"no_estimator"));
    assert!(!codes(&rule_only).contains(&"no_decision_rule"));
}

/// A plan without `paired_run` is verified exactly as before.
#[trace("FR-132-AC-9", "TC-3503")]
#[test]
fn tc_3503_a_plan_without_paired_run_still_has_no_decision_rule() {
    let first = run("a", &[("r1", 3.0)]);
    let second = run("b", &[("r1", 3.0)]);
    let verdict = decide(&plans(""), &[&first, &second]);
    assert_eq!(verdict.verdict, Verdict::Inconclusive);
    assert_eq!(codes(&verdict), ["no_decision_rule"]);
    assert!(verdict.decisions.is_empty());
}

/// Fewer than two collections is unpaired and states no count.
#[trace("FR-132-AC-10", "TC-3504")]
#[test]
fn tc_3504_fewer_than_two_collections_are_unpaired() {
    let plans = plans(EQUAL);
    let one = run("a", &[("r1", 3.0)]);
    for runs in [vec![&one], vec![]] {
        let verdict = decide(&plans, &runs);
        assert_eq!(verdict.verdict, Verdict::Inconclusive);
        assert_eq!(codes(&verdict), ["paired_run_unpaired"]);
        assert!(verdict.decisions.is_empty());
    }
}

/// With three collections the pair is the latest two.
#[trace("FR-132-AC-10", "TC-3505")]
#[test]
fn tc_3505_the_pair_is_the_latest_two() {
    let plans = plans(EQUAL);
    let old = run("a", &[("r1", 1.0)]);
    let middle = run("b", &[("r1", 3.0)]);
    let newest = run("c", &[("r1", 3.0)]);
    assert_eq!(
        decide(&plans, &[&old, &middle, &newest]).verdict,
        Verdict::Accept
    );
    let drifted = run("d", &[("r1", 4.0)]);
    assert_eq!(
        decide(&plans, &[&old, &middle, &drifted]).verdict,
        Verdict::Reject
    );
}

/// A changed build profile makes the pair incomparable, with no count.
#[trace("FR-132-AC-11", "TC-3506")]
#[test]
fn tc_3506_a_changed_build_profile_is_incomparable() {
    let plans = plans(EQUAL);
    let first = run("a", &[("r1", 3.0)]);
    let mut second = run("b", &[("r1", 3.0)]);
    if let Some(stack) = second.verification_stack.as_mut() {
        stack.build_profile = Some(BuildProfile::Debug);
    }
    let verdict = decide(&plans, &[&first, &second]);
    assert_eq!(verdict.verdict, Verdict::Inconclusive);
    assert_eq!(codes(&verdict), ["paired_run_incomparable"]);
    assert!(verdict.decisions.is_empty());
}

/// Too few comparable slices, or a slice without a value, is inconclusive.
#[trace("FR-132-AC-11", "TC-3507")]
#[test]
fn tc_3507_too_few_comparisons_are_inconclusive() {
    let few = plans("paired_run:\n  relation: equal\n  minimum_pairs: 3\n");
    let first = run("a", &[("r1", 3.0)]);
    let second = run("b", &[("r1", 3.0)]);
    let verdict = decide(&few, &[&first, &second]);
    assert_eq!(codes(&verdict), ["paired_run_inconclusive"]);
    assert!(verdict.decisions.is_empty());

    let plans = plans(EQUAL);
    let mut valueless = run("c", &[("r1", 3.0), ("r2", 4.0)]);
    valueless.observations[1].state = MeasurementState::NotComputed;
    valueless.observations[1].value = None;
    let full = run("d", &[("r1", 3.0), ("r2", 4.0)]);
    let verdict = decide(&plans, &[&full, &valueless]);
    assert_eq!(codes(&verdict), ["paired_run_inconclusive"]);
}

/// A slice only one collection states is a violation.
#[trace("FR-132-AC-11", "TC-3508")]
#[test]
fn tc_3508_a_one_sided_slice_rejects() {
    let plans = plans(EQUAL);
    let first = run("a", &[("r1", 3.0), ("r2", 4.0)]);
    let shorter = run("b", &[("r1", 3.0)]);
    let verdict = decide(&plans, &[&first, &shorter]);
    assert_eq!(verdict.verdict, Verdict::Reject);
    assert_eq!(codes(&verdict), ["paired_run_differs"]);
}

/// Order only matters for `not_worse` or more than two collections.
#[trace("FR-132-AC-12", "TC-3509")]
#[test]
fn tc_3509_an_unattested_order_matters_only_when_it_can_change_the_answer() {
    let first = run("a", &[("r1", 3.0)]);
    let second = run("b", &[("r1", 3.0)]);
    let third = run("c", &[("r1", 3.0)]);
    let unordered = |plans: &[MeasurementPlan], runs: &[&MeasurementCollection]| {
        let ranked: Vec<Ranked<'_>> = runs
            .iter()
            .map(|collection| Ranked::new(collection, None))
            .collect();
        verify(
            &plans[0],
            &ranked,
            TamperFacts::default(),
            OrderSource::None,
            None,
        )
    };
    assert_eq!(
        unordered(&plans(EQUAL), &[&first, &second]).verdict,
        Verdict::Accept
    );
    let three = unordered(&plans(EQUAL), &[&first, &second, &third]);
    assert_eq!(codes(&three), ["order_unattested"]);
    let ranked = plans(&format!(
        "objective:\n  direction: lower\npaired_run:\n  relation: not_worse\n  minimum_pairs: 1\n"
    ));
    assert_eq!(
        codes(&unordered(&ranked, &[&first, &second])),
        ["order_unattested"]
    );
}

fn report_of(
    plans: &[MeasurementPlan],
    runs: &[MeasurementCollection],
) -> quoin_measurement::MeasurementReport {
    build_measurement_report_from(Path::new("/repo"), plans, runs, &[], &[]).expect("builds")
}

/// The report prints the pair, and states `unpaired` for one collection.
#[trace("FR-132-AC-13", "TC-3510")]
#[test]
fn tc_3510_the_report_prints_the_paired_runs() {
    let plans = plans(EQUAL);
    let runs = [run("a", &[("r1", 3.0)]), run("b", &[("r1", 4.0)])];
    let report = report_of(&plans, &runs);
    assert_eq!(
        report.paired_runs[0].outcome,
        PairedRunOutcome::Observed { differing: 1 }
    );
    let text = render_measurement_report(&report).expect("renders");
    assert!(text.contains("## Paired runs"), "{text}");
    assert!(
        text.contains("| quality.example | MP-001 | observed | 1 | 1 | 1 |"),
        "{text}"
    );

    let single = report_of(&plans, &runs[..1]);
    let text = render_measurement_report(&single).expect("renders");
    assert!(
        text.contains("| quality.example | MP-001 | unpaired | - |"),
        "{text}"
    );
}

/// The JSON carries `pairedRuns` only when a plan declares `paired_run`.
#[trace("FR-132-AC-13", "TC-3511")]
#[test]
fn tc_3511_the_report_json_carries_paired_runs_only_for_paired_plans() {
    let runs = [run("a", &[("r1", 3.0)]), run("b", &[("r1", 3.0)])];
    let paired: Value = serde_json::from_str(
        &render_measurement_report_json(&report_of(&plans(EQUAL), &runs)).expect("renders"),
    )
    .expect("json");
    assert_eq!(paired["pairedRuns"][0]["status"], json!("observed"));
    assert_eq!(paired["pairedRuns"][0]["differing"], json!(0));
    let plain = render_measurement_report_json(&report_of(&plans(""), &runs)).expect("renders");
    assert!(!plain.contains("pairedRuns"));
    let text = render_measurement_report(&report_of(&plans(""), &runs)).expect("renders");
    assert!(!text.contains("Paired runs"));
}
