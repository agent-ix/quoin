// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The grading maths, exercised offline (PLAT-917).
//!
//! Runs in the default gate: no feature flag, no socket, no key. A live run's
//! headline numbers are produced by `support`'s arithmetic, so that
//! arithmetic has to be under test itself -- otherwise the first real
//! measurement would be interpreted by code nothing had checked.
//!
//! These also pin the corpus: `the_corpus_parses_and_holds_what_the_skill_
//! documents` fails if a fixture is added, removed or relabelled without this
//! test being updated, so the answer key cannot drift from the grader silently.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod support;

use support::{
    Graded, Tier, Verdict, calibration, class_stats, corpus, defect_recall, disagreement,
    expected_calibration_error, grade_weakness, tally, trivial_baseline,
};

use quoin_jev::FrVerdict;

/// An `FrVerdict` shaped as if Jev answered `sound` for the one AC row, with
/// no finding -- the do-nothing lens.
fn all_sound(ac_id: &str) -> FrVerdict {
    FrVerdict {
        classifier: "stub".to_owned(),
        usage_input_tokens: 0,
        usage_output_tokens: 0,
        findings: vec![],
        sound: vec![ac_id.to_owned()],
        unrecognized: vec![],
        unanswered: vec![],
        coverage: None,
    }
}

/// Provenance: PLAT-917. **The reason an agreement percentage is not the
/// correctness gate on this corpus.**
///
/// A lens that answers `sound` to every one of the eleven `weakness_kind`
/// fixtures — finding nothing, ever — agrees with a recorded reading 9 times
/// out of 11. `sound` is one of the two readings on six of the seven
/// contested rows, so the do-nothing answer scores 81.8%.
///
/// This is measured against the real corpus through the real `grade_weakness`
/// path, not asserted from a hand-built table, so it stays true as the
/// fixtures change — and if a future corpus makes the do-nothing lens score
/// lower, this test says so rather than quietly keeping a stale bar.
#[test]
fn the_do_nothing_lens_scores_over_eighty_percent_so_agreement_cannot_be_the_gate() {
    let corpus = corpus();
    let graded: Vec<Graded> = corpus
        .weakness_kind_fixtures
        .iter()
        .map(|fixture| {
            let ac_id = fixture.context().acceptance_criteria[0].id.clone();
            grade_weakness(fixture, &all_sound(&ac_id))
        })
        .collect();

    let agreement = tally(&graded).agreement().unwrap();
    assert!(
        agreement > 80.0,
        "the do-nothing lens is expected to score high on this corpus; got {agreement:.1}%"
    );

    // And it found nothing at all. That is what the gate has to catch.
    assert_eq!(
        defect_recall(&graded, "sound"),
        Some(0.0),
        "a lens answering `sound` everywhere has found none of the defects"
    );

    let (label, rate) = trivial_baseline(&graded);
    assert_eq!(label, "sound");
    assert!((rate - agreement).abs() < 1e-9);
}

/// Provenance: PLAT-917. `defect_recall` asks only "did the lens decline to
/// wave this row through", not "did it pick the same weakness". A mislabelled
/// defect is a finding to fix; a defect called `sound` is the failure the lens
/// exists to prevent, and the gate has to separate the two.
#[test]
fn defect_recall_credits_a_mislabelled_weakness_but_never_a_sound_verdict() {
    let rows = vec![
        // Reader said `unfalsifiable`; lens said `implementation_coupled`.
        // Wrong label, but it refused to wave the row through.
        graded(
            "A",
            Tier::Clean,
            "unfalsifiable",
            "implementation_coupled",
            Verdict::Wrong,
            None,
        ),
        // Reader said `restates_requirement`; lens said `sound`. Missed.
        graded(
            "B",
            Tier::Clean,
            "restates_requirement",
            "sound",
            Verdict::Wrong,
            None,
        ),
        // `sound` was an acceptable reading here, so this row is not a defect
        // and is outside the denominator entirely.
        contested_row("C", "unfalsifiable", "sound", "sound", Verdict::Contested),
    ];
    assert_eq!(
        defect_recall(&rows, "sound"),
        Some(50.0),
        "two rows are unambiguous defects; the lens flagged one of them"
    );
}

