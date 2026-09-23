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
    /// `real` (verbatim from a spec tree under `~/dev`, with `source_repo`,
    /// `source_path` and `source_commit` recorded) or `synthetic` (authored
    /// for this evaluation). Defaults to `synthetic` so the original sixteen
    /// fixtures keep their meaning if the field is ever dropped.
    #[serde(default = "provenance_synthetic")]
    pub(crate) provenance: String,
    #[serde(default)]
    pub(crate) source_repo: Option<String>,
    #[serde(default)]
    pub(crate) source_path: Option<String>,
    #[serde(default)]
    pub(crate) source_commit: Option<String>,
    pub(crate) labels: M2Labels,
}

fn provenance_synthetic() -> String {
    "synthetic".to_owned()
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

// ---------------------------------------------------------------------
// v3: derive the defect call from the `noul` answers alone
// ---------------------------------------------------------------------

/// `v3`'s derivation rule, unchanged from the form pre-registered in commit
/// `2965c25` and measured in `8b5d87f`; it moved here from
/// `tests/live_ears_semantic.rs` so that the M2 grading and the MP-231 M6
/// flagging call **one** function rather than two copies of a rule.
///
/// The shipped rule ([`grade_defect`] / [`jev_flags_defect`]) calls a
/// statement defective when Jev's six-way `ears_pattern_actual` choice
/// disagrees with the engine's naive pattern, **or** when Jev judges the
/// response unmeasurable. `v3` deletes the first term: the label comes only
/// from the three `noul` answers, plus the engine's own deterministic
/// pattern as context. Jev's six-way pick is not consulted at all.
///
/// The priority order is read off `ears-question-set.json`'s own `note`
/// fields, not fitted to any corpus's answer key:
///
/// * `response_measurable` — the note names the denylist it exists to
///   replace ("`shall be robust` sails through"), so an unmeasurable
///   response is a defect on any pattern.
/// * `condition_is_unwanted` — the note says "Disambiguates When/If", so on
///   a statement the engine read as `event_driven`, an unwanted condition
///   means the `When` should have been an `If`.
/// * `trigger_is_momentary` — the note says "Disambiguates When/While", so
///   on the same engine reading, a non-momentary trigger means the `When`
///   should have been a `While`.
///
/// # Defect found in this rule's own documentation, 2026-09-21
///
/// The rule as pre-registered says "a missing answer scores `0.5`, which
/// fires no branch: an answer the service did not give must not manufacture
/// a defect". **That is not what the code does, and the code is what was
/// measured.** `condition_is_unwanted.unwrap_or(0.5) >= 0.5` is `true` at
/// exactly `0.5`, so on an engine-`event_driven` statement an *unanswered*
/// `condition_is_unwanted` raises a defect. Only `response_measurable`
/// (`< 0.5`) and `trigger_is_momentary` (`< 0.5`) behave as the prose
/// claims.
///
/// The code is **left exactly as pre-registered**: narrowing it after seeing
/// a corpus would be fitting the rule to a result, which is the whole thing
/// pre-registration exists to prevent. What is added instead is a count --
/// every live pass reports how many rows had an unanswered consulted noul,
/// so a reader can tell whether this branch could have affected the numbers
/// at all. On every pass measured so far the service answered all three
/// nouls on every row, so the count is zero and the discrepancy is latent.
/// `the_v3_rule_fires_only_on_its_three_documented_branches` asserts the
/// real behaviour, not the prose, so the mismatch cannot be forgotten.
///
/// # Stated blind spot
///
/// Both disambiguators are phrased relative to a `When` statement, so this
/// rule can raise a pattern-confusion defect only where the engine already
/// read `event_driven` — see [`v3_reaches`]. A `While`-really-`When`, an
/// `If`-really-`When`, a `Where`-misuse or a missing trigger is unreachable.
/// The original sixteen-fixture corpus contained no such case, so the cost
/// was unmeasurable there; the grown corpus carries five deliberately, which
/// is what makes the v3 defect-recall number below mean something.
pub(crate) fn derive_defect_from_noul(
    engine_naive_pattern: &str,
    verdict: &EarsVerdict,
) -> &'static str {
    if verdict.response_measurable.unwrap_or(0.5) < 0.5 {
        return "defect";
    }
    if engine_naive_pattern == "event_driven"
        && (verdict.condition_is_unwanted.unwrap_or(0.5) >= 0.5
            || verdict.trigger_is_momentary.unwrap_or(0.5) < 0.5)
    {
        return "defect";
    }
    NO_DEFECT
}

