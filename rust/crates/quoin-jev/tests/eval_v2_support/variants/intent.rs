// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `test_asserts_intent` variants T1, T2, T3 and T4, and the trace check TC
//! (PLAT-1030, parent PLAT-1024; pre-registered in MP-242, T3 in its
//! "Round 2" section; T4 in RES-31).
//!
//! # T4, clause coverage
//!
//! T3 asks whether any one assertion checks the whole criterion. On a
//! criterion with several clauses that misses a test covering one clause
//! and flags a test covering each clause with its own assertion. T4 cuts the
//! criterion into clauses in code ([`criterion_clauses`]) and asks, per
//! clause, whether the listed assertions together check it ([`T4`],
//! [`derive_clause_coverage`]); `yes` needs every clause covered.
//!
//! # T3, round 2
//!
//! Dev run 1 on jev-1.13.0: T0, T1 and T2 tied on every weakened-test pair
//! (15 of 15, 22 of 22). T3 moves the reading into code: code lists the
//! test's assertion statements ([`extract_assertions`]), and Jev judges each
//! one on its own: would it fail on a wrong outcome ([`T3`],
//! [`derive_assertion_selection`]). A test with no assertion is `no` with no
//! call.
//!
//! # Why these shapes
//!
//! PLAT-839 asked `test_asserts_intent` as one `noul` and it tied its constant
//! predictor (75.0%). 18 of that corpus's 29 rows cited a requirement that was
//! not about the code, so every value-judgment question ended up measuring
//! "is the trace right". Two things follow:
//!
//! - **TC asks that directly**, as its own `noul`, graded on `trace_correct`.
//!   Its answers also gate the other variants' reports
//!   ([`render_gated_run`]): each variant is reported separately on rows TC
//!   calls correctly traced and on rows it calls mis-traced, so a wrong trace
//!   and a real semantic failure stop sharing one number.
//! - **T1 splits the question in two** (William Lyon, notebooks 10/12): a
//!   `noul` "does the test drive the behaviour the criterion names at all" and
//!   a `choice` "how do its assertions check it". One question could not tell
//!   the two error types apart. T2 is T1 with worked examples in each
//!   instruction (jev-code's `testFrame`).
//!
//! Lyon also found that ambiguous wording comes back as a *confident* answer
//! to the wrong reading, and that naming exactly which claim is judged fixed
//! it. Every instruction here therefore opens with "Judge exactly one claim"
//! and says what is out of scope.
//!
//! # Modes
//!
//! T1 and T2 refer to the test only, so the same wording runs in `RT` (no
//! code exists yet) and in `RTC`, where the code rides along in the state as
//! context. TC's wording names what it judges, which differs by mode, so it is
//! three registered variants, `TC-RT`, `TC-RC` and `TC-RTC`, sharing one
//! builder. Each declares exactly the artifacts its question names.
//!
//! # T1's derive rule
//!
//! `P(yes) = min(P(exercises_criterion), P(asserts_required_outcome))`, and
//! `test_asserts_intent` is `yes` iff `P(yes) >= TAU`. The confidence is
//! `P(yes)` for a `yes` and `1 - P(yes)` for a `no`, so it is never below 0.5
//! and always agrees with the answer. The same `P(yes)` is the ordinal Bar D
//! reads. See [`TAU`] for why 0.5.
//!
//! `check_kind = cannot_tell` abstains only when the rule cannot decide: with
//! `P(exercises) < TAU` the answer is `no` whatever `check_kind` says. An
//! abstention stays in every denominator and is counted in the report. A
//! response the rule cannot read (a missing answer, a label or probability key
//! outside [`CHECK_KINDS`], a probability outside `[0, 1]`) is not an
//! abstention: the run stops and names the row.
//!
//! # Bar D
//!
//! [`bar_d`] is MP-242's paired mutant contrast, implemented here because no
//! shared helper is on `main`. Each test-weakening (or, for TC, trace-swap)
//! mutation is paired with its source row through `mutation.source_id`, one
//! pair per mutation, RTC row first. It is scored by [`pair_outcome`] and
//! tested with [`sign_test_p`].

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};
use typesafe_sdk_questions::{
    NoulCriteria, Question, Questions, choice, noul, noul_with, questions,
};

use crate::eval_v2_support::corpus::{KindGroup, Row, TruthAnswer, TruthKind};
use crate::eval_v2_support::keys::{KEYS, Mode, NO, YES};
use crate::eval_v2_support::metrics::{Scored, render, scored};
use crate::eval_v2_support::units::{Language, is_script_path, mask_for, mask_script};
use crate::eval_v2_support::variant::{
    Answered, Artifact, Ask, Prediction, Predictions, RawAnswer, RawAnswers, RowResult, RunOutput,
    T0, Variant, noul_prediction, request, state, whole_row,
};
use crate::gap_semantic_support::{Variant as BatteryShape, question_set as battery_questions};

// ---------------------------------------------------------------------------
// Question keys and labels
// ---------------------------------------------------------------------------

/// TC's wire key and graded key.
pub(crate) const TRACE_CORRECT: &str = "trace_correct";
/// The graded key T1 and T2 derive.
pub(crate) const TEST_ASSERTS_INTENT: &str = "test_asserts_intent";
/// T1's `noul`: does the test drive the behaviour the criterion names at all.
pub(crate) const EXERCISES_CRITERION: &str = "exercises_criterion";
/// T1's `choice`: how the test's assertions check it.
pub(crate) const CHECK_KIND: &str = "check_kind";

/// The one `check_kind` label that, with the exercise `noul`, means
/// `test_asserts_intent = yes`.
pub(crate) const ASSERTS_REQUIRED_OUTCOME: &str = "asserts_required_outcome";

/// The `check_kind` label for an assertion that checks the named outcome but
/// accepts wrong values: maps to `no`.
pub(crate) const ASSERTS_OUTCOME_TOO_LOOSELY: &str = "asserts_outcome_too_loosely";

/// `check_kind`'s labels and their descriptions, in precedence order: when a
/// test's assertions fit several, the earliest one listed wins ("the strongest
/// thing any assertion checks"). The order is how much of the system's real
/// behaviour an assertion pins down. `cannot_tell` is last and is for when
/// none can be decided.
pub(crate) const CHECK_KINDS: [(&str, &str); 7] = [
    (
        ASSERTS_REQUIRED_OUTCOME,
        "At least one assertion checks the outcome the criterion requires: the specific \
         value, error, state change or output it names, so the test would fail if that \
         outcome did not happen.",
    ),
    (
        ASSERTS_OUTCOME_TOO_LOOSELY,
        "An assertion checks the outcome the criterion names, but too loosely to catch it \
         being wrong: it accepts wrong values, checks a weaker bound than the criterion \
         states, or checks only that the outcome is present.",
    ),
    (
        "asserts_unrelated",
        "The assertions check a real outcome, but not the one this criterion requires: a \
         different field, a different case, or a different feature.",
    ),
    (
        "asserts_only_that_it_runs",
        "The assertions check only that the call completes, returns Ok or Some, produces \
         non-empty output, or does not panic, and never the outcome the criterion names.",
    ),
    (
        "asserts_only_own_setup",
        "The assertions check only values the test itself put in place: its own fixture, \
         stub or mock return values, read back unchanged.",
    ),
    (
        "asserts_constant_or_restatement",
        "The assertions check only a hard-coded literal or a value that would hold whatever \
         the behaviour is, so they pass whatever the system does.",
    ),
    (
        CANNOT_TELL,
        "What is shown is not enough to decide, for example because the assertions live in \
         a helper that is not shown.",
    ),
];

/// The threshold on T1's `P(yes)`.
///
/// 0.5, fixed before any dev data, for three reasons. It is the threshold
/// every other `noul` in the harness uses (MP-232 to MP-234), so T1 is
/// comparable with T0. A threshold picked on dev would be a free parameter
/// the pre-registration cannot account for. And Lyon's finding is that a
/// misread question returns a confident wrong answer, not a middling one, so
/// moving the threshold does not repair wording; the split question does.
pub(crate) const TAU: f64 = 0.5;

// ---------------------------------------------------------------------------
// TC: the trace check
// ---------------------------------------------------------------------------

/// What TC judges in one mode. Every phrase names only the artifacts that
/// mode carries: `RT` never mentions the code, `RC` never the test.
struct Traced {
    /// The behaviour the requirement must describe.
    behaviour: &'static str,
    /// What does not make the trace wrong: partial or weak coverage, in the
    /// mode's own terms.
    even_if: &'static str,
    /// In `RTC` only, what counts when just one of the two is on the
    /// requirement.
    either: &'static str,
}

/// What TC judges in `mode`. `None` for `R`, which has nothing to trace.
const fn traced(mode: Mode) -> Option<Traced> {
    match mode {
        Mode::Req => None,
        Mode::ReqTest => Some(Traced {
            behaviour: "the behaviour exercised by the test in `test_body`",
            even_if: "even if the test covers that behaviour only partly or checks it weakly",
            either: "",
        }),
        Mode::ReqCode => Some(Traced {
            behaviour: "the behaviour implemented by the code in `symbol_body`",
            even_if: "even if the code implements that behaviour only partly",
            either: "",
        }),
        Mode::ReqTestCode => Some(Traced {
            behaviour: "the behaviour exercised by the test in `test_body` or implemented by the \
                        code in `symbol_body`",
            even_if: "even if that behaviour is covered only partly or checked weakly",
            either: " The trace is right when the requirement describes the behaviour of the \
                     test, of the code, or of both; it is wrong only when it describes neither.",
        }),
    }
}

/// TC's question for `mode`, naming exactly the claim judged.
pub(crate) fn trace_question(mode: Mode) -> Option<Question> {
    let Traced {
        behaviour,
        even_if,
        either,
    } = traced(mode)?;
    let instructions = format!(
        "Judge exactly one claim: the requirement cited here (`fr_statement`, narrowed by \
         `ac_text` when present) describes {behaviour}. They were linked by a trace tag, and \
         trace tags are often wrong: a tag can name a requirement about a different feature, a \
         different subsystem, or a process step such as a release, a review or a port, rather \
         than this behaviour.{either} Answer yes when the requirement describes {behaviour}, \
         {even_if}. Answer no when the requirement describes some other behaviour, so that only \
         the tag links them. Do not judge quality, strength or completeness; judge only whether \
         the two are about the same behaviour."
    );
    Some(noul_with(
        instructions,
        NoulCriteria {
            yes: Some(format!("The requirement describes {behaviour}.").into()),
            no: Some(
                "The requirement describes a different behaviour; only the trace tag links them."
                    .into(),
            ),
        },
    ))
}

fn trace_asks(row: &Row) -> Vec<Ask> {
    trace_question(row.mode)
        .map(|question| {
            vec![Ask {
                unit: None,
                request: request(state(row), questions([(TRACE_CORRECT, question)])),
            }]
        })
        .unwrap_or_default()
}

