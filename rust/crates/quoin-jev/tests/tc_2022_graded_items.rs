// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-108-AC-23: the live harness's graded rows map to constant-predictor
//! items offline, over the committed fixture corpus.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report"
)]

mod support;

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quoin_jev::{GradedCorpus, graded_corpus_collection};
use quoin_measurement::json_bridge::to_serde;
use serde_json::json;
use support::{Graded, Verdict, constant_predictor_items, corpus};

/// The committed corpus graded as if the lens answered every fixture with its
/// primary reading.
fn graded() -> Vec<Graded> {
    let corpus = corpus();
    let weakness = corpus.weakness_kind_fixtures.iter().map(|fixture| {
        let readings = fixture.labels.readings();
        (
            fixture.fixture_id.clone(),
            fixture.confidence,
            fixture.labels.weakness_kind.clone(),
            readings.iter().map(ToString::to_string).collect::<Vec<_>>(),
        )
    });
    let coverage = corpus.adverse_case_coverage_fixtures.iter().map(|fixture| {
        let readings = fixture.labels.readings();
        (
            fixture.fixture_id.clone(),
            fixture.confidence,
            fixture.labels.adverse_case_coverage.to_string(),
            readings.iter().map(ToString::to_string).collect::<Vec<_>>(),
        )
    });
    weakness
        .chain(coverage)
        .map(|(fixture_id, tier, expected, contested)| Graded {
            fixture_id,
            tier,
            actual: expected.clone(),
            actual_class: expected.clone(),
            expected,
            contested,
            verdict: Verdict::Primary,
            confidence: None,
        })
        .collect()
}

fn stored(items: Vec<quoin_jev::ConstantPredictorItem>) -> serde_json::Value {
    let count = items.len();
    let input = GradedCorpus {
        plan_id: "MP-244".to_owned(),
        definition_version: "v1".to_owned(),
        metric: "jev.criterion_strength.agreement".to_owned(),
        collection_id: "run".to_owned(),
        subject: "corpus".to_owned(),
        scope: json!({ "items": count }),
        tool_identity: "harness".to_owned(),
        tool_version: "1".to_owned(),
        timestamp: "2026-11-11T00:00:00.000Z".to_owned(),
        environment: json!({}),
        verification_stack: json!({
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "debug",
            "capabilities": ["c"],
            "artifacts": ["a"],
        }),
        repetitions: 1,
        items,
    };
    to_serde(&graded_corpus_collection(&input).unwrap()).unwrap()
}

/// Provenance: PLAT-1021
#[trace("FR-108-AC-23")]
#[test]
fn tc_2022_the_committed_corpus_maps_to_fifteen_items_in_two_families() {
    let items = constant_predictor_items(&graded());
    assert_eq!(items.len(), 15);
    let mut families: BTreeMap<&str, usize> = BTreeMap::new();
    for item in &items {
        *families.entry(item.family.as_str()).or_default() += 1;
    }
    assert_eq!(
        families,
        BTreeMap::from([("coverage", 4), ("weakness_kind", 11)])
    );

    let document = stored(items);
    assert_eq!(document["observations"][0]["population"]["matched"], 15);
}

/// A row the lens left unanswered or answered outside the answer space is a
/// disagreement even when its leftover label equals the corpus's.
///
/// Provenance: PLAT-1021
#[trace("FR-108-AC-20", "FR-108-AC-23")]
#[test]
fn tc_2022_an_unrecognized_row_never_agrees_through_a_leftover_label() {
    let mut rows = graded();
    rows[0].verdict = Verdict::Unrecognized;
    rows[1].verdict = Verdict::Unanswered;
    rows[1].actual_class = "<unanswered>".to_owned();
    let document = stored(constant_predictor_items(&rows));
    let population = &document["observations"][0]["population"];
    assert_eq!(population["examined"], 15);
    assert_eq!(population["matched"], 13);
}

/// The authored plan `id`'s declared population set.
fn declared_set(id: &str) -> std::collections::BTreeSet<String> {
    use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
    use quoin_measurement::source::MemoryMeasurement;

    let assurance =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spec/assurance");
    let file = declared_set_file(id);
    let document = std::fs::read_to_string(assurance.join(&file)).unwrap();
    let source = MemoryMeasurement::new().with_document(format!("spec/assurance/{file}"), document);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    plans[0]
        .statistical_design
        .as_ref()
        .and_then(|design| design.population_set.as_ref())
        .unwrap()
        .iter()
        .map(str::to_owned)
        .collect()
}

/// The Jev plans' declared sets equal what their sources enumerate: MP-244
/// the corpus's fixtures (and the aggregate the producer stores names exactly
/// them), MP-219 the frozen labels, and MP-229 and MP-230 bars that are plans
/// in this directory.
///
/// Provenance: PLAT-1401
#[trace("FR-131-AC-12")]
#[test]
fn tc_3006_the_jev_plans_declare_the_sets_their_sources_enumerate() {
    let fixtures: std::collections::BTreeSet<String> = {
        let corpus = corpus();
        corpus
            .weakness_kind_fixtures
            .iter()
            .map(|fixture| fixture.fixture_id.clone())
            .chain(
                corpus
                    .adverse_case_coverage_fixtures
                    .iter()
                    .map(|fixture| fixture.fixture_id.clone()),
            )
            .collect()
    };
    assert_eq!(declared_set("MP-244"), fixtures);
    let document = stored(constant_predictor_items(&graded()));
    let stated: std::collections::BTreeSet<String> =
        document["observations"][0]["population"]["examined_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
    assert_eq!(stated, fixtures);

    let labels: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../bench/span-breadth-v1-labels.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let label_ids: std::collections::BTreeSet<String> = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|label| label["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(declared_set("MP-219"), label_ids);

    for (gate, bars) in [("MP-229", 4), ("MP-230", 3)] {
        let declared = declared_set(gate);
        assert_eq!(declared.len(), bars, "{gate}");
        for bar in declared {
            let _ = declared_set_file(&bar);
        }
    }
}

/// The file of the plan `id`, which must exist.
fn declared_set_file(id: &str) -> String {
    std::fs::read_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spec/assurance"),
    )
    .unwrap()
    .filter_map(Result::ok)
    .map(|entry| entry.file_name().to_string_lossy().into_owned())
    .find(|name| name.starts_with(&format!("{id}-")))
    .unwrap_or_else(|| panic!("{id} is not a plan in spec/assurance"))
}
