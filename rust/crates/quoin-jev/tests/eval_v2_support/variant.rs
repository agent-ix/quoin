// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The variant registry, per-unit fan-out, and the runner (PLAT-1027).
//!
//! # A variant
//!
//! A [`Variant`] is a named, versioned pair of plain functions:
//!
//! - `asks(row)` builds the requests the variant sends for one row. One
//!   request for a whole-row question set; one per unit for a per-unit one
//!   (see [`per_unit_asks`]).
//! - `derive(row, answered)` turns the raw answers into one graded
//!   [`Prediction`] per question key. Deriving in code is first-class: a key
//!   may be read straight off one answer, computed by a rule over several
//!   fact answers, or rolled up over units ([`rollup_any_yes`],
//!   [`rollup_max_level`]).
//!
//! Only the keys in `grades` are scored, so variants that share one request
//! shape can each claim the key they are about.
//!
//! # Adding one
//!
//! Write its `asks` and `derive`, declare a `const` [`Variant`] (with the
//! artifacts its questions refer to in `references`), and add it to
//! [`REGISTRY`]. `tc_1027_every_registered_variant_respects_its_modes` then
//! checks its wording against every mode it claims. Bump `version` whenever
//! its wording or its derive rule changes, so two reports never share a
//! label for different instruments.
//!
//! # The wording rule
//!
//! A variant's questions must never refer to an artifact its row's mode does
//! not carry. Every variant declares the artifacts its questions refer to
//! (`references`), and [`wording_violations`] checks, structurally first:
//! the declaration against the mode; the state sent, by field name, for any
//! field of an absent artifact; and, as a second net, the question text
//! (never the row's own text) for test or code vocabulary that is absent from
//! the mode or missing from the declaration. "Implementation" in the abstract
//! (criterion strength's "a property of the implementation") is not a
//! reference to the code artifact and is allowed.
//!
//! # Shipped baselines
//!
//! Only what already exists, so experiments have something to beat:
//!
//! | id | shape | mode | grades |
//! | --- | --- | --- | --- |
//! | `B0` | `FullBatteryV1` (`live_gap_remaining4.rs`) | RTC | all seven battery keys |
//! | `S0` | same request | RTC | `severity` |
//! | `E0` | same request | RTC | `code_exceeds_requirement` |
//! | `T0` | same request | RTC | `test_asserts_intent` |
//! | `C0` | criterion-strength lens as shipped | any | `criterion_sound`, `untestable`, `no_measurable_threshold` |
//!
//! `S0`/`E0`/`T0`/`B0` send the identical request, and the runner asks each
//! distinct request once per run, so naming all four costs one call per row.
//! `FullBatteryV1` mentions both test and code, so there is no `RT` or `RC`
//! baseline here: an `RT` `T0` is new wording, and that is PLAT-1030's.

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use regex::Regex;
use typesafe_sdk_answers::{Answer, SystemOneResponse};
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_questions::{Entry, Question, Questions};

use quoin_jev::{AcRow, ContextPolicy, FrContext, QuestionSet};

use super::corpus::Row;
use super::keys::{Mode, NO, YES};
use super::units::{Unit, split_units};
use super::variants::intent::{T0_RT, T1, T2, T3, TC_RC, TC_RT, TC_RTC};
use super::variants::{exceeds, severity, soundness, statement};
use crate::gap_semantic_support::{
    Variant as BatteryShape, nearest_rubric_label, question_set as battery_questions,
};

/// One answer, reduced to what grading reads.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RawAnswer {
    /// A `noul`: the probability of yes.
    Noul(f64),
    /// A `choice`: the label, its confidence, and every label's probability.
    Choice {
        /// The chosen label.
        label: String,
        /// Its reported confidence.
        confidence: f64,
        /// Every label's probability.
        probabilities: BTreeMap<String, f64>,
    },
    /// A `score`: the expected level, its confidence, and every level's
    /// probability.
    Score {
        /// The expected score, possibly between levels.
        score: f64,
        /// Its reported confidence.
        confidence: f64,
        /// Every level's probability, keyed by the level as the service
        /// spells it (a decimal string). PLAT-1028's S2 reads the mass on
        /// the top level, not only the expected score.
        probabilities: BTreeMap<String, f64>,
    },
}

