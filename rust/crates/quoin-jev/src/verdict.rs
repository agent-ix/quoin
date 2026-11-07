// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Turning a [`SystemOneResponse`] into typed findings (PLAT-837).
//!
//! **On calibration and `noul`.** The ticket leans on calibration -- "a
//! criterion scored 0.5 `sound` / 0.45 `unfalsifiable` is visibly contested"
//! -- and its core check, `falsifiable`, is a `noul` question. Verified
//! against `typesafe-sdk-answers 0.6.2`'s actual types (`answer.rs`):
//! [`typesafe_sdk_answers::NoulAnswer`] is `{ noul: f64 }` -- a bare probability, no `confidence`
//! field at all. Only [`typesafe_sdk_answers::ChoiceAnswer`] (`weakness_kind`) and [`typesafe_sdk_answers::ScoreAnswer`]
//! (`adverse_case_coverage`) carry `confidence` alongside `probabilities`.
//!
//! What that means concretely: the "low confidence annotates, never
//! suppresses" rule this module implements can only ever act on
//! `weakness_kind`'s and `adverse_case_coverage`'s `confidence` fields --
//! there is no confidence to threshold on `falsifiable` itself. A `noul`
//! answer close to the 0.5 boundary (visibly contested, in the ticket's own
//! example) is not distinguishable, by this API, from one the model is
//! merely reporting a genuinely balanced 0.5 probability for -- both look
//! identical: a lone float near 0.5. This module does not paper over that: it
//! carries every raw `noul` value into [`Finding::noul`] for a human to
//! read, but the calibration-driven unconfirmed marker is attached only
//! where the wire format actually supports it, at `weakness_kind`. Whether
//! the important questions should be `choice` rather than `noul` is a
//! question about `question-set.json`'s shape, which this crate does not
//! own and does not change -- see this crate's top-level report for that
//! finding stated as a recommendation, not applied silently here.
//!
//! **Margin gating (PLAT-981).** The same limitation applies to the second
//! gate, [`Certainty::Uncertain`]: it diffs the top two entries of an
//! answer's `probabilities` map, and only `choice` and `score` answers carry
//! one. A `noul` answer is a lone float, so there is nothing to diff and no
//! margin gate on `falsifiable` either.

use serde::Serialize;
use typesafe_sdk_answers::{Answer, SystemOneResponse};

use crate::question_set::QuestionSet;

/// The finding-table severity vocabulary the spec-artifacts-process
/// `SpecReview` schema declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Adds no discriminating power beyond the FR sentence, but is not
    /// itself false (`restates_requirement`), or a `happy_path_only` row.
    Low,
    /// The AC over-specifies (`implementation_coupled`) or asserts a
    /// quantity it never states (`unmeasurable_threshold`).
    Medium,
    /// The AC cannot fail; the matrix row it backs proves nothing
    /// (`unfalsifiable`).
    High,
}

impl Severity {
    /// The wire spelling the `SpecReview` Findings table uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// The severity `weakness_kind` label `kind` maps to, per
    /// `skills/spec-criterion-strength-analysis/SKILL.md`'s "Severity
    /// mapping" section -- `sound` returns [`None`]: no finding.
    ///
    /// This table is this crate's own copy of that mapping, kept in Rust
    /// rather than read from `SKILL.md` at run time, because `SKILL.md` is
    /// prose for an agent to follow and this needs a value the compiler
    /// checks is exhaustive over `question-set.json`'s six-member enum. If
    /// the mapping in `SKILL.md` ever changes, this table changes with it in
    /// the same review -- it is not a second source of truth so much as the
    /// executable form of the one `SKILL.md` states in prose.
    ///
    /// This function is only ever reached, from [`extract`], for a label
    /// [`extract`] has already checked is one of `question-set.json`'s six
    /// documented `answer_space` members -- so the `_` arm below is reached
    /// only by `"sound"` in practice. It stays a wildcard (rather than
    /// listing `"sound"` explicitly) because this function is also unit
    /// tested directly, and a caller handing it an arbitrary string should
    /// get `None`, not a panic; [`extract`] is what gives an out-of-band
    /// label its own outcome (see [`FrVerdict::unrecognized`]) rather than
    /// ever letting it reach this match and read as `"sound"`.
    #[must_use]
    pub fn for_weakness_kind(kind: &str) -> Option<Self> {
        match kind {
            "unfalsifiable" => Some(Self::High),
            "implementation_coupled" | "unmeasurable_threshold" => Some(Self::Medium),
            "restates_requirement" | "happy_path_only" => Some(Self::Low),
            _ => None,
        }
    }
}

