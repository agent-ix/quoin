// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The labelled fixture corpus, and the grading maths over it (PLAT-917).
//!
//! `skills/spec-criterion-strength-analysis/assets/fixtures/` holds 15 real,
//! human-labelled acceptance criteria drawn from this org's own specs. Until
//! this module existed nothing read it: the corpus was ground truth for a
//! grade nobody computed (PLAT-917's own premise).
//!
//! Everything here is offline. It parses the corpus, turns a fixture into the
//! [`FrContext`] the lens already takes, and scores a returned verdict against
//! the recorded label. No network, no key -- `grading_math.rs` exercises all
//! of it in the default gate, so the arithmetic that interprets a live run is
//! itself under test rather than trusted.
//!
//! # Contested labels are graded both ways, never resolved
//!
//! Nine of the fifteen fixtures carry a `*_contested` array -- seven
//! `weakness_kind` and two `adverse_case_coverage`: two readers read the
//! criterion differently and the corpus records both rather than picking one.
//! (The corpus's own `governing_ruling_on_disagreement` says "5 of 14"; the
//! files say 9 of 15, and `grading_math.rs` pins the measured count.) Grading
//! against the first reader alone would silently convert a known-ambiguous
//! case into a confident answer key. [`Graded::verdict`] therefore reports
//! `Primary`, `Contested` and `Wrong` as three distinct outcomes, and the
//! report prints them separately.

#![allow(
    dead_code,
    reason = "each test binary links this module separately, so items only one binary uses read as dead in the other"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "test-only statistics: counts are at most a few hundred and confidences lie in [0, 1], \
              so no cast here can truncate, lose a sign, or lose precision"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Deserialize;

use quoin_jev::{AcRow, FrContext, FrVerdict};

/// The corpus, compiled in from the skill asset so the fixtures and this
/// grader cannot drift -- the same `include_str!` discipline
/// `question-set.json` is already held to (`question_set.rs`, `lens.rs`,
/// `verdict.rs`).
const CORPUS: &str = include_str!(
    "../../../../../skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fixtures.json"
);

/// The FR context each fixture's label was made against, extracted verbatim
/// from the spec file the fixture cites (PLAT-917).
///
/// Separate from [`CORPUS`] so the labelled answer key stays byte-identical;
/// this file adds input, never a label. The corpus alone left `statement`
/// empty on 10 of 11 criteria, so the first live run judged sentences in an
/// isolation the human readers never had.
const FR_CONTEXT: &str = include_str!(
    "../../../../../skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fr-context.json"
);

/// One fixture's full FR context.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FullContext {
    /// The FR's normative sentence: the first `SHALL` paragraph of its
    /// Statement or Description section.
    pub(crate) statement: Option<String>,
    /// The FR's Description (an NFR's Statement), verbatim.
    pub(crate) description: Option<String>,
    /// The FR's Behavior section, verbatim, when it has one.
    pub(crate) behavior: Option<String>,
    /// The FR's Constraints section, verbatim, when it has one.
    pub(crate) constraints: Option<String>,
    /// Informational provenance only. Nothing resolves it.
    pub(crate) extracted_from_commit: String,
}

#[derive(Deserialize)]
struct FullContextFile {
    fixtures: BTreeMap<String, FullContext>,
}

/// The full FR context for one fixture.
///
/// # Panics
///
/// When the sidecar has no entry for `fixture_id` -- a fixture added to the
/// corpus without its context would otherwise be graded on an empty input
/// again, silently.
pub(crate) fn full_context(fixture_id: &str) -> FullContext {
    let file: FullContextFile =
        serde_json::from_str(FR_CONTEXT).expect("the FR context sidecar parses");
    file.fixtures.get(fixture_id).cloned().unwrap_or_else(|| {
        panic!("{fixture_id} has no entry in criterion-strength-fr-context.json")
    })
}

/// Replaces a context's FR prose with the full extracted sections.
fn with_full_prose(mut context: FrContext, fixture_id: &str) -> FrContext {
    let full = full_context(fixture_id);
    context.statement = full.statement.unwrap_or_default();
    context.description = full.description;
    context.behaviour = full.behavior;
    context.constraints = full.constraints;
    context
}

