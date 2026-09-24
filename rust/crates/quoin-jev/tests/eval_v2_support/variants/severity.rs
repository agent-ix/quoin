// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Severity variants S1, S2, S2M and S3 (PLAT-1028, parent PLAT-1024). The
//! bars are pre-registered in
//! `spec/assurance/MP-240-jev-severity-variants.md`.
//!
//! PLAT-1014 (MP-234) found severity "as asked" (`S0`) collapsing to the
//! middle: 1 of 20 rows labelled `high` came back `high`, and 21 of 29 came
//! back `low`/`medium`. Each variant here attacks that from a different side:
//!
//! | id | idea | modes |
//! | --- | --- | --- |
//! | `S1` | never ask severity: ask narrow fact `noul`s, threshold each at τ = 0.5, and compute it with [`rule`], a transcription of the step-5 rubric | `S1` RTC, `S1-RT`, `S1-RC` |
//! | `S2` | ask severity as a `score` whose levels are concrete situations, with worked examples per level (jev-code's shape) | `S2` RTC, `S2-RT`, `S2-RC` |
//! | `S2M` | `S2`'s identical request, graded on the probability mass on `high` instead of the expected level | `S2M` RTC, `S2M-RT`, `S2M-RC` |
//! | `S3` | ask severity as a `score` whose levels are review actions, mapped to the rubric in code | RT, RC, RTC |
//!
//! # Why one variant per mode
//!
//! A [`Variant`] declares the artifacts its questions refer to once, and the
//! wording rule requires every declared artifact in every mode it claims. S1's
//! facts and S2's examples name the test and the code, so each mode gets its
//! own registry entry with its own truthful declaration. The `asks` and
//! `derive` functions are shared and read the row's mode. `S3` names neither
//! artifact, so it is one entry.
//!
//! # S1 in `RT` and `RC`
//!
//! [`Fact::asked_in`] asks only the facts whose artifact the mode carries: in
//! `RT` no code fact is asked, in `RC` no test fact is. A fact that was not
//! asked is absent from the map [`rule`] reads, and every branch that tests
//! it is skipped (neither fired nor counted as clean). So an `RT` row's
//! severity is the rubric applied to axes (a) and (b) only, and an `RC` row's
//! to axis (c) only. The trace check is asked in
//! every mode, worded for the artifacts present.
//!
//! # Examples
//!
//! Every worked example in [`EXAMPLES`] is invented for this module (coupons,
//! withdrawals, uploads, schedulers). None is drawn from a corpus row or from
//! this repository's own requirements, so no prompt carries a row it will
//! later be graded on.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use typesafe_sdk_questions::{Questions, noul, questions, score};

use crate::eval_v2_support::corpus::Row;
use crate::eval_v2_support::keys::Mode;
use crate::eval_v2_support::variant::{
    Answered, Artifact, Ask, Prediction, Predictions, RawAnswer, RawAnswers, Variant, request,
    state,
};

/// The key every variant here is graded on.
pub(crate) const SEVERITY: &str = "severity";

// ---------------------------------------------------------------------------
// Levels
// ---------------------------------------------------------------------------

/// One rung of the severity rubric, lowest first. Mirrors
/// `gap_semantic_support::SEVERITY_RUBRIC`; a test holds the two together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    /// No finding.
    None,
    /// A trivial mismatch.
    Low,
    /// Partial validation or minor drift.
    Medium,
    /// False confidence or contradicted behaviour.
    High,
}

impl Level {
    /// Every level, lowest first.
    pub(crate) const ALL: [Self; 4] = [Self::None, Self::Low, Self::Medium, Self::High];

    /// The rubric label.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// The position on the rubric, as a `score` indexes it.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::None => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
        }
    }

    /// [`Level::index`] as a float, for expected values.
    const fn value(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Low => 1.0,
            Self::Medium => 2.0,
            Self::High => 3.0,
        }
    }

    /// The level nearest a continuous score, or `None` outside `[-0.5, 3.5)`.
    /// A half rounds up (`f64::round`), exactly as S0's
    /// `nearest_rubric_label` does, so S0 and these variants bucket the same
    /// score identically.
    pub(crate) fn nearest(score: f64) -> Option<Self> {
        if !(-0.5..3.5).contains(&score) {
            return None;
        }
        let rounded = score.round();
        Self::ALL
            .into_iter()
            .find(|level| (level.value() - rounded).abs() < 1e-9)
    }
}

/// A probability per level, indexed by [`Level::index`].
pub(crate) type Distribution = [f64; 4];

/// The expected level under `distribution`, normalised by its total mass.
/// `None` when the distribution carries no mass.
pub(crate) fn expected_level(distribution: &Distribution) -> Option<f64> {
    let total: f64 = distribution.iter().sum();
    (total > 0.0).then(|| {
        Level::ALL
            .into_iter()
            .zip(distribution)
            .map(|(level, p)| level.value() * p)
            .sum::<f64>()
            / total
    })
}

