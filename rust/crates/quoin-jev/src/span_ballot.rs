// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Span ballots: constrained span extraction over a closed candidate list
//! (PLAT-980).
//!
//! When a lens needs Jev to point at *part* of a spec text -- "which clause
//! of this AC is the unfalsifiable one" -- the unsafe shape is a free-form
//! span: the model writes text back, and the caller must then decide whether
//! that text is really in the source, a paraphrase of it, or invented. A
//! span ballot removes that decision. Candidate spans are generated
//! deterministically in Rust ([`candidate_spans`]), presented to Jev as the
//! closed label set of one `choice` question ([`SpanBallot::question`]), and
//! the answer is accepted only when its label is literally one this ballot
//! issued ([`SpanBallot::resolve`]). Anything else -- the span's text instead
//! of its label, a paraphrase, a label from another ballot -- is
//! [`BallotOutcome::Unrecognized`], never coerced to the nearest candidate.
//! That is the same rule [`crate::verdict::extract`] applies to
//! `weakness_kind`'s `answer_space`, factored for spans.
//!
//! **Withhold bad options; do not argue against them.** The design this
//! module borrows (a third-party project's span-ballot pattern, cited in
//! PLAT-980 as inspiration only -- nothing here depends on it) reports that
//! prompt wording alone did not stop a bad option from being selected, and
//! removing the option did. So there is no "avoid X" instruction channel
//! here: a caller that does not want a candidate offered filters it out of
//! the list it hands [`SpanBallot::new`]. A withheld candidate cannot be
//! selected, because a label for it was never issued -- if Jev returns its
//! text anyway, that is [`BallotOutcome::Unrecognized`].
//!
//! Labels are generated ids (`span-1`, `span-2`, ...) with the span text as
//! the label's description, rather than the span text itself as the label.
//! Two reasons: two candidates with identical text would collide in the
//! `criteria` map, and a text label could collide with the reserved
//! [`ABSTAIN_LABEL`]. Generated ids make both impossible by construction.
//!
//! Nothing in the crate consumes a ballot yet; no lens today asks Jev for a
//! free span, so this is a preventative addition, not a fix to a live hole.

use serde::Serialize;
use typesafe_sdk_answers::Answer;
use typesafe_sdk_questions::{Question, choice};

use crate::error::{JevError, JevErrorCode, Result};

/// The most candidates one ballot may carry.
///
/// A ballot is one `choice` question; every candidate is a label Jev must
/// weigh, and an AC or FR body long enough to exceed this is better split by
/// the caller (per AC row, per paragraph) than offered as one ballot. Hitting
/// the ceiling is a refusal ([`JevErrorCode::SpanBallotTooLarge`]), never a
/// silent truncation that would drop the tail of the text from the ballot.
pub const MAX_CANDIDATES: usize = 64;

/// The reserved label Jev picks when no offered span fits.
///
/// Always present on a ballot, so Jev is never forced to name a wrong span
/// just because the right answer is "none of them". Generated candidate
/// labels are `span-<n>` and can never equal it.
pub const ABSTAIN_LABEL: &str = "none";

/// A contiguous piece of source text, located by byte offsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Span {
    /// Byte offset of the first byte of [`Self::text`] in the source.
    pub start: usize,
    /// Byte offset one past the last byte of [`Self::text`] in the source.
    pub end: usize,
    /// The span's text, exactly `source[start..end]`.
    pub text: String,
}

/// Abbreviations whose trailing `.` is not a sentence end, written without
/// that final dot and matched ASCII-case-insensitively as a whole word.
///
/// Without this, the delimiter rule cut FR-006's behaviour bullet in the
/// criterion-strength fixture corpus -- `render a fallback UI (e.g., "Error
/// loading node") instead of crashing the tree` -- into `... fallback UI
/// (e.g.` and `"Error loading node") ...`, offering Jev half a phrase
/// (PLAT-980 review). A delimiter is not a break when the word it closes is
/// one of these: the `.` of `e.g. x`, and the `,` of `e.g., x` (the word
/// being `e.g.` there).
///
/// Known trade-off: a sentence that really ends in `etc.` is joined to the
/// next one. That errs toward a longer candidate, never a cut phrase.
const ABBREVIATIONS: [&str; 5] = ["e.g", "i.e", "etc", "vs", "cf"];