fn graded(
    fixture_id: &str,
    tier: Tier,
    expected: &str,
    actual: &str,
    verdict: Verdict,
    confidence: Option<f64>,
) -> Graded {
    Graded {
        fixture_id: fixture_id.to_owned(),
        tier,
        expected: expected.to_owned(),
        contested: vec![expected.to_owned()],
        actual_class: actual.to_owned(),
        actual: actual.to_owned(),
        verdict,
        confidence,
    }
}

/// A row the corpus recorded two readings for.
fn contested_row(
    fixture_id: &str,
    expected: &str,
    alternate: &str,
    actual: &str,
    verdict: Verdict,
) -> Graded {
    Graded {
        fixture_id: fixture_id.to_owned(),
        tier: Tier::Ambiguous,
        expected: expected.to_owned(),
        contested: vec![expected.to_owned(), alternate.to_owned()],
        actual_class: actual.to_owned(),
        actual: actual.to_owned(),
        verdict,
        confidence: None,
    }
}

/// Provenance: PLAT-917. The corpus is the answer key; pin its shape so a
/// fixture cannot be added, dropped or relabelled without this failing.
#[test]
fn the_corpus_parses_and_holds_what_the_skill_documents() {
    let corpus = corpus();
    assert_eq!(corpus.weakness_kind_fixtures.len(), 11);
    assert_eq!(corpus.adverse_case_coverage_fixtures.len(), 4);

    let ambiguous = corpus
        .weakness_kind_fixtures
        .iter()
        .filter(|fixture| fixture.confidence == Tier::Ambiguous)
        .count()
        + corpus
            .adverse_case_coverage_fixtures
            .iter()
            .filter(|fixture| fixture.confidence == Tier::Ambiguous)
            .count();
    assert_eq!(
        ambiguous, 9,
        "SKILL.md states 9 of 15 fixtures are marked ambiguous"
    );

    let contested = corpus
        .weakness_kind_fixtures
        .iter()
        .filter(|fixture| fixture.labels.weakness_kind_contested.is_some())
        .count()
        + corpus
            .adverse_case_coverage_fixtures
            .iter()
            .filter(|fixture| fixture.labels.adverse_case_coverage_contested.is_some())
            .count();
    // MEASURED, and it contradicts the corpus's own prose. The file's
    // `governing_ruling_on_disagreement` says the second reader "disputed 5 of
    // 14 (agreed 9)", but 9 of the 15 fixtures actually carry a `*_contested`
    // array: 7 `weakness_kind` (CS-FIX-004/005/006/007/008/009/014) and 2
    // `adverse_case_coverage` (CS-FIX-012/013). The data is the fact and the
    // sentence is stale -- counted here rather than repeating the sentence,
    // because this number sets how many fixtures are graded against two
    // readings instead of one, and a grader keyed to 5 would score 4 defensible
    // answers as wrong.
    assert_eq!(
        contested, 9,
        "every fixture where the two readers differed carries both readings"
    );
}

/// Provenance: PLAT-917. Every fixture turns into a context the lens can
/// take, with the AC rows the corpus recorded -- the step that has to work
/// before a single call is worth making.
#[test]
fn every_fixture_builds_a_context_carrying_its_own_criterion_text() {
    let corpus = corpus();
    for fixture in &corpus.weakness_kind_fixtures {
        let context = fixture.context();
        assert_eq!(
            context.acceptance_criteria.len(),
            1,
            "{}: a weakness_kind fixture is one criterion",
            fixture.fixture_id
        );
        assert_eq!(
            context.acceptance_criteria[0].text, fixture.criterion_text,
            "{}: the criterion text reaches the request unmodified",
            fixture.fixture_id
        );
        assert!(
            context.fr_id.starts_with("FR-") || context.fr_id.starts_with("NFR-"),
            "{}: derived an FR id from the criterion id, got {:?}",
            fixture.fixture_id,
            context.fr_id
        );
    }
    for fixture in &corpus.adverse_case_coverage_fixtures {
        let context = fixture.context();
        assert_eq!(
            context.acceptance_criteria.len(),
            fixture.ac_set.len(),
            "{}: a coverage fixture sends its whole AC set",
            fixture.fixture_id
        );
    }
}