/// The mass on `level`.
pub(crate) fn mass(distribution: &Distribution, level: Level) -> f64 {
    distribution.get(level.index()).copied().unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// S1: facts and the rule
// ---------------------------------------------------------------------------

/// One fact S1 asks, as a `noul`. Step 5 has two tiers, `high` and
/// `medium`, and each tier of each axis is asked as its own fact, so the
/// model never has to place the line between them.
///
/// There is no "code exceeds the requirement" fact. v1 of this plan had one,
/// reusing `FullBatteryV1`'s wording. MP-234 measured that question as a coin
/// flip (every probability between 0.50 and 0.73), so at the 0.5 threshold it
/// made clean `RC`/`RTC` rows `medium`, which is the central-tendency collapse
/// PLAT-1028 exists to fix. Step 5 and the corpus's `rules.severity` have no
/// exceeds clause either. PR #620 review, before any live call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Fact {
    /// The requirement is about what the test checks / the code does.
    /// Defect when `no`.
    TraceCorrect,
    /// The test would still pass with the stated behaviour broken: it does
    /// not validate intent. High tier of axis (a). Defect when `yes`.
    TestPassesWhenBroken,
    /// The test would still pass against a stub: it does not exercise the
    /// code. Axis (b), high only. Defect when `yes`.
    AssertionVacuous,
    /// The test checks only some of the requirement's clauses. Medium tier
    /// of axis (a). Defect when `yes`.
    TestChecksSomeClauses,
    /// The code contradicts the requirement. High tier of axis (c). Defect
    /// when `yes`.
    CodeContradicts,
    /// The code misses or mishandles a stated case. Medium tier of axis (c).
    /// Defect when `yes`.
    CodeMissesStatedCase,
}

/// Every fact, in the order they are asked.
const FACTS: [Fact; 6] = [
    Fact::TraceCorrect,
    Fact::TestPassesWhenBroken,
    Fact::AssertionVacuous,
    Fact::TestChecksSomeClauses,
    Fact::CodeContradicts,
    Fact::CodeMissesStatedCase,
];

impl Fact {
    /// The wire key. Where a fact has a corpus-v2 key of the same meaning
    /// (`trace_correct`, `assertion_vacuous`), this is that key.
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::TraceCorrect => "trace_correct",
            Self::TestPassesWhenBroken => "test_passes_when_broken",
            Self::AssertionVacuous => "assertion_vacuous",
            Self::TestChecksSomeClauses => "test_checks_only_some_clauses",
            Self::CodeContradicts => "code_contradicts_requirement",
            Self::CodeMissesStatedCase => "code_misses_stated_case",
        }
    }

    /// The answer that is the defect.
    pub(crate) const fn defect_answer(self) -> bool {
        !matches!(self, Self::TraceCorrect)
    }

    /// The level [`rule`] assigns when this fact alone is the defect.
    pub(crate) const fn tier(self) -> Level {
        match self {
            Self::TraceCorrect
            | Self::TestPassesWhenBroken
            | Self::AssertionVacuous
            | Self::CodeContradicts => Level::High,
            Self::TestChecksSomeClauses | Self::CodeMissesStatedCase => Level::Medium,
        }
    }

    /// The artifact the fact is about, besides the requirement. `None` for
    /// the trace check, which is worded for whatever the mode carries.
    const fn needs(self) -> Option<Artifact> {
        match self {
            Self::TraceCorrect => None,
            Self::TestPassesWhenBroken | Self::AssertionVacuous | Self::TestChecksSomeClauses => {
                Some(Artifact::Test)
            }
            Self::CodeContradicts | Self::CodeMissesStatedCase => Some(Artifact::Code),
        }
    }

    /// The facts asked in `mode`: the trace check, plus each fact whose
    /// artifact the mode carries. Empty in `R`, where there is nothing to
    /// trace.
    pub(crate) fn asked_in(mode: Mode) -> Vec<Self> {
        if !(mode.has_test() || mode.has_code()) {
            return Vec::new();
        }
        FACTS
            .into_iter()
            .filter(|fact| match fact.needs() {
                None => true,
                Some(Artifact::Test) => mode.has_test(),
                Some(Artifact::Code) => mode.has_code(),
            })
            .collect()
    }

    /// The question text in `mode`. The test facts say "the implementation",
    /// which the wording rule treats as abstract, so they read the same in
    /// `RT` and `RTC`.
    fn question(self, mode: Mode) -> String {
        match self {
            Self::TraceCorrect => {
                let subject = match (mode.has_test(), mode.has_code()) {
                    (true, true) => "this test checks and this code performs",
                    (true, false) => "this test checks",
                    _ => "this code performs",
                };
                let object = match (mode.has_test(), mode.has_code()) {
                    (true, true) => "the test and the code are",
                    (true, false) => "the test is",
                    _ => "the code is",
                };
                format!(
                    "Is the requirement about the behaviour {subject}? Answer no if the \
                     requirement describes a different feature, subsystem or process step \
                     from the one {object} about."
                )
            }
            Self::TestPassesWhenBroken => "Would this test still pass if the behaviour the \
                 requirement states were broken, while the implementation still compiled and \
                 still returned a well-formed, non-default result? Judge against the \
                 requirement's own stated behaviour, not against other things the \
                 implementation does."
                .to_owned(),
            Self::AssertionVacuous => "Would this test still pass if the implementation were \
                 replaced with a stub returning a default?"
                .to_owned(),
            Self::TestChecksSomeClauses => "Does this test check only some of the clauses, \
                 cases, boundaries or error behaviours the requirement states, leaving at least \
                 one stated one unchecked? Answer no if it checks all of them, and no if the \
                 requirement states a single behaviour and the test checks it."
                .to_owned(),
            Self::CodeContradicts => "Does the code contradict what the requirement states: \
                 does it do the opposite, or lack the stated behaviour altogether?"
                .to_owned(),
            Self::CodeMissesStatedCase => "Does the code do the main behaviour the requirement \
                 states but miss or mishandle a case, boundary or error behaviour the \
                 requirement names? Answer no if it handles every stated case, and no if the \
                 requirement names none."
                .to_owned(),
        }
    }
}