/// The boundary at which a bare `noul` probability reads as "yes". The same
/// `>= 0.5` cut the live suite grades `noul` answers with
/// (`the_noul_answers_are_reported_per_question`, PR #574).
const NOUL_YES: f64 = 0.5;

/// The `noul` sub-question whose text names the defect a `weakness_kind`
/// label reports, paired with the yes/no answer that would corroborate it.
///
/// Read off `question-set.json`'s own question text, not fit to any corpus:
/// `unfalsifiable` is `falsifiable` answered no, `unmeasurable_threshold` is
/// `threshold_present` answered no, and `restates_requirement` /
/// `implementation_coupled` are their namesake questions answered yes.
/// `happy_path_only` has no sub-question: none of the five tests coverage
/// breadth (the blind spot PR #574's `v3` stated before its first call).
/// `states_observable_outcome` names no label on its own and is never
/// returned here.
///
/// Like [`Severity::for_weakness_kind`], this is only reached from
/// [`extract`] for a label already checked against `answer_space`, so the
/// `_` arm is reached by `happy_path_only` and `sound` in practice; an
/// arbitrary string gets [`None`], never a guessed question.
#[must_use]
fn label_sub_question(kind: &str) -> Option<(&'static str, bool)> {
    match kind {
        "unfalsifiable" => Some(("falsifiable", false)),
        "unmeasurable_threshold" => Some(("threshold_present", false)),
        "restates_requirement" => Some(("restates_requirement", true)),
        "implementation_coupled" => Some(("implementation_coupled", true)),
        _ => None,
    }
}

/// Whether the `weakness_kind` label a finding carries is backed by the one
/// `noul` sub-question that names the same defect (PLAT-984).
///
/// **This is not a decomposition of the verdict, and does not claim to be.**
/// PLAT-984 asks for a "which sub-question decided this verdict" breakdown.
/// For this lens nothing decides `weakness_kind` except a single Jev `choice`
/// call: the five `noul` answers are asked in the same request but are not
/// inputs to the label, and PR #574 measured that deriving the label from them
/// is worse than asking for it (`v3-derived`, 33.3%). So "decided-by" here can
/// only mean the narrower, still useful thing: for the label Jev chose, did
/// the sub-question that asks about that very defect answer the same way? A
/// [`Self::Disagrees`] row is a label that is not coming from the question a
/// reader would assume it comes from -- the case the ticket wants visible.
///
/// PR #574's `the_noul_answers_are_reported_per_question` measures each
/// question's agreement against a corpus's recorded answers. That needs
/// ground truth a production run does not have, so it is not what this
/// promotes; this compares Jev's answers with each other, per finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SubQuestionCheck {
    /// The label's sub-question answered in the label's direction.
    Agrees {
        /// The `noul` question id.
        question: &'static str,
        /// The bare probability Jev answered it with.
        noul: f64,
    },
    /// The label's sub-question answered against the label: Jev chose the
    /// label, but its own answer to the question about that defect says the
    /// defect is absent (or, for a "yes" question, present when the label
    /// says otherwise).
    Disagrees {
        /// The `noul` question id.
        question: &'static str,
        /// The bare probability Jev answered it with.
        noul: f64,
    },
    /// The label has a sub-question, but the response carried no `noul`
    /// answer for it on this row. Its own outcome, never read as agreement.
    Unanswered {
        /// The `noul` question id that went unanswered.
        question: &'static str,
    },
    /// No `noul` question covers this label (`happy_path_only`).
    NoSubQuestion,
}

impl SubQuestionCheck {
    /// The check for `kind` against one AC row's `noul` answers.
    #[must_use]
    pub fn for_label(kind: &str, noul: &NoulSignals) -> Self {
        let Some((question, corroborating)) = label_sub_question(kind) else {
            return Self::NoSubQuestion;
        };
        match noul.values.iter().find(|(id, _)| id == question) {
            None => Self::Unanswered { question },
            Some(&(_, value)) if (value >= NOUL_YES) == corroborating => Self::Agrees {
                question,
                noul: value,
            },
            Some(&(_, value)) => Self::Disagrees {
                question,
                noul: value,
            },
        }
    }
}

