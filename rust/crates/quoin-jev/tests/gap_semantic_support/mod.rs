// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Corpus, request-building and grading maths for the gap-analysis semantic
//! lens evaluation (PLAT-839).
//!
//! This is a DIFFERENT lens shape from `quoin_jev`'s existing
//! criterion-strength lens (PLAT-837/PLAT-917, see `../support/mod.rs`):
//! criterion-strength reads spec text only (`FrContext`: an FR statement and
//! its AC rows). PLAT-839's gap-analysis lens additionally reads a tagged
//! test's body and the covered symbol's body -- "one request per
//! requirement<->test<->code triple" per the ticket's own question-set
//! section. [`GapTriple`] is that triple; nothing in `quoin_jev`'s public API
//! models it, and this module does not add it there -- PLAT-839's standing
//! ruling is an EVALUATION, not production wiring, so this stays under
//! `tests/` and is never reachable from the crate's `src/`.
//!
//! # The corpus
//!
//! [`CORPUS`] is 29 triples: 27 real ones pulled from this repository's own
//! `Trace:`-tagged Rust tests (`rust/crates/quoin-core` and five sibling
//! crates) against their owning `spec/functional/FR-*.md` documents, plus two
//! (`GAP-28`, `GAP-29`) constructed for this evaluation per PLAT-839's own
//! acceptance criteria -- "a test that reaches the code and asserts nothing"
//! -- and labelled as constructed in their own `note` rather than presented
//! as pre-existing quoin tests. `mutation_vacuous` is `null` on six rows this
//! crate excluded from grading (a generic function, two re-exported
//! constants, an enum-completeness guard, and a two-symbol triple -- each
//! documented in its own `mutation_method`) and is otherwise filled in by
//! hand after a scratch mutation run (stub the covered symbol, rerun exactly
//! that test, record pass/fail) documented in this crate's PR description,
//! per the ticket's M7: "checked against the compiler and the test runner,"
//! not a human labeller. 23 of the 29 rows carry ground truth; 3 of those 23
//! are confirmed vacuous.
//!
//! # Two pre-registered variants, no more
//!
//! [`Variant::Solo`] asks only `assertion_vacuous`. [`Variant::FullBattery`]
//! asks it alongside the ticket's other four `noul` questions plus
//! `divergence_kind` (choice) and `severity` (score) -- the complete
//! question-set shape the ticket describes, sent as one batched request per
//! triple. Both are fixed here, before the first live call, so the live test
//! file's bars in its own module doc govern both runs; neither is added or
//! dropped after seeing a result.

#![allow(
    dead_code,
    reason = "each test binary links this module separately, so items only one binary uses read as dead in the other"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Deserialize;
use typesafe_sdk_answers::{Answer, SystemOneResponse};
use typesafe_sdk_client::SystemOneRequest;
use typesafe_sdk_questions::{Entry, Questions, choice_of, noul, questions, score};

/// The corpus, compiled in so the fixture and this grader cannot drift (the
/// same discipline `../support/mod.rs` already holds for PLAT-917).
pub(crate) const CORPUS: &str = include_str!("../fixtures/gap-semantic-corpus.json");

/// One requirement<->test<->code triple, exactly the state PLAT-839's
/// question-set section asks be sent: "the FR statement and its AC row, the
/// tagged test body, the covered symbol body."
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GapTriple {
    pub(crate) id: String,
    pub(crate) fr_id: String,
    pub(crate) fr_statement: String,
    pub(crate) ac_id: String,
    pub(crate) ac_text: String,
    pub(crate) test_file: String,
    pub(crate) test_fn_name: String,
    pub(crate) test_body: String,
    pub(crate) symbol_name: String,
    pub(crate) symbol_file: String,
    pub(crate) symbol_body: String,
    /// The corpus author's own (non-mechanical) read on whether this test
    /// looks solid or thin -- a hint for a human reader, never scored against.
    pub(crate) note: String,
    /// Mechanical ground truth: `Some(true)` when the covered symbol was
    /// stubbed and `test_fn_name` still passed (the test WAS vacuous),
    /// `Some(false)` when the stub made it fail, `None` when this triple was
    /// not part of the mutated subset.
    pub(crate) mutation_vacuous: Option<bool>,
    /// How the mutation was performed, for a reader auditing the ground
    /// truth itself (e.g. "stubbed to `Default::default()`", "constant
    /// literal changed"). `None` alongside `mutation_vacuous: None`.
    pub(crate) mutation_method: Option<String>,
}