fn trace_derive(row: &Row, answered: &[Answered]) -> Predictions {
    let prediction = loudly(row, derive_trace(&whole_row(answered)));
    Predictions::from([(TRACE_CORRECT, prediction)])
}

/// TC on requirement-plus-test rows: is the requirement about what the test
/// exercises?
pub(crate) const TC_RT: Variant = Variant {
    id: "TC-RT",
    version: 1,
    summary: "trace check: is the cited requirement about the behaviour this test exercises",
    modes: &[Mode::ReqTest],
    references: &[Artifact::Test],
    grades: &[TRACE_CORRECT],
    asks: trace_asks,
    derive: trace_derive,
};

/// TC on requirement-plus-code rows: is the requirement about what the code
/// does?
pub(crate) const TC_RC: Variant = Variant {
    id: "TC-RC",
    version: 1,
    summary: "trace check: is the cited requirement about the behaviour this code implements",
    modes: &[Mode::ReqCode],
    references: &[Artifact::Code],
    grades: &[TRACE_CORRECT],
    asks: trace_asks,
    derive: trace_derive,
};

/// TC on full triples: is the requirement about what the test and code
/// exercise?
pub(crate) const TC_RTC: Variant = Variant {
    id: "TC-RTC",
    version: 1,
    summary: "trace check: is the cited requirement about the behaviour this test and/or code exercise",
    modes: &[Mode::ReqTestCode],
    references: &[Artifact::Test, Artifact::Code],
    grades: &[TRACE_CORRECT],
    asks: trace_asks,
    derive: trace_derive,
};

/// Every TC variant, one per mode it runs in.
pub(crate) const TC_FAMILY: [Variant; 3] = [TC_RT, TC_RC, TC_RTC];

// ---------------------------------------------------------------------------
// T1 / T2: the split question
// ---------------------------------------------------------------------------

const EXERCISES_INSTRUCTIONS: &str = "Judge exactly one claim: the test in `test_body` drives \
     the behaviour that the acceptance criterion names (`ac_text` when present, otherwise \
     `fr_statement`). Drives means the test sets up the situation the criterion describes and \
     triggers the action or input the criterion is about. Answer yes even if the test then \
     checks the result weakly, partly or not at all: how it checks is asked separately. Answer \
     no when the test never triggers that situation or action, for example because it drives a \
     neighbouring feature, a different class of input, or only its own setup.";

const CHECK_KIND_INSTRUCTIONS: &str = "The test in `test_body` is meant to check the acceptance \
     criterion (`ac_text` when present, otherwise `fr_statement`). Judge exactly one thing: what \
     its assertions actually check, compared against that criterion only. Choose the label that \
     describes the strongest thing any of its assertions checks. The labels are listed \
     strongest first: when the assertions fit several, choose the earliest. Choose cannot_tell \
     only when none can be decided.";

/// T2's worked examples for the exercise `noul`. Synthetic: written for this
/// instruction, never copied from a corpus row.
const EXERCISES_EXAMPLES: &str = "Worked examples (synthetic, not from the material you are \
     judging):\n\
     Example 1. Criterion: \"An empty username is rejected with InvalidName.\" Test: `let err = \
     register(\"\").unwrap_err(); assert_eq!(err, RegError::InvalidName);` Answer: yes. Why: it \
     submits an empty username, the situation the criterion names.\n\
     Example 2. Criterion: \"An empty username is rejected with InvalidName.\" Test: `let user = \
     register(\"ada\").unwrap(); assert_eq!(user.name, \"ada\");` Answer: no. Why: it registers a \
     valid name; the empty-name situation is never triggered.\n\
     Example 3. Criterion: \"When the cache is full, the least recently used entry is evicted.\" \
     Test: `let mut c = Cache::new(2); c.put(1, 'a'); c.put(2, 'b'); c.put(3, 'c'); \
     assert!(c.len() <= 2);` Answer: yes. Why: it fills the cache past capacity, which triggers \
     eviction. How it checks is judged separately: `len() <= 2` never checks which entry was \
     evicted, so its check is asserts_outcome_too_loosely.\n\
     Example 4. Criterion: \"Retries stop after five failed attempts.\" Test: `let policy = \
     RetryPolicy::default(); assert_eq!(policy.max_attempts, 5);` Answer: no. Why: it reads a \
     setting; no attempt ever fails, so stopping is never driven.\n\
     Example 5. Criterion: \"Timestamps in the export are written in UTC.\" Test: `let rows = \
     parse_csv(FIXTURE); assert_eq!(rows.len(), 3);` Answer: no. Why: it parses an input file; \
     no export is ever produced.";

/// T2's worked examples for `check_kind`, one per label, in precedence
/// order. Synthetic.
const CHECK_KIND_EXAMPLES: &str = "Worked examples (synthetic, not from the material you are \
     judging):\n\
     Example 1. Criterion: \"A withdrawal larger than the balance is refused with \
     InsufficientFunds and the balance is unchanged.\" Test: `let mut acct = \
     Account::with_balance(10); let err = acct.withdraw(50).unwrap_err(); assert_eq!(err, \
     Error::InsufficientFunds); assert_eq!(acct.balance(), 10);` Answer: \
     asserts_required_outcome. Why: it checks the named error and the unchanged balance.\n\
     Example 2. Criterion: \"The invoice total is the net amount plus 20% VAT.\" Test: `let inv \
     = invoice(100.0); assert!(inv.total > 100.0);` Answer: asserts_outcome_too_loosely. Why: it \
     checks the total, but any increase passes, not only 120.\n\
     Example 3. Criterion: \"Deleting a user also deletes their sessions.\" Test: \
     `store.delete_user(id); assert!(store.find_user(id).is_none());` Answer: asserts_unrelated. \
     Why: it checks the user is gone, never the sessions.\n\
     Example 4. Criterion: \"The report lists orders newest first.\" Test: `let report = \
     build_report(&orders); assert!(!report.is_empty());` Answer: asserts_only_that_it_runs. \
     Why: a non-empty report holds whatever the order.\n\
     Example 5. Criterion: \"Prices are converted at the day's exchange rate.\" Test: `let rates \
     = FakeRates::fixed(1.25); let conv = Converter::new(&rates); assert_eq!(rates.rate_for(\"EUR\"), \
     1.25);` Answer: asserts_only_own_setup. Why: it reads back the stub's own rate and never \
     converts a price.\n\
     Example 6. Criterion: \"Orders over 100 get a 10% discount.\" Test: `let d = \
     discount_for(150.0); assert_eq!(d, discount_for(150.0));` Answer: \
     asserts_constant_or_restatement. Why: a result compared with itself holds whatever \
     discount it is.\n\
     Example 7. Criterion: \"Uploads over 10 MB are rejected.\" Test: \
     `run_upload_case(\"large.bin\");` Answer: cannot_tell. Why: the checks are inside a helper \
     that is not shown.";

/// T1's two questions; with `examples`, T2's.
pub(crate) fn split_questions(examples: bool) -> Questions {
    let with_examples = |instructions: &str, worked: &str| {
        if examples {
            format!("{instructions}\n\n{worked}")
        } else {
            instructions.to_owned()
        }
    };
    questions([
        (
            EXERCISES_CRITERION,
            noul_with(
                with_examples(EXERCISES_INSTRUCTIONS, EXERCISES_EXAMPLES),
                NoulCriteria {
                    yes: Some(
                        "The test triggers the situation or action the criterion names.".into(),
                    ),
                    no: Some(
                        "The test never triggers the situation or action the criterion names."
                            .into(),
                    ),
                },
            ),
        ),
        (
            CHECK_KIND,
            choice(
                with_examples(CHECK_KIND_INSTRUCTIONS, CHECK_KIND_EXAMPLES),
                CHECK_KINDS,
            ),
        ),
    ])
}

fn t1_asks(row: &Row) -> Vec<Ask> {
    vec![Ask {
        unit: None,
        request: request(state(row), split_questions(false)),
    }]
}

fn t2_asks(row: &Row) -> Vec<Ask> {
    vec![Ask {
        unit: None,
        request: request(state(row), split_questions(true)),
    }]
}

/// The `check_kind` label that abstains: T1 and T2 give no answer for the
/// row. The row stays in every denominator and is counted as an abstention.
pub(crate) const CANNOT_TELL: &str = "cannot_tell";

/// A probability from Jev, refused unless it is a finite number in `[0, 1]`.
fn probability(key: &str, p: f64) -> Result<f64, String> {
    if p.is_finite() && (0.0..=1.0).contains(&p) {
        Ok(p)
    } else {
        Err(format!("`{key}`: probability {p} is outside [0, 1]"))
    }
}

/// A `noul` answer's probability, or why there is none.
fn noul_probability(answers: &RawAnswers, key: &str) -> Result<f64, String> {
    match answers.get(key) {
        Some(RawAnswer::Noul(p)) => probability(key, *p),
        Some(other) => Err(format!("`{key}`: expected a noul, got {other:?}")),
        None => Err(format!("`{key}`: no answer in the response")),
    }
}

/// TC's derive rule: `trace_correct` is the `noul` thresholded at 0.5.
///
/// # Errors
/// When the answer is missing, not a `noul`, or not a probability.
pub(crate) fn derive_trace(answers: &RawAnswers) -> Result<Prediction, String> {
    noul_probability(answers, TRACE_CORRECT)?;
    noul_prediction(answers, TRACE_CORRECT)
        .ok_or_else(|| format!("`{TRACE_CORRECT}`: no answer in the response"))
}

/// T1's derive rule over one response's answers:
/// `P(yes) = min(P(exercises_criterion), P(asserts_required_outcome))`, and
/// `yes` iff `P(yes) >= TAU`. `Ok(None)` is an abstention: `check_kind` came
/// back `cannot_tell` while `P(exercises) >= TAU`, so the rule cannot decide.
///
/// # Errors
/// Loudly, never as a silent unanswered row: when either answer is missing
/// or of the wrong shape, when `check_kind` is not one of [`CHECK_KINDS`],
/// when its probabilities name a label outside them or omit
/// `asserts_required_outcome`, or when any probability is outside `[0, 1]`.
pub(crate) fn derive_asserts_intent(answers: &RawAnswers) -> Result<Option<Prediction>, String> {
    let p_exercises = noul_probability(answers, EXERCISES_CRITERION)?;
    let (label, probabilities) = match answers.get(CHECK_KIND) {
        Some(RawAnswer::Choice {
            label,
            probabilities,
            ..
        }) => (label, probabilities),
        Some(other) => return Err(format!("`{CHECK_KIND}`: expected a choice, got {other:?}")),
        None => return Err(format!("`{CHECK_KIND}`: no answer in the response")),
    };
    let known = |name: &str| CHECK_KINDS.iter().any(|(kind, _)| *kind == name);
    if !known(label) {
        return Err(format!("`{CHECK_KIND}`: unknown label {label:?}"));
    }
    if let Some(stray) = probabilities.keys().find(|name| !known(name)) {
        return Err(format!(
            "`{CHECK_KIND}`: probability for unknown label {stray:?}"
        ));
    }
    let p_required = probabilities
        .get(ASSERTS_REQUIRED_OUTCOME)
        .ok_or_else(|| format!("`{CHECK_KIND}`: no probability for `{ASSERTS_REQUIRED_OUTCOME}`"))
        .and_then(|p| probability(CHECK_KIND, *p))?;
    if label == CANNOT_TELL && p_exercises >= TAU {
        return Ok(None);
    }
    let p_yes = p_exercises.min(p_required);
    let yes = p_yes >= TAU;
    Ok(Some(Prediction {
        answer: if yes { YES } else { NO }.to_owned(),
        confidence: Some(if yes { p_yes } else { 1.0 - p_yes }),
        ordinal: Some(p_yes),
    }))
}