/// Every answer in one response, by wire key.
pub(crate) type RawAnswers = BTreeMap<String, RawAnswer>;

/// Reduces a response to [`RawAnswers`].
pub(crate) fn raw_answers(response: &SystemOneResponse) -> RawAnswers {
    response
        .answers
        .iter()
        .map(|(key, answer)| {
            let raw = match answer {
                Answer::Noul(noul) => RawAnswer::Noul(noul.noul),
                Answer::Choice(choice) => RawAnswer::Choice {
                    label: choice.choice.clone(),
                    confidence: choice.confidence,
                    probabilities: choice
                        .probabilities
                        .iter()
                        .map(|(label, p)| (label.clone(), *p))
                        .collect(),
                },
                Answer::Score(score) => RawAnswer::Score {
                    score: score.score,
                    confidence: score.confidence,
                    probabilities: score
                        .probabilities
                        .iter()
                        .map(|(level, p)| (level.clone(), *p))
                        .collect(),
                },
            };
            (key.clone(), raw)
        })
        .collect()
}

/// One request a variant sends for a row. `unit` is the unit index for a
/// per-unit request, `None` for a whole-row one.
#[derive(Debug, Clone)]
pub(crate) struct Ask {
    /// Which unit this asks about, if any.
    pub(crate) unit: Option<usize>,
    /// The request.
    pub(crate) request: SystemOneRequest,
}

/// The answers to one [`Ask`], in the same order as the asks.
#[derive(Debug, Clone)]
pub(crate) struct Answered {
    /// The ask's unit.
    pub(crate) unit: Option<usize>,
    /// Its answers.
    pub(crate) answers: RawAnswers,
}

/// A graded answer for one question key.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Prediction {
    /// A label from the key's answer space.
    pub(crate) answer: String,
    /// The confidence in `answer`, when the variant has one.
    pub(crate) confidence: Option<f64>,
    /// A number that orders predictions (higher = higher level, or more
    /// likely yes), for ordering quality.
    pub(crate) ordinal: Option<f64>,
}

/// Graded answers by question key.
pub(crate) type Predictions = BTreeMap<&'static str, Prediction>;

/// One registered variant. See the module doc.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Variant {
    /// Short id, e.g. `S0`.
    pub(crate) id: &'static str,
    /// Bumped on any wording or derive change.
    pub(crate) version: u32,
    /// One line on what it asks.
    pub(crate) summary: &'static str,
    /// The modes it may run on.
    pub(crate) modes: &'static [Mode],
    /// The artifacts besides the requirement its questions refer to. Checked
    /// against every mode in `modes` by [`wording_violations`].
    pub(crate) references: &'static [Artifact],
    /// The question keys it is graded on.
    pub(crate) grades: &'static [&'static str],
    /// Builds its requests for one row.
    pub(crate) asks: fn(&Row) -> Vec<Ask>,
    /// Turns its answers into graded answers.
    pub(crate) derive: fn(&Row, &[Answered]) -> Predictions,
}

impl Variant {
    /// `id@vN`, the label every report uses.
    pub(crate) fn label(&self) -> String {
        format!("{}@v{}", self.id, self.version)
    }

    /// Whether this variant runs on `row`.
    pub(crate) fn applies_to(&self, row: &Row) -> bool {
        self.modes.contains(&row.mode)
    }
}

/// Every variant, in the order a default run uses.
pub(crate) const REGISTRY: &[Variant] = &[
    B0,
    S0,
    E0,
    T0,
    C0,
    // PLAT-1028 severity variants; bars in spec/assurance/MP-240.
    severity::S1,
    severity::S1_RT,
    severity::S1_RC,
    severity::S2,
    severity::S2_RT,
    severity::S2_RC,
    severity::S2M,
    severity::S2M_RT,
    severity::S2M_RC,
    severity::S3,
    // PLAT-1029 code_exceeds_requirement variants; bars in spec/assurance/MP-241.
    exceeds::E0_RC,
    exceeds::E1,
    exceeds::E2,
    exceeds::E4,
    // PLAT-1024 round 2 (MP-241 "Round 2"): per-unit outcome necessity.
    exceeds::E5,
    // PLAT-1030 test_asserts_intent and trace variants; bars in spec/assurance/MP-242.
    T0_RT,
    TC_RT,
    TC_RC,
    TC_RTC,
    T1,
    T2,
    // PLAT-1024 round 2 (MP-242 "Round 2"): assertion selection.
    T3,
    // PLAT-1031 criterion-soundness variants; bars in spec/assurance/MP-243.
    soundness::K1,
    soundness::K2,
    // PLAT-1024 experiment 3: the requirement statement's own checks (dev only).
    statement::F0,
    statement::F1,
];