/// Parses the compiled-in corpus.
///
/// # Panics
/// If the fixture stops matching this shape -- deliberate, per the same
/// discipline `../support/mod.rs::corpus` documents.
pub(crate) fn corpus() -> Vec<GapTriple> {
    serde_json::from_str(CORPUS).expect("the gap-semantic corpus parses")
}

/// Only the rows a mutation run actually graded -- the M7 denominator.
pub(crate) fn mutated_corpus() -> Vec<GapTriple> {
    corpus()
        .into_iter()
        .filter(|triple| triple.mutation_vacuous.is_some())
        .collect()
}

/// The pre-registered request shapes (see this module's doc, and
/// `../live_gap_battery.rs` for `FullBatteryV1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Variant {
    /// `assertion_vacuous` asked alone.
    Solo,
    /// The full noul battery + `divergence_kind` + `severity`, one call.
    FullBattery,
    /// `FullBattery` with two questions reworded (PLAT-979): the two whose
    /// error analysis found the question text asking something other than
    /// what its ground truth measures. Every other question is identical.
    FullBatteryV1,
}

/// `test_asserts_intent` as the ticket worded it (`FullBattery`).
const TEST_ASSERTS_INTENT_V0: &str = "Does the test assert the behaviour the requirement states, \
                                      rather than merely invoking it?";

/// `test_asserts_intent` reworded for `FullBatteryV1` as the counterfactual its
/// ground truth checks: a targeted mutant contradicting the stated behaviour,
/// and whether the owning test fails. "Rather than merely invoking it" sets
/// the bar at any assertion at all, which a partial test clears.
const TEST_ASSERTS_INTENT_V1: &str = "Suppose the code were changed so that it no longer does \
     the specific thing the requirement states, while still compiling and still returning a \
     well-formed, non-default result. Would this test fail? Judge against the requirement's \
     own stated behaviour, not against other things the code does.";

/// `tests_only_its_own_mock` as the ticket worded it (`FullBattery`).
const TESTS_ONLY_ITS_OWN_MOCK_V0: &str =
    "Do the assertions check only values the test itself configured?";

/// `tests_only_its_own_mock` reworded for `FullBatteryV1` to state the
/// definition its static-check ground truth applies. "Configured" is
/// ambiguous: every test writes its own inputs and its own expected values.
const TESTS_ONLY_ITS_OWN_MOCK_V1: &str = "Does every assertion compare only against values the \
     test already supplied as input, to the code or to a fake, mock or fixture it set up, so that \
     no assertion states an independent expected value (a constant, enum variant, error code, \
     count or literal that does not appear among the test's own inputs)? Answer no if the test \
     makes no assertions.";

/// The wire key `assertion_vacuous`'s answer is sent under, in both variants.
pub(crate) const ASSERTION_VACUOUS_KEY: &str = "assertion_vacuous";
/// The wire key `divergence_kind`'s answer is sent under (`FullBattery` only).
pub(crate) const DIVERGENCE_KIND_KEY: &str = "divergence_kind";
/// The wire key `severity`'s answer is sent under (`FullBattery` only).
pub(crate) const SEVERITY_KEY: &str = "severity";

/// `divergence_kind`'s closed answer space, verbatim from the ticket.
pub(crate) const DIVERGENCE_KINDS: [&str; 6] = [
    "aligned",
    "test_weaker_than_requirement",
    "test_stronger_than_requirement",
    "code_short_of_requirement",
    "code_exceeds_requirement",
    "requirement_ambiguous",
];

/// `severity`'s rubric, verbatim from the ticket's verdict-rule mapping:
/// "high -> FAIL, medium/low -> CONDITIONAL." `none` is added as the fourth,
/// lowest rung for a triple with no divergence at all -- the ticket's rubric
/// only names the three that already map to a verdict.
pub(crate) const SEVERITY_RUBRIC: [&str; 4] = ["none", "low", "medium", "high"];

/// The `state` object sent as this triple's context -- exactly the six
/// fields the ticket's question-set section names, nothing else.
fn state(triple: &GapTriple) -> serde_json::Value {
    serde_json::json!({
        "fr_id": triple.fr_id,
        "fr_statement": triple.fr_statement,
        "ac_id": triple.ac_id,
        "ac_text": triple.ac_text,
        "test_file": triple.test_file,
        "test_fn_name": triple.test_fn_name,
        "test_body": triple.test_body,
        "symbol_name": triple.symbol_name,
        "symbol_file": triple.symbol_file,
        "symbol_body": triple.symbol_body,
    })
}