/// Which rubric branch fired. Each branch quotes the rubric line it
/// transcribes (see [`Branch::rubric`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Branch {
    /// The requirement is not about the test or code shown.
    TraceMismatch,
    /// Axis (a), high tier: the test does not validate intent.
    TestMissesIntent,
    /// Axis (b): the test does not exercise the code.
    TestHollow,
    /// Axis (c), high tier: the code contradicts the requirement.
    CodeContradicts,
    /// Axis (a), medium tier: partial validation.
    TestPartial,
    /// Axis (c), medium tier: a stated case missed or mishandled.
    CodeMissesCase,
    /// No axis fails.
    Clean,
}

impl Branch {
    /// The level the branch assigns.
    pub(crate) const fn level(self) -> Level {
        match self {
            Self::TraceMismatch
            | Self::TestMissesIntent
            | Self::TestHollow
            | Self::CodeContradicts => Level::High,
            Self::TestPartial | Self::CodeMissesCase => Level::Medium,
            Self::Clean => Level::None,
        }
    }

    /// The rubric text the branch transcribes, quoted from
    /// `skills/gap-analysis/references/step-5-semantic-review.md`. Quoted
    /// rather than cited by line number, because line numbers move.
    pub(crate) const fn rubric(self) -> &'static str {
        match self {
            Self::TraceMismatch => {
                "step 5 (a): \"A test tagged `FR-007-AC-1` that asserts something unrelated to \
                 AC-1 fails here.\" -> \"`high` — test does not validate intent\"; with no test, \
                 axis (c) fails outright -> \"or code contradicts the requirement\""
            }
            Self::TestMissesIntent => "step 5: \"`high` — test does not validate intent\"",
            Self::TestHollow => {
                "step 5: \"`high` — ... **or** does not exercise code (false confidence)\""
            }
            Self::CodeContradicts => "step 5: \"`high` — ... or code contradicts the requirement\"",
            Self::TestPartial => "step 5: \"`medium` — partial validation\"",
            Self::CodeMissesCase => {
                "step 5: \"`medium` — ... meaningful edge cases unchecked, minor drift\""
            }
            Self::Clean => "step 5: no failing axis, so no finding",
        }
    }
}

/// S1's rule: the step-5 rubric over fact answers. A fact absent from
/// `facts` was not asked in the row's mode, and every branch reading it is
/// skipped. High branches are tried before medium ones, so the worst failing
/// axis sets the level, as "each failing axis → a finding" and the report's
/// worst-finding verdict imply.
///
/// Step 5 names only `high` and `medium`; `low` has no condition there, so
/// this rule never returns it. That is a property of the rubric, recorded in
/// MP-240, not a choice made here.
pub(crate) fn rule(facts: &BTreeMap<Fact, bool>) -> Branch {
    let defect = |fact: Fact| facts.get(&fact) == Some(&fact.defect_answer());
    if defect(Fact::TraceCorrect) {
        Branch::TraceMismatch
    } else if defect(Fact::TestPassesWhenBroken) {
        Branch::TestMissesIntent
    } else if defect(Fact::AssertionVacuous) {
        Branch::TestHollow
    } else if defect(Fact::CodeContradicts) {
        Branch::CodeContradicts
    } else if defect(Fact::TestChecksSomeClauses) {
        Branch::TestPartial
    } else if defect(Fact::CodeMissesStatedCase) {
        Branch::CodeMissesCase
    } else {
        Branch::Clean
    }
}