/// Resolves a comma-separated id list against [`REGISTRY`].
///
/// # Errors
/// On an unknown id, naming the known ones.
pub(crate) fn resolve(ids: &str) -> Result<Vec<&'static Variant>, String> {
    ids.split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(|id| {
            REGISTRY
                .iter()
                .find(|variant| variant.id == id)
                .ok_or_else(|| {
                    let known: Vec<&str> = REGISTRY.iter().map(|variant| variant.id).collect();
                    format!("unknown variant {id:?}; registered: {known:?}")
                })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// State and reading answers
// ---------------------------------------------------------------------------

/// The `state` every variant sends for a row, built from the row's mode: the
/// requirement always, the test and the code only when the mode carries them.
/// Field names match the gap-analysis battery's state (PLAT-839) so the
/// `RTC` baseline is the request that battery was measured with, plus
/// `requirement_context` when the row has one.
pub(crate) fn state(row: &Row) -> serde_json::Value {
    let mut state = serde_json::Map::new();
    let mut put = |key: &str, value: &str| {
        state.insert(key.to_owned(), serde_json::Value::from(value));
    };
    let requirement = &row.requirement;
    put("fr_id", &requirement.fr_id);
    put("fr_statement", &requirement.statement);
    if let Some(ac_id) = &requirement.ac_id {
        put("ac_id", ac_id);
    }
    if let Some(ac_text) = &requirement.ac_text {
        put("ac_text", ac_text);
    }
    if let Some(context) = &requirement.context {
        put("requirement_context", context);
    }
    if let Some(test) = &row.test {
        put("test_file", &test.path);
        put("test_fn_name", &test.fn_name);
        put("test_body", &test.body);
    }
    if let Some(code) = &row.code {
        put("symbol_name", &code.symbol);
        put("symbol_file", &code.path);
        put("symbol_body", &code.body);
    }
    serde_json::Value::Object(state)
}

/// A request from a state and a question set.
pub(crate) fn request(state: serde_json::Value, questions: Questions) -> SystemOneRequest {
    let entry: Entry = state.into();
    SystemOneRequest::new(entry, questions)
}

/// A `noul` answer thresholded at 0.5 into `yes`/`no`, with the confidence of
/// the chosen label (`max(p, 1 - p)`) and `p` as the ordinal.
pub(crate) fn noul_prediction(answers: &RawAnswers, key: &str) -> Option<Prediction> {
    let RawAnswer::Noul(p) = answers.get(key)? else {
        return None;
    };
    Some(Prediction {
        answer: if *p >= 0.5 { YES } else { NO }.to_owned(),
        confidence: Some(p.max(1.0 - p)),
        ordinal: Some(*p),
    })
}

/// A `noul` answer, negated: for a key whose defect is the question's `no`.
pub(crate) fn negated_noul_prediction(answers: &RawAnswers, key: &str) -> Option<Prediction> {
    let RawAnswer::Noul(p) = answers.get(key)? else {
        return None;
    };
    let q = 1.0 - p;
    Some(Prediction {
        answer: if q > 0.5 { YES } else { NO }.to_owned(),
        confidence: Some(q.max(1.0 - q)),
        ordinal: Some(q),
    })
}

/// A `choice` answer as asked.
pub(crate) fn choice_prediction(answers: &RawAnswers, key: &str) -> Option<Prediction> {
    let RawAnswer::Choice {
        label, confidence, ..
    } = answers.get(key)?
    else {
        return None;
    };
    Some(Prediction {
        answer: label.clone(),
        confidence: Some(*confidence),
        ordinal: None,
    })
}

/// A severity `score` answer: rounded to the nearest rubric level, with the
/// raw score as the ordinal so ordering quality sees between-level values.
pub(crate) fn severity_prediction(answers: &RawAnswers, key: &str) -> Option<Prediction> {
    let RawAnswer::Score {
        score, confidence, ..
    } = answers.get(key)?
    else {
        return None;
    };
    Some(Prediction {
        answer: nearest_rubric_label(*score).unwrap_or_else(|| format!("<out of range {score}>")),
        confidence: Some(*confidence),
        ordinal: Some(*score),
    })
}

/// The answers of the single whole-row ask, or empty.
pub(crate) fn whole_row(answered: &[Answered]) -> RawAnswers {
    answered
        .iter()
        .find(|answered| answered.unit.is_none())
        .map(|answered| answered.answers.clone())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Per-unit fan-out
// ---------------------------------------------------------------------------

/// The row's code split into units in its file's language ([`split_units`]),
/// or empty when the row carries no code.
pub(crate) fn code_units(row: &Row) -> Vec<Unit> {
    row.code
        .as_ref()
        .map(|code| split_units(&code.path, &code.body))
        .unwrap_or_default()
}

/// One ask per code unit. Each unit's state is the row's [`state`] with
/// `symbol_body` replaced by the unit's text and `code_unit` naming it; the
/// questions come from `questions(unit)`.
///
/// On an RTC row the state keeps the row's test fields (`test_body` and
/// its path and name), even for a code-only variant whose questions never
/// refer to them.
pub(crate) fn per_unit_asks(row: &Row, questions: fn(&Unit) -> Questions) -> Vec<Ask> {
    code_units(row)
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            let mut unit_state = state(row);
            if let serde_json::Value::Object(fields) = &mut unit_state {
                fields.insert("symbol_body".to_owned(), unit.text.clone().into());
                fields.insert("code_unit".to_owned(), unit.label.clone().into());
            }
            Ask {
                unit: Some(index),
                request: request(unit_state, questions(unit)),
            }
        })
        .collect()
}

/// Rolls a per-unit `noul` up: `yes` when any unit's probability is at least
/// 0.5. The ordinal is the highest unit probability; the confidence is that
/// probability for `yes`, and one minus it for `no`.
pub(crate) fn rollup_any_yes(answered: &[Answered], key: &str) -> Option<Prediction> {
    let highest = answered
        .iter()
        .filter(|answered| answered.unit.is_some())
        .filter_map(|answered| match answered.answers.get(key) {
            Some(RawAnswer::Noul(p)) => Some(*p),
            _ => None,
        })
        .reduce(f64::max)?;
    let yes = highest >= 0.5;
    Some(Prediction {
        answer: if yes { YES } else { NO }.to_owned(),
        confidence: Some(if yes { highest } else { 1.0 - highest }),
        ordinal: Some(highest),
    })
}

/// Rolls a per-unit `score` up to the highest unit's level on `levels`
/// (lowest first). The ordinal is that highest raw score.
pub(crate) fn rollup_max_level(
    answered: &[Answered],
    key: &str,
    levels: &[&str],
) -> Option<Prediction> {
    let (score, confidence) = answered
        .iter()
        .filter(|answered| answered.unit.is_some())
        .filter_map(|answered| match answered.answers.get(key) {
            Some(RawAnswer::Score {
                score, confidence, ..
            }) => Some((*score, *confidence)),
            _ => None,
        })
        .reduce(|best, next| if next.0 > best.0 { next } else { best })?;
    let distance =
        |index: usize| (score - f64::from(u32::try_from(index).unwrap_or(u32::MAX))).abs();
    let level = (0..levels.len())
        .min_by(|left, right| distance(*left).total_cmp(&distance(*right)))
        .and_then(|index| levels.get(index))
        .map(|label| (*label).to_owned())?;
    Some(Prediction {
        answer: level,
        confidence: Some(confidence),
        ordinal: Some(score),
    })
}

// ---------------------------------------------------------------------------
// The wording rule
// ---------------------------------------------------------------------------

/// An artifact besides the requirement that a question may refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Artifact {
    /// The row's test.
    Test,
    /// The row's code.
    Code,
}

