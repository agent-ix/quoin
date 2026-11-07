// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Unit tests for `span_ballot`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
use typesafe_sdk_answers::Answer;

use super::{
    ABSTAIN_LABEL, BallotOutcome, Granularity, MAX_CANDIDATES, Span, SpanBallot, candidate_spans,
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
    assert_eq!(candidate_spans("", Granularity::Clause).len(), 0);
    assert_eq!(candidate_spans(" . , ;\n", Granularity::Clause).len(), 0);
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
