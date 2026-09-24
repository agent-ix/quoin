// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `test_asserts_intent` T1/T2 and the trace check TC, offline (PLAT-1030).
//!
//! No network and no key: T1's derive rule over hand-built answers, the
//! wording each variant sends per mode, and the TC-gated report end to end
//! through the runner over a scripted fake Jev. The live numbers are
//! MP-242's, from `live_eval_v2.rs`.
//!
//! Provenance: PLAT-1030, PLAT-1024, MP-242.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod eval_v2_support;
mod gap_semantic_support;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use serde_json::{Value, json};
use typesafe_sdk_env::Fixed;
use typesafe_sdk_error::Result as SdkResult;
use typesafe_sdk_headers::Headers;
use typesafe_sdk_http::{RawResponse, Request, Transport};

use eval_v2_support::corpus::{self, CorpusFile, Row, TruthKind};
use eval_v2_support::keys::Mode;
use eval_v2_support::metrics::{Scored, scored};
use eval_v2_support::variant::{
    self, Prediction, RawAnswer, RawAnswers, Variant, wording_violations,
};
use eval_v2_support::variants::intent::{
    self, ASSERTS_OUTCOME_TOO_LOOSELY, ASSERTS_REQUIRED_OUTCOME, CANNOT_TELL, CHECK_KIND,
    EXERCISES_CRITERION, Gate, Paired, T0_RT, T0_RT_EDIT, T1, T2, TC_FAMILY, TC_RC, TC_RT, TC_RTC,
    derive_asserts_intent, derive_trace, paired, render_gated_run, split_by_gate, tc_gates,
    trace_question,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const TEST_BODY: &str = "#[test]\nfn tc_001_refuses_oversize() {\n    \
    let error = check(\"x\".repeat(5000)).unwrap_err();\n    \
    assert_eq!(error.kind, Kind::Refused);\n}";

const CODE_BODY: &str = "pub fn check(input: String) -> Result<(), Error> {\n    \
    if input.len() > 4096 { return Err(Error::refused()); }\n    Ok(())\n}";

/// One synthetic row of `mode`; its FR is `FR-` plus the id's last three
/// digits, which is also how the scripted fake tells rows apart.
fn row(id: &str, mode: Mode, truth: &Value) -> Value {
    let fr = format!("FR-{}", &id[id.len() - 3..]);
    let test = mode.has_test().then(|| {
        json!({"path": "tests/tc_001.rs", "fn_name": "tc_001_refuses_oversize", "body": TEST_BODY})
    });
    let code = mode
        .has_code()
        .then(|| json!({"path": "src/check.rs", "symbol": "check", "body": CODE_BODY}));
    json!({
        "id": id,
        "mode": mode.as_str(),
        "split": "dev",
        "strata": {"fr_id": fr, "req_kind": "functional", "test_kind": null,
                   "crate": "quoin-core", "ears_pattern": null},
        "requirement": {"fr_id": fr, "ac_id": format!("{fr}-AC-1"),
                        "statement": "The system shall refuse a request larger than 4096 bytes.",
                        "ac_text": "A 5000-byte request is refused.",
                        "context": null},
        "test": test,
        "code": code,
        "ref": null,
        "mutation": null,
        "truth": truth,
    })
}

fn truth(answer: bool) -> Value {
    json!({"answer": answer, "kind": "agent_dual", "alternatives": [], "rationale": "stated"})
}

fn parse(rows: &[Value]) -> CorpusFile {
    let text = serde_json::to_string(&json!({
        "schema": corpus::SCHEMA,
        "sampling_rule": "every row, synthetic",
        "seed": 7,
        "source_commit": "0000000",
        "rows": rows,
    }))
    .unwrap();
    corpus::parse(&text).expect("synthetic corpus parses")
}

/// Four rows: two RT, one RTC, one RC. Truth, and what the fake answers:
///
/// | row | mode | trace truth | intent truth | TC p | exercises p | `check_kind` |
/// | --- | --- | --- | --- | --- | --- | --- |
/// | 101 | RT | yes | yes | 0.9 | 0.8 | `asserts_required_outcome` |
/// | 102 | RT | no | no | 0.1 | 0.2 | `asserts_unrelated` |
/// | 103 | RTC | yes | no | 0.7 | 0.9 | `asserts_only_that_it_runs` |
/// | 104 | RC | no | - | 0.3 | - | - |
fn corpus_rows() -> Vec<Row> {
    let both = |trace: bool, intent: bool| json!({"trace_correct": truth(trace), "test_asserts_intent": truth(intent)});
    parse(&[
        row("EV2-0101", Mode::ReqTest, &both(true, true)),
        row("EV2-0102", Mode::ReqTest, &both(false, false)),
        row("EV2-0103", Mode::ReqTestCode, &both(true, false)),
        row(
            "EV2-0104",
            Mode::ReqCode,
            &json!({"trace_correct": truth(false)}),
        ),
    ])
    .rows
}

fn noul_answer(p: f64) -> Value {
    json!({"type": "noul", "noul": p})
}

/// A `choice` answer with `label` at `p` and, for any other label,
/// `asserts_required_outcome` at `1 - p`.
fn choice_answer(label: &str, p: f64) -> Value {
    let mut probabilities = serde_json::Map::new();
    probabilities.insert(label.to_owned(), json!(p));
    if label != ASSERTS_REQUIRED_OUTCOME {
        probabilities.insert(ASSERTS_REQUIRED_OUTCOME.to_owned(), json!(1.0 - p));
    }
    json!({"type": "choice", "choice": label, "confidence": p, "probabilities": probabilities})
}

/// The fake's answers, by FR then question key. See [`corpus_rows`].
fn script() -> BTreeMap<String, BTreeMap<String, Value>> {
    let entry = |fr: &str, answers: Vec<(&str, Value)>| {
        (
            fr.to_owned(),
            answers
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    };
    BTreeMap::from([
        entry(
            "FR-101",
            vec![
                ("trace_correct", noul_answer(0.9)),
                ("test_asserts_intent", noul_answer(0.8)),
                (EXERCISES_CRITERION, noul_answer(0.8)),
                (CHECK_KIND, choice_answer(ASSERTS_REQUIRED_OUTCOME, 0.7)),
            ],
        ),
        entry(
            "FR-102",
            vec![
                ("trace_correct", noul_answer(0.1)),
                ("test_asserts_intent", noul_answer(0.7)),
                (EXERCISES_CRITERION, noul_answer(0.2)),
                (CHECK_KIND, choice_answer("asserts_unrelated", 0.6)),
            ],
        ),
        entry(
            "FR-103",
            vec![
                ("trace_correct", noul_answer(0.7)),
                (EXERCISES_CRITERION, noul_answer(0.9)),
                (CHECK_KIND, choice_answer("asserts_only_that_it_runs", 0.8)),
            ],
        ),
        entry("FR-104", vec![("trace_correct", noul_answer(0.3))]),
    ])
}

/// A fake Jev that answers each question from [`script`], keyed by the
/// request's `fr_id`. A question the script has no answer for fails the call.
struct ScriptedJev {
    script: BTreeMap<String, BTreeMap<String, Value>>,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for ScriptedJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let fr = body["state"]["fr_id"].as_str().unwrap();
        let answers: serde_json::Map<String, Value> = body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .map(|key| (key.clone(), self.script[fr][key].clone()))
            .collect();
        Ok(RawResponse {
            status: 200,
            headers: Headers::new(),
            body: json!({"model": "jev-fake", "answers": answers,
                         "usage": {"input_tokens": 1, "output_tokens": 1}})
            .to_string(),
        })
    }
}

fn scripted_client() -> (typesafe_sdk_client::Client, Arc<ScriptedJev>) {
    client_with(script())
}

fn client_with(
    script: BTreeMap<String, BTreeMap<String, Value>>,
) -> (typesafe_sdk_client::Client, Arc<ScriptedJev>) {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(ScriptedJev {
        script,
        calls: AtomicUsize::new(0),
    });
    (
        quoin_jev::client::with_transport(config, fake.clone()),
        fake,
    )
}

// ---------------------------------------------------------------------------
// T1's derive rule
// ---------------------------------------------------------------------------

fn split_answers(p_exercises: f64, label: &str, probabilities: &[(&str, f64)]) -> RawAnswers {
    RawAnswers::from([
        (EXERCISES_CRITERION.to_owned(), RawAnswer::Noul(p_exercises)),
        (
            CHECK_KIND.to_owned(),
            RawAnswer::Choice {
                label: label.to_owned(),
                confidence: 0.5,
                probabilities: probabilities
                    .iter()
                    .map(|(label, p)| ((*label).to_owned(), *p))
                    .collect(),
            },
        ),
    ])
}

fn close(actual: Option<f64>, expected: f64) -> bool {
    actual.is_some_and(|actual| (actual - expected).abs() < 1e-9)
}

/// `(P(exercises), chosen label, label probabilities, expected answer,
/// expected confidence)`.
type Case = (
    f64,
    &'static str,
    &'static [(&'static str, f64)],
    &'static str,
    f64,
);