/// How sure the corpus is of its own label.
///
/// `ambiguous` is not a defect in the fixture: PLAT-837 requires cases "the
/// lens is expected to get *wrong-ish*", because a corpus of only clean
/// positives and negatives proves nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tier {
    /// The label is not seriously contested.
    Clean,
    /// The lens is expected to find this hard.
    Ambiguous,
}

/// Where a fixture's criterion came from, verbatim.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Source {
    /// The repository the criterion lives in.
    pub(crate) repo: String,
    /// The spec file's path within that repository.
    pub(crate) path: String,
    /// The AC row id, on a `weakness_kind` fixture.
    pub(crate) criterion_id: Option<String>,
    /// The FR id, on an `adverse_case_coverage` fixture.
    pub(crate) fr_id: Option<String>,
    /// The owning FR's Description section, when the corpus captured it.
    pub(crate) fr_description: Option<String>,
    /// The owning FR's Behavior section, when the corpus captured it.
    pub(crate) fr_behavior: Option<String>,
    /// The single Behavior bullet this criterion answers to, when recorded.
    pub(crate) fr_behavior_bullet: Option<String>,
    /// The owning FR's Constraints section, when the corpus captured it.
    pub(crate) fr_constraint: Option<String>,
}

impl Source {
    /// The FR id to send, however the fixture happens to record it.
    ///
    /// A `weakness_kind` fixture names `criterion_id` (`FR-004-AC-2`) and no
    /// `fr_id`; the FR is its prefix. Deriving it here rather than at three
    /// call sites keeps the two fixture families building the same context.
    fn fr_id(&self) -> String {
        if let Some(fr_id) = &self.fr_id {
            return fr_id.clone();
        }
        let criterion = self.criterion_id.as_deref().unwrap_or_default();
        criterion
            .find("-AC-")
            .map_or_else(|| criterion.to_owned(), |cut| criterion[..cut].to_owned())
    }

    /// The Behavior text to send: the specific bullet when the corpus
    /// recorded one, else the whole section.
    fn behaviour(&self) -> Option<String> {
        self.fr_behavior_bullet
            .clone()
            .or_else(|| self.fr_behavior.clone())
    }
}

/// The human labels on one `weakness_kind` fixture.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeaknessLabels {
    /// The reader's `weakness_kind` verdict.
    pub(crate) weakness_kind: String,
    /// Both readings, when two readers disagreed. Includes
    /// [`Self::weakness_kind`].
    pub(crate) weakness_kind_contested: Option<Vec<String>>,
    /// The reader's answer to `falsifiable`, where they recorded one.
    pub(crate) falsifiable: Option<bool>,
    /// The reader's answer to `states_observable_outcome`.
    pub(crate) states_observable_outcome: Option<bool>,
    /// The reader's answer to `threshold_present`.
    pub(crate) threshold_present: Option<bool>,
    /// The reader's answer to `restates_requirement`.
    pub(crate) restates_requirement: Option<bool>,
    /// The reader's answer to `implementation_coupled`.
    pub(crate) implementation_coupled: Option<bool>,
}

impl WeaknessLabels {
    /// Every reading a reader recorded, primary first.
    fn readings(&self) -> Vec<&str> {
        self.weakness_kind_contested.as_ref().map_or_else(
            || vec![self.weakness_kind.as_str()],
            |all| all.iter().map(String::as_str).collect(),
        )
    }

    /// The `noul` ids the corpus recorded an answer for, with that answer.
    ///
    /// Three fixtures omit some ids; an omitted id is "this reader did not
    /// record one", never "false", so it is absent here rather than
    /// defaulted.
    pub(crate) fn noul(&self) -> Vec<(&'static str, bool)> {
        [
            ("falsifiable", self.falsifiable),
            ("states_observable_outcome", self.states_observable_outcome),
            ("threshold_present", self.threshold_present),
            ("restates_requirement", self.restates_requirement),
            ("implementation_coupled", self.implementation_coupled),
        ]
        .into_iter()
        .filter_map(|(id, value)| value.map(|value| (id, value)))
        .collect()
    }
}