/// Runs a fallible derive, turning a malformed response into a panic that
/// names the row: a run over answers it cannot read must stop, not score
/// them as unanswered.
fn loudly<T>(row: &Row, derived: Result<T, String>) -> T {
    derived.unwrap_or_else(|error| {
        panic!(
            "{}: malformed Jev response ({error}); refusing to score it as unanswered",
            row.id
        )
    })
}

fn split_derive(row: &Row, answered: &[Answered]) -> Predictions {
    loudly(row, derive_asserts_intent(&whole_row(answered)))
        .map(|prediction| Predictions::from([(TEST_ASSERTS_INTENT, prediction)]))
        .unwrap_or_default()
}

const TEST_MODES: &[Mode] = &[Mode::ReqTest, Mode::ReqTestCode];

/// T1: Lyon's split, a `noul` for "exercises it at all" and a `choice` for
/// "how it checks", combined by [`derive_asserts_intent`].
pub(crate) const T1: Variant = Variant {
    id: "T1",
    version: 1,
    summary: "test_asserts_intent split: exercises-the-criterion noul + check-kind choice",
    modes: TEST_MODES,
    references: &[Artifact::Test],
    grades: &[TEST_ASSERTS_INTENT],
    asks: t1_asks,
    derive: split_derive,
};

/// T2: T1 with synthetic worked examples in each instruction.
pub(crate) const T2: Variant = Variant {
    id: "T2",
    version: 1,
    summary: "T1 with 5 and 7 synthetic worked examples in the two instructions",
    modes: TEST_MODES,
    references: &[Artifact::Test],
    grades: &[TEST_ASSERTS_INTENT],
    asks: t2_asks,
    derive: split_derive,
};

// ---------------------------------------------------------------------------
// T3: assertion selection (PLAT-1024 round 2, MP-242 "Round 2")
// ---------------------------------------------------------------------------

/// The state field T3 lists the test's assertions in, `A1` .. `An`. It is a
/// `test_` field, so the wording rule reads it as the test's.
pub(crate) const ASSERTIONS_FIELD: &str = "test_assertions";

/// T3 lists at most this many assertions; the overflow joins the last, so no
/// assertion text is dropped.
pub(crate) const MAX_ASSERTIONS: usize = 20;

/// A Rust assertion's opening: an assert-family macro (`assert!`,
/// `assert_eq!`, `assert_ne!`, `assert_matches!`, `debug_assert*!`,
/// `prop_assert*!`, so `assert!(matches!(..))` too), a call that insists on
/// an error (`.unwrap_err()`, `.expect_err(..)`), or a failure point: a
/// `panic!(..)` or a proptest `Err(TestCaseError::fail(..))`, which
/// [`failure_span`] widens to the arm, `let .. else` or `if` it fails in.
/// Adapted from jev-code's `ASSERTION` (`src/workflows/hunks.ts`), which reads
/// diff lines; this reads whole statements.
///
/// `.unwrap()` and `.expect(..)` are not assertions (T3 v3, MP-242): they
/// check only that a call returned a success value, which T3's own
/// instruction already answers `no` for. A test whose only check is one is
/// `no` with no call. A `panic!` inside a closure (`.unwrap_or_else(|e|
/// panic!(..))`) is the same check spelled out, and is not listed either.
static RUST_ASSERTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:debug_)?(?:prop_)?assert\w*!\s*[(\[{]|\.(?:unwrap_err|expect_err)\s*\(|\bpanic!\s*[(\[{]|(?:\breturn\s+)?\bErr\s*\(\s*(?:\w+::)*TestCaseError::fail\s*\(",
    )
    .unwrap_or_else(|error| unreachable!("a literal regex compiles: {error}"))
});

/// A Python assertion's opening: an `assert` statement at a line's start or
/// after a `:` on the same line (`if x: assert y`), a unittest
/// `self.assert*(..)`, `pytest.raises(..)`, or a dotted `assert_*(..)` call
/// (`mock.assert_called_once_with(..)`, `np.testing.assert_allclose(..)`).
static PYTHON_ASSERTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^[ \t]*assert\b|:[ \t]*assert\b|\bself\.assert\w*\s*\(|\bpytest\.raises\s*\(|\.assert_\w+\s*\(",
    )
    .unwrap_or_else(|error| unreachable!("a literal regex compiles: {error}"))
});

/// Where the statement holding an assertion that opens at `at` begins, in
/// masked bytes. An assert macro or `self.assert*` begins where it matched.
/// An `.unwrap_err()` / `.expect_err()` call, a Python `assert` and a
/// `pytest.raises` begin at their statement's first token: back to the
/// previous `;`, `{` or `}` (Rust) or line break (Python) outside brackets.
fn statement_start(masked: &[u8], at: usize, python: bool) -> usize {
    let opens_at_match = masked
        .get(at..)
        .is_some_and(|rest| rest.starts_with(b"self.") || !(python || rest.starts_with(b".")));
    if opens_at_match {
        return at;
    }
    let mut depth = 0usize;
    let mut start = at;
    while let Some(byte) = start.checked_sub(1).and_then(|before| masked.get(before)) {
        match byte {
            b')' | b']' => depth += 1,
            b'(' | b'[' if depth == 0 => break,
            b'(' | b'[' => depth -= 1,
            b'{' | b'}' | b';' if depth == 0 && !python => break,
            b'\n' | b';' if depth == 0 && python => break,
            _ => {}
        }
        start -= 1;
    }
    while masked.get(start).is_some_and(u8::is_ascii_whitespace) && start < at {
        start += 1;
    }
    start
}

/// Where the statement from `from` ends (exclusive), in masked bytes: after
/// its `;` (Rust), at its line break (Python, outside brackets and not after
/// a `\`), at a `,` outside brackets (a Rust match arm), or at the `}` that
/// closes the block it is the tail of. With `macro_call` it ends at the
/// closing bracket of the first group it opens, and the `;` when one follows:
/// a brace-delimited macro statement needs no `;` (`assert_matches! { x, P }`
/// on its own line), so without this it would run on into the next statement.
fn statement_end(masked: &[u8], from: usize, python: bool, macro_call: bool) -> usize {
    let mut depth = 0usize;
    let mut at = from;
    while let Some(byte) = masked.get(at) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => return at,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 && macro_call {
                    let after = (at + 1..masked.len())
                        .find(|index| !masked.get(*index).is_some_and(u8::is_ascii_whitespace))
                        .unwrap_or(masked.len());
                    return if masked.get(after) == Some(&b';') {
                        after + 1
                    } else {
                        at + 1
                    };
                }
            }
            b',' if depth == 0 && !python => return at,
            b';' if depth == 0 => return if python { at } else { at + 1 },
            b'\n' if depth == 0 && python => {
                let continued = at
                    .checked_sub(1)
                    .and_then(|before| masked.get(before))
                    .is_some_and(|before| *before == b'\\');
                if !continued {
                    return at;
                }
            }
            _ => {}
        }
        at += 1;
    }
    masked.len()
}

/// Where the Rust head ending just before `before` begins, in masked bytes:
/// back to the previous `;`, `{`, `}` or `,` outside brackets, then past
/// whitespace. A match arm's pattern, or a `let .. else` / `if` head.
fn head_start(masked: &[u8], before: usize) -> usize {
    let mut depth = 0usize;
    let mut start = before;
    while let Some(byte) = start.checked_sub(1).and_then(|at| masked.get(at)) {
        match byte {
            b')' | b']' => depth += 1,
            b'(' | b'[' | b'{' | b'}' | b';' | b',' if depth == 0 => break,
            b'(' | b'[' => depth -= 1,
            _ => {}
        }
        start -= 1;
    }
    skip_whitespace(masked, start, before)
}

/// The first index at or after `at`, before `end`, that is not whitespace.
fn skip_whitespace(masked: &[u8], at: usize, end: usize) -> usize {
    (at..end)
        .find(|index| !masked.get(*index).is_some_and(u8::is_ascii_whitespace))
        .unwrap_or(end)
}

/// The statement a Rust failure point (`panic!(..)`, `Err(TestCaseError::
/// fail(..))`) opening at `at` asserts with, in masked bytes, or `None` when
/// it is not one T3 lists:
///
/// - after `=>`: the match arm, pattern through the failure
///   (`Err(e) => panic!("{e}")`);
/// - first in a block whose head is a `let .. else`, an `if`, or a match arm:
///   head through the block's `}` (and a `;` after it), so
///   `let Some(x) = y else { panic!(..) };` is one statement;
/// - after a `;`, a `}` or any other `{`, or first in the body: the failure
///   statement alone;
/// - anywhere else, inside a call or a closure: `None`. That is
///   `.unwrap_or_else(|e| panic!(..))`, an `.expect(..)` spelled out.
fn failure_span(masked: &[u8], at: usize) -> Option<(usize, usize)> {
    let alone = || (at, statement_end(masked, at, false, true));
    let Some(before) = (0..at)
        .rev()
        .find(|index| !masked.get(*index).is_some_and(u8::is_ascii_whitespace))
    else {
        return Some(alone());
    };
    let arrow = before
        .checked_sub(1)
        .filter(|eq| masked.get(*eq..=before) == Some(b"=>".as_slice()));
    if let Some(arrow) = arrow {
        let start = head_start(masked, arrow);
        return Some((start, statement_end(masked, at, false, true)));
    }
    match masked.get(before) {
        Some(b'{') => {
            let start = head_start(masked, before);
            let head = String::from_utf8_lossy(masked.get(start..before).unwrap_or_default());
            let mut words = head.split_whitespace();
            let first = words.next();
            let last = words.next_back().or(first);
            let owned =
                first == Some("if") || last == Some("else") || head.trim_end().ends_with("=>");
            Some(if owned {
                (start, statement_end(masked, before, false, true))
            } else {
                alone()
            })
        }
        Some(b';' | b'}') => Some(alone()),
        _ => None,
    }
}

/// A TypeScript/JavaScript assertion's opening (T3 v4): an `expect(..)`
/// chain (Jest, Vitest, chai) or a `node:assert` call: `assert(..)`,
/// `assert.equal(..)`, `assert.strictEqual(..)`, `assert.deepStrictEqual(..)`,
/// `assert.throws(..)`, `assert.strict.equal(..)`. A member call
/// (`x.expect(..)`, `x.assert(..)`) is not one; [`script_spans`] drops it.
static SCRIPT_ASSERTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:expect|assert(?:\s*\.\s*\w+)*)\s*\(")
        .unwrap_or_else(|error| unreachable!("a literal regex compiles: {error}"))
});

/// `text` masked as the test at `path` is read: [`mask_script`] for a
/// TypeScript or JavaScript file, [`mask_for`] otherwise.
fn mask_test(path: &str, text: &str) -> Vec<u8> {
    if is_script_path(path) {
        mask_script(text)
    } else {
        mask_for(path, text)
    }
}

/// Whether the masked byte `byte` can continue a JavaScript identifier.
const fn is_script_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

/// The last index before `at` that is not whitespace, in masked bytes.
fn previous_token(masked: &[u8], at: usize) -> Option<usize> {
    (0..at)
        .rev()
        .find(|index| !masked.get(*index).is_some_and(u8::is_ascii_whitespace))
}

/// Where a script assertion opening at `at` begins: at the match, or at an
/// `await`, `return` or `void` directly before it.
fn script_statement_start(masked: &[u8], at: usize) -> usize {
    let Some(last) = previous_token(masked, at) else {
        return at;
    };
    let head = masked.get(..=last).unwrap_or_default();
    for keyword in [b"await".as_slice(), b"return", b"void"] {
        if head.ends_with(keyword) {
            let start = head.len() - keyword.len();
            let standalone = !start
                .checked_sub(1)
                .and_then(|before| masked.get(before))
                .copied()
                .is_some_and(is_script_ident);
            if standalone {
                return start;
            }
        }
    }
    at
}

/// Where the script statement from `from` ends (exclusive), in masked
/// bytes: after its `;`, at a `,` outside brackets (an argument list), at
/// the `)`, `]` or `}` that closes what holds it (an arrow callback's body),
/// or at a line break outside brackets unless the next line continues the
/// chain with a `.` (a statement with no `;`).
fn script_statement_end(masked: &[u8], from: usize) -> usize {
    let mut depth = 0usize;
    let mut at = from;
    while let Some(byte) = masked.get(at) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' | b',' if depth == 0 => return at,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 => return at + 1,
            b'\n' if depth == 0 => {
                let next = skip_whitespace(masked, at, masked.len());
                if masked.get(next) != Some(&b'.') {
                    return at;
                }
            }
            _ => {}
        }
        at += 1;
    }
    masked.len()
}