/// Provenance: PLAT-1030, MP-242 (PR #623 review finding 1). `P(yes) =
/// min(P(exercises), P(asserts_required_outcome))`, `yes` iff `P(yes) >=
/// 0.5`, confidence `P(yes)` for `yes` and `1 - P(yes)` for `no`. Each value
/// is worked by hand. The old product rule gave the first case 0.56.
#[test]
fn tc_1030_t1_p_yes_is_the_smaller_half() {
    let cases: [Case; 5] = [
        // min(0.8, 0.7) = 0.7.
        (
            0.8,
            ASSERTS_REQUIRED_OUTCOME,
            &[(ASSERTS_REQUIRED_OUTCOME, 0.7)],
            "yes",
            0.7,
        ),
        // At the threshold exactly: min(0.5, 0.9) = 0.5, `yes`.
        (
            0.5,
            ASSERTS_REQUIRED_OUTCOME,
            &[(ASSERTS_REQUIRED_OUTCOME, 0.9)],
            "yes",
            0.5,
        ),
        // Below it: min(0.49, 0.9) = 0.49, confidence in `no` 0.51.
        (
            0.49,
            ASSERTS_REQUIRED_OUTCOME,
            &[(ASSERTS_REQUIRED_OUTCOME, 0.9)],
            "no",
            0.51,
        ),
        // Exercises, but checks its own setup: min(0.9, 0.1) = 0.1.
        (
            0.9,
            "asserts_only_own_setup",
            &[
                (ASSERTS_REQUIRED_OUTCOME, 0.1),
                ("asserts_only_own_setup", 0.8),
            ],
            "no",
            0.9,
        ),
        // Checks the outcome too loosely: min(0.9, 0.3) = 0.3.
        (
            0.9,
            ASSERTS_OUTCOME_TOO_LOOSELY,
            &[
                (ASSERTS_REQUIRED_OUTCOME, 0.3),
                (ASSERTS_OUTCOME_TOO_LOOSELY, 0.6),
            ],
            "no",
            0.7,
        ),
    ];
    for (p, label, probabilities, answer, confidence) in cases {
        let prediction = derive_asserts_intent(&split_answers(p, label, probabilities))
            .unwrap()
            .unwrap();
        assert_eq!(prediction.answer, answer, "{p} {label}");
        assert!(
            close(prediction.confidence, confidence),
            "{p} {label}: {prediction:?}"
        );
    }
}

/// Provenance: PLAT-1030, MP-242 (PR #623 review finding 1). Over a grid of
/// both halves, the answer, the confidence and the ordinal Bar D reads agree:
/// `yes` iff the ordinal is at least 0.5, and the confidence is never below
/// 0.5.
#[test]
fn tc_1030_t1_answer_and_confidence_are_coherent() {
    let grid = [0.0, 0.1, 0.3, 0.49, 0.5, 0.51, 0.7, 0.9, 1.0];
    for p_exercises in grid {
        for p_required in grid {
            let answers = split_answers(
                p_exercises,
                ASSERTS_REQUIRED_OUTCOME,
                &[(ASSERTS_REQUIRED_OUTCOME, p_required)],
            );
            let prediction = derive_asserts_intent(&answers).unwrap().unwrap();
            let p_yes = prediction.ordinal.unwrap();
            let at = format!("{p_exercises} {p_required}: {prediction:?}");
            assert_eq!(prediction.answer == "yes", p_yes >= 0.5, "{at}");
            assert!(prediction.confidence.unwrap() >= 0.5, "{at}");
        }
    }
}

/// Provenance: PLAT-1030, MP-242 rule 5, PR #623 review finding 7.
/// `cannot_tell` abstains only when the rule cannot decide: with
/// `P(exercises) < 0.5` the answer is `no`, with confidence
/// `1 - min(0.3, 0.2) = 0.8`.
#[test]
fn tc_1030_cannot_tell_abstains_only_when_undecided() {
    let probabilities = [(CANNOT_TELL, 0.6), (ASSERTS_REQUIRED_OUTCOME, 0.2)];
    let answers = split_answers(0.9, CANNOT_TELL, &probabilities);
    assert_eq!(derive_asserts_intent(&answers), Ok(None));

    let answers = split_answers(0.3, CANNOT_TELL, &probabilities);
    let prediction = derive_asserts_intent(&answers).unwrap().unwrap();
    assert_eq!(prediction.answer, "no");
    assert!(close(prediction.confidence, 0.8), "{prediction:?}");
}