/// Provenance: PLAT-917. A contested match is agreement, not a miss. Scoring
/// it wrong would penalise the lens for a disagreement the two human readers
/// had themselves, which is the whole reason the corpus records both.
#[test]
fn a_contested_match_counts_as_agreement_and_is_still_reported_separately() {
    let rows = vec![
        graded(
            "A",
            Tier::Clean,
            "sound",
            "sound",
            Verdict::Primary,
            Some(0.9),
        ),
        graded(
            "B",
            Tier::Ambiguous,
            "unfalsifiable",
            "sound",
            Verdict::Contested,
            Some(0.6),
        ),
        graded(
            "C",
            Tier::Ambiguous,
            "sound",
            "happy_path_only",
            Verdict::Wrong,
            Some(0.8),
        ),
    ];
    let tally = tally(&rows);
    assert_eq!(tally.agreed(), 2);
    assert_eq!(tally.primary, 1);
    assert_eq!(tally.contested, 1);
    assert_eq!(tally.wrong, 1);
    assert!((tally.agreement().unwrap() - 66.666_666_666_666_67).abs() < 1e-9);
}

/// Provenance: PLAT-917. An unanswered or unrecognized row stays in the
/// denominator. A lens that answers nothing has not scored 100%, and the
/// PLAT-913 defect was exactly this class of row reading as a clean pass.
#[test]
fn unanswered_and_unrecognized_rows_are_counted_against_the_agreement_rate() {
    let rows = vec![
        graded(
            "A",
            Tier::Clean,
            "sound",
            "sound",
            Verdict::Primary,
            Some(0.9),
        ),
        graded(
            "B",
            Tier::Clean,
            "sound",
            "quantum_uncertainty",
            Verdict::Unrecognized,
            None,
        ),
        graded(
            "C",
            Tier::Clean,
            "sound",
            "<unanswered>",
            Verdict::Unanswered,
            None,
        ),
    ];
    let tally = tally(&rows);
    assert_eq!(tally.total(), 3);
    assert_eq!(tally.agreed(), 1);
    assert_eq!(tally.unrecognized, 1);
    assert_eq!(tally.unanswered, 1);
    assert!((tally.agreement().unwrap() - 33.333_333_333_333_336).abs() < 1e-9);
}

/// Provenance: PLAT-917. PLAT-837's M2 requires precision and recall
/// reported separately, with false positives called out: "a lens that flags
/// everything has perfect recall and is worthless".
#[test]
fn per_class_precision_and_recall_separate_a_flag_everything_lens_from_a_good_one() {
    // A lens that answered `unfalsifiable` for all three rows, when only one
    // reader said so: perfect recall on that class, 33% precision.
    let rows = vec![
        graded(
            "A",
            Tier::Clean,
            "unfalsifiable",
            "unfalsifiable",
            Verdict::Primary,
            Some(0.9),
        ),
        graded(
            "B",
            Tier::Clean,
            "sound",
            "unfalsifiable",
            Verdict::Wrong,
            Some(0.9),
        ),
        graded(
            "C",
            Tier::Clean,
            "sound",
            "unfalsifiable",
            Verdict::Wrong,
            Some(0.9),
        ),
    ];
    let stats = class_stats(&rows);
    let unfalsifiable = &stats["unfalsifiable"];
    assert_eq!(unfalsifiable.predicted, 3);
    assert_eq!(unfalsifiable.expected, 1);
    assert_eq!(unfalsifiable.correct, 1);
    assert!((unfalsifiable.recall().unwrap() - 100.0).abs() < 1e-9);
    assert!((unfalsifiable.precision().unwrap() - 33.333_333_333_333_336).abs() < 1e-9);
    assert_eq!(unfalsifiable.false_positives(), 2);

    let sound = &stats["sound"];
    assert_eq!(sound.expected, 2);
    assert_eq!(sound.correct, 0);
    assert!((sound.recall().unwrap() - 0.0).abs() < 1e-9);
    assert!(
        sound.precision().is_none(),
        "the lens never returned `sound`"
    );
}

