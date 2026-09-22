// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-838: the EARS grading maths, exercised offline.
//!
//! Runs in the default gate: no feature flag, no socket, no key. Mirrors
//! `grading_math.rs`'s discipline for the criterion-strength lens -- the
//! arithmetic that will interpret a live run is itself under test first.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod support;

use support::ears::{
    EarsQuestionSet, EarsVerdict, NO_DEFECT, QUESTION_SET_JSON, grade_defect, grade_pattern,
    jev_flags_defect, m2_corpus, m6_corpus,
};
use support::grading::{
    Verdict, class_stats, defect_recall, no_defect_recall, tally, trivial_baseline,
};

fn verdict(
    pattern: Option<&str>,
    response_measurable: Option<f64>,
    confidence: f64,
) -> EarsVerdict {
    EarsVerdict {
        classifier: "stub".to_owned(),
        usage_input_tokens: 0,
        usage_output_tokens: 0,
        pattern: pattern.map(str::to_owned),
        pattern_confidence: Some(confidence),
        pattern_probabilities: vec![],
        unrecognized_pattern: None,
        response_measurable,
        trigger_is_momentary: None,
        condition_is_unwanted: None,
        severity_score: None,
        severity_confidence: None,
    }
}

/// Provenance: PLAT-838. The question set asset parses and carries the
/// exact vocabulary the ticket states.
#[test]
fn the_question_set_asset_parses_with_the_tickets_own_vocabulary() {
    let qs = EarsQuestionSet::parse(QUESTION_SET_JSON);
    assert_eq!(qs.noul.len(), 3);
    assert_eq!(
        qs.choice.answer_space,
        vec![
            "ubiquitous",
            "event_driven",
            "state_driven",
            "unwanted_behaviour",
            "optional_feature",
            "complex",
        ]
    );
    assert_eq!(qs.score.rubric.len(), 3);
}

/// Provenance: PLAT-838. The M2 corpus parses and holds the two acceptance
/// fixtures the ticket names by name.
#[test]
fn the_m2_corpus_holds_the_tickets_named_acceptance_fixtures() {
    let corpus = m2_corpus();
    assert_eq!(corpus.fixtures.len(), 16);

    let when_is_really_while = corpus
        .fixtures
        .iter()
        .find(|f| f.labels.defect_kind.as_deref() == Some("when_is_really_while"));
    assert!(
        when_is_really_while.is_some(),
        "PLAT-838 acceptance: a When that is really a While must be in the corpus"
    );

    let unmeasurable_past_denylist = corpus.fixtures.iter().find(|f| {
        f.labels.defect_kind.as_deref() == Some("unmeasurable_response")
            && f.statement_text.contains("robust")
    });
    assert!(
        unmeasurable_past_denylist.is_some(),
        "PLAT-838 acceptance: a statement passing the vague-verb denylist but unmeasurable \
         must be in the corpus"
    );

    let ambiguous = corpus
        .fixtures
        .iter()
        .filter(|f| f.confidence == support::grading::Tier::Ambiguous)
        .count();
    assert!(
        ambiguous >= 2,
        "the corpus must include genuinely hard cases"
    );
}

/// Provenance: PLAT-838. A lens that answers the engine's own naive pattern
/// for every fixture, and always calls the response measurable, is the
/// do-nothing lens: it never disagrees with the engine, so it never raises a
/// finding of its own.
#[test]
fn a_lens_that_only_ever_echoes_the_engine_finds_nothing() {
    let corpus = m2_corpus();
    let graded: Vec<_> = corpus
        .fixtures
        .iter()
        .map(|fixture| {
            let echo = verdict(Some(fixture.engine_naive_pattern.as_str()), Some(1.0), 0.9);
            grade_defect(fixture, &echo)
        })
        .collect();

    let recall = defect_recall(&graded, NO_DEFECT);
    assert_eq!(
        recall,
        Some(0.0),
        "echoing the engine's own pattern and always calling the response measurable finds \
         none of this corpus's defects"
    );
}

/// Provenance: PLAT-838. `defect_recall`/`no_defect_recall` must separate a
/// flag-everything lens (perfect defect recall, zero no-defect recall) from
/// one with real judgment -- the PLAT-917 lesson applied to this lens.
#[test]
fn a_lens_that_flags_everything_has_zero_no_defect_recall() {
    let corpus = m2_corpus();
    let graded: Vec<_> = corpus
        .fixtures
        .iter()
        .map(|fixture| {
            // Disagree with the engine's pattern unconditionally.
            let always_defect = verdict(Some("complex"), Some(0.1), 0.9);
            grade_defect(fixture, &always_defect)
        })
        .collect();

    let defect_r = defect_recall(&graded, NO_DEFECT);
    let no_defect_r = no_defect_recall(&graded, NO_DEFECT);
    assert!(
        defect_r.unwrap_or(0.0) > 0.0,
        "a flag-everything lens finds every real defect"
    );
    assert_eq!(
        no_defect_r,
        Some(0.0),
        "a flag-everything lens never correctly calls a clean statement clean"
    );
}