/// Provenance: PLAT-1030, MP-242 (review of PR #620). A response the rule
/// cannot read is an error naming what is wrong, never a silent unanswered
/// row: a missing or mis-shaped half, an unknown label, a probability keyed
/// to a label outside the set, a missing probability for the required
/// label, a probability outside [0, 1].
#[test]
fn tc_1030_a_malformed_response_is_an_error_not_an_abstention() {
    let required = [(ASSERTS_REQUIRED_OUTCOME, 0.7)];
    let mut no_choice = split_answers(0.9, ASSERTS_REQUIRED_OUTCOME, &required);
    no_choice.remove(CHECK_KIND);
    let mut no_noul = split_answers(0.9, ASSERTS_REQUIRED_OUTCOME, &required);
    no_noul.remove(EXERCISES_CRITERION);
    let mut noul_as_choice = split_answers(0.9, ASSERTS_REQUIRED_OUTCOME, &required);
    noul_as_choice.insert(CHECK_KIND.to_owned(), RawAnswer::Noul(0.9));
    let cases = [
        (no_choice, "`check_kind`: no answer in the response"),
        (no_noul, "`exercises_criterion`: no answer in the response"),
        (noul_as_choice, "`check_kind`: expected a choice"),
        (
            split_answers(0.9, "asserts_nothing", &required),
            "`check_kind`: unknown label \"asserts_nothing\"",
        ),
        (
            split_answers(
                0.9,
                ASSERTS_REQUIRED_OUTCOME,
                &[(ASSERTS_REQUIRED_OUTCOME, 0.7), ("asserts_outcome", 0.3)],
            ),
            "`check_kind`: probability for unknown label \"asserts_outcome\"",
        ),
        (
            split_answers(0.9, "asserts_unrelated", &[("asserts_unrelated", 0.8)]),
            "`check_kind`: no probability for `asserts_required_outcome`",
        ),
        (
            split_answers(1.2, ASSERTS_REQUIRED_OUTCOME, &required),
            "`exercises_criterion`: probability 1.2 is outside [0, 1]",
        ),
        (
            split_answers(
                0.9,
                ASSERTS_REQUIRED_OUTCOME,
                &[(ASSERTS_REQUIRED_OUTCOME, f64::NAN)],
            ),
            "`check_kind`: probability NaN is outside [0, 1]",
        ),
    ];
    for (answers, expected) in cases {
        let error = derive_asserts_intent(&answers).unwrap_err();
        assert!(error.starts_with(expected), "{expected:?}: got {error:?}");
    }
    assert_eq!(
        derive_trace(&RawAnswers::new()).unwrap_err(),
        "`trace_correct`: no answer in the response"
    );
    let bad_trace = RawAnswers::from([("trace_correct".to_owned(), RawAnswer::Noul(-0.1))]);
    assert_eq!(
        derive_trace(&bad_trace).unwrap_err(),
        "`trace_correct`: probability -0.1 is outside [0, 1]"
    );
}

/// Provenance: PLAT-1030, MP-242 (review of PR #620). Through the runner, a
/// response missing the required label's probability stops the run and
/// names the row.
#[tokio::test]
#[should_panic(expected = "EV2-0105: malformed Jev response (`check_kind`: no probability")]
async fn tc_1030_the_runner_stops_on_a_malformed_response() {
    let rows = parse(&[row(
        "EV2-0105",
        Mode::ReqTest,
        &json!({"test_asserts_intent": truth(false)}),
    )])
    .rows;
    let mut script = script();
    script.insert(
        "FR-105".to_owned(),
        BTreeMap::from([
            (EXERCISES_CRITERION.to_owned(), noul_answer(0.8)),
            (
                CHECK_KIND.to_owned(),
                json!({"type": "choice", "choice": "asserts_unrelated", "confidence": 0.6,
                       "probabilities": {"asserts_unrelated": 0.6}}),
            ),
        ]),
    );
    let (client, _) = client_with(script);
    let _ = variant::run(&client, &rows, &[&T1]).await;
}

/// Provenance: PLAT-1030, MP-242 rule 5. The comparison with T0 counts only
/// rows both answered, and reports each side's abstentions.
#[test]
fn tc_1030_paired_comparison_uses_rows_both_answered() {
    let scored_row = |id: &str, expected: &str, answer: Option<&str>| Scored {
        row_id: id.to_owned(),
        mode: Mode::ReqTestCode,
        kind: TruthKind::Mechanical,
        expected: expected.to_owned(),
        alternatives: Vec::new(),
        prediction: answer.map(|answer| Prediction {
            answer: answer.to_owned(),
            confidence: Some(0.9),
            ordinal: None,
        }),
    };
    let t1 = [
        scored_row("a", "yes", Some("yes")),
        scored_row("b", "no", None),
        scored_row("c", "no", Some("no")),
        scored_row("d", "yes", Some("no")),
        scored_row("only-t1", "yes", Some("yes")),
    ];
    let t0 = [
        scored_row("a", "yes", Some("no")),
        scored_row("b", "no", Some("no")),
        scored_row("c", "no", None),
        scored_row("d", "yes", Some("yes")),
    ];
    assert_eq!(
        paired(&t1, &t0),
        Paired {
            shared: 4,
            both_answered: 2,
            left_correct: 1,
            right_correct: 1,
            left_abstained: 1,
            right_abstained: 1,
        }
    );
}

// ---------------------------------------------------------------------------
// Wording per mode
// ---------------------------------------------------------------------------

fn question_text(variant: &Variant, row: &Row) -> String {
    (variant.asks)(row)
        .iter()
        .map(|ask| serde_json::to_string(&ask.request.questions).unwrap())
        .collect()
}

/// Provenance: PLAT-1030, PLAT-1024 (Peter's scope ruling). T1 and T2 send the same
/// questions in RT and RTC, never naming code; in RTC the code rides along
/// in the state as context only. Neither runs where there is no test.
#[test]
fn tc_1030_t1_and_t2_ask_the_same_thing_with_or_without_code() {
    let rows = corpus_rows();
    let (rt, rtc, rc) = (&rows[0], &rows[2], &rows[3]);
    for variant in [T1, T2] {
        assert_eq!(question_text(&variant, rt), question_text(&variant, rtc));
        assert!(wording_violations(&variant, rt).is_empty());
        assert!(wording_violations(&variant, rtc).is_empty());
        assert!(!variant.applies_to(rc));
        let with_code = serde_json::to_value(&(variant.asks)(rtc)[0].request.state).unwrap();
        assert_eq!(with_code["symbol_body"], CODE_BODY);
        let without_code = serde_json::to_value(&(variant.asks)(rt)[0].request.state).unwrap();
        assert!(without_code.get("symbol_body").is_none(), "{without_code}");

        // Not vacuous: misapplied to an RC row, the rule flags the test.
        let misapplied = Variant {
            modes: &Mode::ALL,
            ..variant
        };
        let found = wording_violations(&misapplied, rc);
        assert!(found.iter().any(|v| v.contains("the test")), "{found:#?}");
    }
}