/// Whether the `expect(` whose `(` is at `open` is followed, before `end`,
/// by a member chain (`.toBe(..)`, `.rejects.toThrow(..)`, chai's
/// `.to.be.true`). A bare `expect(x)` checks nothing.
fn has_matcher(masked: &[u8], open: usize, end: usize) -> bool {
    let mut depth = 0usize;
    let close = (open..end).find(|index| match masked.get(*index) {
        Some(b'(' | b'[' | b'{') => {
            depth += 1;
            false
        }
        Some(b')' | b']' | b'}') => {
            depth = depth.saturating_sub(1);
            depth == 0
        }
        _ => false,
    });
    close.is_some_and(|close| {
        let next = skip_whitespace(masked, close + 1, end);
        masked.get(next) == Some(&b'.')
    })
}

/// The assertion statements of a TypeScript or JavaScript test, as masked
/// byte spans in source order (T3 v4). An `expect(..)` chain ending in a
/// matcher is one statement, and so is a `node:assert` call, each with an
/// `await`, `return` or `void` before it; one nested inside another is part
/// of it.
fn script_spans(masked: &[u8]) -> Vec<(usize, usize)> {
    let masked_text = String::from_utf8_lossy(masked);
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for found in SCRIPT_ASSERTION.find_iter(&masked_text) {
        let previous_end = spans.last().map_or(0, |(_, end)| *end);
        if found.start() < previous_end {
            continue;
        }
        let member = previous_token(masked, found.start())
            .and_then(|before| masked.get(before))
            .is_some_and(|byte| *byte == b'.' || *byte == b'$');
        if member {
            continue;
        }
        let end = script_statement_end(masked, found.start()).max(found.end());
        let open = found.end() - 1;
        if found.as_str().starts_with("expect") && !has_matcher(masked, open, end) {
            continue;
        }
        let start = script_statement_start(masked, found.start());
        spans.push((start.max(previous_end), end));
    }
    spans
}

/// Which bytes of `body` lie inside a comment or a string or char literal,
/// as [`mask_test`] reads them. Accurate for whitespace bytes, the only ones
/// [`collapse_code_whitespace`] asks about: `body` is masked with every
/// space and tab swapped for a control byte first, so a whitespace byte the
/// mask leaves unchanged is code, and one it blanks is inside a literal.
fn literal_bytes(path: &str, body: &str) -> Vec<bool> {
    let marked: String = body
        .chars()
        .map(|ch| if ch == ' ' || ch == '\t' { '\u{1}' } else { ch })
        .collect();
    let blanked = mask_test(path, &marked);
    marked
        .bytes()
        .zip(blanked)
        .map(|(original, blank)| original != blank)
        .collect()
}

/// `body[start..end]` with each run of whitespace outside comments and
/// literals collapsed to one space and the ends trimmed. A comment's or a
/// literal's own bytes stay verbatim, and a run ending a line comment keeps
/// one line break, so the code after it is not read as part of the comment.
fn collapse_code_whitespace(body: &str, literal: &[bool], start: usize, end: usize) -> String {
    let in_literal = |index: usize| literal.get(index).copied().unwrap_or(false);
    let mut out = String::new();
    let mut pending: Option<char> = None;
    // Where the literal just emitted began, while the byte before is in it.
    let mut literal_from: Option<usize> = None;
    for (offset, ch) in body.get(start..end).unwrap_or_default().char_indices() {
        let index = start + offset;
        if in_literal(index) {
            if let Some(gap) = pending.take() {
                out.push(gap);
            }
            literal_from.get_or_insert(index);
            out.push(ch);
            continue;
        }
        let after = literal_from.take();
        if ch.is_whitespace() {
            let ends_line_comment = ch == '\n'
                && after.is_some_and(|from| {
                    let rest = body.get(from..).unwrap_or_default();
                    rest.starts_with("//") || rest.starts_with('#')
                });
            if ends_line_comment {
                pending = Some('\n');
            } else if !out.is_empty() && pending.is_none() {
                pending = Some(' ');
            }
            continue;
        }
        if let Some(gap) = pending.take() {
            out.push(gap);
        }
        out.push(ch);
    }
    out
}

/// Every assertion statement in a test body, in source order, verbatim with
/// whitespace collapsed outside comments and string literals
/// ([`collapse_code_whitespace`]). Comments and string literals are masked
/// first ([`mask_for`]), so an `assert!` inside a string or a comment is not
/// one; an assertion nested inside another (`assert_eq!(f().unwrap_err(),
/// ..)`) is part of the outer one. The language is `path`'s: Python for
/// `.py`, TypeScript/JavaScript for `.ts`, `.tsx`, `.js`, `.mjs` and `.cjs`
/// ([`script_spans`], T3 v4), Rust otherwise. Code, not Jev, does this: T3
/// asks Jev only about the statements found.
pub(crate) fn extract_assertions(path: &str, body: &str) -> Vec<String> {
    let spans = if is_script_path(path) {
        script_spans(&mask_script(body))
    } else {
        native_spans(path, body)
    };
    let literal = literal_bytes(path, body);
    spans
        .into_iter()
        .map(|(start, end)| collapse_code_whitespace(body, &literal, start, end))
        .filter(|text| !text.is_empty())
        .collect()
}

/// The assertion statements of a Rust or Python test, as masked byte spans
/// in source order: see [`RUST_ASSERTION`] and [`PYTHON_ASSERTION`].
fn native_spans(path: &str, body: &str) -> Vec<(usize, usize)> {
    let python = Language::of_path(path) == Some(Language::Python);
    let masked = mask_for(path, body);
    let masked_text = String::from_utf8_lossy(&masked);
    let pattern: &Regex = if python {
        &PYTHON_ASSERTION
    } else {
        &RUST_ASSERTION
    };
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for found in pattern.find_iter(&masked_text) {
        let previous_end = spans.last().map_or(0, |(_, end)| *end);
        if found.start() < previous_end {
            continue;
        }
        let text = found.as_str();
        let failure = !python && (text.starts_with("panic") || text.contains("TestCaseError"));
        let (start, end) = if failure {
            let Some(span) = failure_span(&masked, found.start()) else {
                continue;
            };
            span
        } else {
            let start = statement_start(&masked, found.start(), python);
            let start = if python {
                // `(?m)^[ \t]*assert` matches from the line start.
                skip_whitespace(&masked, start, found.end())
            } else {
                start
            };
            let macro_call = !python && text.contains('!');
            (
                start,
                statement_end(&masked, found.start(), python, macro_call),
            )
        };
        spans.push((start.max(previous_end), end.max(found.end())));
    }
    spans
}

/// The assertions T3 lists for `row`: [`extract_assertions`] over its test,
/// the overflow past [`MAX_ASSERTIONS`] joined into the last. Empty when the
/// row has no test or its test has no assertion.
pub(crate) fn row_assertions(row: &Row) -> Vec<String> {
    let Some(test) = &row.test else {
        return Vec::new();
    };
    let mut found = extract_assertions(&test.path, &test.body);
    if found.len() > MAX_ASSERTIONS {
        let overflow = found.split_off(MAX_ASSERTIONS - 1).join(" ");
        found.push(overflow);
    }
    found
}

/// The assertion labels: `A1` .. `An`.
pub(crate) fn assertion_label(index: usize) -> String {
    format!("A{}", index + 1)
}

/// T3's instruction for the assertion labelled `label`. It names the test
/// and the requirement only: in `RT` there is nothing else, and in `RTC` the
/// rest of the state is context.
fn assertion_instruction(label: &str) -> String {
    format!(
        "Judge exactly one claim about assertion {label} in `{ASSERTIONS_FIELD}`, which lists, \
         verbatim, the assertion statements of the test in `test_body`. The requirement is \
         `fr_statement`, narrowed by `ac_text` when present. The claim: assertion {label}, on \
         its own, checks the outcome the requirement states, strictly enough that it would fail \
         if the system produced a different outcome from the one the requirement states. Answer \
         no when it checks only that a call completes or returns a success value; checks a \
         different outcome, field or case; checks a value the test set up itself; or would also \
         pass for a wrong outcome, as a presence, non-empty, `is_some`, `contains` or bound \
         check does where the requirement states an exact value."
    )
}