/// One labelled acceptance criterion.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeaknessFixture {
    /// The corpus id, e.g. `CS-FIX-004`.
    pub(crate) fixture_id: String,
    /// How sure the corpus is of its label.
    pub(crate) confidence: Tier,
    /// Where the criterion came from.
    pub(crate) source: Source,
    /// The criterion's own text, unmodified.
    pub(crate) criterion_text: String,
    /// The human labels.
    pub(crate) labels: WeaknessLabels,
}

impl WeaknessFixture {
    /// [`Self::context`] with the full FR prose PLAT-837's request shape
    /// names, from [`full_context`].
    pub(crate) fn context_full(&self) -> FrContext {
        with_full_prose(self.context(), &self.fixture_id)
    }

    /// The fixture as the lens's own input type: one FR, one AC row.
    pub(crate) fn context(&self) -> FrContext {
        FrContext {
            fr_id: self.source.fr_id(),
            statement: self.source.fr_description.clone().unwrap_or_default(),
            description: self.source.fr_description.clone(),
            behaviour: self.source.behaviour(),
            constraints: self.source.fr_constraint.clone(),
            acceptance_criteria: vec![AcRow {
                id: self
                    .source
                    .criterion_id
                    .clone()
                    .unwrap_or_else(|| self.fixture_id.clone()),
                text: self.criterion_text.clone(),
            }],
        }
    }
}

/// The human labels on one `adverse_case_coverage` fixture.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CoverageLabels {
    /// The reader's 0-3 rubric level for the FR's whole AC set.
    pub(crate) adverse_case_coverage: u8,
    /// Both readings, when two readers disagreed.
    pub(crate) adverse_case_coverage_contested: Option<Vec<u8>>,
}

impl CoverageLabels {
    /// Every level a reader recorded, primary first.
    fn readings(&self) -> Vec<u8> {
        self.adverse_case_coverage_contested
            .clone()
            .unwrap_or_else(|| vec![self.adverse_case_coverage])
    }
}

/// One labelled AC set, scored at the FR level.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CoverageFixture {
    /// The corpus id, e.g. `CS-FIX-011`.
    pub(crate) fixture_id: String,
    /// How sure the corpus is of its label.
    pub(crate) confidence: Tier,
    /// Where the FR came from.
    pub(crate) source: Source,
    /// Every AC row under the FR, in document order.
    pub(crate) ac_set: Vec<Ac>,
    /// The human labels.
    pub(crate) labels: CoverageLabels,
}

/// One AC row inside a coverage fixture's `ac_set`.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Ac {
    /// The row's id.
    pub(crate) id: String,
    /// The row's text.
    pub(crate) text: String,
}

impl CoverageFixture {
    /// [`Self::context`] with the full FR prose, from [`full_context`].
    pub(crate) fn context_full(&self) -> FrContext {
        with_full_prose(self.context(), &self.fixture_id)
    }

    /// The fixture as the lens's own input type: one FR, its whole AC set.
    pub(crate) fn context(&self) -> FrContext {
        FrContext {
            fr_id: self.source.fr_id(),
            statement: self.source.fr_description.clone().unwrap_or_default(),
            description: self.source.fr_description.clone(),
            behaviour: self.source.behaviour(),
            constraints: self.source.fr_constraint.clone(),
            acceptance_criteria: self
                .ac_set
                .iter()
                .map(|ac| AcRow {
                    id: ac.id.clone(),
                    text: ac.text.clone(),
                })
                .collect(),
        }
    }
}

/// The whole corpus.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Corpus {
    /// The eleven per-criterion fixtures.
    pub(crate) weakness_kind_fixtures: Vec<WeaknessFixture>,
    /// The four FR-level coverage fixtures.
    pub(crate) adverse_case_coverage_fixtures: Vec<CoverageFixture>,
}

/// Parses the compiled-in corpus.
///
/// # Panics
/// If the asset stops matching these types -- which is the point: a fixture
/// field renamed upstream must break this loudly rather than grade against a
/// silently-defaulted label.
pub(crate) fn corpus() -> Corpus {
    serde_json::from_str(CORPUS).expect("the fixture corpus parses")
}

/// How a returned label compared to what the readers recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Matched the primary reading.
    Primary,
    /// Matched a second reader's recorded alternative, not the primary.
    Contested,
    /// Matched no recorded reading.
    Wrong,
    /// The lens returned a label outside the question set's `answer_space`.
    Unrecognized,
    /// The lens got no answer for this row at all.
    Unanswered,
}