/// The distribution of [`rule`]'s level when each fact is `yes` with its
/// `noul` probability, independently: every combination of answers is
/// enumerated (at most 2^6) and weighted by its probability.
///
/// Independence is an assumption the facts do not satisfy (a trace mismatch
/// makes every other defect likely). S1 reads nothing from it: not its
/// level, its ordinal or its confidence. `high` collects every combination
/// with any high-tier defect, so its mass is biased upward by the number of
/// high-tier facts asked (MP-240 "Known properties"); it is kept only to
/// state that bias in tests.
pub(crate) fn level_distribution(facts: &[(Fact, f64)]) -> Distribution {
    let mut distribution = [0.0; 4];
    for mask in 0..(1_usize << facts.len()) {
        let mut weight = 1.0;
        let mut values = BTreeMap::new();
        for (bit, (fact, p_yes)) in facts.iter().enumerate() {
            let yes = (mask >> bit) & 1 == 1;
            weight *= if yes { *p_yes } else { 1.0 - p_yes };
            values.insert(*fact, yes);
        }
        if let Some(slot) = distribution.get_mut(rule(&values).level().index()) {
            *slot += weight;
        }
    }
    distribution
}

/// Noul's decision threshold τ: a fact counts as a defect when the
/// probability of its defect answer is at least this.
pub(crate) const FACT_THRESHOLD: f64 = 0.5;

/// The probability that `fact` holds its defect answer, from its `noul`
/// probability of `yes`.
fn defect_probability(fact: Fact, p_yes: f64) -> f64 {
    if fact.defect_answer() {
        p_yes
    } else {
        1.0 - p_yes
    }
}

/// S1's level: each fact thresholded at [`FACT_THRESHOLD`] on its own, then
/// [`rule`] over the thresholded answers, which takes the most severe tier
/// with any fact at or above τ. `none` when no fact reaches τ. A fact
/// exactly at τ counts as the defect.
///
/// This replaced the most probable level under [`level_distribution`]
/// before any live call (PR #620 re-review, finding 1): `high` is the union
/// of up to four independent high-tier facts, so its mass grows with the
/// number of facts asked, and a row where trace correct is 0.9 and every
/// defect fact is 0.15 came out `high` at 0.447 while every fact read "no".
pub(crate) fn thresholded_branch(facts: &[(Fact, f64)]) -> Branch {
    let answers: BTreeMap<Fact, bool> = facts
        .iter()
        .map(|(fact, p_yes)| {
            let defect = defect_probability(*fact, *p_yes) >= FACT_THRESHOLD;
            (
                *fact,
                if defect {
                    fact.defect_answer()
                } else {
                    !fact.defect_answer()
                },
            )
        })
        .collect();
    rule(&answers)
}

/// The answers of the single whole-row ask, or empty.
fn whole_row(answered: &[Answered]) -> RawAnswers {
    answered
        .iter()
        .find(|answered| answered.unit.is_none())
        .map(|answered| answered.answers.clone())
        .unwrap_or_default()
}

/// S1's questions for `mode`: one `noul` per fact [`Fact::asked_in`] names.
pub(crate) fn s1_questions(mode: Mode) -> Questions {
    questions(
        Fact::asked_in(mode)
            .into_iter()
            .map(|fact| (fact.key(), noul(fact.question(mode)))),
    )
}

fn s1_asks(row: &Row) -> Vec<Ask> {
    if Fact::asked_in(row.mode).is_empty() {
        return Vec::new();
    }
    vec![Ask {
        unit: None,
        request: request(state(row), s1_questions(row.mode)),
    }]
}

/// S1's confidence in `level`, read off the thresholded facts rather than
/// [`level_distribution`], so it agrees with the answer: for a finding, the
/// largest defect probability among the facts of that tier (the fact that
/// put the row there); for `none`, one minus the largest defect probability
/// of any fact (the fact nearest to making it a finding).
pub(crate) fn s1_confidence(facts: &[(Fact, f64)], level: Level) -> f64 {
    let defect = facts
        .iter()
        .filter(|(fact, _)| level == Level::None || fact.tier() == level)
        .map(|(fact, p_yes)| defect_probability(*fact, *p_yes))
        .fold(0.0_f64, f64::max);
    if level == Level::None {
        1.0 - defect
    } else {
        defect
    }
}