/// T3's questions for `count` assertions: one `noul` per assertion, keyed by
/// its label (`A1` .. `An`), each judged on its own.
pub(crate) fn assertion_questions(count: usize) -> Questions {
    questions((0..count).map(|index| {
        let label = assertion_label(index);
        let question = noul_with(
            assertion_instruction(&label),
            NoulCriteria {
                yes: Some(
                    format!(
                        "Assertion {label} alone would fail if the outcome the requirement \
                         states did not happen."
                    )
                    .into(),
                ),
                no: Some(
                    format!(
                        "Assertion {label} would still pass with a wrong outcome, or checks \
                         something else."
                    )
                    .into(),
                ),
            },
        );
        (label, question)
    }))
}

/// One ask carrying the row's assertions in [`ASSERTIONS_FIELD`], or none
/// when the test has no assertion: that row's answer is derived in code.
fn t3_asks(row: &Row) -> Vec<Ask> {
    let assertions = row_assertions(row);
    if assertions.is_empty() {
        return Vec::new();
    }
    let listed: serde_json::Map<String, serde_json::Value> = assertions
        .iter()
        .enumerate()
        .map(|(index, text)| (assertion_label(index), text.clone().into()))
        .collect();
    let mut row_state = state(row);
    if let serde_json::Value::Object(fields) = &mut row_state {
        fields.insert(
            ASSERTIONS_FIELD.to_owned(),
            serde_json::Value::Object(listed),
        );
    }
    vec![Ask {
        unit: None,
        request: request(row_state, assertion_questions(assertions.len())),
    }]
}

/// T3's derive rule over `count` listed assertions and the one response:
/// `P(any)` is the highest `P(An)` over the assertions, `test_asserts_intent`
/// is `yes` iff `P(any) >= TAU`, the confidence is `P(any)` for `yes` and
/// `1 - P(any)` for `no`, and `P(any)` is the ordinal Bar D reads. With
/// `count == 0` nothing was asked: a test with no assertion is `no`,
/// `P(any) = 0`, confidence 1.
///
/// # Errors
/// When an assertion's answer is missing, not a `noul`, or not a
/// probability in `[0, 1]`, or when the response answers a label that was
/// not asked.
pub(crate) fn derive_assertion_selection(
    count: usize,
    answers: &RawAnswers,
) -> Result<Prediction, String> {
    let labels: Vec<String> = (0..count).map(assertion_label).collect();
    if let Some(stray) = answers.keys().find(|key| !labels.contains(key)) {
        return Err(format!(
            "`{stray}`: answered, but no such assertion was asked"
        ));
    }
    let mut p_any = 0.0f64;
    for label in &labels {
        p_any = p_any.max(noul_probability(answers, label)?);
    }
    let yes = count > 0 && p_any >= TAU;
    Ok(Prediction {
        answer: if yes { YES } else { NO }.to_owned(),
        confidence: Some(if yes { p_any } else { 1.0 - p_any }),
        ordinal: Some(p_any),
    })
}

fn t3_derive(row: &Row, answered: &[Answered]) -> Predictions {
    let count = row_assertions(row).len();
    let prediction = loudly(row, derive_assertion_selection(count, &whole_row(answered)));
    Predictions::from([(TEST_ASSERTS_INTENT, prediction)])
}

/// T3: code lists the test's assertion statements; Jev judges each on its
/// own. `yes` iff the highest `P(An) >= 0.5`.
///
/// v1 (dev round 2, run 1) asked one `choice` over the listed assertions plus
/// `none`, `yes` iff `1 - P(none) >= 0.5`: weakened mutants fell but stayed
/// above 0.5, the mass the remaining assertions shared keeping `P(any)` up.
/// v2 asks one strict `noul` per assertion and takes the highest, and ends a
/// statement at a `,` outside brackets so two match arms' assertions no
/// longer merge. v3 asks v2's question over a wider list: `panic!` and
/// proptest `TestCaseError::fail` failure points, Python mock and
/// `np.testing` `assert_*` calls and an `assert` after a `:`; and string
/// literals and comments in a listed assertion stay verbatim. v4 asks v3's
/// question and lists TypeScript and JavaScript assertions too
/// ([`script_spans`]): before it, a `.ts` or `.js` test was read as Rust,
/// listed nothing, and was `no` with no call. Rust and Python listings are
/// unchanged.
pub(crate) const T3: Variant = Variant {
    id: "T3",
    version: 4,
    summary: "assertion check: code lists the test's assertions, one strict noul per \
              assertion (alone, would it fail on a wrong outcome); yes iff max P >= 0.5",
    modes: TEST_MODES,
    references: &[Artifact::Test],
    grades: &[TEST_ASSERTS_INTENT],
    asks: t3_asks,
    derive: t3_derive,
};

// ---------------------------------------------------------------------------
// T4: clause coverage (RES-31)
// ---------------------------------------------------------------------------

/// The state field T4 lists the criterion's clauses in, `C1` .. `Cn`.
pub(crate) const CLAUSES_FIELD: &str = "criterion_clauses";

/// T4 lists at most this many clauses; the overflow joins the last, so no
/// clause text is dropped.
pub(crate) const MAX_CLAUSES: usize = 8;

/// `text` cut at every top-level point `separator` names, outside backticks
/// and parentheses. `separator(text, at)` is the length of the separator
/// starting at byte `at`, or `None`; every separator starts with an ASCII
/// byte and spans ASCII bytes, so each cut is on a char boundary.
fn split_top_level(text: &str, separator: impl Fn(&str, usize) -> Option<usize>) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut pieces = Vec::new();
    let (mut start, mut at, mut depth, mut code) = (0usize, 0usize, 0usize, false);
    while let Some(byte) = bytes.get(at) {
        match byte {
            b'`' => code = !code,
            b'(' if !code => depth += 1,
            b')' if !code => depth = depth.saturating_sub(1),
            _ if !code && depth == 0 => {
                if let Some(len) = separator(text, at) {
                    pieces.push(text.get(start..at).unwrap_or_default());
                    at += len;
                    start = at;
                    continue;
                }
            }
            _ => {}
        }
        at += 1;
    }
    pieces.push(text.get(start..).unwrap_or_default());
    pieces
}

/// A sentence end at `at`: a `.`, whitespace, then an uppercase letter or a
/// backtick. The length covers the `.` and the whitespace.
fn sentence_end(text: &str, at: usize) -> Option<usize> {
    let rest = text.get(at..)?.strip_prefix('.')?;
    let next = rest.trim_start();
    let gap = rest.len() - next.len();
    let opens = next
        .chars()
        .next()
        .is_some_and(|ch| ch.is_uppercase() || ch == '`');
    (gap > 0 && opens).then_some(1 + gap)
}

/// A `;` at `at`.
fn semicolon(text: &str, at: usize) -> Option<usize> {
    text.get(at..)?.starts_with(';').then_some(1)
}

/// A serial list's last `, and ` at `at`.
fn and_conjunct(text: &str, at: usize) -> Option<usize> {
    const SEPARATOR: &str = ", and ";
    text.get(at..)?
        .starts_with(SEPARATOR)
        .then_some(SEPARATOR.len())
}

/// A list comma, `, `, at `at`. A comma with no space after it (`1,000`) is
/// not one.
fn list_comma(text: &str, at: usize) -> Option<usize> {
    text.get(at..)?.starts_with(", ").then_some(2)
}

/// `piece` cut into its conjuncts: at each top-level `, and `, with the
/// comma-separated items before it, which are the same serial list. A piece
/// with no `, and ` is one conjunct.
fn conjuncts(piece: &str) -> Vec<&str> {
    let segments = split_top_level(piece, and_conjunct);
    let Some((last, items)) = segments.split_last() else {
        return Vec::new();
    };
    if items.is_empty() {
        return vec![*last];
    }
    items
        .iter()
        .flat_map(|segment| split_top_level(segment, list_comma))
        .chain(std::iter::once(*last))
        .collect()
}

/// A clause as listed: trimmed, without the `.` that ended its sentence.
fn tidy(clause: &str) -> String {
    let clause = clause.trim();
    clause.strip_suffix('.').unwrap_or(clause).trim().to_owned()
}

/// The clauses of an acceptance criterion, in order, for T4. Deterministic
/// code, no Jev.
///
/// The text is cut at every `;` and every sentence end (`. ` before an
/// uppercase letter or a backtick), then each piece at a top-level `, and `
/// into its conjuncts, together with the comma-separated items before it in
/// the same serial list; a `; and` joins the piece before it the same way.
/// Nothing inside backticks or parentheses is a cut point. Pieces are
/// trimmed and empty ones dropped; past [`MAX_CLAUSES`] the overflow joins
/// the last. A criterion with no cut point is one clause.
pub(crate) fn criterion_clauses(text: &str) -> Vec<String> {
    let mut clauses: Vec<String> = Vec::new();
    for sentence in split_top_level(text, sentence_end) {
        let mut pieces: Vec<Vec<&str>> = Vec::new();
        for piece in split_top_level(sentence, semicolon) {
            let piece = piece.trim();
            if let Some(rest) = piece.strip_prefix("and ") {
                // `A, B; and C`: the piece before is the list's other items.
                if let Some(before) = pieces.last_mut()
                    && let [whole] = before.as_slice()
                {
                    *before = split_top_level(whole, list_comma);
                }
                pieces.push(conjuncts(rest));
            } else {
                pieces.push(conjuncts(piece));
            }
        }
        clauses.extend(
            pieces
                .into_iter()
                .flatten()
                .map(tidy)
                .filter(|clause| !clause.is_empty()),
        );
    }
    if clauses.len() > MAX_CLAUSES {
        let overflow = clauses.split_off(MAX_CLAUSES - 1).join("; ");
        clauses.push(overflow);
    }
    clauses
}

/// The clauses T4 lists for `row`: [`criterion_clauses`] over its
/// acceptance criterion, or its statement when it has none.
pub(crate) fn row_clauses(row: &Row) -> Vec<String> {
    let requirement = &row.requirement;
    let text = requirement
        .ac_text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or(&requirement.statement);
    criterion_clauses(text)
}

/// The clause labels: `C1` .. `Cn`.
pub(crate) fn clause_label(index: usize) -> String {
    format!("C{}", index + 1)
}

/// T4's questions for `count` clauses: one `noul` per clause, keyed by its
/// label (`C1` .. `Cn`).
pub(crate) fn clause_questions(count: usize) -> Questions {
    questions((0..count).map(|index| {
        let label = clause_label(index);
        let question = noul_with(
            format!(
                "Judge exactly one claim about clause {label} in `{CLAUSES_FIELD}`, a part of \
                 the acceptance criterion in `ac_text`. The claim: at least one assertion in \
                 `{ASSERTIONS_FIELD}`, alone or together with the others, checks the outcome \
                 clause {label} states, strictly enough that the test would fail if the system \
                 did not produce that outcome. Answer no when no assertion addresses this \
                 clause; when the assertions that address it check only that a call completes, \
                 a presence, non-empty or bound where the clause states an exact value, or a \
                 value the test set up itself; or when they check a different case than the \
                 clause names."
            ),
            NoulCriteria {
                yes: Some(
                    format!(
                        "Some assertion would fail if clause {label}'s outcome did not happen."
                    )
                    .into(),
                ),
                no: Some(
                    format!("No assertion would fail if clause {label}'s outcome did not happen.")
                        .into(),
                ),
            },
        );
        (label, question)
    }))
}