impl Verdict {
    /// Whether this counts as agreement with the corpus.
    ///
    /// `Contested` counts: the corpus recorded that reading as defensible, so
    /// scoring it wrong would penalise the lens for a disagreement the
    /// readers themselves had.
    pub(crate) const fn agrees(self) -> bool {
        matches!(self, Self::Primary | Self::Contested)
    }
}

/// One fixture's graded result.
#[derive(Debug, Clone)]
pub(crate) struct Graded {
    /// The corpus id.
    pub(crate) fixture_id: String,
    /// How sure the corpus was.
    pub(crate) tier: Tier,
    /// The primary recorded label.
    pub(crate) expected: String,
    /// Every reading the corpus recorded, including [`Self::expected`].
    ///
    /// Carried on the row rather than looked up again from the corpus because
    /// [`trivial_baseline`] and [`defect_recall`] both need to know whether
    /// `sound` was an acceptable answer for this row, and re-deriving it at
    /// each call site is how the two would drift apart.
    pub(crate) contested: Vec<String>,
    /// What the lens returned, for display.
    ///
    /// On a coverage row this is the raw continuous score (`"1.80"`), which
    /// is deliberately not the same string as [`Self::actual_class`]: the
    /// rubric is 0-3 but the API answers an expected value that falls between
    /// levels when it is unsure, and rounding that away in the report would
    /// hide exactly the rows where the service was on the fence.
    pub(crate) actual: String,
    /// What the lens returned, as the class it is compared at.
    ///
    /// Equal to [`Self::actual`] on a `weakness_kind` row; the rounded rubric
    /// level on a coverage row. Every count keys off this and never off
    /// [`Self::actual`] -- when they were the same field, a coverage row
    /// scored `Primary` at level 2 from a raw `1.80` incremented `correct`
    /// for class `"2"` and `predicted` for class `"1.80"`, making `correct`
    /// exceed `predicted` and underflowing `false_positives`. Found by the
    /// first live run, not by review.
    pub(crate) actual_class: String,
    /// How the two compared.
    pub(crate) verdict: Verdict,
    /// The lens's reported confidence, where it carried one.
    ///
    /// `None` for a `sound` row: [`FrVerdict::sound`] is a bare list of AC
    /// ids and drops the confidence and probability distribution the response
    /// carried. That is a real limit on what can be calibrated here, recorded
    /// rather than worked around -- see `live_criterion_strength.rs`.
    pub(crate) confidence: Option<f64>,
}

/// Grades one `weakness_kind` fixture against the verdict the lens returned.
pub(crate) fn grade_weakness(fixture: &WeaknessFixture, verdict: &FrVerdict) -> Graded {
    let (actual, confidence) = if let Some(finding) = verdict.findings.first() {
        (finding.weakness_kind.clone(), Some(finding.confidence))
    } else if !verdict.sound.is_empty() {
        ("sound".to_owned(), None)
    } else if let Some((_, label)) = verdict.unrecognized.first() {
        (label.clone(), None)
    } else {
        ("<unanswered>".to_owned(), None)
    };

    let readings = fixture.labels.readings();
    let outcome = if !verdict.unrecognized.is_empty() {
        Verdict::Unrecognized
    } else if verdict.findings.is_empty() && verdict.sound.is_empty() {
        Verdict::Unanswered
    } else if actual == fixture.labels.weakness_kind {
        Verdict::Primary
    } else if readings.contains(&actual.as_str()) {
        Verdict::Contested
    } else {
        Verdict::Wrong
    };

    Graded {
        fixture_id: fixture.fixture_id.clone(),
        tier: fixture.confidence,
        expected: fixture.labels.weakness_kind.clone(),
        contested: readings.iter().map(|label| (*label).to_owned()).collect(),
        actual_class: actual.clone(),
        actual,
        verdict: outcome,
        confidence,
    }
}

