// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The PLAT-1014 agent-labelled answer key, checked offline.
//!
//! Runs in the default gate: no feature flag, no socket, no key. The live
//! grade in `live_gap_remaining4.rs` is only as good as the labels it reads
//! and the glue that turns a response into graded rows, so both are checked
//! here: the key covers exactly the corpus, every label is an answer the lens
//! could give, the provenance still says the labels are agent-written, and a
//! lens that answered every label exactly would score 100% on every gated
//! question.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod gap_battery_support;
mod gap_remaining4_support;
mod gap_semantic_support;

use std::collections::BTreeMap;

use gap_battery_support::BatteryAnswers;
use gap_remaining4_support::grading::{defect_recall, no_defect_recall, tally};
use gap_remaining4_support::{gated, grade, label_file, labels};
use gap_semantic_support::{DIVERGENCE_KINDS, SEVERITY_RUBRIC, corpus};

/// Provenance: PLAT-1014. The key labels every corpus triple once, in corpus
/// order, and nothing else.
#[test]
fn the_labels_cover_exactly_the_corpus() {
    let corpus_ids: Vec<String> = corpus().into_iter().map(|triple| triple.id).collect();
    let label_ids: Vec<String> = labels().into_iter().map(|label| label.id).collect();
    assert_eq!(label_ids, corpus_ids);
}

/// Provenance: PLAT-1014. The provenance sentence is load-bearing: every
/// number graded against this file is graded against one agent's reading.
#[test]
fn the_labels_say_they_are_agent_labelled() {
    let file = label_file();
    assert!(
        file.provenance.contains("AGENT-LABELLED"),
        "{}",
        file.provenance
    );
    assert!(file.provenance.contains("not human ground truth"));
    for question in [
        "code_implements_intent",
        "code_exceeds_requirement",
        "divergence_kind",
        "severity",
    ] {
        assert!(
            file.rules.contains_key(question),
            "no stated labelling rule for {question}"
        );
    }
    assert!(!file.unit.is_empty());
}

/// Provenance: PLAT-1014. A label the lens could never emit makes every
/// agreement figure computed against it wrong, silently.
#[test]
fn every_label_is_a_legal_answer() {
    for label in labels() {
        assert!(!label.rationale.trim().is_empty(), "{}", label.id);
        for kind in std::iter::once(&label.divergence_kind).chain(&label.divergence_kind_contested)
        {
            assert!(
                DIVERGENCE_KINDS.contains(&kind.as_str()),
                "{}: {kind}",
                label.id
            );
        }
        assert!(
            !label
                .divergence_kind_contested
                .contains(&label.divergence_kind),
            "{}: a contested reading repeats the primary one",
            label.id
        );
        for level in std::iter::once(&label.severity).chain(&label.severity_contested) {
            assert!(
                SEVERITY_RUBRIC.contains(&level.as_str()),
                "{}: {level}",
                label.id
            );
        }
        assert!(
            !label.severity_contested.contains(&label.severity),
            "{}: a contested level repeats the primary one",
            label.id
        );
    }
}

/// The answers a lens that agreed with every primary label would give.
fn oracle() -> BTreeMap<String, BatteryAnswers> {
    labels()
        .into_iter()
        .map(|label| {
            let mut noul = BTreeMap::new();
            for (key, value) in [
                ("code_implements_intent", label.code_implements_intent),
                ("code_exceeds_requirement", label.code_exceeds_requirement),
            ] {
                if let Some(value) = value {
                    noul.insert(key.to_owned(), if value { 0.9 } else { 0.1 });
                }
            }
            let level = SEVERITY_RUBRIC
                .iter()
                .position(|level| *level == label.severity)
                .unwrap();
            let answers = BatteryAnswers {
                noul,
                divergence_kind: Some(label.divergence_kind.clone()),
                severity: Some(f64::from(u8::try_from(level).unwrap())),
            };
            (label.id, answers)
        })
        .collect()
}

/// Provenance: PLAT-1014. The glue between a response and the shared maths
/// is itself under test: the perfect answerer agrees on every gated row, and
/// every gated population carries both a defect and a no-defect class, so the
/// pre-registered recall bars are measurable at all.
#[test]
fn a_lens_matching_every_label_scores_perfectly_and_every_bar_is_measurable() {
    let grades = grade(&labels(), &oracle());
    for (name, rows, no_defect) in gated(&grades) {
        let counts = tally(rows);
        assert_eq!(counts.primary, rows.len(), "{name}");
        assert!(
            defect_recall(rows, no_defect).is_some(),
            "{name}: no defect-class row, so defect recall is unmeasurable"
        );
        assert!(
            no_defect_recall(rows, no_defect).is_some(),
            "{name}: no `{no_defect}` row, so no-defect recall is unmeasurable"
        );
    }
    // One triple (`GAP-01`) is excluded from `code_exceeds_requirement` by
    // its own stated rule; every other population is the whole corpus.
    assert_eq!(grades.implements.len(), 29);
    assert_eq!(grades.exceeds.len(), 28);
    assert_eq!(grades.divergence_asked.len(), 29);
    assert_eq!(grades.severity_verdict.len(), 29);
}

/// Provenance: PLAT-1014. A triple the lens never answered is graded as
/// unanswered — in the denominator — rather than dropped.
#[test]
fn an_unanswered_triple_stays_in_the_denominator() {
    let mut said = oracle();
    said.remove("GAP-03");
    let grades = grade(&labels(), &said);
    for (name, rows, _) in gated(&grades) {
        let counts = tally(rows);
        assert_eq!(counts.unanswered, 1, "{name}");
        assert_eq!(counts.total(), rows.len(), "{name}");
    }
}
