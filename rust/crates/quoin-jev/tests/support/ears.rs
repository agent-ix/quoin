// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-838's EARS semantic lens: request building, response extraction and
//! grading, built the same way PLAT-837's criterion-strength lens was but
//! kept entirely inside this crate's `tests/` tree.
//!
//! **This is deliberately not production code.** The standing ruling for
//! PLAT-838 is an evaluation with a GO/NO-GO verdict, not an integration --
//! nothing here is reachable from `skills/spec-ears-analysis` or any other
//! skill, and this module lives under `tests/`, never under `src/`, so
//! nothing in this crate's public API depends on it. It reuses
//! [`quoin_jev::question_set::{NoulEntry, ChoiceBlock, ScoreBlock}`] (already
//! `pub` for PLAT-837) to parse [`EarsQuestionSet`] and the
//! label-space-agnostic maths in [`crate::support::grading`] to grade
//! against the two corpora ([`M2Corpus`] and [`M6Corpus`]).
//!
//! # The "defect vs. clean" reduction
//!
//! `ears_pattern_actual` is a 6-way choice, but PLAT-838's own MP-222/223/224
//! population definitions are binary: "a matching pattern for EARS" is the
//! no-defect answer. [`grade_m2`] reduces Jev's 6-way answer plus
//! `response_measurable` to that binary read (`"defect"`/`"clean"`) against
//! each fixture's agent-labelled `has_defect`, and reports the raw 6-way
//! pattern agreement separately as supplementary detail
//! ([`pattern_class_stats`]) -- informative, not gated.

#![allow(
    dead_code,
    reason = "each test binary links this module separately, so items only one binary uses read as dead in the other"
)]

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use typesafe_sdk_answers::{Answer, SystemOneResponse};
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_questions::{Entry, Questions, choice_of, noul, score};

use quoin_jev::error::{JevError, classify};
use quoin_jev::question_set::{ChoiceBlock, NoulEntry, ScoreBlock};

use super::grading::{Graded, Tier, Verdict};

/// PLAT-838's question set, parsed from `tests/fixtures/ears-question-set.json`.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct EarsQuestionSet {
    pub(crate) choice: ChoiceBlock,
    pub(crate) noul: Vec<NoulEntry>,
    pub(crate) score: ScoreBlock,
}

impl EarsQuestionSet {
    pub(crate) fn parse(json: &str) -> Self {
        serde_json::from_str(json).expect("the shipped EARS question set parses")
    }
}

/// One requirement-bearing statement, sent as `state` -- never told the
/// engine's own keyword-derived pattern, so Jev's answer is an independent
/// read (see this crate's `MP-231` for why that independence is the point).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct StatementContext {
    pub(crate) statement_id: String,
    pub(crate) statement_type: String,
    pub(crate) statement_text: String,
    pub(crate) description: Option<String>,
    pub(crate) behaviour: Option<String>,
    pub(crate) constraints: Option<String>,
}

impl From<&StatementContext> for Entry {
    fn from(context: &StatementContext) -> Self {
        serde_json::json!({
            "statement_id": context.statement_id,
            "statement_type": context.statement_type,
            "statement_text": context.statement_text,
            "description": context.description,
            "behaviour": context.behaviour,
            "constraints": context.constraints,
        })
        .into()
    }
}

/// Builds the one-request-per-statement call PLAT-838's question set asks for.
pub(crate) fn build_request(context: &StatementContext, qs: &EarsQuestionSet) -> SystemOneRequest {
    let state: Entry = context.into();
    let mut questions = Questions::new();
    for entry in &qs.noul {
        questions.insert(entry.id.clone(), noul(entry.question.as_str()));
    }
    questions.insert(
        qs.choice.id.clone(),
        choice_of(
            "Classify this requirement statement's actual EARS pattern, independent of \
             whichever trigger keyword it happens to use.",
            qs.choice.answer_space.iter().map(String::as_str),
        ),
    );
    let labels = qs.score.rubric.iter().map(|level| level.label.as_str());
    questions.insert(
        qs.score.id.clone(),
        score(
            "Rate how much this statement's ambiguity, if any, would change what gets built \
             if left unresolved.",
            labels,
        ),
    );
    SystemOneRequest::new(state, questions)
}