/// Builds the questions for `variant`, verbatim question text from the
/// ticket's `noul` table and `divergence_kind`/severity sections.
pub(crate) fn question_set(variant: Variant) -> Questions {
    match variant {
        Variant::Solo => questions([(
            ASSERTION_VACUOUS_KEY,
            noul(
                "Would this test still pass if the implementation were replaced \
                 with a stub returning a default?",
            ),
        )]),
        Variant::FullBattery | Variant::FullBatteryV1 => battery(variant == Variant::FullBatteryV1),
    }
}

/// The seven-question battery; `reworded` swaps in the two `FullBatteryV1`
/// question texts and changes nothing else.
fn battery(reworded: bool) -> Questions {
    let (asserts_intent, only_its_own_mock) = if reworded {
        (TEST_ASSERTS_INTENT_V1, TESTS_ONLY_ITS_OWN_MOCK_V1)
    } else {
        (TEST_ASSERTS_INTENT_V0, TESTS_ONLY_ITS_OWN_MOCK_V0)
    };
    questions([
        ("test_asserts_intent", noul(asserts_intent)),
        (
            ASSERTION_VACUOUS_KEY,
            noul(
                "Would this test still pass if the implementation were \
                     replaced with a stub returning a default?",
            ),
        ),
        ("tests_only_its_own_mock", noul(only_its_own_mock)),
        (
            "code_implements_intent",
            noul("Does the code do what the requirement says?"),
        ),
        (
            "code_exceeds_requirement",
            noul("Does the code implement behaviour no requirement states?"),
        ),
        (
            DIVERGENCE_KIND_KEY,
            choice_of(
                "Classify how the test, the code and the requirement diverge, if at all.",
                DIVERGENCE_KINDS,
            ),
        ),
        (
            SEVERITY_KEY,
            score(
                "Rate the severity of any divergence found, against the rubric.",
                SEVERITY_RUBRIC,
            ),
        ),
    ])
}

/// Builds the request for one triple under `variant`, without sending it.
pub(crate) fn build_request(triple: &GapTriple, variant: Variant) -> SystemOneRequest {
    let entry: Entry = state(triple).into();
    SystemOneRequest::new(entry, question_set(variant))
}

/// One triple's `assertion_vacuous` answer, graded against mutation truth.
#[derive(Debug, Clone)]
pub(crate) struct Graded {
    pub(crate) id: String,
    pub(crate) truth: bool,
    /// The raw `noul` probability that the test is vacuous.
    pub(crate) predicted_prob: f64,
    /// `predicted_prob >= 0.5`.
    pub(crate) predicted: bool,
    /// `divergence_kind`, when the variant asked for it.
    pub(crate) divergence_kind: Option<String>,
    /// `severity`'s rounded rubric label, when the variant asked for it.
    pub(crate) severity: Option<String>,
}

/// The `SpecReview` verdict PLAT-839's rubric maps `severity` to: `high` ->
/// `"FAIL"`, `medium`/`low` -> `"CONDITIONAL"`, anything else (`none`, or no
/// severity asked) -> `"PASS"`. A per-triple proxy for the real audit-level
/// `Verdict`, which this evaluation does not aggregate -- see the M6 measure
/// in `../../tests/live_gap_semantic.rs` for why that distinction matters.
pub(crate) fn severity_verdict(severity: Option<&str>) -> &'static str {
    match severity {
        Some("high") => "FAIL",
        Some("medium" | "low") => "CONDITIONAL",
        _ => "PASS",
    }
}

/// Extracts `assertion_vacuous` (and, if present, `divergence_kind` and
/// `severity`) from `response` and grades against `triple.mutation_vacuous`.
///
/// # Panics
/// If `triple.mutation_vacuous` is `None` -- callers must filter to
/// [`mutated_corpus`] first; grading a row with no ground truth would be a
/// silent `None`-as-`false`, not a missing measurement.
pub(crate) fn grade(triple: &GapTriple, response: &SystemOneResponse) -> Option<Graded> {
    let truth = triple.mutation_vacuous.expect(
        "grade() called on a triple with no mutation ground truth -- filter with mutated_corpus() first",
    );
    let Some(Answer::Noul(answer)) = response.answer(ASSERTION_VACUOUS_KEY) else {
        return None;
    };
    let predicted_prob = answer.noul;
    let divergence_kind = match response.answer(DIVERGENCE_KIND_KEY) {
        Some(Answer::Choice(choice)) => Some(choice.choice.clone()),
        _ => None,
    };
    let severity = match response.answer(SEVERITY_KEY) {
        Some(Answer::Score(score)) => nearest_rubric_label(score.score),
        _ => None,
    };
    Some(Graded {
        id: triple.id.clone(),
        truth,
        predicted_prob,
        predicted: predicted_prob >= 0.5,
        divergence_kind,
        severity,
    })
}