/// The two caller-owned numbers [`extract`] gates on.
///
/// Both are required rather than defaulted, for the reason [`extract`]'s doc
/// gives for `confidence`: `ix-board` owns the number, and this crate does
/// not invent one. They travel as one struct rather than two adjacent `f64`
/// parameters because two same-typed positional floats are silently
/// swappable at a call site -- `extract(.., 0.1, 0.5)` and
/// `extract(.., 0.5, 0.1)` both compile. Named fields cannot be crossed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// A reported `confidence` strictly below this is
    /// [`Certainty::Unconfirmed`].
    pub confidence: f64,
    /// A gap between the top two `probabilities` strictly below this is
    /// [`Certainty::Uncertain`].
    pub margin: f64,
}

/// How far a `choice` or `score` verdict can be trusted (PLAT-981).
///
/// One enum rather than an `uncertain: bool` beside the old `unconfirmed:
/// bool`: two bools make four states, and the ticket wants exactly three --
/// "right, wrong, or honestly unsure". With two bools, `unconfirmed &&
/// uncertain` is representable, and every reader has to re-decide which one
/// wins. Here the precedence is decided once, in [`Certainty::assess`], and
/// a consumer's `match` is exhaustive over the three answers there are.
///
/// Every variant still carries its finding: like the `unconfirmed` bool this
/// replaces, a low-certainty verdict is annotated and never suppressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Certainty {
    /// Neither gate fired: the reported confidence met its threshold and the
    /// top label led the runner-up by at least the margin (or there was no
    /// runner-up to lead).
    Confident,
    /// The reported confidence fell below [`Thresholds::confidence`], and the
    /// top two labels were NOT within the margin -- the model is unsure of
    /// its answer, but not torn between two answers.
    Unconfirmed,
    /// The top two `probabilities` were within [`Thresholds::margin`] of each
    /// other: the model could not separate two answers. Takes precedence over
    /// [`Self::Unconfirmed`] -- see [`Certainty::assess`].
    Uncertain,
}

impl Certainty {
    /// Assesses one answer's `confidence` and `probabilities` against
    /// `thresholds`.
    ///
    /// **Precedence: `Uncertain` over `Unconfirmed`.** When both gates fire,
    /// the answer is `Uncertain`. "Low confidence" says the model doubts its
    /// pick; "within the margin" says *which* doubt -- a second label it
    /// nearly chose. That is the more specific statement, and the one PLAT-981
    /// exists to make visible as its own bucket. Folding it into
    /// `Unconfirmed` whenever confidence is also low would hide exactly the
    /// cases the ticket is about, since a near-tie usually drags confidence
    /// down with it.
    ///
    /// **Fewer than two probabilities: no margin exists.** With zero or one
    /// label there is no runner-up to be close to, so the margin gate does
    /// not fire and the answer falls through to the confidence gate alone.
    /// Treating a lone label as a tie would report "torn between two answers"
    /// when there is only one.
    #[must_use]
    pub fn assess(
        confidence: f64,
        probabilities: impl IntoIterator<Item = f64>,
        thresholds: Thresholds,
    ) -> Self {
        let within_margin =
            top_two_margin(probabilities).is_some_and(|margin| margin < thresholds.margin);
        if within_margin {
            Self::Uncertain
        } else if confidence < thresholds.confidence {
            Self::Unconfirmed
        } else {
            Self::Confident
        }
    }

    /// The wire spelling, matching the `Serialize` form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confident => "confident",
            Self::Unconfirmed => "unconfirmed",
            Self::Uncertain => "uncertain",
        }
    }
}

/// The highest probability minus the second highest, or [`None`] when there
/// are fewer than two.
///
/// One pass keeping the top two rather than sorting: the ordering is
/// `f64::total_cmp`, so a NaN from a malformed response has a defined place
/// instead of making the comparison panic or depend on input order.
fn top_two_margin(probabilities: impl IntoIterator<Item = f64>) -> Option<f64> {
    let mut first: Option<f64> = None;
    let mut second: Option<f64> = None;
    for p in probabilities {
        match first {
            Some(top) if p.total_cmp(&top).is_le() => {
                if second.is_none_or(|runner_up| p.total_cmp(&runner_up).is_gt()) {
                    second = Some(p);
                }
            }
            _ => {
                second = first;
                first = Some(p);
            }
        }
    }
    Some(first? - second?)
}

/// The raw `noul` answers for one AC row, carried through for transparency
/// even though they cannot carry the confidence-annotation rule (see this
/// module's doc).
#[derive(Debug, Clone, Serialize)]
pub struct NoulSignals {
    /// `question-set.json` id -> the bare 0-1 probability Jev answered.
    pub values: Vec<(String, f64)>,
}