/// Whether the delimiter `ch` at byte `index` of `text` closes one of
/// [`ABBREVIATIONS`] rather than a sentence or clause.
fn closes_abbreviation(text: &str, index: usize, ch: char) -> bool {
    let Some(before) = text.get(..index) else {
        return false;
    };
    // `.` closes the abbreviation itself (`e.g` + `.`); any other delimiter
    // closes it only when it directly follows the abbreviation's own dot.
    let stem = if ch == '.' {
        Some(before)
    } else {
        before.strip_suffix('.')
    };
    let Some(stem) = stem else {
        return false;
    };
    ABBREVIATIONS.iter().any(|abbreviation| {
        let Some(word_start) = stem.len().checked_sub(abbreviation.len()) else {
            return false;
        };
        let (Some(head), Some(word)) = (stem.get(..word_start), stem.get(word_start..)) else {
            return false;
        };
        word.eq_ignore_ascii_case(abbreviation)
            && head
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric())
    })
}

/// How finely [`candidate_spans`] cuts a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    /// Break at `.`, `!`, `?` and `;` when followed by whitespace or the end
    /// of the text, and at every line break.
    Sentence,
    /// Everything [`Self::Sentence`] breaks at, plus `,` followed by
    /// whitespace.
    Clause,
}

impl Granularity {
    /// Whether `ch` ends a candidate at this granularity, given it is
    /// followed by whitespace or the end of the text.
    const fn ends_candidate(self, ch: char) -> bool {
        match self {
            Self::Sentence => matches!(ch, '.' | '!' | '?' | ';'),
            Self::Clause => matches!(ch, '.' | '!' | '?' | ';' | ','),
        }
    }
}

/// Cuts `text` into candidate spans, deterministically.
///
/// The rule, in full:
///
/// - A line break always ends a candidate.
/// - A delimiter from [`Granularity`] ends a candidate when the next
///   character is whitespace or there is no next character. That keeps
///   `1.5`, `v0.6.2` and `a,b` whole.
/// - A delimiter that closes one of a short list of abbreviations
///   (`e.g.`, `i.e.`, `etc.`, `vs.`, `cf.`) does not break, so
///   `a fallback UI (e.g., "Error loading node")` stays one candidate.
/// - Nothing inside a backtick code span breaks: spec text names symbols
///   such as `` `Client::system_one` `` and paths such as `` `src/a.rs` ``,
///   and cutting through one would offer Jev half a symbol. An unclosed
///   backtick runs to the end of the text, line breaks included.
///
///   Each backtick toggles "inside code", so only single-backtick inline
///   code is modelled. A Markdown double-backtick span that contains a
///   lone backtick (``` ``a`b`` ```) toggles an odd number of times inside
///   and leaves the rest of the text treated as code. That shape is not
///   handled because it does not occur in this crate's input: FR/AC prose
///   in the criterion-strength fixture corpus carries no double backtick.
/// - The delimiter itself is not part of either candidate. Each candidate is
///   trimmed of surrounding whitespace, and a candidate with no alphanumeric
///   character (a stray `-`, an empty line) is dropped.
///
/// Offsets are byte offsets into `text`, always on `char` boundaries, and
/// `text[span.start..span.end] == span.text` holds for every span returned.
/// Duplicate texts are kept here -- [`SpanBallot::new`] decides what a
/// duplicate means for a ballot.
#[must_use]
pub fn candidate_spans(text: &str, granularity: Granularity) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut start = 0;
    let mut in_code = false;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch == '`' {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let at_break = ch == '\n'
            || (granularity.ends_candidate(ch)
                && chars.peek().is_none_or(|(_, next)| next.is_whitespace())
                && !closes_abbreviation(text, index, ch));
        if at_break {
            push_trimmed(text, start, index, &mut spans);
            start = index + ch.len_utf8();
        }
    }
    push_trimmed(text, start, text.len(), &mut spans);
    spans
}