/// The rubric label nearest a continuous score, or `None` outside `[0, 3]`.
pub(crate) fn nearest_rubric_label(raw: f64) -> Option<String> {
    if !(-0.5..3.5).contains(&raw) {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "raw is checked to be within [-0.5, 3.5) immediately above"
    )]
    let level = raw.round().clamp(0.0, 3.0) as usize;
    SEVERITY_RUBRIC.get(level).map(|label| (*label).to_owned())
}

/// Confusion counts for the vacuous class (positive = `predicted == true`).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BinaryStats {
    pub(crate) true_positive: usize,
    pub(crate) false_positive: usize,
    pub(crate) true_negative: usize,
    pub(crate) false_negative: usize,
}

impl BinaryStats {
    pub(crate) const fn total(&self) -> usize {
        self.true_positive + self.false_positive + self.true_negative + self.false_negative
    }

    pub(crate) fn accuracy(&self) -> Option<f64> {
        let total = self.total();
        (total > 0).then(|| percent(self.true_positive + self.true_negative, total))
    }

    /// Of rows Jev called vacuous, the share the mutation run confirmed.
    pub(crate) fn precision(&self) -> Option<f64> {
        let predicted_positive = self.true_positive + self.false_positive;
        (predicted_positive > 0).then(|| percent(self.true_positive, predicted_positive))
    }

    /// Of rows mutation confirmed vacuous, the share Jev flagged (defect
    /// recall for this class).
    pub(crate) fn recall(&self) -> Option<f64> {
        let actual_positive = self.true_positive + self.false_negative;
        (actual_positive > 0).then(|| percent(self.true_positive, actual_positive))
    }

    /// Of rows mutation confirmed NOT vacuous, the share Jev correctly called
    /// not-vacuous. PLAT-917's own lesson: a lens that never answers "not
    /// vacuous" has zero recall here regardless of how it scores otherwise.
    pub(crate) fn no_defect_recall(&self) -> Option<f64> {
        let actual_negative = self.true_negative + self.false_positive;
        (actual_negative > 0).then(|| percent(self.true_negative, actual_negative))
    }
}

pub(crate) fn tally(graded: &[Graded]) -> BinaryStats {
    let mut stats = BinaryStats::default();
    for row in graded {
        match (row.predicted, row.truth) {
            (true, true) => stats.true_positive += 1,
            (true, false) => stats.false_positive += 1,
            (false, true) => stats.false_negative += 1,
            (false, false) => stats.true_negative += 1,
        }
    }
    stats
}

/// The best constant predictor this graded set admits: always-vacuous or
/// always-not, whichever agrees with mutation truth more often. Computed from
/// the data rather than written down, so it cannot go stale (same discipline
/// as `../support/mod.rs::trivial_baseline`).
pub(crate) fn constant_baseline(graded: &[Graded]) -> (&'static str, f64) {
    if graded.is_empty() {
        return ("<none>", 0.0);
    }
    let vacuous = graded.iter().filter(|row| row.truth).count();
    let not_vacuous = graded.len() - vacuous;
    if vacuous >= not_vacuous {
        ("always vacuous", percent(vacuous, graded.len()))
    } else {
        ("always not-vacuous", percent(not_vacuous, graded.len()))
    }
}

/// Expected calibration error over `assertion_vacuous`'s own stated
/// probability: buckets rows by `predicted_prob` decile and compares the
/// bucket's mean stated probability against the observed fraction of
/// mutation-confirmed-vacuous rows in it. This is calibration of the raw
/// `noul` value directly -- distinct from `../support/mod.rs`'s ECE, which
/// buckets by confidence-in-the-predicted-label because `weakness_kind`
/// carries a separate `confidence` field a bare `noul` does not have.
pub(crate) fn ece(graded: &[Graded]) -> Option<f64> {
    if graded.is_empty() {
        return None;
    }
    let mut buckets: Vec<(f64, usize, usize)> = vec![(0.0, 0, 0); 10]; // (prob sum, vacuous count, total)
    for row in graded {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "predicted_prob is a noul answer in [0, 1]; the .min(9) bound \
                      below covers the boundary value 1.0 rounding up to bucket 10"
        )]
        let index = ((row.predicted_prob * 10.0) as usize).min(9);
        buckets[index].0 += row.predicted_prob;
        if row.truth {
            buckets[index].1 += 1;
        }
        buckets[index].2 += 1;
    }
    let total = graded.len();
    let as_f64 = |count: usize| f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    let error: f64 = buckets
        .iter()
        .filter(|(_, _, count)| *count > 0)
        .map(|(prob_sum, vacuous, count)| {
            let mean_predicted = prob_sum / as_f64(*count);
            let observed = as_f64(*vacuous) / as_f64(*count);
            let weight = as_f64(*count) / as_f64(total);
            weight * (mean_predicted - observed).abs()
        })
        .sum();
    Some(error)
}