/// Grades one `adverse_case_coverage` fixture.
///
/// The returned score is continuous -- PLAT-837's rubric is 0-3 but the API
/// answers an expected value that "falls between rubric levels when the model
/// is unsure" (`ScoreAnswer`'s own doc). It is compared at the nearest
/// integer level, and the raw value is kept in [`Graded::actual`] so a
/// half-level answer is visible rather than rounded away in the report.
pub(crate) fn grade_coverage(fixture: &CoverageFixture, verdict: &FrVerdict) -> Graded {
    let readings = fixture.labels.readings();
    let contested: Vec<String> = readings.iter().map(ToString::to_string).collect();
    let Some(coverage) = &verdict.coverage else {
        return Graded {
            fixture_id: fixture.fixture_id.clone(),
            tier: fixture.confidence,
            expected: fixture.labels.adverse_case_coverage.to_string(),
            contested,
            actual: "<unanswered>".to_owned(),
            actual_class: "<unanswered>".to_owned(),
            verdict: Verdict::Unanswered,
            confidence: None,
        };
    };

    let rounded = nearest_level(coverage.score);
    let outcome = if rounded == Some(fixture.labels.adverse_case_coverage) {
        Verdict::Primary
    } else if rounded.is_some_and(|level| readings.contains(&level)) {
        Verdict::Contested
    } else {
        Verdict::Wrong
    };

    Graded {
        fixture_id: fixture.fixture_id.clone(),
        tier: fixture.confidence,
        expected: fixture.labels.adverse_case_coverage.to_string(),
        contested,
        actual: format!("{:.2}", coverage.score),
        actual_class: rounded
            .map_or_else(|| "<out-of-rubric>".to_owned(), |level| level.to_string()),
        verdict: outcome,
        confidence: Some(coverage.confidence),
    }
}

/// The rubric level a continuous score rounds to, or `None` when it falls
/// outside the rubric's own 0-3 range.
///
/// Out of range is not clamped: a score of 4 means the response disagreed
/// with the rubric this crate sent, which is a finding about the call, not a
/// level 3.
fn nearest_level(score: f64) -> Option<u8> {
    if !(-0.5..3.5).contains(&score) {
        return None;
    }
    u8::try_from(score.round().max(0.0) as i64)
        .ok()
        .map(|level| level.min(3))
}

/// Counts over a graded set, split the ways the report needs them.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tally {
    /// Matched the primary reading.
    pub(crate) primary: usize,
    /// Matched a second reader's alternative.
    pub(crate) contested: usize,
    /// Matched no recorded reading.
    pub(crate) wrong: usize,
    /// Label outside the declared answer space.
    pub(crate) unrecognized: usize,
    /// No answer at all.
    pub(crate) unanswered: usize,
}

impl Tally {
    /// Every graded row counted.
    pub(crate) const fn total(&self) -> usize {
        self.primary + self.contested + self.wrong + self.unrecognized + self.unanswered
    }

    /// Rows where the lens agreed with some recorded reading.
    pub(crate) const fn agreed(&self) -> usize {
        self.primary + self.contested
    }

    /// Agreement as a percentage of every row, or `None` over an empty set.
    ///
    /// Unrecognized and unanswered rows are in the denominator deliberately:
    /// a lens that answers nothing has not scored 100%.
    pub(crate) fn agreement(&self) -> Option<f64> {
        let total = self.total();
        (total > 0).then(|| percent(self.agreed(), total))
    }
}

/// Tallies a graded set.
pub(crate) fn tally(graded: &[Graded]) -> Tally {
    let mut tally = Tally::default();
    for row in graded {
        match row.verdict {
            Verdict::Primary => tally.primary += 1,
            Verdict::Contested => tally.contested += 1,
            Verdict::Wrong => tally.wrong += 1,
            Verdict::Unrecognized => tally.unrecognized += 1,
            Verdict::Unanswered => tally.unanswered += 1,
        }
    }
    tally
}

/// Per-class precision and recall over a graded set, keyed by label.
///
/// PLAT-837's M2 requires both, and the false-positive count separately: "a
/// lens that flags everything has perfect recall and is worthless".
/// `predicted` and `expected` are counted against the primary reading only --
/// a per-class confusion matrix has no way to express "either of two labels
/// was acceptable", so contested rows are reported alongside rather than
/// folded in here.
#[derive(Debug, Clone, Default)]
pub(crate) struct ClassStats {
    /// Times the lens returned this label.
    pub(crate) predicted: usize,
    /// Times a reader recorded this label as primary.
    pub(crate) expected: usize,
    /// Times both agreed.
    pub(crate) correct: usize,
}