/// Provenance: PLAT-1030, MP-242, Lyon nb 10/12. TC names exactly what it judges in
/// each mode: the test in RT, the code in RC, both in RTC; R has no TC.
#[test]
fn tc_1030_tc_names_exactly_what_each_mode_carries() {
    let text = |mode: Mode| serde_json::to_string(&trace_question(mode).unwrap()).unwrap();
    let rt = text(Mode::ReqTest);
    assert!(rt.contains("the test in `test_body`") && !rt.contains("symbol_body"));
    let rc = text(Mode::ReqCode);
    assert!(rc.contains("the code in `symbol_body`") && !rc.contains("test_body"));
    // PR #623 review finding 12: the code implements, and RTC says what
    // counts when only one of the two is on the requirement.
    assert!(rc.contains("the behaviour implemented by the code in `symbol_body`"));
    let rtc = text(Mode::ReqTestCode);
    assert!(rtc.contains(
        "exercised by the test in `test_body` or implemented by the code in `symbol_body`"
    ));
    assert!(
        rtc.contains("of the test, of the code, or of both"),
        "{rtc}"
    );
    // The wording rule: RT never mentions code, RC never mentions a test,
    // not even as "checked weakly" or "covered".
    for word in ["code", "implement"] {
        assert!(!rt.contains(word), "RT mentions {word:?}: {rt}");
    }
    for word in ["test", "check", "cover", "assert"] {
        assert!(!rc.contains(word), "RC mentions {word:?}: {rc}");
    }
    assert!(rc.contains("even if the code implements that behaviour only partly"));
    assert!(rt.contains("even if the test covers that behaviour only partly or checks it weakly"));
    assert!(trace_question(Mode::Req).is_none());

    let rows = corpus_rows();
    for tc in TC_FAMILY {
        let mine: Vec<&Row> = rows.iter().filter(|row| tc.applies_to(row)).collect();
        assert_eq!(
            mine.len(),
            if tc.id == "TC-RT" { 2 } else { 1 },
            "{}",
            tc.id
        );
        for row in mine {
            assert!(wording_violations(&tc, row).is_empty(), "{}", tc.id);
        }
    }
    // The declaration is checked: TC-RT's declaration on an RC row lies.
    let misdeclared = Variant {
        modes: &[Mode::ReqCode],
        ..TC_RT
    };
    let found = wording_violations(&misdeclared, &rows[3]);
    assert!(
        found.iter().any(|v| v.contains("does not declare")),
        "{found:#?}"
    );
    assert_eq!(
        [TC_RT.id, TC_RC.id, TC_RTC.id],
        ["TC-RT", "TC-RC", "TC-RTC"]
    );
}