/// The share of triples whose `predicted` bool differs between two runs over
/// the same corpus (M1), keyed by triple id.
pub(crate) fn disagreement(left: &[Graded], right: &[Graded]) -> Option<f64> {
    let index = |rows: &[Graded]| {
        rows.iter()
            .map(|row| (row.id.clone(), row.predicted))
            .collect::<BTreeMap<_, _>>()
    };
    let (left, right) = (index(left), index(right));
    let mut ids: Vec<&String> = left.keys().chain(right.keys()).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return None;
    }
    let changed = ids
        .iter()
        .filter(|id| left.get(**id) != right.get(**id))
        .count();
    Some(percent(changed, ids.len()))
}

/// The share of triples whose `severity_verdict` (M6's per-triple proxy)
/// differs between two runs. `None` when a row never carried a severity
/// (Solo variant).
pub(crate) fn verdict_disagreement(left: &[Graded], right: &[Graded]) -> Option<f64> {
    let index = |rows: &[Graded]| {
        rows.iter()
            .filter(|row| row.severity.is_some())
            .map(|row| (row.id.clone(), severity_verdict(row.severity.as_deref())))
            .collect::<BTreeMap<_, _>>()
    };
    let (left, right) = (index(left), index(right));
    let mut ids: Vec<&String> = left.keys().chain(right.keys()).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return None;
    }
    let changed = ids
        .iter()
        .filter(|id| left.get(**id) != right.get(**id))
        .count();
    Some(percent(changed, ids.len()))
}

pub(crate) fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    // `u32::try_from` + `f64::from`, not `as f64`: a `u32` fits exactly in an
    // `f64` mantissa, so this loses no precision a raw cast could hide. Same
    // idiom `../support/mod.rs::percent` already uses.
    let part = f64::from(u32::try_from(part).unwrap_or(u32::MAX));
    let whole = f64::from(u32::try_from(whole).unwrap_or(u32::MAX));
    part / whole * 100.0
}

/// Renders the per-triple table plus the M7 rollup.
pub(crate) fn report(title: &str, graded: &[Graded]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n## {title}\n");
    let _ = writeln!(
        out,
        "| Triple | Truth (vacuous?) | P(vacuous) | Predicted | Correct |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for row in graded {
        let _ = writeln!(
            out,
            "| {} | {} | {:.2} | {} | {} |",
            row.id,
            row.truth,
            row.predicted_prob,
            row.predicted,
            row.predicted == row.truth,
        );
    }

    let stats = tally(graded);
    let (baseline_label, baseline) = constant_baseline(graded);
    let _ = writeln!(
        out,
        "\n**M7 — assertion_vacuous vs. mutation truth** ({} rows):\n",
        graded.len()
    );
    let _ = writeln!(
        out,
        "- agreement: {}",
        stats
            .accuracy()
            .map_or_else(|| "n/a".to_owned(), |v| format!("{v:.1}%"))
    );
    let _ = writeln!(
        out,
        "- constant-predictor baseline: `{baseline_label}` scores {baseline:.1}%"
    );
    let _ = writeln!(
        out,
        "- vacuous-class precision: {}",
        stats
            .precision()
            .map_or_else(|| "n/a".to_owned(), |v| format!("{v:.1}%"))
    );
    let _ = writeln!(
        out,
        "- vacuous-class recall (defect recall): {}",
        stats
            .recall()
            .map_or_else(|| "n/a".to_owned(), |v| format!("{v:.1}%"))
    );
    let _ = writeln!(
        out,
        "- not-vacuous-class recall (no-defect recall): {}",
        stats
            .no_defect_recall()
            .map_or_else(|| "n/a".to_owned(), |v| format!("{v:.1}%"))
    );
    let _ = writeln!(
        out,
        "- confusion: TP={} FP={} TN={} FN={}",
        stats.true_positive, stats.false_positive, stats.true_negative, stats.false_negative
    );
    let _ = writeln!(
        out,
        "- ECE (on the raw `noul` probability): {}",
        ece(graded).map_or_else(|| "n/a".to_owned(), |v| format!("{v:.4}"))
    );
    out
}