/// Provenance: PLAT-917. A row carrying no confidence is excluded from the
/// calibration curve and counted, never silently dropped -- otherwise the
/// curve looks denser than the evidence behind it.
#[test]
fn rows_without_a_confidence_are_excluded_from_calibration_and_counted() {
    let rows = vec![
        graded("A", Tier::Clean, "sound", "sound", Verdict::Primary, None),
        graded(
            "B",
            Tier::Clean,
            "unfalsifiable",
            "unfalsifiable",
            Verdict::Primary,
            Some(0.95),
        ),
        graded(
            "C",
            Tier::Clean,
            "sound",
            "happy_path_only",
            Verdict::Wrong,
            Some(0.95),
        ),
    ];
    let (buckets, without) = calibration(&rows);
    assert_eq!(without, 1, "the `sound` row carries no confidence");
    let top = buckets.iter().find(|bucket| bucket.count > 0).unwrap();
    assert!((top.floor - 0.9).abs() < 1e-9);
    assert_eq!(top.count, 2);
    assert_eq!(top.correct, 1);
    assert!((top.accuracy().unwrap() - 50.0).abs() < 1e-9);
}

/// Provenance: PLAT-917. An ECE over no data is `None`, not zero. Zero would
/// read as perfect calibration, which is the exact opposite of "nothing was
/// measured".
#[test]
fn an_expected_calibration_error_over_nothing_is_none_rather_than_zero() {
    let rows = vec![graded(
        "A",
        Tier::Clean,
        "sound",
        "sound",
        Verdict::Primary,
        None,
    )];
    assert!(expected_calibration_error(&rows).is_none());

    // 0.95 stated, 50% observed -> |0.95 - 0.5| = 0.45, one bucket, weight 1.
    let rows = vec![
        graded("A", Tier::Clean, "a", "a", Verdict::Primary, Some(0.95)),
        graded("B", Tier::Clean, "a", "b", Verdict::Wrong, Some(0.95)),
    ];
    let ece = expected_calibration_error(&rows).unwrap();
    assert!((ece - 0.45).abs() < 1e-9, "got {ece}");
}

/// Provenance: PLAT-917. M1's own definition: a finding that appears in one
/// run and not the other is a change, so the denominator is every id either
/// run produced.
#[test]
fn disagreement_counts_a_row_present_in_only_one_run_as_a_change() {
    let first = vec![
        graded("A", Tier::Clean, "sound", "sound", Verdict::Primary, None),
        graded("B", Tier::Clean, "sound", "sound", Verdict::Primary, None),
    ];
    let second = vec![
        graded("A", Tier::Clean, "sound", "sound", Verdict::Primary, None),
        graded(
            "C",
            Tier::Clean,
            "sound",
            "unfalsifiable",
            Verdict::Wrong,
            None,
        ),
    ];
    // ids A, B, C: A agrees, B missing from the right, C missing from the left.
    let rate = disagreement(&first, &second).unwrap();
    assert!((rate - 66.666_666_666_666_67).abs() < 1e-9, "got {rate}");

    assert!(
        disagreement(&[], &[]).is_none(),
        "a disagreement rate over nothing is not 0%"
    );
}

/// Provenance: PLAT-917. Two byte-identical runs disagree on nothing. The
/// floor case, so a broken comparator cannot report a flattering number.
#[test]
fn two_identical_runs_disagree_on_nothing() {
    let run = vec![
        graded(
            "A",
            Tier::Clean,
            "sound",
            "sound",
            Verdict::Primary,
            Some(0.9),
        ),
        graded(
            "B",
            Tier::Ambiguous,
            "unfalsifiable",
            "sound",
            Verdict::Contested,
            Some(0.6),
        ),
    ];
    assert!((disagreement(&run, &run).unwrap() - 0.0).abs() < 1e-9);
}