/// Provenance: PLAT-1030, MP-242, jev-code `testFrame`. T2 carries 5 worked examples
/// for the exercise question and 7 for the check-kind question (one per
/// label); T1 carries none. They are synthetic: no quoin FR, AC or corpus id
/// appears in them.
#[test]
fn tc_1030_t2_carries_synthetic_worked_examples() {
    let instructions = |examples: bool, key: &str| {
        serde_json::to_value(&intent::split_questions(examples)[key]).unwrap()["instructions"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    for (key, count) in [(EXERCISES_CRITERION, 5), (CHECK_KIND, 7)] {
        let with = instructions(true, key);
        assert_eq!(with.matches("\nExample ").count(), count, "{key}");
        assert_eq!(instructions(false, key).matches("Example ").count(), 0);
        for id in ["FR-", "-AC-", "EV2-", "EVX-", "GAP-"] {
            assert!(!with.contains(id), "{key} example cites {id}");
        }
    }
    let check_kind = instructions(true, CHECK_KIND);
    for (label, _) in intent::CHECK_KINDS {
        assert!(
            check_kind.contains(&format!("Answer: {label}.")),
            "no worked example answers {label}"
        );
    }
    // PR #623 review finding 2: the LRU example's loose check is named.
    let exercises = instructions(true, EXERCISES_CRITERION);
    assert!(
        exercises.contains(
            "`len() <= 2` never checks which entry was evicted, so its check is \
             asserts_outcome_too_loosely"
        ),
        "{exercises}"
    );
}

/// Provenance: PLAT-1030, PR #623 review findings 2 and 10. `check_kind` has
/// seven labels in a stated precedence order, strongest first, which the
/// instruction tells Jev to apply; the constant label needs no code to judge.
#[test]
fn tc_1030_check_kinds_carry_a_precedence_order() {
    let labels: Vec<&str> = intent::CHECK_KINDS
        .iter()
        .map(|(label, _)| *label)
        .collect();
    assert_eq!(
        labels,
        [
            "asserts_required_outcome",
            "asserts_outcome_too_loosely",
            "asserts_unrelated",
            "asserts_only_that_it_runs",
            "asserts_only_own_setup",
            "asserts_constant_or_restatement",
            "cannot_tell",
        ]
    );
    let question = serde_json::to_string(&intent::split_questions(false)[CHECK_KIND]).unwrap();
    assert!(
        question.contains(
            "The labels are listed strongest first: when the assertions fit \
             several, choose the earliest."
        ),
        "{question}"
    );
    let constant = intent::CHECK_KINDS[5].1;
    assert!(
        constant
            .contains("a hard-coded literal or a value that would hold whatever the behaviour is"),
        "{constant}"
    );
    assert!(!constant.contains("logic"), "{constant}");
}

/// Provenance: PLAT-1030, MP-242. T0-RT is `FullBatteryV1`'s
/// `test_asserts_intent` question with one edit, "the code" -> "the system"
/// (it occurs twice), and nothing else; it runs on RT rows only and names no
/// code.
#[test]
fn tc_1030_t0_rt_is_t0_with_only_the_code_references_removed() {
    let t0 = intent::t0_instruction();
    assert!(t0.starts_with("Suppose the code were changed"), "{t0}");
    assert_eq!(t0.matches(T0_RT_EDIT.0).count(), 2, "{t0}");
    let t0_rt = intent::t0_rt_instruction();
    assert_eq!(
        t0_rt,
        "Suppose the system were changed so that it no longer does the specific thing the \
         requirement states, while still compiling and still returning a well-formed, \
         non-default result. Would this test fail? Judge against the requirement's own stated \
         behaviour, not against other things the system does."
    );
    let rows = corpus_rows();
    assert!(wording_violations(&T0_RT, &rows[0]).is_empty());
    assert!(!T0_RT.applies_to(&rows[2]));
    let everywhere = Variant {
        modes: &Mode::ALL,
        ..T0_RT
    };
    let found = wording_violations(&everywhere, &rows[3]);
    assert!(found.iter().any(|v| v.contains("the test")), "{found:#?}");
}

// ---------------------------------------------------------------------------
// The gating wrapper, end to end
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1030, MP-242. TC runs once per row in its own mode, T1 once per row
/// with a test; T1's answers are split by TC's verdict and, separately, by
/// the rows' own `trace_correct` truth.
#[tokio::test]
async fn tc_1030_tc_gates_the_other_variants_report() {
    let rows = corpus_rows();
    let (client, fake) = scripted_client();
    let variants: Vec<&Variant> = vec![&TC_RT, &TC_RC, &TC_RTC, &T0_RT, &T1];
    let output = variant::run(&client, &rows, &variants).await.unwrap();
    // Four TC requests (one per row), two T0-RT requests (the RT rows) and
    // three T1 requests (RT, RT, RTC).
    assert_eq!(fake.calls.load(Ordering::SeqCst), 9);

    let tc_rt = scored(&rows, &output, "TC-RT@v1", "trace_correct");
    let answers: Vec<&str> = tc_rt
        .iter()
        .map(|row| row.prediction.as_ref().unwrap().answer.as_str())
        .collect();
    assert_eq!(answers, ["yes", "no"]);

    let t1 = scored(&rows, &output, "T1@v1", "test_asserts_intent");
    let answers: Vec<(&str, &str)> = t1
        .iter()
        .map(|row| {
            (
                row.row_id.as_str(),
                row.prediction.as_ref().unwrap().answer.as_str(),
            )
        })
        .collect();
    assert_eq!(
        answers,
        [("EV2-0101", "yes"), ("EV2-0102", "no"), ("EV2-0103", "no")]
    );

    let gates = tc_gates(&output);
    assert_eq!(
        gates,
        BTreeMap::from([
            ("EV2-0101".to_owned(), Gate::TraceRight),
            ("EV2-0102".to_owned(), Gate::TraceWrong),
            ("EV2-0103".to_owned(), Gate::TraceRight),
            ("EV2-0104".to_owned(), Gate::TraceWrong),
        ])
    );
    let split = split_by_gate(&t1, &gates, &Gate::BY_TC);
    let ids =
        |gate: Gate| -> Vec<&str> { split[&gate].iter().map(|r| r.row_id.as_str()).collect() };
    assert_eq!(ids(Gate::TraceRight), ["EV2-0101", "EV2-0103"]);
    assert_eq!(ids(Gate::TraceWrong), ["EV2-0102"]);
    assert!(ids(Gate::NoTraceJudgment).is_empty());

    let report = render_gated_run(&rows, &output, &variants);
    println!("{report}");
    for needle in [
        "### T1@v1 — `test_asserts_intent`, split by trace",
        "2 of 3 rows: TC says trace right",
        "1 of 3 rows: TC says trace wrong",
        "0 of 3 rows abstained (no answer; kept in every denominator)",
        // PR #623 re-review B1: the pooled, unsplit table Bars A and B read.
        "### T1@v1 | counted, pooled — `test_asserts_intent`",
        "| all [AGENT-LABELLED] | 3 |",
        // And TC's pooled table, Bar C's.
        "### TC-RT@v1 + TC-RC@v1 + TC-RTC@v1 | counted, pooled — `trace_correct`",
        "| all [AGENT-LABELLED] | 4 |",
        // T1 right on 101 and 102; T0-RT says yes on both (0.8, 0.7), so
        // right on 101 only. The RTC row has no T0 in this run: not shared.
        "Paired with T0-RT@v1: 2 rows shared, 2 answered by both; on those, T1@v1 2 correct, \
         baseline 1 correct; abstained T1@v1 0, baseline 0",
        "0 of 3 rows: TC says no trace judgment",
        "### T1@v1 | TC: trace right — `test_asserts_intent`",
        "2 of 3 rows: trace_correct label [AGENT-LABELLED] says trace right",
        "0 of 3 rows: trace_correct label [AGENT-LABELLED] says contested",
        "### T1@v1 | trace_correct label [AGENT-LABELLED]: trace wrong — `test_asserts_intent`",
    ] {
        assert!(report.contains(needle), "missing {needle:?}:\n{report}");
    }
    assert!(
        !report.contains("TC-RT@v1 | TC:") && !report.contains("TC-RT@v1 — `trace_correct`, split"),
        "TC is not gated by itself"
    );
}

/// Provenance: PLAT-1030, MP-242, PR #623 review finding 11. Without TC in
/// the run there is no split by TC's answers, and it says so, but the split
/// by the rows' own label is still printed.
#[tokio::test]
async fn tc_1030_without_tc_the_label_split_still_prints() {
    let rows = corpus_rows();
    let (client, _) = scripted_client();
    let output = variant::run(&client, &rows, &[&T1]).await.unwrap();
    let report = render_gated_run(&rows, &output, &[&T1]);
    println!("{report}");
    assert!(report.contains("No TC variant in this run"), "{report}");
    assert!(!report.contains("TC says"), "{report}");
    assert!(
        report.contains("2 of 3 rows: trace_correct label [AGENT-LABELLED] says trace right"),
        "{report}"
    );
}

/// Provenance: PLAT-1030, PR #623 review finding 8. A contested
/// `trace_correct` label is its own slice of the label split, whatever its
/// primary answer.
#[test]
fn tc_1030_a_contested_trace_label_is_its_own_slice() {
    let contested = json!({"trace_correct": {"answer": true, "kind": "agent_contested",
                                             "alternatives": [false], "rationale": "stated"}});
    let rows = parse(&[
        row(
            "EV2-0101",
            Mode::ReqTest,
            &json!({"trace_correct": truth(true)}),
        ),
        row("EV2-0102", Mode::ReqTest, &contested),
        row("EV2-0103", Mode::ReqTest, &json!({})),
    ])
    .rows;
    assert_eq!(
        intent::truth_gates(&rows),
        BTreeMap::from([
            ("EV2-0101".to_owned(), Gate::TraceRight),
            ("EV2-0102".to_owned(), Gate::ContestedLabel),
        ])
    );
}

/// Provenance: PLAT-1030, PR #623 review finding 5. A mutant row whose label
/// is agent-written (inherited from its source) is not counted; a natural
/// row's agent label and a mutant's own mechanical label are.
#[test]
fn tc_1030_inherited_agent_labels_on_mutants_never_count() {
    let mutant = |id: &str, kind: &str| {
        let mut value = row(
            id,
            Mode::ReqTest,
            &json!({"test_asserts_intent": {"answer": false, "kind": kind,
                                            "alternatives": [], "rationale": "stated"}}),
        );
        value["mutation"] = json!({"id": id, "target": "test", "kind": "test_weakening",
                                   "description": "d", "patch": "p"});
        value
    };
    let rows = parse(&[
        row(
            "EV2-0101",
            Mode::ReqTest,
            &json!({"test_asserts_intent": truth(true)}),
        ),
        mutant("EV2-0102", "agent_dual"),
        mutant("EV2-0103", "mechanical"),
    ])
    .rows;
    let graded: Vec<Scored> = rows
        .iter()
        .map(|row| {
            let truth = &row.truth["test_asserts_intent"];
            Scored {
                row_id: row.id.clone(),
                mode: row.mode,
                kind: truth.kind,
                expected: truth.answer.label(),
                alternatives: Vec::new(),
                prediction: None,
            }
        })
        .collect();
    let kept: Vec<String> = intent::counted(&rows, graded)
        .into_iter()
        .map(|row| row.row_id)
        .collect();
    assert_eq!(kept, ["EV2-0101", "EV2-0103"]);
}

/// Provenance: PLAT-1030, PR #623 review finding 14 and re-review L3. A live
/// run that selects any of MP-242's own variants needs a recording cassette;
/// other variants do not, and neither does the shared baseline T0, which
/// other experiments run too.
#[test]
fn tc_1030_mp_242_variants_need_a_cassette() {
    let error = intent::require_cassette(&[&variant::C0, &variant::T0, &T1], None).unwrap_err();
    assert!(
        error.contains("[\"T1@v1\"]") && error.contains("QUOIN_JEV_CASSETTE"),
        "{error}"
    );
    intent::require_cassette(&[&T1], Some("cassette.jsonl")).unwrap();
    intent::require_cassette(&[&variant::C0], None).unwrap();
    intent::require_cassette(&[&variant::S0, &variant::T0, &variant::C0], None).unwrap();
    for own in [&TC_RT, &TC_RC, &TC_RTC, &T0_RT, &T1, &T2] {
        assert!(
            intent::require_cassette(&[own], None).is_err(),
            "{}",
            own.id
        );
    }
}

/// Provenance: PLAT-1030, PR #623 re-review L2. A held-out run that stops
/// part way, here on T1's malformed-answer panic, is still on the held-out
/// log when the spend guard drops, so the next attempt is refused without a
/// rerun reason. A finished run is logged once, with its models.
#[test]
fn tc_1030_a_stopped_heldout_run_is_still_logged() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("heldout-runs.jsonl");
    let labels = vec![T1.label()];
    let run = || corpus::HeldoutRun {
        unix_seconds: 0,
        variants: labels.clone(),
        seals: vec!["abc".to_owned()],
        rows: 1,
        models: BTreeMap::new(),
        rerun_reason: None,
    };
    let rows = corpus_rows();
    let malformed = variant::Answered {
        unit: None,
        answers: split_answers(0.9, "asserts_nothing", &[(ASSERTS_REQUIRED_OUTCOME, 0.7)]),
    };
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _spend = corpus::HeldoutSpend::begin(log.clone(), run());
        (T1.derive)(&rows[0], std::slice::from_ref(&malformed))
    }));
    let message = stopped.unwrap_err();
    assert!(
        message
            .downcast_ref::<String>()
            .is_some_and(|m| m.contains("malformed Jev response")),
        "the stop was not the malformed-answer panic"
    );
    let refused = corpus::check_heldout_rerun(&log, &labels, None).unwrap_err();
    assert!(refused.contains("T1@v1"), "{refused}");

    let finished = dir.path().join("finished.jsonl");
    corpus::HeldoutSpend::begin(finished.clone(), run())
        .finish(BTreeMap::from([("jev-fake".to_owned(), 3)]))
        .unwrap();
    let lines: Vec<corpus::HeldoutRun> = std::fs::read_to_string(&finished)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 1, "finish must not log twice");
    assert_eq!(lines[0].models["jev-fake"], 3);
    assert!(lines[0].unix_seconds > 0);
}