/// Pushes `text[start..end]`, trimmed, when it carries any alphanumeric
/// character.
fn push_trimmed(text: &str, start: usize, end: usize, spans: &mut Vec<Span>) {
    let Some(raw) = text.get(start..end) else {
        return;
    };
    let trimmed = raw.trim();
    if !trimmed.chars().any(char::is_alphanumeric) {
        return;
    }
    let leading = raw.len() - raw.trim_start().len();
    let span_start = start + leading;
    spans.push(Span {
        start: span_start,
        end: span_start + trimmed.len(),
        text: trimmed.to_owned(),
    });
}

/// One `choice` question whose answers are exactly a closed list of spans.
#[derive(Debug, Clone)]
pub struct SpanBallot {
    instructions: String,
    options: Vec<(String, Span)>,
}

impl SpanBallot {
    /// Builds a ballot from `candidates`, in the order given.
    ///
    /// A candidate whose text equals an earlier candidate's is dropped: two
    /// labels for one text would split Jev's probability mass between them
    /// and make "which occurrence" a question the model cannot see. The
    /// first occurrence, and its offsets, is the one offered.
    ///
    /// # Errors
    /// - [`JevErrorCode::SpanBallotEmpty`] when no candidate remains -- a
    ///   ballot whose only option is [`ABSTAIN_LABEL`] asks nothing.
    /// - [`JevErrorCode::SpanBallotTooLarge`] when more than
    ///   [`MAX_CANDIDATES`] distinct candidates remain. The refusal fires at
    ///   the first distinct candidate past the ceiling; the rest of
    ///   `candidates` is never read, so the ceiling bounds the work as well
    ///   as the result (each candidate is compared against at most
    ///   [`MAX_CANDIDATES`] kept ones).
    pub fn new(
        instructions: impl Into<String>,
        candidates: impl IntoIterator<Item = Span>,
    ) -> Result<Self> {
        let mut distinct: Vec<Span> = Vec::new();
        for candidate in candidates {
            if distinct.iter().any(|kept| kept.text == candidate.text) {
                continue;
            }
            if distinct.len() == MAX_CANDIDATES {
                return Err(JevError::new(
                    JevErrorCode::SpanBallotTooLarge,
                    format!(
                        "more than {MAX_CANDIDATES} distinct candidate spans exceed the ballot ceiling"
                    ),
                ));
            }
            distinct.push(candidate);
        }
        if distinct.is_empty() {
            return Err(JevError::new(
                JevErrorCode::SpanBallotEmpty,
                "a span ballot needs at least one candidate span",
            ));
        }
        let options = (1..)
            .zip(distinct)
            .map(|(number, span)| (format!("span-{number}"), span))
            .collect();
        Ok(Self {
            instructions: instructions.into(),
            options,
        })
    }

    /// The wire key to ask a ballot under: `<scope>::<ballot_id>`.
    ///
    /// The same `<scope>::<question id>` shape
    /// [`crate::question_set::QuestionSet::weakness_kind_key`] uses, so a
    /// ballot scoped to an AC row (`FR-006-AC-1::unfalsifiable_clause`) can
    /// ride in the same `Questions` map as that row's lens questions without
    /// colliding with them. Pass the same key to `response.answer(key)` when
    /// resolving.
    #[must_use]
    pub fn key(scope: &str, ballot_id: &str) -> String {
        format!("{scope}::{ballot_id}")
    }

    /// The issued labels and the span each stands for, in ballot order.
    /// [`ABSTAIN_LABEL`] is not listed; it is not a span.
    #[must_use]
    pub fn options(&self) -> &[(String, Span)] {
        &self.options
    }

    /// The `choice` question to send: one label per candidate, described by
    /// the candidate's text, then [`ABSTAIN_LABEL`].
    #[must_use]
    pub fn question(&self) -> Question {
        let spans = self
            .options
            .iter()
            .map(|(label, span)| (label.as_str(), span.text.as_str()));
        let abstain = std::iter::once((ABSTAIN_LABEL, "None of the listed spans applies."));
        choice(self.instructions.as_str(), spans.chain(abstain))
    }