/// Whether [`derive_defect_from_noul`] is even able to raise this fixture's
/// recorded defect — an honest split of the defect-recall denominator, not a
/// filter: both halves are reported.
///
/// An `unmeasurable_response` defect is reachable on any pattern (the
/// `response_measurable` branch has no pattern precondition). Every other
/// defect kind is a pattern confusion, and the rule can only raise one where
/// the engine's naive read was `event_driven`.
pub(crate) fn v3_reaches(fixture: &M2Fixture) -> bool {
    if fixture.labels.defect_kind.as_deref() == Some("unmeasurable_response") {
        return true;
    }
    fixture.engine_naive_pattern == "event_driven"
}

/// Grades one M2 fixture under [`derive_defect_from_noul`], through the same
/// primary/contested/wrong comparison [`grade_defect`] uses, so the two
/// columns differ only in how the label was reached.
pub(crate) fn grade_defect_derived(fixture: &M2Fixture, verdict: &EarsVerdict) -> Graded {
    let expected = expected_defect_label(&fixture.labels);
    let contested: Vec<String> = fixture
        .defect_readings()
        .iter()
        .map(|reading| (*reading).to_owned())
        .collect();
    let actual_class = derive_defect_from_noul(&fixture.engine_naive_pattern, verdict).to_owned();
    let outcome = if actual_class == expected {
        Verdict::Primary
    } else if contested.contains(&actual_class) {
        Verdict::Contested
    } else {
        Verdict::Wrong
    };
    Graded {
        fixture_id: fixture.fixture_id.clone(),
        tier: fixture.confidence,
        expected,
        contested,
        actual: actual_class.clone(),
        actual_class,
        verdict: outcome,
        confidence: verdict.pattern_confidence,
    }
}

/// The `v3` counterpart of [`jev_flags_defect`]: whether Jev's answers for
/// one M6 statement give reason to treat it as a semantic defect **under the
/// rule `v3` is gated on**, so MP-231's delta and MP-222/223/224's labels
/// come from one classification method rather than two.
///
/// `None` when the service answered none of the `noul` questions this rule
/// consults — a statement with no consulted answer is not evidence either
/// way, and MP-231 counts it out of the denominator rather than as agreement.
pub(crate) fn jev_flags_defect_v3(statement: &M6Statement, verdict: &EarsVerdict) -> Option<bool> {
    let consults_disambiguators = statement.engine_naive_pattern == "event_driven";
    let answered = verdict.response_measurable.is_some()
        || (consults_disambiguators
            && (verdict.condition_is_unwanted.is_some() || verdict.trigger_is_momentary.is_some()));
    answered.then(|| derive_defect_from_noul(&statement.engine_naive_pattern, verdict) == "defect")
}

// ---------------------------------------------------------------------
// PLAT-979: the error-analysis variants (v4, v5, v6)
// ---------------------------------------------------------------------

/// `tests/fixtures/ears-question-set-v5.json`: the `v5` wording variant.
/// Only two `noul` question texts differ from [`QUESTION_SET_JSON`]; ids,
/// the six-way choice and the score are unchanged, so [`extract`] reads it
/// with no change.
pub(crate) const QUESTION_SET_V5_JSON: &str = include_str!("../fixtures/ears-question-set-v5.json");

/// Which derivation rule turns the `noul` answers into a defect call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefectRule {
    /// `v3`'s rule, unchanged: [`derive_defect_from_noul`]. The two
    /// disambiguators are consulted only where the engine read `When`.
    WhenOnly,
    /// `v6`: `v3`'s rule plus the two keyword readings the existing
    /// disambiguators can also contradict -- a `While` whose trigger is
    /// momentary (really a `When`) and an `If` whose condition is not
    /// unwanted (really a `When`). No question is added; the answers are
    /// already asked on every request and `v3` did not read them.
    KeywordContradiction,
}

/// The defect-side probability of every `noul` answer `rule` consults on a
/// statement the engine read as `engine_naive_pattern`: `1 - p` for a
/// question whose *no* is the defect, `p` for one whose *yes* is. An
/// unanswered question contributes nothing.
fn defect_side_probabilities(
    rule: DefectRule,
    engine_naive_pattern: &str,
    verdict: &EarsVerdict,
) -> Vec<f64> {
    let measurable = verdict.response_measurable.map(|p| 1.0 - p);
    let (unwanted, momentary) = match (rule, engine_naive_pattern) {
        (_, "event_driven") => (
            verdict.condition_is_unwanted,
            verdict.trigger_is_momentary.map(|p| 1.0 - p),
        ),
        (DefectRule::KeywordContradiction, "state_driven") => (None, verdict.trigger_is_momentary),
        (DefectRule::KeywordContradiction, "unwanted_behaviour") => {
            (verdict.condition_is_unwanted.map(|p| 1.0 - p), None)
        }
        _ => (None, None),
    };
    [measurable, unwanted, momentary]
        .into_iter()
        .flatten()
        .collect()
}