/// One `SpecReview` Findings-table row, plus the data that produced it.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// The AC row this finding is about (e.g. `FR-079-AC-4`).
    pub ac_id: String,
    /// The `weakness_kind` label Jev returned.
    pub weakness_kind: String,
    /// The mapped severity. A `Finding` is only ever constructed for a
    /// non-`sound` verdict (see [`extract`]), so this is never the `sound`
    /// no-finding case.
    pub severity: Severity,
    /// `weakness_kind`'s reported confidence, unmodified.
    pub confidence: f64,
    /// How far this verdict can be trusted: [`Certainty::assess`] over
    /// `confidence` and `probabilities`.
    ///
    /// PLAT-837 states this is the opposite of the `ix-board` rule: a
    /// low-certainty verdict is annotated here, never dropped from the
    /// returned list -- there is no suppression path in this function at
    /// all, by construction.
    pub certainty: Certainty,
    /// Every label's probability, for a reader who wants to see how
    /// contested the choice was (the ticket's own worked example: `0.5
    /// sound` / `0.45 unfalsifiable`).
    pub probabilities: Vec<(String, f64)>,
    /// The five raw `noul` values for this row, carried through even though
    /// they have no confidence to annotate.
    pub noul: NoulSignals,
    /// Whether `weakness_kind`'s label is backed by the `noul` sub-question
    /// that names the same defect (PLAT-984). See [`SubQuestionCheck`] for
    /// why this is attribution, not a decomposition of the verdict.
    pub label_sub_question: SubQuestionCheck,
}

/// The FR-level `adverse_case_coverage` verdict.
#[derive(Debug, Clone, Serialize)]
pub struct CoverageVerdict {
    /// The 0-3 score Jev returned (may fall between rubric levels).
    pub score: f64,
    /// The rubric label for the nearest integer level, when the asset names one.
    pub label: Option<String>,
    /// Reported confidence in the score.
    pub confidence: f64,
    /// [`Certainty::assess`] over the score answer's `confidence` and
    /// `probabilities`. Same annotate-never-suppress rule as
    /// [`Finding::certainty`].
    pub certainty: Certainty,
}

/// Everything this module produces for one FR's Jev call.
#[derive(Debug, Clone, Serialize)]
pub struct FrVerdict {
    /// The concrete model version the response reported (e.g. `jev-1.13.0`),
    /// never the alias that was requested. PLAT-837's "every report names
    /// the classifier that produced it" is satisfied by carrying this
    /// through unchanged; nothing here renders `jev-latest`.
    pub classifier: String,
    /// Input/output token counts, for cost accounting.
    pub usage_input_tokens: u64,
    /// Output token counts.
    pub usage_output_tokens: u64,
    /// One [`Finding`] per non-`sound` AC row. A `sound` row is not a
    /// missing finding; [`Self::sound`] names which rows were sound, so a
    /// reader can tell "every AC was sound" from "the response was empty".
    pub findings: Vec<Finding>,
    /// AC ids Jev answered `sound` for, and so produced no finding.
    pub sound: Vec<String>,
    /// AC ids whose `weakness_kind` answer was a string outside
    /// `question-set.json`'s declared `answer_space` -- a Jev version skew,
    /// or a malformed response. This is its own outcome and never folds into
    /// [`Self::sound`]: an unrecognised label is evidence of *something*
    /// gone wrong, the opposite of a confirmed-sound row, and conflating the
    /// two would silently misreport "not sure what this means" as "checked
    /// and fine". Pairs each AC id with the raw label Jev actually returned.
    pub unrecognized: Vec<(String, String)>,
    /// AC ids Jev never answered a `weakness_kind` choice for at all (the key
    /// was missing from the response, or answered with the wrong answer
    /// type). Distinct from [`Self::unrecognized`]: this is "no verdict",
    /// not "a verdict this crate doesn't understand". Tracked explicitly
    /// rather than silently dropped, so a reader can tell "every AC row was
    /// reviewed" from "some rows were never reviewed at all".
    pub unanswered: Vec<String>,
    /// The FR's `adverse_case_coverage` verdict, when the response answered it.
    pub coverage: Option<CoverageVerdict>,
}

