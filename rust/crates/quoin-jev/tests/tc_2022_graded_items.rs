// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-108-AC-23: the live harness's graded rows map to constant-predictor
//! items offline, over the committed fixture corpus.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
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
#[trace("FR-108-AC-23", "TC-2022")]
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
#[trace("FR-108-AC-20", "FR-108-AC-23", "TC-2022")]
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