/// Provenance: PLAT-1030, PR #623 re-review B1. TC's `trace_correct` obeys
/// the same counting rule as every other key: a mutant row whose trace label
/// is inherited from its source (agent-written) is left out of TC's pooled
/// table, while a trace-swap mutant's own by-construction label is kept. The
/// harness's generic table counts all four rows, which is why MP-242 reads
/// Bar C from the split section.
#[test]
fn tc_1030_inherited_trace_labels_never_count() {
    let rows = parse(&[
        row(
            "EV2-0101",
            Mode::ReqTest,
            &json!({"trace_correct": truth(true)}),
        ),
        row(
            "EV2-0102",
            Mode::ReqCode,
            &json!({"trace_correct": truth(false)}),
        ),
        // Inherited: an agent label copied from the source.
        mutant_with(
            "EV2-0201",
            Mode::ReqTest,
            ("M-1", "test_weakening", Some("EV2-0101")),
            &json!({"trace_correct": truth(true)}),
        ),
        // The mutant's own label: true by the swap that made it.
        mutant_with(
            "EV2-0202",
            Mode::ReqCode,
            ("M-2", "trace_swap", Some("EV2-0102")),
            &json!({"trace_correct": by_construction(false)}),
        ),
    ])
    .rows;
    let output = variant::RunOutput {
        results: vec![
            result(&TC_RT, "trace_correct", "EV2-0101", Some(0.9)),
            result(&TC_RT, "trace_correct", "EV2-0201", Some(0.9)),
            result(&TC_RC, "trace_correct", "EV2-0102", Some(0.2)),
            result(&TC_RC, "trace_correct", "EV2-0202", Some(0.2)),
        ],
        ..variant::RunOutput::default()
    };
    let tc: Vec<&Variant> = TC_FAMILY.iter().collect();
    let kept: Vec<String> = intent::tc_counted(&rows, &output, &tc)
        .into_iter()
        .map(|row| row.row_id)
        .collect();
    assert_eq!(kept, ["EV2-0101", "EV2-0102", "EV2-0202"]);

    let report = render_gated_run(&rows, &output, &tc);
    println!("{report}");
    for needle in [
        "### TC — `trace_correct`, pooled over its modes",
        "### TC-RT@v1 + TC-RC@v1 + TC-RTC@v1 | counted, pooled — `trace_correct`",
        "| all (incl. 2 AGENT-LABELLED) | 3 |",
        "| mode RT [AGENT-LABELLED] | 1 |",
        "| mode RC (incl. 1 AGENT-LABELLED) | 2 |",
        "| AGENT-LABELLED | 2 |",
    ] {
        assert!(report.contains(needle), "missing {needle:?}:\n{report}");
    }
    // Not vacuous: the generic per-variant table counts the inherited row.
    let generic = eval_v2_support::metrics::render_run(&rows, &[], &output, &[&TC_RT]);
    assert!(
        generic.contains("| all [AGENT-LABELLED] | 2 |"),
        "{generic}"
    );
}

