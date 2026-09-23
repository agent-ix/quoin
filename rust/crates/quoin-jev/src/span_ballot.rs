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
/// - Nothing inside a backtick code span breaks: spec text names symbols
///   such as `` `Client::system_one` `` and paths such as `` `src/a.rs` ``,
///   and cutting through one would offer Jev half a symbol. An unclosed
///   backtick runs to the end of the text.
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
                && chars.peek().is_none_or(|(_, next)| next.is_whitespace()));
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
    ///   [`MAX_CANDIDATES`] distinct candidates remain.
    pub fn new(
        instructions: impl Into<String>,
        candidates: impl IntoIterator<Item = Span>,
    ) -> Result<Self> {
        let mut distinct: Vec<Span> = Vec::new();
        for candidate in candidates {
            if !distinct.iter().any(|kept| kept.text == candidate.text) {
                distinct.push(candidate);
            }
        }
        if distinct.is_empty() {
            return Err(JevError::new(
                JevErrorCode::SpanBallotEmpty,
                "a span ballot needs at least one candidate span",
            ));
        }
        if distinct.len() > MAX_CANDIDATES {
            return Err(JevError::new(
                JevErrorCode::SpanBallotTooLarge,
                format!(
                    "{} distinct candidate spans exceed the ballot ceiling of {MAX_CANDIDATES}",
                    distinct.len()
                ),
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
    #[must_use]
    pub fn resolve(&self, answer: Option<&Answer>) -> BallotOutcome<'_> {
        let Some(Answer::Choice(choice)) = answer else {
            return BallotOutcome::Unanswered;
        };
        if choice.choice == ABSTAIN_LABEL {
            return BallotOutcome::Abstained {
                confidence: choice.confidence,
            };
        }
        match self
            .options
            .iter()
            .find(|(label, _)| *label == choice.choice)
        {
            Some((_, span)) => BallotOutcome::Selected {
                span,
                confidence: choice.confidence,
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
    },
    /// Jev picked [`ABSTAIN_LABEL`]: no offered span applies.
    Abstained {
        /// The choice answer's reported confidence, unmodified.
        confidence: f64,
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
            ballot.resolve(Some(&answer)),
            BallotOutcome::Selected {
                span: &expected,
                confidence: 0.8
            }
        );
    }

    /// Provenance: PLAT-980
    #[test]
    fn the_abstain_label_resolves_to_abstained() {
        let ballot = ballot("The run ends, a report exists.");
        let answer = choice_answer(ABSTAIN_LABEL, 0.6);
        assert_eq!(
            ballot.resolve(Some(&answer)),
            BallotOutcome::Abstained { confidence: 0.6 }
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
                ballot.resolve(Some(&answer)),
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
                ballot.resolve(Some(&answer)),
                BallotOutcome::Unrecognized { .. }
            ));
        }
    }

    /// Provenance: PLAT-980
    #[test]
    fn a_missing_or_wrongly_typed_answer_is_unanswered() {
        let ballot = ballot("The run ends, a report exists.");
        assert_eq!(ballot.resolve(None), BallotOutcome::Unanswered);
        let noul: Answer =
            serde_json::from_value(serde_json::json!({"type": "noul", "noul": 0.9})).unwrap();
        assert_eq!(ballot.resolve(Some(&noul)), BallotOutcome::Unanswered);
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
}