/// What Jev answered for one statement.
#[derive(Debug, Clone)]
pub(crate) struct EarsVerdict {
    pub(crate) classifier: String,
    pub(crate) usage_input_tokens: u64,
    pub(crate) usage_output_tokens: u64,
    /// The recognized `ears_pattern_actual` label, when the choice answered
    /// with a member of the declared `answer_space`.
    pub(crate) pattern: Option<String>,
    pub(crate) pattern_confidence: Option<f64>,
    pub(crate) pattern_probabilities: Vec<(String, f64)>,
    /// A `ears_pattern_actual` answer outside the declared `answer_space`.
    pub(crate) unrecognized_pattern: Option<String>,
    pub(crate) response_measurable: Option<f64>,
    pub(crate) trigger_is_momentary: Option<f64>,
    pub(crate) condition_is_unwanted: Option<f64>,
    pub(crate) severity_score: Option<f64>,
    pub(crate) severity_confidence: Option<f64>,
}

/// Extracts an [`EarsVerdict`] from a raw response, against `qs`'s wire keys.
pub(crate) fn extract(response: &SystemOneResponse, qs: &EarsQuestionSet) -> EarsVerdict {
    let noul_value = |id: &str| match response.answer(id) {
        Some(Answer::Noul(answer)) => Some(answer.noul),
        _ => None,
    };

    let (pattern, pattern_confidence, pattern_probabilities, unrecognized_pattern) =
        match response.answer(&qs.choice.id) {
            Some(Answer::Choice(choice)) => {
                let recognized = qs
                    .choice
                    .answer_space
                    .iter()
                    .any(|label| label == &choice.choice);
                if recognized {
                    (
                        Some(choice.choice.clone()),
                        Some(choice.confidence),
                        choice.probabilities.clone().into_iter().collect(),
                        None,
                    )
                } else {
                    (None, None, Vec::new(), Some(choice.choice.clone()))
                }
            }
            _ => (None, None, Vec::new(), None),
        };

    let (severity_score, severity_confidence) = match response.answer(&qs.score.id) {
        Some(Answer::Score(score)) => (Some(score.score), Some(score.confidence)),
        _ => (None, None),
    };

    EarsVerdict {
        classifier: response.model.clone(),
        usage_input_tokens: response.usage.input_tokens,
        usage_output_tokens: response.usage.output_tokens,
        pattern,
        pattern_confidence,
        pattern_probabilities,
        unrecognized_pattern,
        response_measurable: noul_value("response_measurable"),
        trigger_is_momentary: noul_value("trigger_is_momentary"),
        condition_is_unwanted: noul_value("condition_is_unwanted"),
        severity_score,
        severity_confidence,
    }
}

/// Sends one statement and extracts its verdict.
///
/// # Errors
/// Any [`JevError`] the client raises, classified the same way
/// [`quoin_jev::lens::run`] does.
pub(crate) async fn run(
    client: &Client,
    context: &StatementContext,
    qs: &EarsQuestionSet,
) -> Result<EarsVerdict, JevError> {
    let request = build_request(context, qs);
    let response = client
        .system_one(request)
        .await
        .map_err(|error| classify(&error))?;
    Ok(extract(&response, qs))
}

/// Convenience: builds a client over an arbitrary transport (mock or real),
/// re-exported here so live and offline tests share one import path.
pub(crate) fn with_transport(
    config: typesafe_sdk_config::Config,
    transport: Arc<dyn typesafe_sdk_http::Transport>,
) -> Client {
    quoin_jev::client::with_transport(config, transport)
}

// ---------------------------------------------------------------------
// M2: the agent-labelled fixture corpus
// ---------------------------------------------------------------------

/// `tests/fixtures/ears-m2-fixtures.json`, compiled in so the fixtures and
/// this grader cannot drift.
const M2_CORPUS: &str = include_str!("../fixtures/ears-m2-fixtures.json");

/// `tests/fixtures/ears-question-set.json`, compiled in.
pub(crate) const QUESTION_SET_JSON: &str = include_str!("../fixtures/ears-question-set.json");

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct M2Labels {
    pub(crate) true_pattern: String,
    pub(crate) true_pattern_contested: Option<Vec<String>>,
    pub(crate) response_measurable: bool,
    pub(crate) response_measurable_contested: Option<Vec<bool>>,
    pub(crate) has_defect: bool,
    pub(crate) has_defect_contested: Option<Vec<bool>>,
    #[serde(default)]
    pub(crate) defect_kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct M2Fixture {
    pub(crate) fixture_id: String,
    pub(crate) confidence: Tier,
    pub(crate) statement_id: String,
    pub(crate) statement_type: String,
    pub(crate) statement_text: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) behaviour: Option<String>,
    #[serde(default)]
    pub(crate) constraints: Option<String>,
    pub(crate) engine_naive_pattern: String,
    pub(crate) labels: M2Labels,
}

