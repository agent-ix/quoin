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
    /// Whether `confidence` fell below the caller's threshold.
    ///
    /// PLAT-837 states this is the opposite of the `ix-board` rule: a
    /// low-confidence verdict is annotated here, never dropped from the
    /// returned list -- there is no suppression path in this function at
    /// all, by construction.
    pub unconfirmed: bool,
    /// Every label's probability, for a reader who wants to see how
    /// contested the choice was (the ticket's own worked example: `0.5
    /// sound` / `0.45 unfalsifiable`).
    pub probabilities: Vec<(String, f64)>,
    /// The five raw `noul` values for this row, carried through even though
    /// they have no confidence to annotate.
    pub noul: NoulSignals,
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
    /// Whether `confidence` fell below the caller's threshold. Same
    /// annotate-never-suppress rule as [`Finding::unconfirmed`].
    pub unconfirmed: bool,
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
/// `confidence_threshold` is a required argument rather than a baked-in
/// constant on purpose: PLAT-837 says this crate must use "the same
/// threshold" as `ix-board`'s existing low-confidence rule, and that
/// concrete number lives in `ix-board`, outside this crate's ownership and
/// outside this ticket's stated scope. Hard-coding a guessed value here
/// would be exactly the kind of folklore-that-compiles this codebase's own
/// review conventions warn against; the caller supplies it explicitly, and
/// this crate's own report names that as an open item rather than silently
/// picking a number.
///
/// Never fails: a missing or wrongly-typed `weakness_kind` answer for an AC
/// row is tracked in [`FrVerdict::unanswered`] rather than panicking or
/// refusing the whole FR over one row; a label outside `question-set.json`'s
/// `answer_space` is tracked in [`FrVerdict::unrecognized`], also never
/// silently dropped and never folded into [`FrVerdict::sound`].
#[must_use]
pub fn extract(
    response: &SystemOneResponse,
    question_set: &QuestionSet,
    ac_ids: &[String],
    confidence_threshold: f64,
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
            Some(severity) => findings.push(Finding {
                ac_id: ac_id.clone(),
                weakness_kind: choice.choice.clone(),
                severity,
                confidence: choice.confidence,
                unconfirmed: choice.confidence < confidence_threshold,
                probabilities: choice.probabilities.clone().into_iter().collect(),
                noul: NoulSignals {
                    values: noul_values,
                },
            }),
        }
    }

    let coverage = match response.answer(question_set.adverse_case_coverage_key()) {
        Some(Answer::Score(score)) => Some(CoverageVerdict {
            score: score.score,
            label: rubric_label_for(question_set, score.score),
            confidence: score.confidence,
            unconfirmed: score.confidence < confidence_threshold,
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Severity, extract};
    use crate::question_set::QuestionSet;

    const ASSET: &str = include_str!(
        "../../../../skills/spec-criterion-strength-analysis/assets/question-set.json"
    );

    /// Provenance: PLAT-837
    #[test]
    fn severity_mapping_matches_skill_md() {
        assert_eq!(
            Severity::for_weakness_kind("unfalsifiable"),
            Some(Severity::High)
        );
        assert_eq!(
            Severity::for_weakness_kind("implementation_coupled"),
            Some(Severity::Medium)
        );
        assert_eq!(
            Severity::for_weakness_kind("unmeasurable_threshold"),
            Some(Severity::Medium)
        );
        assert_eq!(
            Severity::for_weakness_kind("restates_requirement"),
            Some(Severity::Low)
        );
        assert_eq!(
            Severity::for_weakness_kind("happy_path_only"),
            Some(Severity::Low)
        );
        assert_eq!(Severity::for_weakness_kind("sound"), None);
    }

    fn response_fixture(confidence: f64) -> typesafe_sdk_answers::SystemOneResponse {
        let body = serde_json::json!({
            "model": "jev-1.13.0",
            "answers": {
                "FR-001-AC-1::falsifiable": {"type": "noul", "noul": 0.9},
                "FR-001-AC-1::states_observable_outcome": {"type": "noul", "noul": 0.8},
                "FR-001-AC-1::threshold_present": {"type": "noul", "noul": 0.1},
                "FR-001-AC-1::restates_requirement": {"type": "noul", "noul": 0.05},
                "FR-001-AC-1::implementation_coupled": {"type": "noul", "noul": 0.02},
                "FR-001-AC-1::weakness_kind": {
                    "type": "choice",
                    "choice": "unfalsifiable",
                    "confidence": confidence,
                    "probabilities": {"unfalsifiable": confidence, "sound": 1.0 - confidence}
                },
                "adverse_case_coverage": {
                    "type": "score",
                    "score": 1.4,
                    "confidence": confidence,
                    "legend": {},
                    "probabilities": {}
                }
            },
            "usage": {"input_tokens": 100, "output_tokens": 0}
        });
        serde_json::from_value(body).expect("well-formed fixture")
    }

    /// Provenance: PLAT-837. The classifier name recorded is the concrete
    /// response `model` field, never the requested alias.
    #[test]
    fn the_classifier_is_the_concrete_model_from_the_response() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let response = response_fixture(0.9);
        let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], 0.5);
        assert_eq!(verdict.classifier, "jev-1.13.0");
    }

    /// Provenance: PLAT-837. High confidence: a finding is produced and NOT
    /// marked unconfirmed.
    #[test]
    fn a_high_confidence_verdict_is_not_marked_unconfirmed() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let response = response_fixture(0.95);
        let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], 0.5);
        assert_eq!(verdict.findings.len(), 1);
        assert!(!verdict.findings[0].unconfirmed);
        assert_eq!(verdict.findings[0].severity, Severity::High);
    }

    /// Provenance: PLAT-837. This is the acceptance criterion itself: low
    /// confidence must still produce a finding (annotated), never suppress
    /// it -- the opposite of ix-board's rule, and stated as such in this
    /// ticket.
    #[test]
    fn a_low_confidence_verdict_still_produces_a_finding_marked_unconfirmed() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let response = response_fixture(0.3);
        let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], 0.5);
        assert_eq!(
            verdict.findings.len(),
            1,
            "low confidence must not suppress the finding"
        );
        assert!(verdict.findings[0].unconfirmed);
    }

    /// Provenance: PLAT-837. A `sound` verdict produces no finding, and is
    /// named in `sound` rather than silently absent.
    #[test]
    fn a_sound_verdict_produces_no_finding() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let body = serde_json::json!({
            "model": "jev-1.13.0",
            "answers": {
                "FR-001-AC-1::falsifiable": {"type": "noul", "noul": 0.95},
                "FR-001-AC-1::states_observable_outcome": {"type": "noul", "noul": 0.9},
                "FR-001-AC-1::threshold_present": {"type": "noul", "noul": 0.9},
                "FR-001-AC-1::restates_requirement": {"type": "noul", "noul": 0.02},
                "FR-001-AC-1::implementation_coupled": {"type": "noul", "noul": 0.02},
                "FR-001-AC-1::weakness_kind": {
                    "type": "choice", "choice": "sound", "confidence": 0.9,
                    "probabilities": {"sound": 0.9}
                },
                "adverse_case_coverage": {
                    "type": "score", "score": 3.0, "confidence": 0.9,
                    "legend": {}, "probabilities": {}
                }
            },
            "usage": {"input_tokens": 50, "output_tokens": 0}
        });
        let response: typesafe_sdk_answers::SystemOneResponse =
            serde_json::from_value(body).expect("well-formed");
        let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], 0.5);
        assert!(verdict.findings.is_empty());
        assert_eq!(verdict.sound, vec!["FR-001-AC-1".to_owned()]);
    }

    /// Provenance: PLAT-837 review, finding 1. A `weakness_kind` label
    /// outside `question-set.json`'s declared `answer_space` (a version-skewed
    /// Jev inventing a seventh label, or a malformed response) must get its
    /// own outcome -- `unrecognized` -- and must NEVER be counted as `sound`.
    /// `Severity::for_weakness_kind`'s wildcard arm alone cannot make this
    /// distinction; `extract` must validate against `answer_space` first.
    #[test]
    fn an_unrecognized_weakness_kind_label_is_never_counted_as_sound() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let body = serde_json::json!({
            "model": "jev-1.13.0",
            "answers": {
                "FR-001-AC-1::falsifiable": {"type": "noul", "noul": 0.5},
                "FR-001-AC-1::states_observable_outcome": {"type": "noul", "noul": 0.5},
                "FR-001-AC-1::threshold_present": {"type": "noul", "noul": 0.5},
                "FR-001-AC-1::restates_requirement": {"type": "noul", "noul": 0.5},
                "FR-001-AC-1::implementation_coupled": {"type": "noul", "noul": 0.5},
                "FR-001-AC-1::weakness_kind": {
                    "type": "choice", "choice": "quantum_uncertainty", "confidence": 0.9,
                    "probabilities": {"quantum_uncertainty": 0.9}
                },
                "adverse_case_coverage": {
                    "type": "score", "score": 3.0, "confidence": 0.9,
                    "legend": {}, "probabilities": {}
                }
            },
            "usage": {"input_tokens": 50, "output_tokens": 0}
        });
        let response: typesafe_sdk_answers::SystemOneResponse =
            serde_json::from_value(body).expect("well-formed");
        let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], 0.5);
        assert!(verdict.sound.is_empty(), "must never land in sound");
        assert!(verdict.findings.is_empty(), "not a mapped finding either");
        assert_eq!(
            verdict.unrecognized,
            vec![("FR-001-AC-1".to_owned(), "quantum_uncertainty".to_owned())]
        );
    }

    /// Provenance: PLAT-837 review, finding 2. An AC row Jev never answered a
    /// `weakness_kind` choice for at all is tracked as `unanswered` -- not
    /// silently dropped from both `findings` and `sound`.
    #[test]
    fn an_unanswered_ac_row_is_tracked_not_dropped() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let response = response_fixture(0.9); // only answers FR-001-AC-1
        let verdict = extract(
            &response,
            &set,
            &["FR-001-AC-1".to_owned(), "FR-001-AC-2".to_owned()],
            0.5,
        );
        assert_eq!(verdict.unanswered, vec!["FR-001-AC-2".to_owned()]);
        assert!(!verdict.sound.contains(&"FR-001-AC-2".to_owned()));
        assert!(!verdict.findings.iter().any(|f| f.ac_id == "FR-001-AC-2"));
    }

    /// Provenance: PLAT-837. A score outside the rubric's `[0, 3]` range --
    /// negative, in this case -- must report no label, not the level-0 label
    /// a bare saturating cast would produce (`as u8` on a negative float
    /// saturates to 0, which IS a valid rubric level and would silently
    /// misreport "no label" as "happy path only").
    #[test]
    fn an_out_of_range_negative_score_reports_no_label() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let body = serde_json::json!({
            "model": "jev-1.13.0",
            "answers": {
                "adverse_case_coverage": {
                    "type": "score", "score": -1.0, "confidence": 0.9,
                    "legend": {}, "probabilities": {}
                }
            },
            "usage": {"input_tokens": 10, "output_tokens": 0}
        });
        let response: typesafe_sdk_answers::SystemOneResponse =
            serde_json::from_value(body).expect("well-formed");
        let verdict = extract(&response, &set, &[], 0.5);
        let coverage = verdict.coverage.expect("a score answer was present");
        assert_eq!(coverage.label, None);
    }

    /// Provenance: PLAT-837. A `noul` answer has no `confidence` field to
    /// read -- this test asserts the type-level fact the module doc
    /// describes, not just the prose claim.
    #[test]
    fn noul_answers_carry_no_confidence_field() {
        let value = serde_json::json!({"type": "noul", "noul": 0.5});
        let parsed: typesafe_sdk_answers::Answer =
            serde_json::from_value(value).expect("a bare noul answer parses");
        let noul = parsed.as_noul().expect("this is a noul answer");
        assert!((noul.noul - 0.5).abs() < f64::EPSILON);
        // NoulAnswer has exactly one field (`noul`); there is no `.confidence`
        // to read here at all -- this is a compile-time fact, demonstrated by
        // the absence of any such access in this file.
    }
}