    /// Resolves Jev's answer to this ballot's question.
    ///
    /// `answer` is the response's entry for the key this ballot was asked
    /// under (`response.answer(key)`); [`None`] or a non-`choice` answer is
    /// [`BallotOutcome::Unanswered`]. A label is accepted only on an exact,
    /// case-sensitive match with one this ballot issued -- no trimming, no
    /// case folding, no matching against span text.
    ///
    /// `confidence_threshold` marks a selection or abstention `unconfirmed`
    /// when its confidence falls below it -- annotated, never suppressed,
    /// under the same caller-supplied-threshold rule as
    /// [`crate::verdict::extract`] (see that function for why this crate
    /// bakes in no number of its own).
    #[must_use]
    pub fn resolve(&self, answer: Option<&Answer>, confidence_threshold: f64) -> BallotOutcome<'_> {
        let Some(Answer::Choice(choice)) = answer else {
            return BallotOutcome::Unanswered;
        };
        let confidence = choice.confidence;
        let unconfirmed = confidence < confidence_threshold;
        if choice.choice == ABSTAIN_LABEL {
            return BallotOutcome::Abstained {
                confidence,
                unconfirmed,
            };
        }
        match self
            .options
            .iter()
            .find(|(label, _)| *label == choice.choice)
        {
            Some((_, span)) => BallotOutcome::Selected {
                span,
                confidence,
                unconfirmed,
            },
            None => BallotOutcome::Unrecognized {
                label: choice.choice.clone(),
            },
        }
    }
}