impl ClassStats {
    /// Of the rows the lens gave this label, the share a reader agreed with.
    pub(crate) fn precision(&self) -> Option<f64> {
        (self.predicted > 0).then(|| percent(self.correct, self.predicted))
    }

    /// Of the rows a reader gave this label, the share the lens found.
    pub(crate) fn recall(&self) -> Option<f64> {
        (self.expected > 0).then(|| percent(self.correct, self.expected))
    }

    /// Rows the lens gave this label and a reader did not.
    pub(crate) const fn false_positives(&self) -> usize {
        // `saturating_sub`, not `-`: the invariant that `correct <=
        // predicted` holds only while both are keyed off `actual_class`, and
        // a subtraction that panics in a measurement harness destroys the
        // whole run's numbers over one skewed row.
        self.predicted.saturating_sub(self.correct)
    }
}

/// Builds the per-class confusion counts.
pub(crate) fn class_stats(graded: &[Graded]) -> BTreeMap<String, ClassStats> {
    let mut stats: BTreeMap<String, ClassStats> = BTreeMap::new();
    for row in graded {
        stats.entry(row.actual_class.clone()).or_default().predicted += 1;
        stats.entry(row.expected.clone()).or_default().expected += 1;
        if row.verdict == Verdict::Primary {
            stats.entry(row.expected.clone()).or_default().correct += 1;
        }
    }
    stats
}

/// One confidence decile's accuracy, for the M3 calibration curve.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bucket {
    /// The decile's lower bound, e.g. `0.9` for 0.9-1.0.
    pub(crate) floor: f64,
    /// Rows whose reported confidence fell in this decile.
    pub(crate) count: usize,
    /// Of those, how many agreed with a recorded reading.
    pub(crate) correct: usize,
}

impl Bucket {
    /// Observed accuracy in this decile.
    pub(crate) fn accuracy(&self) -> Option<f64> {
        (self.count > 0).then(|| percent(self.correct, self.count))
    }
}

/// Buckets graded rows by reported confidence decile.
///
/// Rows carrying no confidence are excluded and their count returned
/// separately -- silently dropping them would make the calibration curve look
/// denser than the evidence behind it.
pub(crate) fn calibration(graded: &[Graded]) -> (Vec<Bucket>, usize) {
    let mut buckets: Vec<Bucket> = (0..10)
        .map(|decile| Bucket {
            floor: f64::from(decile) / 10.0,
            count: 0,
            correct: 0,
        })
        .collect();
    let mut without = 0usize;

    for row in graded {
        let Some(confidence) = row.confidence else {
            without += 1;
            continue;
        };
        let index = ((confidence * 10.0) as usize).min(9);
        buckets[index].count += 1;
        if row.verdict.agrees() {
            buckets[index].correct += 1;
        }
    }
    (buckets, without)
}

/// Expected calibration error: the count-weighted mean gap between a
/// decile's stated confidence and its observed accuracy.
///
/// Returns `None` when no row carried a confidence at all -- an ECE over
/// nothing is not zero.
pub(crate) fn expected_calibration_error(graded: &[Graded]) -> Option<f64> {
    let (buckets, _) = calibration(graded);
    let total: usize = buckets.iter().map(|bucket| bucket.count).sum();
    if total == 0 {
        return None;
    }
    let error: f64 = buckets
        .iter()
        .filter(|bucket| bucket.count > 0)
        .map(|bucket| {
            let stated = bucket.floor + 0.05;
            let observed = f64::from(u32::try_from(bucket.correct).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(bucket.count).unwrap_or(u32::MAX));
            let weight = f64::from(u32::try_from(bucket.count).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total).unwrap_or(u32::MAX));
            weight * (stated - observed).abs()
        })
        .sum();
    Some(error)
}