/// `(label, text)` pairs as a JSON object, `{label: text, ..}`.
fn labelled(items: &[String], label: fn(usize) -> String) -> serde_json::Value {
    serde_json::Value::Object(
        items
            .iter()
            .enumerate()
            .map(|(index, text)| (label(index), text.clone().into()))
            .collect(),
    )
}

/// One ask carrying T3's state (the assertions in [`ASSERTIONS_FIELD`]) and
/// the clauses in [`CLAUSES_FIELD`], or none when the test has no assertion:
/// that row is `no` in code, as in T3.
fn t4_asks(row: &Row) -> Vec<Ask> {
    let assertions = row_assertions(row);
    let clauses = row_clauses(row);
    if assertions.is_empty() || clauses.is_empty() {
        return Vec::new();
    }
    let mut row_state = state(row);
    if let serde_json::Value::Object(fields) = &mut row_state {
        fields.insert(
            ASSERTIONS_FIELD.to_owned(),
            labelled(&assertions, assertion_label),
        );
        fields.insert(CLAUSES_FIELD.to_owned(), labelled(&clauses, clause_label));
    }
    vec![Ask {
        unit: None,
        request: request(row_state, clause_questions(clauses.len())),
    }]
}

/// T4's derive rule over `count` asked clauses and the one response:
/// `P(yes)` is the lowest `P(Ck)` over the clauses, `test_asserts_intent`
/// is `yes` iff `P(yes) >= TAU`, the confidence is `P(yes)` for `yes` and
/// `1 - P(yes)` for `no`, and `P(yes)` is the ordinal Bar D reads. With
/// `count == 0` nothing was asked: `no`, `P(yes) = 0`, confidence 1.
///
/// # Errors
/// When a clause's answer is missing, not a `noul`, or not a probability in
/// `[0, 1]`, or when the response answers a label that was not asked.
pub(crate) fn derive_clause_coverage(
    count: usize,
    answers: &RawAnswers,
) -> Result<Prediction, String> {
    let labels: Vec<String> = (0..count).map(clause_label).collect();
    if let Some(stray) = answers.keys().find(|key| !labels.contains(key)) {
        return Err(format!("`{stray}`: answered, but no such clause was asked"));
    }
    let mut p_all = if count == 0 { 0.0f64 } else { 1.0f64 };
    for label in &labels {
        p_all = p_all.min(noul_probability(answers, label)?);
    }
    let yes = count > 0 && p_all >= TAU;
    Ok(Prediction {
        answer: if yes { YES } else { NO }.to_owned(),
        confidence: Some(if yes { p_all } else { 1.0 - p_all }),
        ordinal: Some(p_all),
    })
}

/// How many clauses T4 asked about for `row`: none when its test has no
/// assertion.
fn t4_count(row: &Row) -> usize {
    if row_assertions(row).is_empty() {
        0
    } else {
        row_clauses(row).len()
    }
}

fn t4_derive(row: &Row, answered: &[Answered]) -> Predictions {
    let prediction = loudly(
        row,
        derive_clause_coverage(t4_count(row), &whole_row(answered)),
    );
    Predictions::from([(TEST_ASSERTS_INTENT, prediction)])
}

/// T4: code cuts the criterion into clauses ([`criterion_clauses`]) and
/// lists the test's assertions as T3 does; Jev judges, per clause, whether
/// some assertion, alone or with the others, would fail if that clause's
/// outcome did not happen. `yes` iff the lowest `P(Ck) >= 0.5`, so every
/// clause must be covered, and a clause may be covered by any assertion.
pub(crate) const T4: Variant = Variant {
    id: "T4",
    version: 1,
    summary: "clause coverage: code cuts the criterion into clauses, one noul per clause (would \
              some assertion fail without its outcome); yes iff min P >= 0.5",
    modes: TEST_MODES,
    references: &[Artifact::Test],
    grades: &[TEST_ASSERTS_INTENT],
    asks: t4_asks,
    derive: t4_derive,
};

// ---------------------------------------------------------------------------
// T0-RT: the baseline on requirement-plus-test rows
// ---------------------------------------------------------------------------

/// The one edit that makes `FullBatteryV1`'s `test_asserts_intent` question
/// usable with no code: "the code" (twice) becomes "the system". Nothing else
/// changes.
pub(crate) const T0_RT_EDIT: (&str, &str) = ("the code", "the system");

/// `FullBatteryV1`'s `test_asserts_intent` instruction, read from the battery
/// itself so this baseline cannot drift from T0's wording.
///
/// # Panics
/// When the battery has no such `noul` or its instruction is not a string:
/// T0-RT must never be sent with an empty question.
pub(crate) fn t0_instruction() -> String {
    let battery = battery_questions(BatteryShape::FullBatteryV1);
    match battery.get(TEST_ASSERTS_INTENT) {
        Some(Question::Noul {
            instructions: Some(entry),
            ..
        }) => match entry.0.as_str() {
            Some(text) => text.to_owned(),
            None => panic!("FullBatteryV1's `{TEST_ASSERTS_INTENT}` instruction is not a string"),
        },
        other => {
            panic!("FullBatteryV1 has no `{TEST_ASSERTS_INTENT}` noul with instructions: {other:?}")
        }
    }
}

/// T0-RT's instruction: T0's with [`T0_RT_EDIT`] applied.
pub(crate) fn t0_rt_instruction() -> String {
    t0_instruction().replace(T0_RT_EDIT.0, T0_RT_EDIT.1)
}

fn t0_rt_asks(row: &Row) -> Vec<Ask> {
    vec![Ask {
        unit: None,
        request: request(
            state(row),
            questions([(TEST_ASSERTS_INTENT, noul(t0_rt_instruction()))]),
        ),
    }]
}

fn t0_rt_derive(row: &Row, answered: &[Answered]) -> Predictions {
    let answers = whole_row(answered);
    loudly(row, noul_probability(&answers, TEST_ASSERTS_INTENT));
    noul_prediction(&answers, TEST_ASSERTS_INTENT)
        .map(|prediction| Predictions::from([(TEST_ASSERTS_INTENT, prediction)]))
        .unwrap_or_default()
}

/// T0 on requirement-plus-test rows: the RT comparator for T1 and T2
/// (MP-242). Its wording differs from T0 only by "the code" -> "the system",
/// but its context differs too: T0 asks the question inside the seven-question
/// `FullBatteryV1` request, T0-RT asks it alone. A T0 vs T0-RT difference is
/// wording and context together.
pub(crate) const T0_RT: Variant = Variant {
    id: "T0-RT",
    version: 1,
    summary: "FullBatteryV1's test_asserts_intent question alone, \"the code\" -> \"the system\" (RT)",
    modes: &[Mode::ReqTest],
    references: &[Artifact::Test],
    grades: &[TEST_ASSERTS_INTENT],
    asks: t0_rt_asks,
    derive: t0_rt_derive,
};

/// The baselines T1 and T2 are paired with: T0 on RTC rows, T0-RT on RT rows.
const COMPARATORS: [Variant; 2] = [T0, T0_RT];

// ---------------------------------------------------------------------------
// The gating wrapper
// ---------------------------------------------------------------------------

/// Which side of the trace check a row fell on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Gate {
    /// The trace was judged correct.
    TraceRight,
    /// The trace was judged wrong.
    TraceWrong,
    /// The row's `trace_correct` label is contested: two agent labels
    /// disagreed. Only the label split has this slice.
    ContestedLabel,
    /// No trace judgment for this row.
    NoTraceJudgment,
}

impl Gate {
    /// The slices of the split by TC's answers, in report order.
    pub(crate) const BY_TC: [Self; 3] = [Self::TraceRight, Self::TraceWrong, Self::NoTraceJudgment];

    /// The slices of the split by the rows' own label, in report order.
    pub(crate) const BY_LABEL: [Self; 4] = [
        Self::TraceRight,
        Self::TraceWrong,
        Self::ContestedLabel,
        Self::NoTraceJudgment,
    ];

    /// The report's name for the gate.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::TraceRight => "trace right",
            Self::TraceWrong => "trace wrong",
            Self::ContestedLabel => "contested",
            Self::NoTraceJudgment => "no trace judgment",
        }
    }

    fn from_label(label: &str) -> Self {
        match label {
            YES => Self::TraceRight,
            NO => Self::TraceWrong,
            _ => Self::NoTraceJudgment,
        }
    }
}

/// Each row's gate from TC's answers in `output`, by row id. A row TC did not
/// run on, or left unanswered, is absent (read as [`Gate::NoTraceJudgment`]).
pub(crate) fn tc_gates(output: &RunOutput) -> BTreeMap<String, Gate> {
    let labels: Vec<String> = TC_FAMILY.iter().map(Variant::label).collect();
    output
        .results
        .iter()
        .filter(|result| labels.contains(&result.variant))
        .filter_map(|result| {
            let prediction = result.predictions.get(TRACE_CORRECT)?;
            Some((result.row_id.clone(), Gate::from_label(&prediction.answer)))
        })
        .collect()
}

/// Each row's gate from its own `trace_correct` label, by row id. A contested
/// label is its own slice. The label is whatever kind of truth the corpus
/// recorded, often agent-written, so this split is not ground truth.
pub(crate) fn truth_gates(rows: &[Row]) -> BTreeMap<String, Gate> {
    rows.iter()
        .filter_map(|row| {
            let truth = row.truth.get(TRACE_CORRECT)?;
            let gate = if truth.kind == TruthKind::AgentContested {
                Gate::ContestedLabel
            } else {
                Gate::from_label(&truth.answer.label())
            };
            Some((row.id.clone(), gate))
        })
        .collect()
}

/// The name of the label split, saying how much of it is agent-written.
fn label_split_name(rows: &[Row]) -> String {
    let kinds: Vec<KindGroup> = rows
        .iter()
        .filter_map(|row| row.truth.get(TRACE_CORRECT))
        .map(|truth| truth.kind.group())
        .collect();
    let agent = kinds
        .iter()
        .filter(|kind| **kind == KindGroup::Agent)
        .count();
    if agent == 0 {
        "trace_correct label".to_owned()
    } else if agent == kinds.len() {
        "trace_correct label [AGENT-LABELLED]".to_owned()
    } else {
        format!("trace_correct label (incl. {agent} AGENT-LABELLED)")
    }
}

/// `rows` split by `gates` into `slices`, every slice present even when
/// empty.
pub(crate) fn split_by_gate(
    rows: &[Scored],
    gates: &BTreeMap<String, Gate>,
    slices: &[Gate],
) -> BTreeMap<Gate, Vec<Scored>> {
    let mut out: BTreeMap<Gate, Vec<Scored>> =
        slices.iter().map(|gate| (*gate, Vec::new())).collect();
    for row in rows {
        let gate = gates
            .get(&row.row_id)
            .copied()
            .unwrap_or(Gate::NoTraceJudgment);
        out.entry(gate).or_default().push(row.clone());
    }
    out
}