/// S1's level is [`thresholded_branch`]'s: each fact thresholded at τ on
/// its own, the most severe tier with a fact at or above τ winning. The
/// ordinal every bar reads (Bar C's ordering, Bar D's contrast) is that
/// level's index, never the expectation under [`level_distribution`]: the
/// expectation carries the union bias MP-240 records, and a row S1 answers
/// `none` could otherwise cross τ=2.0 in Bar D (PR #620 delta review, B2).
/// The confidence is [`s1_confidence`]. Unanswered when any asked fact is
/// missing, rather than guessing its branch.
fn s1_derive(row: &Row, answered: &[Answered]) -> Predictions {
    let answers = whole_row(answered);
    let mut probabilities = Vec::new();
    for fact in Fact::asked_in(row.mode) {
        let Some(RawAnswer::Noul(p)) = answers.get(fact.key()) else {
            return Predictions::new();
        };
        probabilities.push((fact, *p));
    }
    if probabilities.is_empty() {
        return Predictions::new();
    }
    let level = thresholded_branch(&probabilities).level();
    Predictions::from([(
        SEVERITY,
        Prediction {
            answer: level.label().to_owned(),
            confidence: Some(s1_confidence(&probabilities, level)),
            ordinal: Some(level.value()),
        },
    )])
}

const RTC: &[Mode] = &[Mode::ReqTestCode];
const RT: &[Mode] = &[Mode::ReqTest];
const RC: &[Mode] = &[Mode::ReqCode];
const TEST_AND_CODE: &[Artifact] = &[Artifact::Test, Artifact::Code];
const TEST: &[Artifact] = &[Artifact::Test];
const CODE: &[Artifact] = &[Artifact::Code];

/// S1 on full triples.
pub(crate) const S1: Variant = Variant {
    id: "S1",
    version: 1,
    summary: "severity derived in code from six fact nouls by the step-5 rubric (RTC)",
    modes: RTC,
    references: TEST_AND_CODE,
    grades: &[SEVERITY],
    asks: s1_asks,
    derive: s1_derive,
};

/// S1 on requirement-plus-test rows: axes (a) and (b) only.
pub(crate) const S1_RT: Variant = Variant {
    id: "S1-RT",
    summary: "severity derived in code from four fact nouls by the step-5 rubric (RT)",
    modes: RT,
    references: TEST,
    ..S1
};

/// S1 on requirement-plus-code rows: axis (c) only.
pub(crate) const S1_RC: Variant = Variant {
    id: "S1-RC",
    summary: "severity derived in code from three fact nouls by the step-5 rubric (RC)",
    modes: RC,
    references: CODE,
    ..S1
};

// ---------------------------------------------------------------------------
// S2 / S2M: levels as situations, with worked examples
// ---------------------------------------------------------------------------

/// One worked example, for the mode whose artifacts it describes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Example {
    /// The mode it is written for: it names exactly that mode's artifacts.
    pub(crate) mode: Mode,
    /// Its level.
    pub(crate) level: Level,
    /// The situation and why it is that level.
    pub(crate) text: &'static str,
}

const fn example(mode: Mode, level: Level, text: &'static str) -> Example {
    Example { mode, level, text }
}