/// The defect call under `rule`.
///
/// [`DefectRule::WhenOnly`] is exactly [`derive_defect_from_noul`], quirk
/// included. [`DefectRule::KeywordContradiction`] is that call, or any of its
/// two added branches reading past `0.5` on the defect side.
pub(crate) fn derive_defect(
    rule: DefectRule,
    engine_naive_pattern: &str,
    verdict: &EarsVerdict,
) -> &'static str {
    let when_only = derive_defect_from_noul(engine_naive_pattern, verdict);
    match rule {
        DefectRule::WhenOnly => when_only,
        DefectRule::KeywordContradiction => {
            let added = match engine_naive_pattern {
                "state_driven" => verdict.trigger_is_momentary,
                "unwanted_behaviour" => verdict.condition_is_unwanted.map(|p| 1.0 - p),
                _ => None,
            };
            if when_only == "defect" || added.is_some_and(|p| p > 0.5) {
                "defect"
            } else {
                NO_DEFECT
            }
        }
    }
}

/// The confidence of [`derive_defect`]'s own call, read off the answers that
/// made it -- **not** the six-way pick's confidence, which every grading up
/// to `v3` attached and which the `v3` rule never consults.
///
/// The call is an OR over branches, so the strongest defect-side answer
/// decides it: a `defect` call is as sure as its strongest firing branch
/// (`max`), and a `clean` call is only as sure as its weakest resisting
/// branch (`1 - max`). `None` when no consulted question was answered.
pub(crate) fn derived_confidence(
    rule: DefectRule,
    engine_naive_pattern: &str,
    verdict: &EarsVerdict,
) -> Option<f64> {
    let strongest = defect_side_probabilities(rule, engine_naive_pattern, verdict)
        .into_iter()
        .reduce(f64::max)?;
    Some(
        if derive_defect(rule, engine_naive_pattern, verdict) == NO_DEFECT {
            1.0 - strongest
        } else {
            strongest
        },
    )
}

/// Grades one M2 fixture under `rule`, carrying [`derived_confidence`] --
/// the grading every PLAT-979 variant (`v4` on) reports, so MP-226's
/// calibration error is computed over the confidence of the call actually
/// graded.
pub(crate) fn grade_under(fixture: &M2Fixture, verdict: &EarsVerdict, rule: DefectRule) -> Graded {
    let expected = expected_defect_label(&fixture.labels);
    let contested: Vec<String> = fixture
        .defect_readings()
        .iter()
        .map(|reading| (*reading).to_owned())
        .collect();
    let actual_class = derive_defect(rule, &fixture.engine_naive_pattern, verdict).to_owned();
    let outcome = if actual_class == expected {
        Verdict::Primary
    } else if contested.contains(&actual_class) {
        Verdict::Contested
    } else {
        Verdict::Wrong
    };
    Graded {
        fixture_id: fixture.fixture_id.clone(),
        tier: fixture.confidence,
        expected,
        contested,
        actual: actual_class.clone(),
        actual_class,
        verdict: outcome,
        confidence: derived_confidence(rule, &fixture.engine_naive_pattern, verdict),
    }
}

/// MP-231's flag under `rule`: `None` when no consulted question was
/// answered, so the statement leaves the denominator.
pub(crate) fn jev_flags_under(
    statement: &M6Statement,
    verdict: &EarsVerdict,
    rule: DefectRule,
) -> Option<bool> {
    let answered =
        !defect_side_probabilities(rule, &statement.engine_naive_pattern, verdict).is_empty();
    answered.then(|| derive_defect(rule, &statement.engine_naive_pattern, verdict) == "defect")
}

/// Whether `rule` can raise this fixture's recorded defect at all -- the
/// reachable/unreachable split [`v3_reaches`] reports, per rule.
pub(crate) fn reaches(fixture: &M2Fixture, rule: DefectRule) -> bool {
    match rule {
        DefectRule::WhenOnly => v3_reaches(fixture),
        DefectRule::KeywordContradiction => {
            v3_reaches(fixture)
                || matches!(
                    fixture.labels.defect_kind.as_deref(),
                    Some("while_is_really_when" | "if_is_really_when")
                )
        }
    }
}