/// The graded rows MP-242 counts: a mutant row whose truth for the key is
/// agent-written (inherited from its source) never counts, so agent-labelled
/// numbers come from natural rows only.
pub(crate) fn counted(rows: &[Row], graded: Vec<Scored>) -> Vec<Scored> {
    let mutants: Vec<&str> = rows
        .iter()
        .filter(|row| row.mutation.is_some())
        .map(|row| row.id.as_str())
        .collect();
    graded
        .into_iter()
        .filter(|row| {
            !(row.kind.group() == KindGroup::Agent && mutants.contains(&row.row_id.as_str()))
        })
        .collect()
}

fn is_trace_check(variant: &Variant) -> bool {
    TC_FAMILY.iter().any(|tc| tc.id == variant.id)
}

/// MP-242's own variants: a live run that selects any of them must record a
/// cassette. T0 is not one of them. It is PLAT-1027's shared baseline, which
/// other experiments run too, so a run of T0 without any of these needs no
/// cassette (PR #623 re-review L3).
pub(crate) const MP_242: [Variant; 8] = [TC_RT, TC_RC, TC_RTC, T0_RT, T1, T2, T3, T4];

/// Refuses a live run of an MP-242 variant with no cassette. A malformed
/// response stops the run (see [`loudly`]); with a recording cassette every
/// answer already received is on disk, so the rerun replays it instead of
/// paying for it again.
///
/// # Errors
/// When `variants` holds one of [`MP_242`] and `cassette` is `None`.
pub(crate) fn require_cassette(
    variants: &[&Variant],
    cassette: Option<&str>,
) -> Result<(), String> {
    let mine: Vec<String> = variants
        .iter()
        .filter(|variant| MP_242.iter().any(|v| v.id == variant.id))
        .map(|variant| variant.label())
        .collect();
    if mine.is_empty() || cassette.is_some() {
        return Ok(());
    }
    Err(format!(
        "{mine:?} are MP-242 variants and must run with a recording cassette: set \
         QUOIN_JEV_CASSETTE and QUOIN_JEV_MODEL, so a rerun after a stopped run replays what \
         was already answered"
    ))
}

/// One way of splitting the report: each row's gate, the slices to print,
/// and the split's name.
struct Split<'a> {
    gates: &'a BTreeMap<String, Gate>,
    slices: &'a [Gate],
    by: &'a str,
}

fn render_split(out: &mut String, label: &str, key: &str, rows: &[Scored], split: &Split<'_>) {
    let by = split.by;
    for (gate, slice) in split_by_gate(rows, split.gates, split.slices) {
        let _ = writeln!(
            out,
            "\n{} of {} rows: {by} says {}",
            slice.len(),
            rows.len(),
            gate.label()
        );
        if !slice.is_empty() {
            let _ = write!(
                out,
                "{}",
                render(&format!("{label} | {by}: {}", gate.label()), key, &slice)
            );
        }
    }
}

/// Two variants compared on the rows both answered (MP-242 rule 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Paired {
    /// Rows both were run on.
    pub(crate) shared: usize,
    /// Of those, rows both answered: the comparison's population.
    pub(crate) both_answered: usize,
    /// On `both_answered`, rows the first variant agreed with truth on.
    pub(crate) left_correct: usize,
    /// On `both_answered`, rows the second variant agreed with truth on.
    pub(crate) right_correct: usize,
    /// Of `shared`, rows the first variant abstained on.
    pub(crate) left_abstained: usize,
    /// Of `shared`, rows the second variant abstained on.
    pub(crate) right_abstained: usize,
}

fn agrees(row: &Scored) -> Option<bool> {
    row.prediction.as_ref().map(|prediction| {
        prediction.answer == row.expected || row.alternatives.contains(&prediction.answer)
    })
}