/// Every worked example, invented for PLAT-1028 (see the module doc). Three
/// per mode for `high`, the level S0 almost never gave; two per mode for the
/// others.
pub(crate) const EXAMPLES: &[Example] = &[
    // --- requirement + test ---
    example(
        Mode::ReqTest,
        Level::High,
        "Requirement: an expired coupon shall be rejected at checkout. Test: named for that \
         criterion, it applies a valid coupon and asserts the discount. High: it never tries an \
         expired coupon, so it checks something other than what the requirement states.",
    ),
    example(
        Mode::ReqTest,
        Level::High,
        "Requirement: passwords shall be stored only as salted hashes. Test: calls save_user and \
         asserts only that it returned Ok. High: it would still pass if the implementation \
         stored nothing at all.",
    ),
    example(
        Mode::ReqTest,
        Level::High,
        "Requirement: the scheduler shall retry a failed job at most three times. Test: checks \
         that dates are formatted as YYYY-MM-DD. High: the requirement is not about what this \
         test checks.",
    ),
    example(
        Mode::ReqTest,
        Level::Medium,
        "Requirement: uploads shall accept PNG and JPEG images and reject everything else. Test: \
         asserts that a PNG is accepted and a PDF rejected, and never tries a JPEG. Medium: the \
         main behaviour is checked, one stated case is not.",
    ),
    example(
        Mode::ReqTest,
        Level::Medium,
        "Requirement: a username shall be 3 to 20 characters long. Test: asserts that a \
         2-character name is rejected and a 10-character name accepted, and never tries 21. \
         Medium: one stated boundary is unchecked.",
    ),
    example(
        Mode::ReqTest,
        Level::Low,
        "Requirement: an empty cart shall show the message 'Your cart is empty'. Test: asserts \
         exactly that message, but its name says 'basket'. Low: a naming slip that changes \
         nothing checked.",
    ),
    example(
        Mode::ReqTest,
        Level::Low,
        "Requirement: invoice totals shall be rounded to two decimal places. Test: checks the \
         rounding fully, but its comment cites an outdated ticket number. Low: a cosmetic \
         inaccuracy only.",
    ),
    example(
        Mode::ReqTest,
        Level::None,
        "Requirement: division by zero shall return the error DIV_ZERO. Test: divides by zero and \
         asserts DIV_ZERO, and asserts that 6 / 3 returns 2. None: it checks exactly what is \
         stated.",
    ),
    example(
        Mode::ReqTest,
        Level::None,
        "Requirement: the list shall be sorted by name, ascending. Test: inserts names out of \
         order and asserts the exact sorted result. None.",
    ),
    // --- requirement + code ---
    example(
        Mode::ReqCode,
        Level::High,
        "Requirement: a withdrawal larger than the balance shall be refused. Code: subtracts the \
         amount and lets the balance go negative. High: it does the opposite of the requirement.",
    ),
    example(
        Mode::ReqCode,
        Level::High,
        "Requirement: session tokens shall expire after 30 minutes. Code: issues tokens with no \
         expiry at all. High: the stated behaviour is absent.",
    ),
    example(
        Mode::ReqCode,
        Level::High,
        "Requirement: the scheduler shall retry a failed job at most three times. Code: a \
         date-formatting helper. High: the requirement is not about this code.",
    ),
    example(
        Mode::ReqCode,
        Level::Medium,
        "Requirement: search shall ignore letter case and leading or trailing spaces. Code: \
         lowercases the query but never trims it. Medium: the main behaviour is right, one \
         stated case is missed.",
    ),
    example(
        Mode::ReqCode,
        Level::Medium,
        "Requirement: a CSV export shall quote any field that contains a comma or a line break. \
         Code: quotes fields containing a comma but not fields containing a line break. Medium: \
         the main behaviour is right, one stated case is mishandled.",
    ),
    example(
        Mode::ReqCode,
        Level::Low,
        "Requirement: a failed login shall log a warning. Code: logs at warning level, with \
         message wording that differs from the requirement's example. Low: cosmetic only.",
    ),
    example(
        Mode::ReqCode,
        Level::Low,
        "Requirement: order ids shall be unique. Code: generates a UUID per order, in a variable \
         named order_no rather than order_id. Low: naming only.",
    ),
    example(
        Mode::ReqCode,
        Level::None,
        "Requirement: division by zero shall return the error DIV_ZERO. Code: returns DIV_ZERO \
         when the divisor is zero and divides otherwise. None.",
    ),
    example(
        Mode::ReqCode,
        Level::None,
        "Requirement: the list shall be sorted by name, ascending. Code: sorts by name with an \
         ascending comparator. None.",
    ),
    // --- requirement + test + code ---
    example(
        Mode::ReqTestCode,
        Level::High,
        "Requirement: an expired coupon shall be rejected at checkout. Test: applies a valid \
         coupon and asserts the discount. Code: rejects expired coupons correctly. High: the \
         test gives false confidence, since it never checks the stated behaviour, even though \
         the code is right.",
    ),
    example(
        Mode::ReqTestCode,
        Level::High,
        "Requirement: a withdrawal larger than the balance shall be refused. Test: asserts a \
         refusal that its own mock was configured to return. Code: lets the balance go negative. \
         High: the code does the opposite, and the test cannot notice.",
    ),
    example(
        Mode::ReqTestCode,
        Level::High,
        "Requirement: the scheduler shall retry a failed job at most three times. Test and code: \
         both about formatting dates. High: the requirement is not about either of them.",
    ),
    example(
        Mode::ReqTestCode,
        Level::Medium,
        "Requirement: uploads shall accept PNG and JPEG images and reject everything else. Test: \
         checks a PNG and a PDF only. Code: handles all three correctly. Medium: partial \
         validation.",
    ),
    example(
        Mode::ReqTestCode,
        Level::Medium,
        "Requirement: search shall ignore letter case and leading or trailing spaces. Test: \
         checks letter case only. Code: lowercases but never trims. Medium: one stated case is \
         missed by both, the main behaviour is right.",
    ),
    example(
        Mode::ReqTestCode,
        Level::Low,
        "Requirement: an empty cart shall show 'Your cart is empty'. Test: asserts exactly that \
         message but is named for a 'basket'. Code: shows the message. Low: naming only.",
    ),
    example(
        Mode::ReqTestCode,
        Level::Low,
        "Requirement: a failed login shall log a warning. Test: asserts a warning was logged. \
         Code: logs one, worded differently from the requirement's example. Low: cosmetic only.",
    ),
    example(
        Mode::ReqTestCode,
        Level::None,
        "Requirement: division by zero shall return DIV_ZERO. Test: asserts DIV_ZERO for a zero \
         divisor and 2 for 6 / 3. Code: checks the divisor and divides otherwise. None.",
    ),
    example(
        Mode::ReqTestCode,
        Level::None,
        "Requirement: the list shall be sorted by name, ascending. Test: asserts the exact sorted \
         result from unsorted input. Code: sorts with an ascending comparator. None.",
    ),
];