/// The best score a constant predictor can get on this corpus, and the label
/// that gets it.
///
/// # Why this exists, and why raw agreement is not the gate
///
/// MEASURED on the shipped corpus: answering `sound` to every one of the
/// eleven `weakness_kind` fixtures agrees with a recorded reading **9 times
/// out of 11 — 81.8%** — because `sound` is one of the two readings on six of
/// the seven contested rows. On the coverage fixtures, always answering level
/// 2 scores 75%.
///
/// So a lens that never flags anything scores 81.8% agreement and has found
/// nothing. That is the mirror image of the trap PLAT-837's M2 already names
/// ("a lens that flags everything has perfect recall and is worthless"), and
/// it means **an agreement percentage cannot be the correctness gate on this
/// corpus.** The gate is: beat this baseline, and have non-zero recall on the
/// classes that are not `sound` — actually find the defects the readers found.
///
/// Computed from the corpus rather than written down as a constant, so
/// changing the fixture set moves the bar automatically instead of leaving a
/// stale number that silently stops being a bar at all.
pub(crate) fn trivial_baseline(graded: &[Graded]) -> (String, f64) {
    // One constant per family, never one across both. The first version of
    // this function picked a single label over all fifteen rows, which scored
    // the constant predictor at 9/15 (60%) -- `sound` everywhere, earning
    // nothing on the coverage rows. But a constant predictor is free to answer
    // `sound` on a criterion and a fixed level on an FR, and doing so scores
    // higher. The single-label form understated the bar, in the lens's favour.
    let mut labels = Vec::new();
    let mut hits = 0;
    for family in [false, true] {
        let rows: Vec<&Graded> = graded
            .iter()
            .filter(|row| is_coverage(row) == family)
            .collect();
        let mut candidates: Vec<&String> = rows.iter().map(|row| &row.expected).collect();
        candidates.sort();
        candidates.dedup();
        let best = candidates
            .into_iter()
            .map(|label| {
                let count = rows
                    .iter()
                    .filter(|row| row.expected == *label || row.contested.contains(label))
                    .count();
                (label.clone(), count)
            })
            .max_by_key(|(_, count)| *count);
        if let Some((label, count)) = best {
            labels.push(if family {
                format!("level {label}")
            } else {
                label
            });
            hits += count;
        }
    }
    let rate = if graded.is_empty() {
        0.0
    } else {
        percent(hits, graded.len())
    };
    (labels.join(" + "), rate)
}

/// Whether a row grades an FR-level coverage fixture rather than a criterion.
///
/// Coverage rows record a 0-3 rubric level; `weakness_kind` rows a label.
/// Keyed off the recorded label's shape because `Graded` carries no family
/// field, and every coverage label in the corpus is an integer while no
/// weakness label is.
fn is_coverage(row: &Graded) -> bool {
    row.expected.parse::<u8>().is_ok()
}

/// Recall over every class that is not `sound`: of the criteria a reader
/// marked as carrying some weakness, the share the lens also flagged as some
/// weakness.
///
/// Deliberately coarse — it does not require the lens to pick the *same*
/// weakness, only to not wave the row through. Calling a tautological
/// criterion `implementation_coupled` is a mislabel; calling it `sound` is the
/// failure this whole lens exists to prevent, and the gate has to separate
/// those two.
///
/// Coverage rows are excluded. They never return `sound`, so the first
/// version of this function counted all four as "found" and inflated recall.
pub(crate) fn defect_recall(graded: &[Graded]) -> Option<f64> {
    let defects: Vec<&Graded> = graded
        .iter()
        .filter(|row| !is_coverage(row))
        .filter(|row| row.expected != "sound" && !row.contested.contains(&"sound".to_owned()))
        .collect();
    if defects.is_empty() {
        return None;
    }
    let found = defects
        .iter()
        .filter(|row| row.actual_class != "sound")
        .count();
    Some(percent(found, defects.len()))
}

/// Of the criteria whose primary reading is `sound`, how many the lens also
/// called `sound`: `(returned sound, expected sound)`.
///
/// The mirror of [`defect_recall`]. The first live run returned `sound` zero
/// times out of five; a lens that never clears a criterion makes every one of
/// its flags on a sound criterion a false positive, which PLAT-837's M2 names
/// as the headline cost.
pub(crate) fn sound_recall(graded: &[Graded]) -> Option<(usize, usize)> {
    let sound: Vec<&Graded> = graded
        .iter()
        .filter(|row| !is_coverage(row) && row.expected == "sound")
        .collect();
    if sound.is_empty() {
        return None;
    }
    let cleared = sound
        .iter()
        .filter(|row| row.actual_class == "sound")
        .count();
    Some((cleared, sound.len()))
}

