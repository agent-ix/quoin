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
mod tests;
