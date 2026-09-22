// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The labelled fixture corpus, and the grading maths over it (PLAT-917,
//! PLAT-838).
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
//! The label-space-agnostic maths (tally, per-class precision/recall,
//! calibration, the constant-predictor baseline, defect/no-defect recall, M1
//! disagreement) now lives in [`grading`], factored out so PLAT-838's EARS
//! grader ([`ears`]) reuses it rather than re-deriving the same arithmetic a
//! second time (AP-202: "reuse, don't rebuild"). This module keeps only what
//! is specific to the criterion-strength corpus's own two fixture families.
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
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

pub(crate) mod ears;
pub(crate) mod grading;

use serde::Deserialize;

use quoin_jev::{AcRow, FrContext, FrVerdict};

#[allow(
    unused_imports,
    reason = "each test binary links this module separately, so items only one binary uses read as unused in the other"
)]
pub(crate) use grading::{
    Bucket, ClassStats, Graded, Tally, Tier, Verdict, calibration, class_stats, defect_recall,
    disagreement, expected_calibration_error, no_defect_recall, percent, report, tally,
    trivial_baseline,
};

/// The corpus, compiled in from the skill asset so the fixtures and this
/// grader cannot drift -- the same `include_str!` discipline
/// `question-set.json` is already held to (`question_set.rs`, `lens.rs`,
/// `verdict.rs`).
const CORPUS: &str = include_str!(
    "../../../../../skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fixtures.json"
);

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
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "score is checked to be within [-0.5, 3.5) immediately above and then clamped \
                  to >= 0.0, so this cast cannot truncate or wrap"
    )]
    let level = score.round().max(0.0) as u8;
    Some(level.min(3))
}