impl Artifact {
    const fn name(self) -> &'static str {
        match self {
            Self::Test => "the test",
            Self::Code => "the code",
        }
    }

    const fn present_in(self, mode: Mode) -> bool {
        match self {
            Self::Test => mode.has_test(),
            Self::Code => mode.has_code(),
        }
    }

    /// Whether a state field name carries this artifact.
    fn owns_field(self, field: &str) -> bool {
        match self {
            Self::Test => field.starts_with("test_"),
            Self::Code => field.starts_with("symbol_") || field.starts_with("code_"),
        }
    }

    /// Question-text vocabulary that refers to this artifact. The second
    /// net, behind the declaration and the state check: it catches a
    /// paraphrase the declaration forgot, and is never applied to row
    /// content.
    fn vocabulary(self) -> &'static Regex {
        match self {
            Self::Test => &TEST_VOCABULARY,
            Self::Code => &CODE_VOCABULARY,
        }
    }
}

const ARTIFACTS: [Artifact; 2] = [Artifact::Test, Artifact::Code];

static TEST_VOCABULARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\btests?\b|\btesting\b|\bassertions?\b")
        .unwrap_or_else(|error| unreachable!("a literal regex compiles: {error}"))
});

static CODE_VOCABULARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bcode\b|\bsource code\b|\bimplementation under test\b|\b(function|method|symbol) bod(y|ies)\b|\bcovered (symbol|function|method)s?\b",
    )
    .unwrap_or_else(|error| unreachable!("a literal regex compiles: {error}"))
});