impl M2Fixture {
    pub(crate) fn context(&self) -> StatementContext {
        StatementContext {
            statement_id: self.statement_id.clone(),
            statement_type: self.statement_type.clone(),
            statement_text: self.statement_text.clone(),
            description: self.description.clone(),
            behaviour: self.behaviour.clone(),
            constraints: self.constraints.clone(),
        }
    }

    /// Every defect/clean reading this fixture recorded, "clean" first.
    fn defect_readings(&self) -> Vec<&'static str> {
        let primary = if self.labels.has_defect {
            "defect"
        } else {
            "clean"
        };
        match &self.labels.has_defect_contested {
            Some(all) => all
                .iter()
                .map(|v| if *v { "defect" } else { "clean" })
                .collect(),
            None => vec![primary],
        }
    }

    /// Every EARS-pattern reading this fixture recorded, primary first.
    fn pattern_readings(&self) -> Vec<&str> {
        self.labels.true_pattern_contested.as_ref().map_or_else(
            || vec![self.labels.true_pattern.as_str()],
            |all| all.iter().map(String::as_str).collect(),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct M2Corpus {
    pub(crate) fixtures: Vec<M2Fixture>,
}

/// Parses the compiled-in M2 corpus.
///
/// # Panics
/// If the asset stops matching this shape.
pub(crate) fn m2_corpus() -> M2Corpus {
    serde_json::from_str(M2_CORPUS).expect("the M2 fixture corpus parses")
}

/// The label naming "no defect" in every M2 binary grading -- see this
/// module's doc for the reduction this implements.
pub(crate) const NO_DEFECT: &str = "clean";

/// Grades one M2 fixture on the binary defect/clean reduction (MP-222/223/224's
/// own population).
///
/// "Defect" is Jev's own independent judgment disagreeing with the
/// statement's naive engine-derived pattern, OR Jev itself judging the
/// response unmeasurable -- never whether Jev's answer matches this corpus's
/// `true_pattern` directly (that comparison is [`grade_pattern`], reported
/// separately as supplementary detail).
pub(crate) fn grade_defect(fixture: &M2Fixture, verdict: &EarsVerdict) -> Graded {
    let readings = fixture.defect_readings();

    if let Some(label) = &verdict.unrecognized_pattern {
        return Graded {
            fixture_id: fixture.fixture_id.clone(),
            tier: fixture.confidence,
            expected: expected_defect_label(&fixture.labels),
            contested: readings.iter().map(|r| (*r).to_owned()).collect(),
            actual: label.clone(),
            actual_class: label.clone(),
            verdict: Verdict::Unrecognized,
            confidence: verdict.pattern_confidence,
        };
    }
    let Some(pattern) = &verdict.pattern else {
        return Graded {
            fixture_id: fixture.fixture_id.clone(),
            tier: fixture.confidence,
            expected: expected_defect_label(&fixture.labels),
            contested: readings.iter().map(|r| (*r).to_owned()).collect(),
            actual: "<unanswered>".to_owned(),
            actual_class: "<unanswered>".to_owned(),
            verdict: Verdict::Unanswered,
            confidence: None,
        };
    };

    let pattern_mismatch = pattern != &fixture.engine_naive_pattern;
    let unmeasurable = verdict.response_measurable.is_some_and(|value| value < 0.5);
    let actual_class = if pattern_mismatch || unmeasurable {
        "defect"
    } else {
        "clean"
    }
    .to_owned();

    let expected = expected_defect_label(&fixture.labels);
    let outcome = if actual_class == expected {
        Verdict::Primary
    } else if readings.contains(&actual_class.as_str()) {
        Verdict::Contested
    } else {
        Verdict::Wrong
    };

    Graded {
        fixture_id: fixture.fixture_id.clone(),
        tier: fixture.confidence,
        expected,
        contested: readings.iter().map(|r| (*r).to_owned()).collect(),
        actual: actual_class.clone(),
        actual_class,
        verdict: outcome,
        confidence: verdict.pattern_confidence,
    }
}

fn expected_defect_label(labels: &M2Labels) -> String {
    if labels.has_defect { "defect" } else { "clean" }.to_owned()
}

/// Grades one M2 fixture on the raw 6-way `ears_pattern_actual` choice,
/// against this corpus's `true_pattern` -- supplementary detail, not gated
/// (MP-229 gates on [`grade_defect`] only).
pub(crate) fn grade_pattern(fixture: &M2Fixture, verdict: &EarsVerdict) -> Graded {
    let readings = fixture.pattern_readings();

    if let Some(label) = &verdict.unrecognized_pattern {
        return Graded {
            fixture_id: fixture.fixture_id.clone(),
            tier: fixture.confidence,
            expected: fixture.labels.true_pattern.clone(),
            contested: readings.iter().map(|r| (*r).to_owned()).collect(),
            actual: label.clone(),
            actual_class: label.clone(),
            verdict: Verdict::Unrecognized,
            confidence: verdict.pattern_confidence,
        };
    }
    let Some(pattern) = &verdict.pattern else {
        return Graded {
            fixture_id: fixture.fixture_id.clone(),
            tier: fixture.confidence,
            expected: fixture.labels.true_pattern.clone(),
            contested: readings.iter().map(|r| (*r).to_owned()).collect(),
            actual: "<unanswered>".to_owned(),
            actual_class: "<unanswered>".to_owned(),
            verdict: Verdict::Unanswered,
            confidence: None,
        };
    };

    let outcome = if pattern == &fixture.labels.true_pattern {
        Verdict::Primary
    } else if readings.contains(&pattern.as_str()) {
        Verdict::Contested
    } else {
        Verdict::Wrong
    };

    Graded {
        fixture_id: fixture.fixture_id.clone(),
        tier: fixture.confidence,
        expected: fixture.labels.true_pattern.clone(),
        contested: readings.iter().map(|r| (*r).to_owned()).collect(),
        actual: pattern.clone(),
        actual_class: pattern.clone(),
        verdict: outcome,
        confidence: verdict.pattern_confidence,
    }
}

// ---------------------------------------------------------------------
// M6: the real-statement engine/semantic delta corpus
// ---------------------------------------------------------------------

const M6_CORPUS: &str = include_str!("../fixtures/ears-m6-corpus.json");

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct M6Statement {
    pub(crate) repo: String,
    pub(crate) path: String,
    pub(crate) line: Option<u64>,
    pub(crate) statement_id: String,
    pub(crate) statement_type: String,
    pub(crate) statement_text: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) behaviour: Option<String>,
    #[serde(default)]
    pub(crate) constraints: Option<String>,
    pub(crate) engine_tags: Vec<String>,
    pub(crate) engine_naive_pattern: String,
}

impl M6Statement {
    pub(crate) fn context(&self) -> StatementContext {
        StatementContext {
            statement_id: self.statement_id.clone(),
            statement_type: self.statement_type.clone(),
            statement_text: self.statement_text.clone(),
            description: self.description.clone(),
            behaviour: self.behaviour.clone(),
            constraints: self.constraints.clone(),
        }
    }

    pub(crate) fn id(&self) -> String {
        format!("{}:{}:{}", self.repo, self.path, self.line.unwrap_or(0))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct M6Corpus {
    pub(crate) flagged: Vec<M6Statement>,
    pub(crate) clean: Vec<M6Statement>,
}

/// Parses the compiled-in M6 corpus.
///
/// # Panics
/// If the asset stops matching this shape.
pub(crate) fn m6_corpus() -> M6Corpus {
    serde_json::from_str(M6_CORPUS).expect("the M6 corpus parses")
}

/// Whether Jev's answer for one M6 statement gives reason to treat it as a
/// semantic defect: its pattern disagrees with the statement's own
/// engine-derived trigger keyword, or Jev itself judges the response
/// unmeasurable.
pub(crate) fn jev_flags_defect(statement: &M6Statement, verdict: &EarsVerdict) -> Option<bool> {
    let pattern = verdict.pattern.as_ref()?;
    let pattern_mismatch = pattern != &statement.engine_naive_pattern;
    let unmeasurable = verdict.response_measurable.is_some_and(|value| value < 0.5);
    Some(pattern_mismatch || unmeasurable)
}