/// Extracts every finding from `response` for the AC rows in `ac_ids`,
/// against `questions` (for the wire-key shape) and severity mapping.
///
/// `thresholds` is a required argument rather than a baked-in constant on
/// purpose: PLAT-837 says this crate must use "the same threshold" as
/// `ix-board`'s existing low-confidence rule, and that concrete number lives
/// in `ix-board`, outside this crate's ownership and outside this ticket's
/// stated scope. Hard-coding a guessed value here would be exactly the kind
/// of folklore-that-compiles this codebase's own review conventions warn
/// against; the caller supplies it explicitly, and this crate's own report
/// names that as an open item rather than silently picking a number.
/// PLAT-981's margin is caller-supplied for the same reason.
///
/// Never fails: a missing or wrongly-typed `weakness_kind` answer for an AC
/// row is tracked in [`FrVerdict::unanswered`] rather than panicking or
/// refusing the whole FR over one row; a label outside `question-set.json`'s
/// `answer_space` is tracked in [`FrVerdict::unrecognized`], also never
/// silently dropped and never folded into [`FrVerdict::sound`].
///
/// Does not check `response.model` against anything: it records it verbatim
/// in [`FrVerdict::classifier`].
#[must_use]
pub fn extract(
    response: &SystemOneResponse,
    question_set: &QuestionSet,
    ac_ids: &[String],
    thresholds: Thresholds,
) -> FrVerdict {
    let mut findings = Vec::new();
    let mut sound = Vec::new();
    let mut unrecognized = Vec::new();
    let mut unanswered = Vec::new();

    for ac_id in ac_ids {
        let noul_values: Vec<(String, f64)> = question_set
            .noul
            .iter()
            .filter_map(|entry| {
                let key = QuestionSet::noul_key(ac_id, entry);
                match response.answer(&key) {
                    Some(Answer::Noul(answer)) => Some((entry.id.clone(), answer.noul)),
                    _ => None,
                }
            })
            .collect();

        let weakness_key = question_set.weakness_kind_key(ac_id);
        let Some(Answer::Choice(choice)) = response.answer(&weakness_key) else {
            unanswered.push(ac_id.clone());
            continue;
        };

        // Validate against the closed answer space *before* consulting the
        // severity table: `Severity::for_weakness_kind`'s wildcard arm alone
        // cannot tell "sound" (the documented sixth label) apart from a
        // label a version-skewed Jev invented, and conflating the two would
        // silently count an unrecognised verdict as a confirmed-sound one.
        let is_recognized = question_set
            .choice
            .answer_space
            .iter()
            .any(|label| label == &choice.choice);
        if !is_recognized {
            unrecognized.push((ac_id.clone(), choice.choice.clone()));
            continue;
        }

        match Severity::for_weakness_kind(&choice.choice) {
            None => sound.push(ac_id.clone()),
            Some(severity) => {
                let noul = NoulSignals {
                    values: noul_values,
                };
                findings.push(Finding {
                    ac_id: ac_id.clone(),
                    weakness_kind: choice.choice.clone(),
                    severity,
                    confidence: choice.confidence,
                    certainty: Certainty::assess(
                        choice.confidence,
                        choice.probabilities.values().copied(),
                        thresholds,
                    ),
                    probabilities: choice.probabilities.clone().into_iter().collect(),
                    label_sub_question: SubQuestionCheck::for_label(&choice.choice, &noul),
                    noul,
                });
            }
        }
    }

    let coverage = match response.answer(question_set.adverse_case_coverage_key()) {
        Some(Answer::Score(score)) => Some(CoverageVerdict {
            score: score.score,
            label: rubric_label_for(question_set, score.score),
            confidence: score.confidence,
            certainty: Certainty::assess(
                score.confidence,
                score.probabilities.values().copied(),
                thresholds,
            ),
        }),
        _ => None,
    };

    FrVerdict {
        classifier: response.model.clone(),
        usage_input_tokens: response.usage.input_tokens,
        usage_output_tokens: response.usage.output_tokens,
        findings,
        sound,
        unrecognized,
        unanswered,
        coverage,
    }
}

/// The rubric label for `score`, rounded to the nearest level.
///
/// Bounds are checked explicitly rather than cast-and-let-`rubric_label`-miss:
/// a bare `as u8` on a negative float **saturates to 0** under Rust's cast
/// semantics, which would misreport an out-of-range negative score as the
/// valid level-0 label ("happy path only") instead of "no label". Checking
/// `(0.0..256.0).contains(&rounded)` first means an out-of-range value -- on
/// either side -- genuinely produces [`None`], which is what "no label" is
/// supposed to mean.
fn rubric_label_for(question_set: &QuestionSet, raw_score: f64) -> Option<String> {
    let rounded = raw_score.round();
    if !(0.0..256.0).contains(&rounded) {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rounded is checked to be within [0.0, 256.0) immediately above, so this cast cannot truncate or wrap"
    )]
    let level = rounded as u8;
    question_set.rubric_label(level).map(str::to_owned)
}

#[cfg(test)]
mod tests;
