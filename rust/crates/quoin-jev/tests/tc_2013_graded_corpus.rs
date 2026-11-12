// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-108-AC-15..AC-22: `graded_corpus_collection` builds the collection a
//! graded pass is stored as, and the stored collection is decided against a
//! recomputed constant-predictor baseline.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::float_cmp,
    reason = "in a test, a panic IS the failure report; the fixture's counts are exact"
)]

use ix_trace_rs::trace;
use quoin_jev::{ConstantPredictorItem, GradedCorpus, graded_corpus_collection, item_observations};
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::to_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::{
    OrderSource, Ranked, Reason, TamperFacts, validate, verify, write_measurement_collection,
};
use quoin_store::canonical_json;
use serde_json::{Value, json};

const METRIC: &str = "quality.gate";
const PLAN: &str = "MP-900";
const VERSION: &str = "quality.gate-v1";

fn item(
    id: &str,
    family: &str,
    expected: &str,
    contested: &[&str],
    actual: &str,
) -> ConstantPredictorItem {
    ConstantPredictorItem {
        item_id: id.to_owned(),
        family: family.to_owned(),
        expected: expected.to_owned(),
        contested: contested.iter().map(|label| (*label).to_owned()).collect(),
        actual: actual.to_owned(),
    }
}

/// FR-108's worked example: 11 `weakness_kind` (best `sound`, 9) and 4
/// `coverage` (best `2`, 3), so the baseline is `12 / 15 = 0.8`. The tool
/// answers the first `wrong` items `unknown` and the rest correctly.
fn items(wrong: usize) -> Vec<ConstantPredictorItem> {
    let weakness: [(&str, &[&str]); 11] = [
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("gap", &["gap", "sound"]),
        ("gap", &["gap", "sound"]),
        ("gap", &["gap", "sound"]),
        ("weakness", &["weakness"]),
        ("weakness", &["weakness"]),
    ];
    let coverage: [(&str, &[&str]); 4] =
        [("2", &["2"]), ("2", &["2"]), ("2", &["2"]), ("1", &["1"])];
    let mut out = Vec::new();
    for (index, (expected, contested)) in weakness.into_iter().enumerate() {
        out.push(item(
            &format!("w{index:02}"),
            "weakness_kind",
            expected,
            contested,
            expected,
        ));
    }
    for (index, (expected, contested)) in coverage.into_iter().enumerate() {
        out.push(item(
            &format!("c{index:02}"),
            "coverage",
            expected,
            contested,
            expected,
        ));
    }
    for row in out.iter_mut().take(wrong) {
        "unknown".clone_into(&mut row.actual);
    }
    out
}

fn corpus(items: Vec<ConstantPredictorItem>) -> GradedCorpus {
    GradedCorpus {
        plan_id: PLAN.to_owned(),
        definition_version: VERSION.to_owned(),
        metric: METRIC.to_owned(),
        collection_id: "run-1".to_owned(),
        subject: "fixture corpus".to_owned(),
        scope: json!({ "items": items.len() }),
        tool_identity: "fixture grader".to_owned(),
        tool_version: "1".to_owned(),
        timestamp: "2026-11-11T00:00:00.000Z".to_owned(),
        environment: json!({ "runner": "test" }),
        verification_stack: json!({
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "debug",
            "capabilities": ["fixture.capability"],
            "artifacts": ["spec/assurance/MP-900.md"],
        }),
        repetitions: 1,
        items,
    }
}

fn document(corpus: &GradedCorpus) -> Value {
    to_serde(&graded_corpus_collection(corpus).unwrap()).unwrap()
}