/// The artifacts a mode carries, as the prompt names them.
const fn artifacts(mode: Mode) -> &'static str {
    match (mode.has_test(), mode.has_code()) {
        (true, true) => "the test and the code",
        (true, false) => "the test",
        (false, true) => "the code",
        (false, false) => "nothing",
    }
}

/// S2's four levels, each a concrete situation, worded for `mode`.
fn s2_levels(mode: Mode) -> [String; 4] {
    let none = match (mode.has_test(), mode.has_code()) {
        (true, true) => "the test checks, and the code does, exactly what the requirement states",
        (true, false) => "the test checks exactly what the requirement states",
        _ => "the code does exactly what the requirement states",
    };
    let medium = match (mode.has_test(), mode.has_code()) {
        (true, true) => {
            "the test checks the main behaviour but misses a case, boundary or error the \
             requirement states; or the code does the main behaviour but misses or mishandles \
             a stated case, boundary or error"
        }
        (true, false) => {
            "the test checks the main behaviour but misses a case, boundary or error the \
             requirement states"
        }
        _ => {
            "the code does the main behaviour but misses or mishandles a stated case, boundary \
             or error"
        }
    };
    let high = match (mode.has_test(), mode.has_code()) {
        (true, true) => {
            "the test does not check what the requirement states, or would still pass if the \
             implementation did nothing; or the code does the opposite of the requirement or \
             lacks its stated behaviour; or the requirement is not about this test and code at all"
        }
        (true, false) => {
            "the test does not check what the requirement states, or would still pass if the \
             implementation did nothing, or the requirement is not about this test at all"
        }
        _ => {
            "the code does the opposite of the requirement or lacks its stated behaviour, or \
             the requirement is not about this code at all"
        }
    };
    [
        format!("none: nothing is wrong; {none}."),
        "low: a cosmetic mismatch only; a name, a comment or message wording differs, and no \
         behaviour the requirement states is affected."
            .to_owned(),
        format!("medium: mostly right with one gap; {medium}."),
        format!("high: false confidence or wrong behaviour; {high}."),
    ]
}

/// S2's instruction for `mode`, with that mode's worked examples.
fn s2_instruction(mode: Mode) -> String {
    let mut text = format!(
        "How severe is the worst mismatch between the requirement and {}? Pick the level whose \
         situation matches what you see. If the high situation applies, answer high even when \
         everything else looks fine. Worked examples (invented, not from this repository):",
        artifacts(mode)
    );
    for level in Level::ALL.into_iter().rev() {
        for example in EXAMPLES
            .iter()
            .filter(|example| example.mode == mode && example.level == level)
        {
            let _ = write!(text, "\n- {}", example.text);
        }
    }
    text
}

fn s2_asks(row: &Row) -> Vec<Ask> {
    if !(row.mode.has_test() || row.mode.has_code()) {
        return Vec::new();
    }
    let set = questions([(
        SEVERITY,
        score(s2_instruction(row.mode), s2_levels(row.mode)),
    )]);
    vec![Ask {
        unit: None,
        request: request(state(row), set),
    }]
}

/// A `score`'s level probabilities as a [`Distribution`]. The service keys
/// them by the level as a decimal string; a key that is not a whole level in
/// range makes the whole distribution unreadable (`None`), as does an empty
/// map, rather than silently dropping mass.
pub(crate) fn distribution(probabilities: &BTreeMap<String, f64>) -> Option<Distribution> {
    if probabilities.is_empty() {
        return None;
    }
    let mut distribution = [0.0; 4];
    for (key, p) in probabilities {
        let value: f64 = key.trim().parse().ok()?;
        let level = Level::ALL
            .into_iter()
            .find(|level| (level.value() - value).abs() < 1e-9)?;
        let slot = distribution.get_mut(level.index())?;
        *slot += p;
    }
    Some(distribution)
}

/// A severity `score` read as S2 grades it: the level nearest the expected
/// score, the expected score as the ordinal, and the answered level's mass as
/// the confidence. `None` without a readable distribution; the runner
/// (`variant::check_score_levels`) refuses such an answer before any derive
/// sees it, so that is not a path a run can take.
pub(crate) fn expected_prediction(answer: &RawAnswer) -> Option<Prediction> {
    let RawAnswer::Score {
        score,
        probabilities,
        ..
    } = answer
    else {
        return None;
    };
    let level = Level::nearest(*score)?;
    let distribution = distribution(probabilities)?;
    Some(Prediction {
        answer: level.label().to_owned(),
        confidence: Some(mass(&distribution, level)),
        ordinal: Some(*score),
    })
}

/// S2M's floor: `high` whenever the mass on `high` is at least a uniform
/// four-level prior's share. Pre-registered in MP-240; changing it is a new
/// version.
pub(crate) const HIGH_MASS_FLOOR: f64 = 0.25;