/// What a ballot's answer resolved to.
///
/// [`Self::Unrecognized`] and [`Self::Unanswered`] are distinct for the same
/// reason [`crate::verdict::FrVerdict`] keeps `unrecognized` and `unanswered`
/// apart: "a verdict outside the list" is evidence something went wrong,
/// "no verdict" is an absence, and neither is a selection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum BallotOutcome<'a> {
    /// Jev picked one of the issued span labels.
    Selected {
        /// The span that label stands for.
        span: &'a Span,
        /// The choice answer's reported confidence, unmodified.
        confidence: f64,
        /// Whether `confidence` fell below the caller's threshold. Same
        /// annotate-never-suppress rule as [`crate::verdict::Finding::unconfirmed`].
        unconfirmed: bool,
    },
    /// Jev picked [`ABSTAIN_LABEL`]: no offered span applies.
    Abstained {
        /// The choice answer's reported confidence, unmodified.
        confidence: f64,
        /// Whether `confidence` fell below the caller's threshold.
        unconfirmed: bool,
    },
    /// Jev returned a label this ballot never issued.
    Unrecognized {
        /// The label exactly as returned.
        label: String,
    },
    /// No `choice` answer was present for this ballot.
    Unanswered,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use typesafe_sdk_answers::Answer;

    use super::{
        ABSTAIN_LABEL, BallotOutcome, Granularity, MAX_CANDIDATES, Span, SpanBallot,
        candidate_spans,
    };
    use crate::error::JevErrorCode;

    /// A threshold every scripted confidence in these tests sits above,
    /// except where a test is about the threshold itself.
    const THRESHOLD: f64 = 0.5;

    fn texts(spans: &[Span]) -> Vec<&str> {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    fn choice_answer(label: &str, confidence: f64) -> Answer {
        serde_json::from_value(serde_json::json!({
            "type": "choice",
            "choice": label,
            "confidence": confidence,
            "probabilities": {label: confidence}
        }))
        .unwrap()
    }

    fn ballot(text: &str) -> SpanBallot {
        SpanBallot::new(
            "Which span is untestable?",
            candidate_spans(text, Granularity::Clause),
        )
        .unwrap()
    }

    /// Provenance: PLAT-980
    #[test]
    fn sentences_split_at_terminators_followed_by_whitespace() {
        let text = "The report is written. It names v0.6.2 and 1.5 s! Does it exit? Yes; always";
        let spans = candidate_spans(text, Granularity::Sentence);
        assert_eq!(
            texts(&spans),
            vec![
                "The report is written",
                "It names v0.6.2 and 1.5 s",
                "Does it exit",
                "Yes",
                "always",
            ]
        );
    }

    /// Provenance: PLAT-980
    #[test]
    fn clauses_also_split_at_a_comma_followed_by_whitespace() {
        let text = "On timeout, the client retries twice, then fails with a,b intact.";
        let spans = candidate_spans(text, Granularity::Clause);
        assert_eq!(
            texts(&spans),
            vec![
                "On timeout",
                "the client retries twice",
                "then fails with a,b intact"
            ]
        );
        let sentence = candidate_spans(text, Granularity::Sentence);
        assert_eq!(texts(&sentence), vec![text.trim_end_matches('.')]);
    }

    /// Provenance: PLAT-980. A code span is never cut, even where it holds a
    /// delimiter followed by a space.
    #[test]
    fn a_backtick_code_span_is_never_cut() {
        let text = "It calls `a. b, c` once. Then stops.";
        let spans = candidate_spans(text, Granularity::Clause);
        assert_eq!(texts(&spans), vec!["It calls `a. b, c` once", "Then stops"]);
    }

    /// Provenance: PLAT-980
    #[test]
    fn line_breaks_split_and_punctuation_only_pieces_are_dropped() {
        let text = "- first item\n-\n\n  second item  \n";
        let spans = candidate_spans(text, Granularity::Sentence);
        assert_eq!(texts(&spans), vec!["- first item", "second item"]);
    }

    /// Provenance: PLAT-980. Offsets are byte offsets that slice back to the
    /// span's text, including across multi-byte characters.
    #[test]
    fn offsets_slice_back_to_the_span_text() {
        let text = "  Émis — une fois.  Café, puis thé.";
        let spans = candidate_spans(text, Granularity::Clause);
        assert_eq!(texts(&spans), vec!["Émis — une fois", "Café", "puis thé"]);
        for span in &spans {
            assert_eq!(&text[span.start..span.end], span.text);
        }
        assert_eq!(spans[0].start, 2);
    }

    /// Provenance: PLAT-980
    #[test]
    fn an_empty_text_has_no_candidates() {
        assert!(candidate_spans("", Granularity::Clause).is_empty());
        assert!(candidate_spans(" . , ;\n", Granularity::Clause).is_empty());
    }

    /// Provenance: PLAT-980. The question's closed label set is exactly the
    /// issued span labels plus the abstain label, each span label described
    /// by its span's text.
    #[test]
    fn the_question_offers_exactly_the_candidates_and_abstain() {
        let ballot = ballot("The run ends, a report exists.");
        let wire = serde_json::to_string(&ballot.question()).unwrap();
        assert_eq!(
            wire,
            concat!(
                r#"{"type":"choice","instructions":"Which span is untestable?","#,
                r#""criteria":{"span-1":"The run ends","span-2":"a report exists","#,
                r#""none":"None of the listed spans applies."}}"#
            )
        );
    }

    /// Provenance: PLAT-980. Duplicate texts are offered once, at their first
    /// occurrence.
    #[test]
    fn duplicate_texts_are_offered_once_at_the_first_occurrence() {
        let ballot = ballot("It retries, It retries, it stops.");
        let offered: Vec<(&str, &str, usize)> = ballot
            .options()
            .iter()
            .map(|(label, span)| (label.as_str(), span.text.as_str(), span.start))
            .collect();
        assert_eq!(
            offered,
            vec![("span-1", "It retries", 0), ("span-2", "it stops", 24)]
        );
    }

    /// Provenance: PLAT-980
    #[test]
    fn an_issued_label_resolves_to_its_span() {
        let ballot = ballot("The run ends, a report exists.");
        let answer = choice_answer("span-2", 0.8);
        let expected = Span {
            start: 14,
            end: 29,
            text: "a report exists".to_owned(),
        };
        assert_eq!(
            ballot.resolve(Some(&answer), THRESHOLD),
            BallotOutcome::Selected {
                span: &expected,
                confidence: 0.8,
                unconfirmed: false,
            }
        );
    }

    /// Provenance: PLAT-980
    #[test]
    fn the_abstain_label_resolves_to_abstained() {
        let ballot = ballot("The run ends, a report exists.");
        let answer = choice_answer(ABSTAIN_LABEL, 0.6);
        assert_eq!(
            ballot.resolve(Some(&answer), THRESHOLD),
            BallotOutcome::Abstained {
                confidence: 0.6,
                unconfirmed: false,
            }
        );
    }

    /// Provenance: PLAT-980 review finding 6. A confidence below the
    /// caller's threshold is annotated `unconfirmed`, never dropped: the
    /// selection and the abstention both still resolve.
    #[test]
    fn a_low_confidence_answer_resolves_marked_unconfirmed() {
        let ballot = ballot("The run ends, a report exists.");
        let low = choice_answer("span-1", 0.3);
        let first = Span {
            start: 0,
            end: 12,
            text: "The run ends".to_owned(),
        };
        assert_eq!(
            ballot.resolve(Some(&low), THRESHOLD),
            BallotOutcome::Selected {
                span: &first,
                confidence: 0.3,
                unconfirmed: true,
            }
        );
        let low_abstain = choice_answer(ABSTAIN_LABEL, 0.3);
        assert_eq!(
            ballot.resolve(Some(&low_abstain), THRESHOLD),
            BallotOutcome::Abstained {
                confidence: 0.3,
                unconfirmed: true,
            }
        );
    }

    /// Provenance: PLAT-980. The span's own text, a near-miss label, and a
    /// label past the issued range are all unrecognised: acceptance is a
    /// literal match on an issued label, nothing looser.
    #[test]
    fn anything_not_literally_issued_is_unrecognized() {
        let ballot = ballot("The run ends, a report exists.");
        for returned in ["a report exists", "Span-1", " span-1", "span-3", "None"] {
            let answer = choice_answer(returned, 0.9);
            assert_eq!(
                ballot.resolve(Some(&answer), THRESHOLD),
                BallotOutcome::Unrecognized {
                    label: returned.to_owned()
                },
                "{returned:?} must not resolve"
            );
        }
    }

    /// Provenance: PLAT-980. A withheld candidate is never offered, so Jev
    /// naming it -- by the label it would have had, or by its text -- cannot
    /// select it.
    #[test]
    fn a_withheld_candidate_cannot_be_selected() {
        let mut candidates = candidate_spans("The run ends, a report exists.", Granularity::Clause);
        candidates.retain(|span| span.text != "The run ends");
        let ballot = SpanBallot::new("Which span?", candidates).unwrap();
        assert_eq!(ballot.options().len(), 1);
        assert_eq!(ballot.options()[0].1.text, "a report exists");
        for returned in ["span-2", "The run ends"] {
            let answer = choice_answer(returned, 0.9);
            assert!(matches!(
                ballot.resolve(Some(&answer), THRESHOLD),
                BallotOutcome::Unrecognized { .. }
            ));
        }
    }

    /// Provenance: PLAT-980
    #[test]
    fn a_missing_or_wrongly_typed_answer_is_unanswered() {
        let ballot = ballot("The run ends, a report exists.");
        assert_eq!(ballot.resolve(None, THRESHOLD), BallotOutcome::Unanswered);
        let noul: Answer =
            serde_json::from_value(serde_json::json!({"type": "noul", "noul": 0.9})).unwrap();
        assert_eq!(
            ballot.resolve(Some(&noul), THRESHOLD),
            BallotOutcome::Unanswered
        );
    }

    /// Provenance: PLAT-980
    #[test]
    fn an_empty_ballot_is_refused() {
        let error = SpanBallot::new("Which span?", Vec::new()).unwrap_err();
        assert_eq!(error.code, JevErrorCode::SpanBallotEmpty);
    }

    /// Provenance: PLAT-980. The ceiling is inclusive and exceeding it is a
    /// refusal, not a truncation.
    #[test]
    fn a_ballot_over_the_ceiling_is_refused_not_truncated() {
        let text = |count: usize| {
            (0..count)
                .map(|n| format!("clause {n}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let at = SpanBallot::new(
            "q",
            candidate_spans(&text(MAX_CANDIDATES), Granularity::Clause),
        )
        .unwrap();
        assert_eq!(at.options().len(), MAX_CANDIDATES);
        let error = SpanBallot::new(
            "q",
            candidate_spans(&text(MAX_CANDIDATES + 1), Granularity::Clause),
        )
        .unwrap_err();
        assert_eq!(error.code, JevErrorCode::SpanBallotTooLarge);
    }

    /// Provenance: PLAT-980 review finding 1. The ceiling counts DISTINCT
    /// candidates: exactly the ceiling's worth of distinct texts followed by
    /// repeats of them is accepted, so the early refusal does not fire on a
    /// duplicate.
    #[test]
    fn repeats_past_the_ceiling_do_not_count_against_it() {
        let distinct = (0..MAX_CANDIDATES).map(|n| format!("clause {n}"));
        let repeats = (0..MAX_CANDIDATES).map(|n| format!("clause {n}"));
        let candidates = distinct.chain(repeats).map(|text| Span {
            start: 0,
            end: text.len(),
            text,
        });
        let ballot = SpanBallot::new("q", candidates).unwrap();
        assert_eq!(ballot.options().len(), MAX_CANDIDATES);
    }

    /// Provenance: PLAT-980 review finding 1. The refusal fires at the first
    /// distinct candidate past the ceiling and reads nothing after it, so a
    /// huge input costs at most `MAX_CANDIDATES + 1` candidates' work -- not
    /// a full quadratic dedup of the tail before the refusal.
    #[test]
    fn the_ceiling_refusal_stops_reading_the_candidates() {
        let consumed = std::cell::Cell::new(0_usize);
        let candidates = (0..100_000_usize)
            .inspect(|_| consumed.set(consumed.get() + 1))
            .map(|n| {
                let text = format!("clause {n}");
                Span {
                    start: 0,
                    end: text.len(),
                    text,
                }
            });
        let error = SpanBallot::new("q", candidates).unwrap_err();
        assert_eq!(error.code, JevErrorCode::SpanBallotTooLarge);
        assert_eq!(consumed.get(), MAX_CANDIDATES + 1);
    }

    /// Provenance: PLAT-980 review finding 2. FR-006's behaviour bullet from
    /// the criterion-strength fixture corpus: `e.g.,` is not a clause
    /// boundary, so the parenthetical stays with the phrase it belongs to and
    /// the only cut is the real one, after "render".
    #[test]
    fn an_abbreviation_does_not_cut_a_candidate() {
        let text = "If a node fails to render, the system SHALL render a fallback UI \
                    (e.g., \"Error loading node\") instead of crashing the tree.";
        assert_eq!(
            texts(&candidate_spans(text, Granularity::Clause)),
            vec![
                "If a node fails to render",
                "the system SHALL render a fallback UI (e.g., \"Error loading node\") \
                 instead of crashing the tree",
            ]
        );
        let text = "Retry on 5xx, i.e. a server fault. Compare A vs. B, etc. then stop.";
        assert_eq!(
            texts(&candidate_spans(text, Granularity::Sentence)),
            vec![
                "Retry on 5xx, i.e. a server fault",
                "Compare A vs. B, etc. then stop"
            ]
        );
    }

    /// Provenance: PLAT-980 review finding 2. An abbreviation matches only as
    /// a whole word: `devs.` ends in `vs` but is a sentence end.
    #[test]
    fn an_abbreviation_matches_only_as_a_whole_word() {
        let text = "It pings the devs. Then it stops.";
        assert_eq!(
            texts(&candidate_spans(text, Granularity::Sentence)),
            vec!["It pings the devs", "Then it stops"]
        );
    }

    /// Provenance: PLAT-980 review finding 3. An unclosed backtick runs to
    /// the end of the text as one candidate, across a line break and past
    /// delimiters that would otherwise cut.
    #[test]
    fn an_unclosed_backtick_runs_to_the_end_of_the_text() {
        let text = "It calls. The `tail, runs.\nOn here. And on";
        assert_eq!(
            texts(&candidate_spans(text, Granularity::Clause)),
            vec!["It calls", "The `tail, runs.\nOn here. And on"]
        );
    }

    /// Provenance: PLAT-980 review finding 4. `BallotOutcome`'s wire shape,
    /// written out literally: an `outcome` tag in snake case, the span by
    /// value, and `unconfirmed` beside `confidence`.
    #[test]
    fn the_outcome_serializes_with_an_outcome_tag() {
        let ballot = ballot("The run ends, a report exists.");
        let wire = |answer: Option<&Answer>| {
            serde_json::to_string(&ballot.resolve(answer, THRESHOLD)).unwrap()
        };
        assert_eq!(
            wire(Some(&choice_answer("span-2", 0.8))),
            concat!(
                r#"{"outcome":"selected","span":{"start":14,"end":29,"text":"a report exists"},"#,
                r#""confidence":0.8,"unconfirmed":false}"#
            )
        );
        assert_eq!(
            wire(Some(&choice_answer(ABSTAIN_LABEL, 0.25))),
            r#"{"outcome":"abstained","confidence":0.25,"unconfirmed":true}"#
        );
        assert_eq!(
            wire(Some(&choice_answer("span-9", 0.9))),
            r#"{"outcome":"unrecognized","label":"span-9"}"#
        );
        assert_eq!(wire(None), r#"{"outcome":"unanswered"}"#);
    }

    /// Provenance: PLAT-980 review finding 5. A ballot composes with a real
    /// request end to end: its question rides in a `Questions` map under
    /// [`SpanBallot::key`], goes out through the client over the mock
    /// transport, and the scripted answer under that key resolves to the
    /// span.
    #[tokio::test]
    async fn a_ballot_round_trips_through_a_request() {
        use std::sync::Arc;

        use typesafe_sdk_client::SystemOneRequest;
        use typesafe_sdk_env::Fixed;
        use typesafe_sdk_http::{Exchange, Mock};
        use typesafe_sdk_questions::Questions;

        use crate::client::with_transport;
        use crate::config::resolve;

        let ballot = ballot("The run ends, a report exists.");
        let key = SpanBallot::key("FR-900-AC-1", "untestable_clause");
        assert_eq!(key, "FR-900-AC-1::untestable_clause");
        let mut questions = Questions::new();
        questions.insert(key.clone(), ballot.question());

        let body = serde_json::json!({
            "model": "jev-1.13.0",
            "answers": {
                "FR-900-AC-1::untestable_clause": {
                    "type": "choice", "choice": "span-2", "confidence": 0.9,
                    "probabilities": {"span-2": 0.9, "none": 0.1}
                }
            },
            "usage": {"input_tokens": 40, "output_tokens": 0}
        });
        let mock = Arc::new(Mock::new(vec![Exchange::ok(&body.to_string())]));
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]);
        let client = with_transport(resolve(&env).unwrap(), mock.clone());
        let response = client
            .system_one(SystemOneRequest::new(
                "The run ends, a report exists.",
                questions,
            ))
            .await
            .unwrap();

        let sent: serde_json::Value =
            serde_json::from_str(mock.requests()[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(
            sent["questions"][key.as_str()]["criteria"]["span-2"],
            "a report exists",
            "the ballot's closed label set went out under its key: {sent}"
        );
        let expected = Span {
            start: 14,
            end: 29,
            text: "a report exists".to_owned(),
        };
        assert_eq!(
            ballot.resolve(response.answer(&key), THRESHOLD),
            BallotOutcome::Selected {
                span: &expected,
                confidence: 0.9,
                unconfirmed: false,
            }
        );
    }
}