/// The share of fixtures whose verdict changed between two runs (M1).
///
/// A row present in one run and missing from the other counts as a change,
/// per the tickets' own definition; the denominator is every fixture id
/// either run graded.
pub(crate) fn disagreement(left: &[Graded], right: &[Graded]) -> Option<f64> {
    let index = |rows: &[Graded]| {
        rows.iter()
            .map(|row| (row.fixture_id.clone(), row.actual_class.clone()))
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

/// `part` as a percentage of `whole`, to one decimal place's worth of
/// precision.
fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    let part = f64::from(u32::try_from(part).unwrap_or(u32::MAX));
    let whole = f64::from(u32::try_from(whole).unwrap_or(u32::MAX));
    part / whole * 100.0
}

/// The per-class precision/recall table, appended to `out`.
fn write_class_table(out: &mut String, graded: &[Graded]) {
    let _ = writeln!(
        out,
        "\n**Per class** (against the primary reading only; contested rows are \
         counted in the agreement block above, not here):\n"
    );
    let _ = writeln!(
        out,
        "| Label | Predicted | Expected | Correct | Precision | Recall | False positives |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- | --- |");
    for (label, stats) in class_stats(graded) {
        let rate = |value: Option<f64>| {
            value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1}%"))
        };
        let _ = writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} |",
            stats.predicted,
            stats.expected,
            stats.correct,
            rate(stats.precision()),
            rate(stats.recall()),
            stats.false_positives(),
        );
    }
}

/// Renders the per-fixture table and the M2 rollup.
///
/// Every number says what it counts, per this repo's standing reporting rule;
/// a rate computed over an empty set prints `n/a` rather than `0.0%`.
pub(crate) fn report(title: &str, graded: &[Graded]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n## {title}\n");
    let _ = writeln!(
        out,
        "| Fixture | Tier | Expected | Returned | Verdict | Confidence |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
    for row in graded {
        let confidence = row
            .confidence
            .map_or_else(|| "-".to_owned(), |value| format!("{value:.2}"));
        let _ = writeln!(
            out,
            "| {} | {:?} | {} | {} | {:?} | {confidence} |",
            row.fixture_id, row.tier, row.expected, row.actual, row.verdict,
        );
    }

    let all = tally(graded);
    let clean: Vec<Graded> = graded
        .iter()
        .filter(|row| row.tier == Tier::Clean)
        .cloned()
        .collect();
    let ambiguous: Vec<Graded> = graded
        .iter()
        .filter(|row| row.tier == Tier::Ambiguous)
        .cloned()
        .collect();

    let _ = writeln!(
        out,
        "\n**Agreement** (primary + contested, over every graded row):\n"
    );
    for (label, set) in [
        ("all", graded),
        ("clean", &clean),
        ("ambiguous", &ambiguous),
    ] {
        let tally = tally(set);
        let rate = tally
            .agreement()
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1}%"));
        let _ = writeln!(
            out,
            "- {label}: {rate} ({}/{} — primary {}, contested {}, wrong {}, unrecognized {}, unanswered {})",
            tally.agreed(),
            tally.total(),
            tally.primary,
            tally.contested,
            tally.wrong,
            tally.unrecognized,
            tally.unanswered,
        );
    }

    write_class_table(&mut out, graded);

    let (buckets, without) = calibration(graded);
    let _ = writeln!(out, "\n**Calibration** (M3):\n");
    for bucket in buckets.iter().filter(|bucket| bucket.count > 0) {
        let _ = writeln!(
            out,
            "- confidence {:.1}-{:.1}: {} row(s), {:.1}% agreed",
            bucket.floor,
            bucket.floor + 0.1,
            bucket.count,
            bucket.accuracy().unwrap_or(0.0),
        );
    }
    let _ = writeln!(
        out,
        "- {without} row(s) carried no confidence and are excluded from the curve",
    );
    let ece = expected_calibration_error(graded).map_or_else(
        || "n/a (no row carried a confidence)".to_owned(),
        |value| format!("{value:.4}"),
    );
    let _ = writeln!(out, "- expected calibration error: {ece}");
    let _ = writeln!(out, "\n(total rows graded: {})", all.total());
    out
}