/// A severity `score` read as S2M grades it: `high` when the mass on `high`
/// reaches [`HIGH_MASS_FLOOR`], otherwise S2's answer. The ordinal is the
/// mass on `high`, so ordering quality measures how well that mass ranks
/// rows; the confidence is the answered level's mass. `None` without a
/// distribution: S2M has nothing to read.
pub(crate) fn high_mass_prediction(answer: &RawAnswer) -> Option<Prediction> {
    let RawAnswer::Score {
        score,
        probabilities,
        ..
    } = answer
    else {
        return None;
    };
    let distribution = distribution(probabilities)?;
    let high = mass(&distribution, Level::High);
    let level = if high >= HIGH_MASS_FLOOR {
        Level::High
    } else {
        Level::nearest(*score)?
    };
    Some(Prediction {
        answer: level.label().to_owned(),
        confidence: Some(mass(&distribution, level)),
        ordinal: Some(high),
    })
}

fn s2_derive(_row: &Row, answered: &[Answered]) -> Predictions {
    whole_row(answered)
        .get(SEVERITY)
        .and_then(expected_prediction)
        .map(|prediction| Predictions::from([(SEVERITY, prediction)]))
        .unwrap_or_default()
}

fn s2m_derive(_row: &Row, answered: &[Answered]) -> Predictions {
    whole_row(answered)
        .get(SEVERITY)
        .and_then(high_mass_prediction)
        .map(|prediction| Predictions::from([(SEVERITY, prediction)]))
        .unwrap_or_default()
}

/// S2 on full triples.
pub(crate) const S2: Variant = Variant {
    id: "S2",
    version: 1,
    summary: "severity score, levels as concrete situations, worked examples per level (RTC)",
    modes: RTC,
    references: TEST_AND_CODE,
    grades: &[SEVERITY],
    asks: s2_asks,
    derive: s2_derive,
};

/// S2 on requirement-plus-test rows.
pub(crate) const S2_RT: Variant = Variant {
    id: "S2-RT",
    summary: "severity score, levels as concrete situations, worked examples per level (RT)",
    modes: RT,
    references: TEST,
    ..S2
};

/// S2 on requirement-plus-code rows.
pub(crate) const S2_RC: Variant = Variant {
    id: "S2-RC",
    summary: "severity score, levels as concrete situations, worked examples per level (RC)",
    modes: RC,
    references: CODE,
    ..S2
};

/// S2's request graded on the mass on `high`. Shares S2's request, so the
/// runner sends it once for both.
pub(crate) const S2M: Variant = Variant {
    id: "S2M",
    summary: "S2's request, graded on the mass on high (high when P(high) >= 0.25) (RTC)",
    derive: s2m_derive,
    ..S2
};

/// S2M on requirement-plus-test rows.
pub(crate) const S2M_RT: Variant = Variant {
    id: "S2M-RT",
    summary: "S2's request, graded on the mass on high (high when P(high) >= 0.25) (RT)",
    derive: s2m_derive,
    ..S2_RT
};

/// S2M on requirement-plus-code rows.
pub(crate) const S2M_RC: Variant = Variant {
    id: "S2M-RC",
    summary: "S2's request, graded on the mass on high (high when P(high) >= 0.25) (RC)",
    derive: s2m_derive,
    ..S2_RC
};

// ---------------------------------------------------------------------------
// S3: levels as review outcomes
// ---------------------------------------------------------------------------

/// S3's levels, lowest first: the action a reviewer takes, mapped to
/// `none`/`low`/`medium`/`high` by position. Deliberately no examples, so S3
/// isolates the effect of naming levels as outcomes.
pub(crate) const S3_LEVELS: [&str; 4] = [
    "no action: merge as is",
    "backlog: note it and fix it when convenient",
    "fix before release: may merge, but must be fixed before the next release",
    "block merge: must be fixed before this change merges",
];

/// S3's instruction. It names no artifact, so one wording serves every mode.
const S3_INSTRUCTION: &str = "Compare the requirement with everything shown alongside it. Some \
     artifacts may not be shown for this row: that is expected, and a missing artifact is not \
     itself a mismatch. Given the worst mismatch you find between the requirement and what is \
     shown, what should a reviewer do with this change?";

fn s3_asks(row: &Row) -> Vec<Ask> {
    if !(row.mode.has_test() || row.mode.has_code()) {
        return Vec::new();
    }
    vec![Ask {
        unit: None,
        request: request(
            state(row),
            questions([(SEVERITY, score(S3_INSTRUCTION, S3_LEVELS))]),
        ),
    }]
}

/// S3 on every mode with something to compare.
pub(crate) const S3: Variant = Variant {
    id: "S3",
    version: 1,
    summary: "severity score, levels named as review outcomes, mapped to the rubric in code",
    modes: &[Mode::ReqTest, Mode::ReqCode, Mode::ReqTestCode],
    references: &[],
    grades: &[SEVERITY],
    asks: s3_asks,
    derive: s2_derive,
};