/// `left` against `right`, matched by row id, over the rows both answered.
pub(crate) fn paired(left: &[Scored], right: &[Scored]) -> Paired {
    let right: BTreeMap<&str, &Scored> =
        right.iter().map(|row| (row.row_id.as_str(), row)).collect();
    let mut out = Paired::default();
    for row in left {
        let Some(other) = right.get(row.row_id.as_str()) else {
            continue;
        };
        out.shared += 1;
        match (agrees(row), agrees(other)) {
            (Some(l), Some(r)) => {
                out.both_answered += 1;
                out.left_correct += usize::from(l);
                out.right_correct += usize::from(r);
            }
            (l, r) => {
                out.left_abstained += usize::from(l.is_none());
                out.right_abstained += usize::from(r.is_none());
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Bar D: the paired mutant contrast
// ---------------------------------------------------------------------------

/// Bar D's minimum move in `P(yes)` between source and mutant. Fixed in
/// MP-242 before any dev data.
pub(crate) const BAR_D_DELTA: f64 = 0.10;
/// Bar D's one-sided sign-test level.
pub(crate) const BAR_D_ALPHA: f64 = 0.05;
/// The fewest non-tie pairs Bar D is gated on.
pub(crate) const BAR_D_MIN_PAIRS: usize = 10;
/// Slack for comparing a move with [`BAR_D_DELTA`]: `0.9 - 0.8` is
/// `0.0999…` in `f64`, and must count as a move of 0.10.
const MOVE_SLACK: f64 = 1e-9;

/// The mutation kind whose mutants Bar D pairs for `key`: test-weakening
/// for `test_asserts_intent`, trace swaps for `trace_correct`. MP-242 names
/// no other.
pub(crate) fn bar_d_mutation_kind(key: &str) -> Option<&'static str> {
    match key {
        TEST_ASSERTS_INTENT => Some("test_weakening"),
        TRACE_CORRECT => Some("trace_swap"),
        _ => None,
    }
}

/// How one source/mutant pair came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairOutcome {
    /// `P(yes)` went from the `yes` side of [`TAU`] to the `no` side and
    /// fell by at least [`BAR_D_DELTA`].
    Success,
    /// `P(yes)` rose by at least [`BAR_D_DELTA`], or the variant abstained on
    /// either row.
    Failure,
    /// Anything else. Excluded from the sign test.
    Tie,
}

/// One pair's outcome from the two rows' `P(yes)`; `None` is an abstention.
pub(crate) fn pair_outcome(source: Option<f64>, mutant: Option<f64>) -> PairOutcome {
    let (Some(source), Some(mutant)) = (source, mutant) else {
        return PairOutcome::Failure;
    };
    let fell = source - mutant;
    if source >= TAU && mutant < TAU && fell + MOVE_SLACK >= BAR_D_DELTA {
        PairOutcome::Success
    } else if -fell + MOVE_SLACK >= BAR_D_DELTA {
        PairOutcome::Failure
    } else {
        PairOutcome::Tie
    }
}

/// The one-sided sign-test p-value: the chance of at least `successes`
/// successes in `successes + failures` fair coin flips.
pub(crate) fn sign_test_p(successes: usize, failures: usize) -> f64 {
    let n = successes + failures;
    // C(n, k) / 2^n, built up term by term so no factorial overflows.
    let mut term = 0.5_f64.powi(i32::try_from(n).unwrap_or(i32::MAX));
    let mut tail = 0.0;
    for k in 0..=n {
        if k >= successes {
            tail += term;
        }
        term *= to_f64(n - k) / to_f64(k + 1);
    }
    tail.min(1.0)
}

/// A pair count as `f64`. Counts here are corpus rows, far below `u32::MAX`.
fn to_f64(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Bar D for one variant (or the pooled TC family) on one key.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BarD {
    /// Mutations paired: one pair per mutation id.
    pub(crate) pairs: usize,
    /// Of `pairs`, successes.
    pub(crate) successes: usize,
    /// Of `pairs`, failures (abstentions included).
    pub(crate) failures: usize,
    /// Of `pairs`, ties: excluded from the test.
    pub(crate) ties: usize,
    /// Mutations left out because the source row is not labelled `yes` for
    /// the key: already on the defect side, or unlabelled.
    pub(crate) source_not_yes: usize,
    /// Mutations left out because the variant never ran on the source row's
    /// mode, or the mutant names no source.
    pub(crate) unpairable: usize,
    /// The one-sided sign-test p-value over the non-tie pairs.
    pub(crate) p_value: f64,
}

impl BarD {
    /// Non-tie pairs: the sign test's population.
    pub(crate) const fn non_ties(&self) -> usize {
        self.successes + self.failures
    }

    /// Whether D can be gated on: at least [`BAR_D_MIN_PAIRS`] non-ties.
    pub(crate) const fn gateable(&self) -> bool {
        self.non_ties() >= BAR_D_MIN_PAIRS
    }

    /// Whether D passes: gateable, and the sign test is significant at
    /// [`BAR_D_ALPHA`].
    pub(crate) fn passes(&self) -> bool {
        self.gateable() && self.p_value <= BAR_D_ALPHA
    }
}

/// Bar D for `variants` (one variant, or the TC family pooled) on `key`,
/// over `rows` and a run's `output`.
///
/// Pairs come only from `mutation.source_id`: each mutation whose kind is
/// [`bar_d_mutation_kind`] gives one pair, its RTC row when the variants ran
/// in RTC, otherwise its first row in a mode they ran in, paired with the
/// source row. A source not labelled `yes` for `key` is left out. `P(yes)` is
/// each prediction's `ordinal`.
///
/// # Errors
/// When a mutant names a source that is not among `rows`, or that is itself
/// a mutant: the corpus or the split filter is wrong, and D would silently
/// shrink.
pub(crate) fn bar_d(
    rows: &[Row],
    output: &RunOutput,
    variants: &[&Variant],
    key: &str,
) -> Result<BarD, String> {
    let Some(kind) = bar_d_mutation_kind(key) else {
        return Ok(BarD::default());
    };
    let labels: Vec<String> = variants.iter().map(|variant| variant.label()).collect();
    let runs_in = |mode: Mode| variants.iter().any(|variant| variant.modes.contains(&mode));
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    // Row id to `P(yes)`, `None` for an abstention; absent = never ran.
    let mut p_yes: BTreeMap<&str, Option<f64>> = BTreeMap::new();
    for result in output
        .results
        .iter()
        .filter(|r| labels.contains(&r.variant))
    {
        let p = result
            .predictions
            .get(key)
            .and_then(|prediction| prediction.ordinal);
        p_yes.insert(result.row_id.as_str(), p);
    }
    // Mutation id to its chosen mutant row: RTC first, else corpus order.
    let mut chosen: BTreeMap<&str, &Row> = BTreeMap::new();
    for row in rows.iter().filter(|row| runs_in(row.mode)) {
        let Some(mutation) = row.mutation.as_ref().filter(|m| m.kind == kind) else {
            continue;
        };
        let slot = chosen.entry(mutation.id.as_str()).or_insert(row);
        if slot.mode != Mode::ReqTestCode && row.mode == Mode::ReqTestCode {
            *slot = row;
        }
    }
    let mut d = BarD::default();
    for mutant in chosen.values() {
        let Some(source_id) = mutant
            .mutation
            .as_ref()
            .and_then(|m| m.source_id.as_deref())
        else {
            d.unpairable += 1;
            continue;
        };
        let source = by_id.get(source_id).ok_or_else(|| {
            format!(
                "{}: source {source_id} is not among the rows run",
                mutant.id
            )
        })?;
        if source.mutation.is_some() {
            return Err(format!(
                "{}: source {source_id} is itself a mutant",
                mutant.id
            ));
        }
        if !runs_in(source.mode) {
            d.unpairable += 1;
            continue;
        }
        let labelled_yes = source
            .truth
            .get(key)
            .is_some_and(|truth| truth.answer.label() == YES);
        if !labelled_yes {
            d.source_not_yes += 1;
            continue;
        }
        let read = |id: &str| p_yes.get(id).copied().flatten();
        d.pairs += 1;
        match pair_outcome(read(&source.id), read(&mutant.id)) {
            PairOutcome::Success => d.successes += 1,
            PairOutcome::Failure => d.failures += 1,
            PairOutcome::Tie => d.ties += 1,
        }
    }
    d.p_value = sign_test_p(d.successes, d.failures);
    Ok(d)
}

fn render_bar_d(out: &mut String, name: &str, key: &str, d: &BarD) {
    let verdict = if !d.gateable() {
        format!("not gateable (fewer than {BAR_D_MIN_PAIRS} non-tie pairs)")
    } else if d.passes() {
        "passes".to_owned()
    } else {
        "fails".to_owned()
    };
    let _ = writeln!(
        out,
        "\nBar D, {name} on `{key}`: {} pairs, {} successes, {} failures, {} ties; one-sided sign \
         test p = {:.4}; {verdict}. Left out: {} with the source not labelled yes, {} unpairable.",
        d.pairs, d.successes, d.failures, d.ties, d.p_value, d.source_not_yes, d.unpairable,
    );
}

/// The TC family's pooled `trace_correct` rows in `output`, [`counted`]: one
/// table covers RT, RC and RTC, and its per-mode lines are each TC variant.
pub(crate) fn tc_counted(rows: &[Row], output: &RunOutput, tc: &[&Variant]) -> Vec<Scored> {
    let pooled: Vec<Scored> = tc
        .iter()
        .flat_map(|variant| scored(rows, output, &variant.label(), TRACE_CORRECT))
        .collect();
    counted(rows, pooled)
}

/// MP-242's section of the run report. Only [`counted`] rows are reported
/// anywhere in it.
///
/// Every non-TC variant in `variants`, for each key it grades: its pooled,
/// unsplit table (Bars A and B are judged on it), its paired line against
/// the baseline, its Bar D, then the same rows split by TC's answers (when TC
/// is in the run) and by the rows' own `trace_correct` label, contested
/// labels apart (whenever any row carries one). Then TC's pooled table over
/// RT, RC and RTC, with a line per mode (Bar C is judged on it), and TC's
/// pooled Bar D. Appended to the run report by `live_eval_v2.rs`.
///
/// # Panics
/// When Bar D finds a mutant whose source is missing or is itself a mutant.
pub(crate) fn render_gated_run(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n## Split by trace (PLAT-1030, MP-242)");
    let with_tc = variants.iter().any(|variant| is_trace_check(variant));
    if !with_tc {
        let _ = writeln!(
            out,
            "\nNo TC variant in this run, so there is no split by TC's answers. Add TC-RT, TC-RC \
             and TC-RTC to QUOIN_JEV_VARIANTS."
        );
    }
    let _ = writeln!(
        out,
        "\nMutant rows whose label is agent-written (inherited from the source row) are left out \
         of every table below."
    );
    let by_tc = tc_gates(output);
    let by_truth = truth_gates(rows);
    let label_split = label_split_name(rows);
    for variant in variants.iter().filter(|variant| !is_trace_check(variant)) {
        let label = variant.label();
        for spec in KEYS
            .iter()
            .filter(|spec| variant.grades.contains(&spec.key) && spec.key != TRACE_CORRECT)
        {
            let graded = counted(rows, scored(rows, output, &label, spec.key));
            if graded.is_empty() {
                continue;
            }
            let _ = writeln!(out, "\n### {label} — `{}`, split by trace", spec.key);
            let abstained = graded.iter().filter(|row| row.prediction.is_none()).count();
            let _ = writeln!(
                out,
                "\n{abstained} of {} rows abstained (no answer; kept in every denominator)",
                graded.len()
            );
            let _ = write!(
                out,
                "{}",
                render(&format!("{label} | counted, pooled"), spec.key, &graded)
            );
            if !COMPARATORS.iter().any(|c| c.id == variant.id) {
                let line = render_paired(rows, output, variants, &label, spec.key, &graded);
                out.push_str(&line);
            }
            if bar_d_mutation_kind(spec.key).is_some() {
                let d = bar_d(rows, output, &[*variant], spec.key)
                    .unwrap_or_else(|error| panic!("Bar D: {error}"));
                render_bar_d(&mut out, &label, spec.key, &d);
            }
            if with_tc {
                let split = Split {
                    gates: &by_tc,
                    slices: &Gate::BY_TC,
                    by: "TC",
                };
                render_split(&mut out, &label, spec.key, &graded, &split);
            }
            if !by_truth.is_empty() {
                let split = Split {
                    gates: &by_truth,
                    slices: &Gate::BY_LABEL,
                    by: &label_split,
                };
                render_split(&mut out, &label, spec.key, &graded, &split);
            }
        }
    }
    let tc: Vec<&Variant> = variants
        .iter()
        .copied()
        .filter(|variant| is_trace_check(variant))
        .collect();
    render_tc(&mut out, rows, output, &tc);
    out
}

/// `label`'s counted rows on `key`, paired with whichever of the baselines
/// in [`COMPARATORS`] ran, on the rows both answered. Nothing when no
/// baseline ran or no row is shared.
fn render_paired(
    rows: &[Row],
    output: &RunOutput,
    variants: &[&Variant],
    label: &str,
    key: &str,
    graded: &[Scored],
) -> String {
    let mut out = String::new();
    let comparators: Vec<&Variant> = COMPARATORS
        .iter()
        .filter(|comparator| variants.iter().any(|v| v.id == comparator.id))
        .collect();
    let names: Vec<String> = comparators.iter().map(|c| c.label()).collect();
    let names = names.join(" + ");
    let baseline: Vec<Scored> = comparators
        .iter()
        .flat_map(|c| counted(rows, scored(rows, output, &c.label(), key)))
        .collect();
    let p = paired(graded, &baseline);
    if p.shared > 0 {
        let _ = writeln!(
            out,
            "\nPaired with {names}: {} rows shared, {} answered by both; on those, {label} {} \
             correct, baseline {} correct; abstained {label} {}, baseline {}",
            p.shared,
            p.both_answered,
            p.left_correct,
            p.right_correct,
            p.left_abstained,
            p.right_abstained,
        );
    }
    out
}

/// TC's pooled counted table over its modes (Bar C) and its pooled Bar D.
/// Nothing when no TC variant ran.
fn render_tc(out: &mut String, rows: &[Row], output: &RunOutput, tc: &[&Variant]) {
    if tc.is_empty() {
        return;
    }
    let names: Vec<String> = tc.iter().map(|variant| variant.label()).collect();
    let names = names.join(" + ");
    let _ = writeln!(
        out,
        "\n### TC — `{TRACE_CORRECT}`, pooled over its modes\n\nOne instrument: {names}. The \
         per-mode lines are each mode's own TC variant."
    );
    let _ = write!(
        out,
        "{}",
        render(
            &format!("{names} | counted, pooled"),
            TRACE_CORRECT,
            &tc_counted(rows, output, tc)
        )
    );
    let d = bar_d(rows, output, tc, TRACE_CORRECT).unwrap_or_else(|error| panic!("Bar D: {error}"));
    render_bar_d(out, &names, TRACE_CORRECT, &d);
}

// ---------------------------------------------------------------------------
// Per-row dump (QUOIN_JEV_INTENT_OUT)
// ---------------------------------------------------------------------------

/// One JSON line per row per variant in `variants` that grades
/// `test_asserts_intent` (the T variants), for reading rows by hand: the
/// variant's label, the row's id and mode, the assertions T3 or T4 listed
/// with each one's `P` (empty for a variant that lists none, and `p` null
/// for an assertion that was not answered, as every T4 one is), the clauses
/// T4 asked about with each one's `P` (empty for every other variant, and
/// for a T4 row asked nothing), the derived prediction (`answer`,
/// `confidence`, `p_yes`; null for an abstention), and the row's truth label
/// (null when it has none).
pub(crate) fn dump(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> Vec<String> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut lines = Vec::new();
    for variant in variants
        .iter()
        .filter(|variant| variant.grades.contains(&TEST_ASSERTS_INTENT))
    {
        let label = variant.label();
        for result in output.results.iter().filter(|r| r.variant == label) {
            let Some(row) = by_id.get(result.row_id.as_str()) else {
                continue;
            };
            lines.push(intent_line(variant, row, result).to_string());
        }
    }
    lines
}

/// One row's line for [`dump`].
fn intent_line(variant: &Variant, row: &Row, result: &RowResult) -> Value {
    let answers = whole_row(&result.answered);
    let listed = |items: Vec<String>, label: fn(usize) -> String| -> Vec<Value> {
        items
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                let label = label(index);
                let p = match answers.get(&label) {
                    Some(RawAnswer::Noul(p)) => Some(*p),
                    _ => None,
                };
                json!({"label": label, "text": text, "p": p})
            })
            .collect()
    };
    let assertions = if variant.id == T3.id || variant.id == T4.id {
        listed(row_assertions(row), assertion_label)
    } else {
        Vec::new()
    };
    let clauses = if variant.id == T4.id && t4_count(row) > 0 {
        listed(row_clauses(row), clause_label)
    } else {
        Vec::new()
    };
    let prediction = result
        .predictions
        .get(TEST_ASSERTS_INTENT)
        .map(|p| json!({"answer": p.answer, "confidence": p.confidence, "p_yes": p.ordinal}));
    let truth = row.truth.get(TEST_ASSERTS_INTENT).map(|truth| {
        json!({
            "answer": truth.answer.label(),
            "kind": truth.kind,
            "alternatives": truth.alternatives.iter().map(TruthAnswer::label).collect::<Vec<_>>(),
        })
    });
    json!({
        "variant": variant.label(),
        "row_id": row.id,
        "mode": row.mode.as_str(),
        "assertions": assertions,
        "clauses": clauses,
        "prediction": prediction,
        "truth": truth,
    })
}
