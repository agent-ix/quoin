// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Runs the PLAT-933 adequacy checks and PLAT-982's answerability check against the real, shipped
//! criterion-strength corpus -- the same file the live-evaluation dogfood
//! (PLAT-917) reads through `include_str!` -- so a stale count or a missing
//! required field in that file fails `make rust-gate`, offline, before
//! anyone runs the crate's `live-api` suite against it for real.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_jev::{QuestionSet, check_answerability, check_required_fields, check_stated_counts};
use serde_json::Value;

const CORPUS: &str = include_str!(
    "../../../../skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fixtures.json"
);

/// The question set the corpus's labels answer, read from the shipped asset
/// so the closed answer spaces are never restated here.
const QUESTION_SET: &str =
    include_str!("../../../../skills/spec-criterion-strength-analysis/assets/question-set.json");

/// Provenance: PLAT-982. Every `weakness_kind` label (and each contested
/// second reading) is a member of `question-set.json`'s `choice.answer_space`,
/// and every `adverse_case_coverage` label (and contested reading) is one of
/// its `score.rubric` levels. A label the lens could never emit makes every
/// accuracy number computed against it wrong, silently.
#[test]
fn the_shipped_corpus_labels_are_legal_answers_to_their_questions() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("the shipped corpus parses");
    let set = QuestionSet::parse(QUESTION_SET).expect("the shipped question set parses");

    let kinds: Vec<Value> = set
        .choice
        .answer_space
        .iter()
        .map(|kind| Value::from(kind.as_str()))
        .collect();
    let mut findings = check_answerability(
        &corpus["weakness_kind_fixtures"],
        &["labels", set.choice.id.as_str()],
        &kinds,
    );

    let levels: Vec<Value> = set
        .score
        .rubric
        .iter()
        .map(|level| Value::from(level.level))
        .collect();
    findings.extend(check_answerability(
        &corpus["adverse_case_coverage_fixtures"],
        &["labels", set.score.id.as_str()],
        &levels,
    ));

    assert!(
        findings.is_empty(),
        "the shipped corpus carries labels outside their question's answer space: {findings:?}"
    );
}

/// Provenance: PLAT-933. The corpus's own `governing_ruling_on_disagreement`
/// used to say "disputed 5 of 14 (agreed 9)" while the fixtures held 9 of 15
/// contested -- the exact incident this ticket exists to catch before a live
/// run does. Fixed in this same change (9 of 15, agreed 6); this asserts it
/// stays fixed.
#[test]
fn the_shipped_corpus_states_counts_that_match_its_own_data() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("the shipped corpus parses");
    let findings = check_stated_counts(&corpus);
    assert!(
        findings.is_empty(),
        "the shipped corpus's stated counts no longer match its data: {findings:?}"
    );
}

/// Provenance: PLAT-933. Every weakness-kind fixture's `source.fr_description`
/// feeds `FrContext::statement` directly (`context()` in
/// `tests/support/mod.rs` on the PLAT-917 branch, via `unwrap_or_default()`).
/// 10 of the 11 shipped fixtures do not carry it -- the exact gap that sent
/// empty sentences to Jev in round one and was only found mid-run. Pinned
/// rather than silently accepted: this crate does not yet carry the
/// full-context sidecar that round two adds on the PLAT-917 branch, so the
/// honest state today is "10 known missing, tracked" -- and this fails
/// loudly, not silently, the moment that count moves in either direction.
#[test]
fn the_shipped_corpus_is_missing_fr_description_on_ten_of_eleven_fixtures() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("the shipped corpus parses");
    let fixtures = corpus["weakness_kind_fixtures"]
        .as_array()
        .expect("weakness_kind_fixtures is an array");

    let sources: Value = Value::Object(
        fixtures
            .iter()
            .map(|fixture| {
                let id = fixture["fixture_id"]
                    .as_str()
                    .expect("every fixture has an id")
                    .to_owned();
                (id, fixture["source"].clone())
            })
            .collect(),
    );

    let findings = check_required_fields(&sources, &["fr_description"]);
    assert_eq!(
        findings.len(),
        10,
        "expected the known 10-of-11 gap; PLAT-917's context sidecar landing should shrink \
         this count, at which point lower the pin rather than raise it -- got: {findings:?}"
    );
}