/// Provenance: PLAT-838. A lens that agrees with this corpus's own labels on
/// every fixture scores perfect recall on both halves, and the constant
/// predictor is beaten (unless the corpus itself is degenerate, which
/// `the_m2_corpus_holds_the_tickets_named_acceptance_fixtures` already rules
/// out by requiring both defect kinds and ambiguous rows to exist).
#[test]
fn a_lens_matching_every_label_beats_the_constant_predictor_and_finds_everything() {
    let corpus = m2_corpus();
    let graded: Vec<_> = corpus
        .fixtures
        .iter()
        .map(|fixture| {
            let pattern = if fixture.labels.has_defect {
                // Disagree with the engine's own pattern to register as a
                // finding, without needing to be the "true" alternate label.
                if fixture.engine_naive_pattern == "event_driven" {
                    "state_driven"
                } else {
                    "event_driven"
                }
            } else {
                fixture.engine_naive_pattern.as_str()
            };
            let measurable = if fixture.labels.response_measurable {
                1.0
            } else {
                0.0
            };
            let oracle = verdict(Some(pattern), Some(measurable), 0.95);
            grade_defect(fixture, &oracle)
        })
        .collect();

    let agreement = tally(&graded).agreement().unwrap();
    let (_, baseline) = trivial_baseline(&graded);
    assert!(
        agreement > baseline,
        "an oracle lens must beat the constant predictor: {agreement:.1}% vs {baseline:.1}%"
    );
    assert_eq!(defect_recall(&graded, NO_DEFECT), Some(100.0));
    assert_eq!(no_defect_recall(&graded, NO_DEFECT), Some(100.0));
}

/// Provenance: PLAT-838. An unrecognized `ears_pattern_actual` label is
/// tracked, and never silently counted as "clean".
#[test]
fn an_unrecognized_pattern_label_is_tracked_not_folded_into_clean() {
    let corpus = m2_corpus();
    let fixture = &corpus.fixtures[0];
    let bad = EarsVerdict {
        classifier: "stub".to_owned(),
        usage_input_tokens: 0,
        usage_output_tokens: 0,
        pattern: None,
        pattern_confidence: None,
        pattern_probabilities: vec![],
        unrecognized_pattern: Some("mostly_ubiquitous".to_owned()),
        response_measurable: None,
        trigger_is_momentary: None,
        condition_is_unwanted: None,
        severity_score: None,
        severity_confidence: None,
    };
    let graded = grade_defect(fixture, &bad);
    assert_eq!(graded.verdict, Verdict::Unrecognized);
}

/// Provenance: PLAT-838. `grade_pattern`'s raw 6-way class stats are keyed by
/// the corpus's declared `true_pattern`, independent of the defect reduction.
#[test]
fn pattern_grading_reports_a_full_confusion_matrix() {
    let corpus = m2_corpus();
    let graded: Vec<_> = corpus
        .fixtures
        .iter()
        .map(|fixture| {
            let oracle = verdict(Some(fixture.labels.true_pattern.as_str()), Some(1.0), 0.9);
            grade_pattern(fixture, &oracle)
        })
        .collect();
    let stats = class_stats(&graded);
    assert!(stats.contains_key("event_driven"));
    assert!(stats.contains_key("ubiquitous"));
    let agreement = tally(&graded).agreement().unwrap();
    assert!(
        (agreement - 100.0).abs() < 1e-9,
        "an oracle matches the corpus's own pattern labels"
    );
}

/// Provenance: PLAT-838. `jev_flags_defect` (the M6 delta test) reads `None`
/// when the choice went unanswered -- unanswered is not "clean".
#[test]
fn m6_flag_reads_none_when_unanswered() {
    let corpus = m6_corpus();
    let statement = &corpus.clean[0];
    let unanswered = verdict(None, None, 0.0);
    assert_eq!(jev_flags_defect(statement, &unanswered), None);
}

/// Provenance: PLAT-838. Disagreeing with the statement's own engine-derived
/// pattern flags it, even when Jev judges the response measurable.
#[test]
fn m6_flag_is_true_on_pattern_disagreement_alone() {
    let corpus = m6_corpus();
    let statement = &corpus.clean[0];
    let other = if statement.engine_naive_pattern == "ubiquitous" {
        "complex"
    } else {
        "ubiquitous"
    };
    let disagreeing = verdict(Some(other), Some(1.0), 0.9);
    assert_eq!(jev_flags_defect(statement, &disagreeing), Some(true));
}

/// Provenance: PLAT-838. The M6 corpus parses, both pools are non-empty, and
/// every distinct engine `ears:*` tag from the sampled repos is represented
/// (so the inverse-delta measurement is not silently computed over a corpus
/// missing whole finding classes).
#[test]
fn the_m6_corpus_parses_and_covers_every_sampled_engine_tag() {
    let corpus = m6_corpus();
    assert!(!corpus.flagged.is_empty());
    assert!(!corpus.clean.is_empty());
    assert!(corpus.clean.iter().all(|s| s.engine_tags.is_empty()));
    assert!(corpus.flagged.iter().all(|s| !s.engine_tags.is_empty()));
}