// ---------------------------------------------------------------------------
// Bar D
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1030, MP-242 Bar D (PR #623 review findings 1, 3 and 6).
/// A pair succeeds only when `P(yes)` crosses 0.5 downwards and falls by at
/// least 0.10; it fails when it rises by 0.10 or either row abstained; every
/// other pair is a tie. The review's case: a mutant still answered `yes`
/// (0.5) is never a success.
#[test]
fn tc_1030_bar_d_scores_each_pair() {
    use intent::PairOutcome::{Failure, Success, Tie};
    let cases = [
        (Some(0.8), Some(0.3), Success),
        // 0.5 - 0.4 is 0.0999... in f64 and still counts as a 0.10 move.
        (Some(0.5), Some(0.4), Success),
        // Fell by 0.22 but the mutant is still `yes`.
        (Some(0.72), Some(0.5), Tie),
        // Crossed, but by less than 0.10.
        (Some(0.52), Some(0.45), Tie),
        // Already `no` at the source: no crossing.
        (Some(0.45), Some(0.1), Tie),
        (Some(0.3), Some(0.45), Failure),
        (None, Some(0.1), Failure),
        (Some(0.9), None, Failure),
    ];
    for (source, mutant, expected) in cases {
        assert_eq!(
            intent::pair_outcome(source, mutant),
            expected,
            "{source:?} -> {mutant:?}"
        );
    }
}

/// Provenance: PLAT-1030, MP-242 Bar D. The one-sided sign test, worked by
/// hand: P(X >= k) for X ~ Binomial(n, 0.5).
#[test]
fn tc_1030_bar_d_sign_test() {
    for (successes, failures, expected) in [
        (10, 0, 1.0 / 1024.0),
        (9, 1, 11.0 / 1024.0),
        (8, 2, 56.0 / 1024.0),
        (0, 0, 1.0),
        (0, 3, 1.0),
    ] {
        let p = intent::sign_test_p(successes, failures);
        assert!(
            (p - expected).abs() < 1e-12,
            "{successes}/{failures}: {p} != {expected}"
        );
    }
    let d = intent::BarD {
        successes: 9,
        failures: 1,
        p_value: intent::sign_test_p(9, 1),
        ..intent::BarD::default()
    };
    assert!(d.gateable() && d.passes());
    let d = intent::BarD {
        successes: 8,
        failures: 1,
        p_value: intent::sign_test_p(8, 1),
        ..intent::BarD::default()
    };
    assert!(!d.gateable() && !d.passes(), "9 non-ties is under 10");
}

fn by_construction(answer: bool) -> Value {
    json!({"answer": answer, "kind": "by_construction", "alternatives": [], "rationale": "stated"})
}

/// A mutant row of `mode` carrying `truth`, from `(mutation id, kind,
/// source)` (`None` = a null `source_id`).
fn mutant_with(
    id: &str,
    mode: Mode,
    (mutation, kind, source): (&str, &str, Option<&str>),
    truth: &Value,
) -> Value {
    let mut value = row(id, mode, truth);
    value["mutation"] = json!({"id": mutation, "target": "test", "kind": kind,
                               "description": "d", "patch": "p", "source_id": source});
    value
}

/// A mutant row of `mode` from mutation `mutation` of `kind`, made from
/// `source` (`None` = a null `source_id`), labelled `no` for intent.
fn mutant_row(id: &str, mode: Mode, mutation: &str, kind: &str, source: Option<&str>) -> Value {
    mutant_with(
        id,
        mode,
        (mutation, kind, source),
        &json!({"test_asserts_intent": {"answer": false, "kind": "mechanical",
                                        "alternatives": [], "rationale": "stated"}}),
    )
}

/// `variant`'s result on `row_id` for `key`, `P(yes)` = `p_yes` (`None` =
/// abstained).
fn result(
    variant: &Variant,
    key: &'static str,
    row_id: &str,
    p_yes: Option<f64>,
) -> variant::RowResult {
    let predictions = p_yes
        .map(|p| {
            variant::Predictions::from([(
                key,
                Prediction {
                    answer: if p >= 0.5 { "yes" } else { "no" }.to_owned(),
                    confidence: Some(p.max(1.0 - p)),
                    ordinal: Some(p),
                },
            )])
        })
        .unwrap_or_default();
    variant::RowResult {
        row_id: row_id.to_owned(),
        variant: variant.label(),
        predictions,
        answered: Vec::new(),
    }
}

fn t1_result(row_id: &str, p_yes: Option<f64>) -> variant::RowResult {
    result(&T1, "test_asserts_intent", row_id, p_yes)
}

/// Provenance: PLAT-1030, MP-242 Bar D (PR #623 review findings 3 and 6).
/// Pairs come from `mutation.source_id`, one per mutation id with the RTC
/// row preferred; only `test_weakening` mutants count for
/// `test_asserts_intent`; a source not labelled `yes` is left out; a null
/// `source_id` is unpairable; an abstention is a failure.
#[test]
fn tc_1030_bar_d_pairs_by_source_id() {
    let rows = parse(&[
        row(
            "EV2-0101",
            Mode::ReqTestCode,
            &json!({"test_asserts_intent": truth(true)}),
        ),
        row(
            "EV2-0102",
            Mode::ReqTestCode,
            &json!({"test_asserts_intent": truth(false)}),
        ),
        // M-1: an RT row listed first, then the RTC row that is used.
        mutant_row(
            "EV2-0201",
            Mode::ReqTest,
            "M-1",
            "test_weakening",
            Some("EV2-0101"),
        ),
        mutant_row(
            "EV2-0202",
            Mode::ReqTestCode,
            "M-1",
            "test_weakening",
            Some("EV2-0101"),
        ),
        // M-2: abstains on the mutant, a failure.
        mutant_row(
            "EV2-0203",
            Mode::ReqTestCode,
            "M-2",
            "test_weakening",
            Some("EV2-0101"),
        ),
        // M-3: its source is already `no`.
        mutant_row(
            "EV2-0204",
            Mode::ReqTestCode,
            "M-3",
            "test_weakening",
            Some("EV2-0102"),
        ),
        // M-4: no source in the corpus.
        mutant_row("EV2-0205", Mode::ReqTestCode, "M-4", "test_weakening", None),
        // M-5: a requirement-text mutant, never paired for intent.
        mutant_row(
            "EV2-0206",
            Mode::ReqTest,
            "M-5",
            "requirement_text",
            Some("EV2-0101"),
        ),
    ])
    .rows;
    let output = variant::RunOutput {
        results: vec![
            t1_result("EV2-0101", Some(0.8)),
            t1_result("EV2-0102", Some(0.3)),
            // Were the RT row used, M-1 would be a tie.
            t1_result("EV2-0201", Some(0.75)),
            t1_result("EV2-0202", Some(0.2)),
            t1_result("EV2-0203", None),
            t1_result("EV2-0204", Some(0.1)),
            t1_result("EV2-0205", Some(0.1)),
            t1_result("EV2-0206", Some(0.1)),
        ],
        ..variant::RunOutput::default()
    };
    let d = intent::bar_d(&rows, &output, &[&T1], "test_asserts_intent").unwrap();
    assert_eq!(
        d,
        intent::BarD {
            pairs: 2,
            successes: 1,
            failures: 1,
            ties: 0,
            source_not_yes: 1,
            unpairable: 1,
            p_value: 0.75,
        }
    );

    // A source that is not among the rows run stops D loudly.
    let orphan = parse(&[mutant_row(
        "EV2-0201",
        Mode::ReqTestCode,
        "M-1",
        "test_weakening",
        Some("EV2-0999"),
    )])
    .rows;
    let error = intent::bar_d(&orphan, &output, &[&T1], "test_asserts_intent").unwrap_err();
    assert!(error.contains("EV2-0999"), "{error}");

    // The report carries D for T1.
    let report = render_gated_run(&rows, &output, &[&T1]);
    assert!(
        report.contains(
            "Bar D, T1@v1 on `test_asserts_intent`: 2 pairs, 1 successes, 1 failures, 0 ties; \
             one-sided sign test p = 0.7500; not gateable (fewer than 10 non-tie pairs). Left \
             out: 1 with the source not labelled yes, 1 unpairable."
        ),
        "{report}"
    );
}

/// Provenance: PLAT-1030, MP-242 Bar D, PR #623 re-review M1. Bar D pairs
/// `trace_swap` mutants for `trace_correct` and `test_weakening` mutants for
/// `test_asserts_intent`, and nothing else.
#[test]
fn tc_1030_bar_d_mutation_kind_per_key() {
    assert_eq!(
        intent::bar_d_mutation_kind("trace_correct"),
        Some("trace_swap")
    );
    assert_eq!(
        intent::bar_d_mutation_kind("test_asserts_intent"),
        Some("test_weakening")
    );
    assert_eq!(intent::bar_d_mutation_kind("criterion_sound"), None);
}

/// Provenance: PLAT-1030, MP-242 Bar D, PR #623 re-review M1. TC's D pools
/// TC-RT, TC-RC and TC-RTC: one trace-swap pair per mode, each answered by
/// that mode's TC variant, gives three pairs pooled and one for TC-RT alone.
/// A test-weakening mutant with a `yes`-labelled trace source is never a
/// `trace_correct` pair.
#[test]
fn tc_1030_bar_d_pools_the_tc_family_on_trace_swaps() {
    let trace = |answer: bool| json!({"trace_correct": truth(answer)});
    let swapped = json!({"trace_correct": by_construction(false)});
    let rows = parse(&[
        row("EV2-0101", Mode::ReqTest, &trace(true)),
        row("EV2-0102", Mode::ReqCode, &trace(true)),
        row("EV2-0103", Mode::ReqTestCode, &trace(true)),
        mutant_with(
            "EV2-0201",
            Mode::ReqTest,
            ("M-1", "trace_swap", Some("EV2-0101")),
            &swapped,
        ),
        mutant_with(
            "EV2-0202",
            Mode::ReqCode,
            ("M-2", "trace_swap", Some("EV2-0102")),
            &swapped,
        ),
        mutant_with(
            "EV2-0203",
            Mode::ReqTestCode,
            ("M-3", "trace_swap", Some("EV2-0103")),
            &swapped,
        ),
        // Would be a fourth success if the kind filter were wrong.
        mutant_with(
            "EV2-0204",
            Mode::ReqTestCode,
            ("M-4", "test_weakening", Some("EV2-0103")),
            &trace(true),
        ),
    ])
    .rows;
    let output = variant::RunOutput {
        results: vec![
            result(&TC_RT, "trace_correct", "EV2-0101", Some(0.9)),
            result(&TC_RT, "trace_correct", "EV2-0201", Some(0.2)),
            result(&TC_RC, "trace_correct", "EV2-0102", Some(0.8)),
            result(&TC_RC, "trace_correct", "EV2-0202", Some(0.1)),
            result(&TC_RTC, "trace_correct", "EV2-0103", Some(0.9)),
            result(&TC_RTC, "trace_correct", "EV2-0203", Some(0.3)),
            result(&TC_RTC, "trace_correct", "EV2-0204", Some(0.1)),
        ],
        ..variant::RunOutput::default()
    };
    let tc: Vec<&Variant> = TC_FAMILY.iter().collect();
    let pooled = intent::bar_d(&rows, &output, &tc, "trace_correct").unwrap();
    assert_eq!(
        pooled,
        intent::BarD {
            pairs: 3,
            successes: 3,
            failures: 0,
            ties: 0,
            source_not_yes: 0,
            unpairable: 0,
            p_value: 0.125,
        }
    );
    let rt_only = intent::bar_d(&rows, &output, &[&TC_RT], "trace_correct").unwrap();
    assert_eq!((rt_only.pairs, rt_only.successes), (1, 1));

    let report = render_gated_run(&rows, &output, &tc);
    assert!(
        report.contains(
            "Bar D, TC-RT@v1 + TC-RC@v1 + TC-RTC@v1 on `trace_correct`: 3 pairs, 3 successes, 0 \
             failures, 0 ties; one-sided sign test p = 0.1250; not gateable"
        ),
        "{report}"
    );
}

/// Provenance: PLAT-1030, MP-242 Bar D, PR #623 re-review L4. A mutant whose
/// `source_id` names another mutant stops D and names both rows.
#[test]
fn tc_1030_bar_d_refuses_a_mutant_as_source() {
    let rows = parse(&[
        row(
            "EV2-0101",
            Mode::ReqTestCode,
            &json!({"test_asserts_intent": truth(true)}),
        ),
        mutant_row(
            "EV2-0201",
            Mode::ReqTestCode,
            "M-1",
            "test_weakening",
            Some("EV2-0101"),
        ),
        mutant_row(
            "EV2-0202",
            Mode::ReqTestCode,
            "M-2",
            "test_weakening",
            Some("EV2-0201"),
        ),
    ])
    .rows;
    let error = intent::bar_d(
        &rows,
        &variant::RunOutput::default(),
        &[&T1],
        "test_asserts_intent",
    )
    .unwrap_err();
    assert_eq!(error, "EV2-0202: source EV2-0201 is itself a mutant");
}