fn refusal(corpus: &GradedCorpus) -> String {
    let error = graded_corpus_collection(corpus).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::CollectionInvalid);
    error.to_string()
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-15")]
#[test]
fn tc_2013_every_member_comes_from_the_input() {
    let input = corpus(items(0));
    let document = document(&input);
    assert_eq!(document["schemaVersion"], 2);
    assert_eq!(document["collectionId"], "run-1");
    assert_eq!(document["subject"], "fixture corpus");
    assert_eq!(document["scope"], json!({ "items": 15 }));
    assert_eq!(document["toolIdentity"], "fixture grader");
    assert_eq!(document["toolVersion"], "1");
    assert_eq!(document["timestamp"], "2026-11-11T00:00:00.000Z");
    assert_eq!(document["environment"], json!({ "runner": "test" }));
    assert_eq!(document["verificationStack"], input.verification_stack);
    assert_eq!(document["rawEvidence"], json!({}));
    assert_eq!(document["observations"].as_array().unwrap().len(), 16);
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-16")]
#[test]
fn tc_2014_the_first_observation_is_the_ratio_aggregate() {
    // Three items disagree; one more agrees only through a contested reading.
    let mut input = items(3);
    input[6].actual = "sound".to_owned();
    let document = document(&corpus(input));
    let aggregate = &document["observations"][0];
    assert_eq!(aggregate["metric"], METRIC);
    assert_eq!(aggregate["state"], "measured");
    assert_eq!(aggregate["shape"], "ratio");
    assert_eq!(aggregate["unit"], "fraction");
    assert!(aggregate.get("dimensions").is_none());
    let population = &aggregate["population"];
    assert_eq!(population["examined"], 15);
    assert_eq!(population["matched"], 12);
    assert_eq!(population["complete"], true);
    assert_eq!(population["repetitions"], 1);
    assert_eq!(population["examined_ids"].as_array().unwrap().len(), 15);
    let matched_ids = population["matched_ids"].as_array().unwrap();
    assert_eq!(matched_ids.len(), 12);
    assert!(
        !matched_ids.contains(&json!("w00")),
        "a disagreeing item is not matched"
    );
    assert_eq!(aggregate["value"], 12.0 / 15.0);
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-17")]
#[test]
fn tc_2015_item_rows_are_the_item_observations_for_the_same_items() {
    let input = corpus(items(2));
    let document = document(&input);
    let rows = &document["observations"].as_array().unwrap()[1..];
    let mut sorted = input.items.clone();
    sorted.sort_by(|left, right| left.item_id.cmp(&right.item_id));
    let expected = item_observations(PLAN, VERSION, METRIC, &sorted).unwrap();
    assert_eq!(rows.len(), expected.len());
    assert_eq!(rows.len(), 15);
    for (row, built) in rows.iter().zip(&expected) {
        assert_eq!(row["metric"], built.metric.as_str());
        assert_eq!(
            row["dimensions"]["item_id"],
            to_serde(&built.dimensions.entries()["item_id"]).unwrap()
        );
        assert_eq!(
            row["dimensions"]["family"],
            to_serde(&built.dimensions.entries()["family"]).unwrap()
        );
        assert_eq!(
            row["dimensions"]["actual"],
            to_serde(&built.dimensions.entries()["actual"]).unwrap()
        );
        assert_eq!(row["value"], built.value.unwrap());
    }
}

const PLAN_DOCUMENT: &str = "---\n\
id: MP-900\n\
title: Example gate\n\
type: MeasurementPlan\n\
status: active\n\
owner: test\n\
stage: gate\n\
metric: quality.gate\n\
definition_version: quality.gate-v1\n\
protected_apparatus:\n\
\x20\x20- spec/assurance/MP-900.md\n\
negative_controls:\n\
\x20\x20- kind: apparatus-edit\n\
\x20\x20\x20\x20description: the plan document defines the measurement\n\
ground_truth_kind: human-labelled\n\
statistical_design:\n\
\x20\x20population: every graded item\n\
\x20\x20minimum_population: 10\n\
\x20\x20sampling: exhaustive\n\
\x20\x20repetitions: 1\n\
\x20\x20estimator: proportion\n\
\x20\x20population_exempt: synthetic fixture items are rows, not ids\n\
\x20\x20error_model: binomial\n\
\x20\x20uncertainty: wilson interval\n\
\x20\x20decision_rule:\n\
\x20\x20\x20\x20comparator: gt\n\
\x20\x20\x20\x20baseline: constant-predictor\n\
\x20\x20\x20\x20margin: 0.05\n\
\x20\x20answer_families:\n\
\x20\x20\x20\x20- weakness_kind\n\
\x20\x20\x20\x20- coverage\n\
---\n\
\n\
# Example gate\n";

/// A repository holding the plan, and the plans loaded from it.
fn repository() -> (tempfile::TempDir, Vec<quoin_measurement::MeasurementPlan>) {
    let root = tempfile::tempdir().unwrap();
    let assurance = root.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::write(assurance.join("MP-900.md"), PLAN_DOCUMENT).unwrap();
    let plans = load_measurement_plans(
        &DiskMeasurement::new(root.path()),
        PlanLoadOptions::default(),
    )
    .unwrap();
    (root, plans)
}

fn decide(wrong: usize) -> quoin_measurement::verify::MeasurementVerdict {
    let (root, plans) = repository();
    let candidate = graded_corpus_collection(&corpus(items(wrong))).unwrap();
    write_measurement_collection(root.path(), &candidate)
        .unwrap_or_else(|error| panic!("{error}: {:?}", error.findings()));
    let admitted = validate::measurement_collection(&candidate, &plans).unwrap();
    verify(
        &plans[0],
        &[Ranked::new(&admitted, Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    )
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-17")]
#[test]
fn tc_2016_the_stored_collection_is_decided_against_a_recomputed_baseline() {
    // 15 of 15 clears 0.8 + 0.05; 12 of 15 equals the baseline and does not.
    for (wrong, holds) in [(0, true), (3, false)] {
        let result = decide(wrong);
        for absent in [
            Reason::ConstantPredictorRowsAbsent,
            Reason::ConstantPredictorRowsMismatch,
        ] {
            assert!(!result.reasons.contains(&absent), "{result:?}");
        }
        assert_eq!(result.decisions[0].baseline, Some(0.8), "{result:?}");
        assert_eq!(
            result.decisions[0].holds,
            Some(holds),
            "{wrong}: {result:?}"
        );
        assert_eq!(
            result.reasons.contains(&Reason::RuleNotMet),
            !holds,
            "{result:?}"
        );
    }
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-18")]
#[test]
fn tc_2017_every_refusal_names_its_field_and_returns_nothing() {
    let base = || corpus(items(0));
    for field in [
        "plan_id",
        "definition_version",
        "metric",
        "collection_id",
        "subject",
        "tool_identity",
        "tool_version",
        "timestamp",
    ] {
        let mut input = base();
        match field {
            "plan_id" => input.plan_id.clear(),
            "definition_version" => input.definition_version.clear(),
            "metric" => input.metric.clear(),
            "collection_id" => input.collection_id.clear(),
            "subject" => input.subject.clear(),
            "tool_identity" => input.tool_identity.clear(),
            "tool_version" => input.tool_version.clear(),
            _ => input.timestamp.clear(),
        }
        assert!(refusal(&input).contains(field), "{field}");
    }
    for member in ["capabilities", "artifacts"] {
        let mut input = base();
        input.verification_stack[member] = json!([]);
        assert!(refusal(&input).contains(member), "{member}");
        let mut input = base();
        input
            .verification_stack
            .as_object_mut()
            .unwrap()
            .remove(member);
        assert!(refusal(&input).contains(member), "{member}");
    }
    let mut input = base();
    input.repetitions = 0;
    assert!(refusal(&input).contains("repetitions"));
    assert!(refusal(&corpus(Vec::new())).contains("item"));
    for blank in ["item_id", "family", "expected"] {
        let mut input = base();
        match blank {
            "item_id" => input.items[0].item_id.clear(),
            "family" => input.items[0].family.clear(),
            _ => input.items[0].expected.clear(),
        }
        assert!(refusal(&input).contains(blank), "{blank}");
    }
    let mut input = base();
    let first = input.items[0].item_id.clone();
    input.items[1].item_id = first;
    assert!(refusal(&input).contains("duplicate"));
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-19")]
#[test]
fn tc_2018_the_document_is_deterministic_and_independent_of_item_order() {
    let input = corpus(items(2));
    let first = graded_corpus_collection(&input).unwrap();
    assert_eq!(
        canonical_json(&first).unwrap(),
        canonical_json(&graded_corpus_collection(&input).unwrap()).unwrap()
    );

    let mut permuted = input.clone();
    permuted.items.reverse();
    permuted.items.swap(0, 7);
    let second = graded_corpus_collection(&permuted).unwrap();
    assert_eq!(
        canonical_json(&first).unwrap(),
        canonical_json(&second).unwrap()
    );

    let ids: Vec<String> = to_serde(&first).unwrap()["observations"]
        .as_array()
        .unwrap()[1..]
        .iter()
        .map(|row| row["dimensions"]["item_id"].as_str().unwrap().to_owned())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-20")]
#[test]
fn tc_2019_a_disagreement_counts_in_examined_and_never_in_matched() {
    let mut input = items(0);
    input[0].actual = "<unanswered>".to_owned();
    input[1].actual = "<unrecognized>".to_owned();
    input[2].actual = String::new();
    // Agreement only through a contested reading still counts.
    input[6].actual = "sound".to_owned();
    let document = document(&corpus(input));
    let population = &document["observations"][0]["population"];
    assert_eq!(population["examined"], 15);
    assert_eq!(population["matched"], 12);
    assert_eq!(document["observations"].as_array().unwrap().len(), 16);
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-22")]
#[test]
fn tc_2021_the_function_states_no_build_profile_of_its_own() {
    for stack in [
        json!({ "schemaVersion": "verification-stack-attestation-v1", "buildProfile": "debug",
                "capabilities": ["c"], "artifacts": ["a"] }),
        json!({ "schemaVersion": "verification-stack-attestation-v1",
                "capabilities": ["c"], "artifacts": ["a"] }),
    ] {
        let mut input = corpus(items(0));
        input.verification_stack = stack.clone();
        assert_eq!(document(&input)["verificationStack"], stack);
    }
}

/// The plan the live harness stores its pass under loads and declares the
/// constant-predictor baseline over the two families.
///
/// Provenance: PLAT-1021
#[trace("FR-108-AC-21")]
#[test]
fn tc_2020_the_harness_target_plan_declares_a_constant_predictor_baseline() {
    let document =
        include_str!("../../../../spec/assurance/MP-244-jev-criterion-strength-agreement.md");
    let source = quoin_measurement::MemoryMeasurement::new()
        .with_document("spec/assurance/MP-244.md", document);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    assert_eq!(plans[0].metric.as_str(), "jev.criterion_strength.agreement");
    assert_eq!(
        plans[0].definition_version.as_str(),
        "jev.criterion-strength-agreement-v1"
    );
    let design = plans[0].statistical_design.as_ref().unwrap();
    assert_eq!(
        design.decision_rule,
        Some(
            engineering_assurance::measurement::DecisionRule::against_baseline(
                engineering_assurance::measurement::Comparator::Gt,
                engineering_assurance::measurement::Baseline::ConstantPredictor,
                None,
            )
            .unwrap()
        )
    );
    assert!(design.answer_families.is_some());
}