/// Every state field name in `value`, at any depth. Names only: the rule
/// never reads a row's own text.
fn field_names(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (name, inner) in fields {
                out.push(name.clone());
                field_names(inner, out);
            }
        }
        serde_json::Value::Array(items) => {
            for inner in items {
                field_names(inner, out);
            }
        }
        _ => {}
    }
}

/// Every way `variant`, run on `row`, refers to an artifact `row`'s mode does
/// not carry. Empty = clean. Three checks, structural first:
///
/// 1. **The declaration.** Every artifact in `variant.references` must be in
///    the row's mode.
/// 2. **The state.** The state sent carries no field of an absent artifact
///    (`test_*` without a test; `symbol_*`/`code_*` without code), by field
///    NAME: a requirement that happens to say "test" is not a reference.
/// 3. **The question text** (instructions and answer labels, never the
///    state): no vocabulary of an absent artifact, and no vocabulary of an
///    artifact the variant did not declare, so a declaration cannot lie.
pub(crate) fn wording_violations(variant: &Variant, row: &Row) -> Vec<String> {
    let label = variant.label();
    let mode = row.mode.as_str();
    let mut violations = Vec::new();
    for artifact in variant.references {
        if !artifact.present_in(row.mode) {
            violations.push(format!(
                "{label} in mode {mode}: declares it refers to {}, which the mode lacks",
                artifact.name()
            ));
        }
    }
    for ask in (variant.asks)(row) {
        let mut fields = Vec::new();
        field_names(
            &serde_json::to_value(&ask.request.state).unwrap_or_default(),
            &mut fields,
        );
        let questions: Vec<String> = ask
            .request
            .questions
            .values()
            .map(|question| serde_json::to_string(question).unwrap_or_default())
            .collect();
        for artifact in ARTIFACTS {
            let absent = !artifact.present_in(row.mode);
            if absent && let Some(field) = fields.iter().find(|f| artifact.owns_field(f)) {
                violations.push(format!(
                    "{label} in mode {mode}: state field {field:?} carries {}, which the mode lacks",
                    artifact.name()
                ));
            }
            let undeclared = !variant.references.contains(&artifact);
            if !(absent || undeclared) {
                continue;
            }
            for text in &questions {
                if let Some(found) = artifact.vocabulary().find(text) {
                    violations.push(format!(
                        "{label} in mode {mode}: question text refers to {} ({:?}){}",
                        artifact.name(),
                        found.as_str(),
                        if absent {
                            ", which the mode lacks"
                        } else {
                            ", which the variant does not declare"
                        }
                    ));
                }
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Baselines
// ---------------------------------------------------------------------------

/// `FullBatteryV1`'s questions, from `gap_semantic_support`, so the baseline
/// cannot drift from the request PLAT-979 and PLAT-1014 measured.
fn battery_asks(row: &Row) -> Vec<Ask> {
    vec![Ask {
        unit: None,
        request: request(state(row), battery_questions(BatteryShape::FullBatteryV1)),
    }]
}

/// Every battery key, read as asked.
fn battery_derive(_row: &Row, answered: &[Answered]) -> Predictions {
    let answers = whole_row(answered);
    let mut out = Predictions::new();
    for key in [
        "test_asserts_intent",
        "assertion_vacuous",
        "tests_only_its_own_mock",
        "code_implements_intent",
        "code_exceeds_requirement",
    ] {
        if let Some(prediction) = noul_prediction(&answers, key) {
            out.insert(key, prediction);
        }
    }
    if let Some(prediction) = choice_prediction(&answers, "divergence_kind") {
        out.insert("divergence_kind", prediction);
    }
    if let Some(prediction) = severity_prediction(&answers, "severity") {
        out.insert("severity", prediction);
    }
    out
}

const RTC_ONLY: &[Mode] = &[Mode::ReqTestCode];
const TEST_AND_CODE: &[Artifact] = &[Artifact::Test, Artifact::Code];

/// The whole `FullBatteryV1` battery, every key graded.
pub(crate) const B0: Variant = Variant {
    id: "B0",
    version: 1,
    summary: "FullBatteryV1 as asked (PLAT-979 wording), all seven keys",
    modes: RTC_ONLY,
    references: TEST_AND_CODE,
    grades: &[
        "test_asserts_intent",
        "assertion_vacuous",
        "tests_only_its_own_mock",
        "code_implements_intent",
        "code_exceeds_requirement",
        "divergence_kind",
        "severity",
    ],
    asks: battery_asks,
    derive: battery_derive,
};

/// Severity as asked: PLAT-1028's S0.
pub(crate) const S0: Variant = Variant {
    id: "S0",
    version: 1,
    summary: "severity as asked in FullBatteryV1, rounded to the nearest rubric level",
    modes: RTC_ONLY,
    references: TEST_AND_CODE,
    grades: &["severity"],
    asks: battery_asks,
    derive: battery_derive,
};

/// `code_exceeds_requirement` as asked: PLAT-1029's E0.
pub(crate) const E0: Variant = Variant {
    id: "E0",
    version: 1,
    summary: "code_exceeds_requirement as asked in FullBatteryV1",
    modes: RTC_ONLY,
    references: TEST_AND_CODE,
    grades: &["code_exceeds_requirement"],
    asks: battery_asks,
    derive: battery_derive,
};

/// `test_asserts_intent` as asked: PLAT-1030's T0, on full triples.
pub(crate) const T0: Variant = Variant {
    id: "T0",
    version: 1,
    summary: "test_asserts_intent as asked in FullBatteryV1 (RTC only)",
    modes: RTC_ONLY,
    references: TEST_AND_CODE,
    grades: &["test_asserts_intent"],
    asks: battery_asks,
    derive: battery_derive,
};

/// The criterion-strength lens's own question set, compiled in from the skill
/// asset exactly as `quoin_jev::QuestionSet` reads it.
const CRITERION_ASSET: &str =
    include_str!("../../../../../skills/spec-criterion-strength-analysis/assets/question-set.json");

/// The criterion under test: the row's AC, or the statement itself when the
/// row is about a requirement with no AC.
pub(crate) fn criterion(row: &Row) -> AcRow {
    let requirement = &row.requirement;
    AcRow {
        id: requirement
            .ac_id
            .clone()
            .unwrap_or_else(|| requirement.fr_id.clone()),
        text: requirement
            .ac_text
            .clone()
            .unwrap_or_else(|| requirement.statement.clone()),
    }
}

fn criterion_asks(row: &Row) -> Vec<Ask> {
    let Ok(set) = QuestionSet::parse(CRITERION_ASSET) else {
        return Vec::new();
    };
    let context = FrContext {
        fr_id: row.requirement.fr_id.clone(),
        statement: row.requirement.statement.clone(),
        description: row.requirement.context.clone(),
        behaviour: None,
        constraints: None,
        acceptance_criteria: vec![criterion(row)],
    };
    let bounded = context.bound(&ContextPolicy::default());
    vec![Ask {
        unit: None,
        request: quoin_jev::lens::build_request(&bounded, &set),
    }]
}

/// `criterion_sound` is `weakness_kind == sound`; `untestable` is the
/// negation of `falsifiable`; `no_measurable_threshold` is the negation of
/// `threshold_present`. The three checklist keys the lens has no question
/// for (`vague_term`, `compound`, `missing_trigger`) are not derived.
fn criterion_derive(row: &Row, answered: &[Answered]) -> Predictions {
    let answers = whole_row(answered);
    let ac = criterion(row).id;
    let mut out = Predictions::new();
    if let Some(RawAnswer::Choice {
        label,
        confidence,
        probabilities,
    }) = answers.get(&format!("{ac}::weakness_kind"))
    {
        let sound = label == "sound";
        let p_sound = probabilities.get("sound").copied();
        // The confidence of the graded answer: P(sound) for `yes`,
        // 1 - P(sound) for `no`. The choice's own confidence is its
        // confidence in the chosen weakness label, which for `no` is a
        // different event from "not sound".
        let answer_confidence = match (sound, p_sound) {
            (true, Some(p)) => Some(p),
            (false, Some(p)) => Some(1.0 - p),
            (true, None) => Some(*confidence),
            (false, None) => None,
        };
        out.insert(
            "criterion_sound",
            Prediction {
                answer: if sound { YES } else { NO }.to_owned(),
                confidence: answer_confidence,
                ordinal: p_sound,
            },
        );
    }
    if let Some(prediction) = negated_noul_prediction(&answers, &format!("{ac}::falsifiable")) {
        out.insert("untestable", prediction);
    }
    if let Some(prediction) = negated_noul_prediction(&answers, &format!("{ac}::threshold_present"))
    {
        out.insert("no_measurable_threshold", prediction);
    }
    out
}

/// The criterion-strength lens as shipped (PLAT-837/PLAT-917): PLAT-1031's
/// holistic baseline. It reads the requirement only, so it runs in any mode.
pub(crate) const C0: Variant = Variant {
    id: "C0",
    version: 1,
    summary: "criterion-strength lens as shipped; criterion_sound from weakness_kind",
    modes: &Mode::ALL,
    references: &[],
    grades: &["criterion_sound", "untestable", "no_measurable_threshold"],
    asks: criterion_asks,
    derive: criterion_derive,
};

// ---------------------------------------------------------------------------
// The runner
// ---------------------------------------------------------------------------

/// One variant's graded answers on one row.
#[derive(Debug, Clone)]
pub(crate) struct RowResult {
    /// The row.
    pub(crate) row_id: String,
    /// The variant's label.
    pub(crate) variant: String,
    /// Graded answers, filtered to the variant's `grades`.
    pub(crate) predictions: Predictions,
    /// The raw answers `derive` read, kept so a variant's own diagnostics
    /// (PLAT-1029's parked units and trace-suspect rows) can be reported
    /// without asking again.
    pub(crate) answered: Vec<Answered>,
}

/// Everything a run produced.
#[derive(Debug, Clone, Default)]
pub(crate) struct RunOutput {
    /// One entry per (variant, applicable row).
    pub(crate) results: Vec<RowResult>,
    /// Distinct requests actually sent.
    pub(crate) requests_sent: usize,
    /// Requests answered from this run's own cache instead.
    pub(crate) requests_reused: usize,
    /// Answering model, by count of requests sent.
    pub(crate) models: BTreeMap<String, usize>,
}

/// How far a `score`'s probabilities may sum from 1, per level, before the
/// answer is refused. The service rounds each mass to two decimals, so each
/// level can be off by 0.005 and four levels by 0.02: a live answer summing
/// to 0.99 (0.37, 0.02, 0.04, 0.56) was refused at a flat 0.01.
pub(crate) const SCORE_MASS_TOLERANCE_PER_LEVEL: f64 = 0.005;

/// Refuses a `score` answer whose level probabilities are not a
/// distribution over the question's own levels:
///
/// - every key must be a whole level, `"0"` up to one below the number of
///   levels (a decimal spelling such as `"2.0"` names the same level);
/// - no level may be spelled twice (`"2"` and `"2.0"` together);
/// - every mass must be finite and non-negative;
/// - the masses must sum to 1 within [`SCORE_MASS_TOLERANCE_PER_LEVEL`] per
///   level (the service's two-decimal rounding);
/// - the map must not be empty.
///
/// A variant reading the distribution (PLAT-1028's S2 and S2M) would
/// otherwise grade a malformed answer silently (PR #620 review, findings 9
/// and F7).
///
/// # Errors
/// Naming the question key and what is wrong with its probabilities.
pub(crate) fn check_score_levels(
    questions: &Questions,
    answers: &RawAnswers,
) -> Result<(), String> {
    for (key, question) in questions {
        let Question::Score { criteria, .. } = question else {
            continue;
        };
        let Some(RawAnswer::Score { probabilities, .. }) = answers.get(key) else {
            continue;
        };
        let levels = criteria.len();
        let spelled_keys: Vec<&String> = probabilities.keys().collect();
        let refuse = |why: String| {
            Err(format!(
                "score `{key}` has {levels} levels, but its probabilities {why} (keys \
                 {spelled_keys:?}, masses {:?}); expected one mass per level \"0\"..\"{}\", \
                 summing to 1",
                probabilities.values().collect::<Vec<_>>(),
                levels.saturating_sub(1)
            ))
        };
        if probabilities.is_empty() {
            return refuse("are empty".to_owned());
        }
        let whole_level = |spelled: &str| -> Option<usize> {
            let value = spelled.trim().parse::<f64>().ok()?;
            (0..levels).find(|level| {
                (f64::from(u32::try_from(*level).unwrap_or(u32::MAX)) - value).abs() < 1e-9
            })
        };
        let mut seen = std::collections::BTreeSet::new();
        let mut total = 0.0;
        for (spelled, p) in probabilities {
            let Some(level) = whole_level(spelled) else {
                return refuse(format!("key {spelled:?} is not a level"));
            };
            if !seen.insert(level) {
                return refuse(format!("spell level {level} more than once"));
            }
            if !p.is_finite() || *p < 0.0 {
                return refuse(format!("give level {level} the mass {p}"));
            }
            total += p;
        }
        let levels_f64 = f64::from(u32::try_from(levels).unwrap_or(u32::MAX));
        if (total - 1.0).abs() > SCORE_MASS_TOLERANCE_PER_LEVEL.mul_add(levels_f64, 1e-9) {
            return refuse(format!("sum to {total}"));
        }
    }
    Ok(())
}

/// Runs every variant over every row it applies to, in row then variant
/// order. Each distinct request is sent once per run: variants sharing a
/// request shape share its answer. A transport failure aborts the run rather
/// than dropping the row from the denominator.
///
/// # Errors
/// The first failed request, naming the row and variant.
pub(crate) async fn run(
    client: &Client,
    rows: &[Row],
    variants: &[&Variant],
) -> Result<RunOutput, String> {
    let mut ids = std::collections::BTreeSet::new();
    if let Some(duplicate) = rows.iter().find(|row| !ids.insert(row.id.as_str())) {
        return Err(format!(
            "row id {} appears more than once; results are keyed by row id",
            duplicate.id
        ));
    }
    let mut output = RunOutput::default();
    let mut cache: HashMap<String, RawAnswers> = HashMap::new();
    for row in rows {
        for variant in variants.iter().filter(|variant| variant.applies_to(row)) {
            let mut answered = Vec::new();
            for ask in (variant.asks)(row) {
                let key = serde_json::to_string(&ask.request)
                    .map_err(|error| format!("{}: {error}", row.id))?;
                let answers = if let Some(answers) = cache.get(&key) {
                    output.requests_reused += 1;
                    answers.clone()
                } else {
                    let questions = ask.request.questions.clone();
                    let response = client
                        .system_one(ask.request)
                        .await
                        .map_err(|error| format!("{} / {}: {error}", row.id, variant.label()))?;
                    output.requests_sent += 1;
                    *output.models.entry(response.model.clone()).or_default() += 1;
                    let answers = raw_answers(&response);
                    // A malformed answer aborts the run, but it was paid
                    // for: the error carries the raw response, and a
                    // recording cassette has already written it.
                    check_score_levels(&questions, &answers).map_err(|error| {
                        let raw = serde_json::to_string(&response)
                            .unwrap_or_else(|error| format!("<unserializable: {error}>"));
                        format!(
                            "{} / {}: {error}; raw response: {raw}",
                            row.id,
                            variant.label()
                        )
                    })?;
                    cache.insert(key, answers.clone());
                    answers
                };
                answered.push(Answered {
                    unit: ask.unit,
                    answers,
                });
            }
            let predictions = (variant.derive)(row, &answered)
                .into_iter()
                .filter(|(key, _)| variant.grades.contains(key))
                .collect();
            output.results.push(RowResult {
                row_id: row.id.clone(),
                variant: variant.label(),
                predictions,
                answered,
            });
        }
    }
    Ok(output)
}
