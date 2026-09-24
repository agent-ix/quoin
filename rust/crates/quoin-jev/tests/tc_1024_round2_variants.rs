// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1024 round 2, offline: T3 (assertion selection, MP-242 "Round 2")
//! and E5 (per-unit outcome necessity, MP-241 "Round 2").
//!
//! No network and no key. T3's assertion extraction and derive rule, the
//! empty-assertion case with no call, and T3's wording per mode; E5's
//! structural triviality filter, its derive rule and breaker, its asks, its
//! wording per mode, and its report section end to end over a fake Jev.
//!
//! Provenance: PLAT-1024, MP-241, MP-242.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod eval_v2_support;
mod gap_semantic_support;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use regex::Regex;
use serde_json::{Value, json};
use typesafe_sdk_env::Fixed;
use typesafe_sdk_error::Result as SdkResult;
use typesafe_sdk_headers::Headers;
use typesafe_sdk_http::{RawResponse, Request, Transport};

use eval_v2_support::corpus::Row;
use eval_v2_support::fixtures::{self, CODE_BODY, TEST_BODY};
use eval_v2_support::keys::Mode;
use eval_v2_support::units::{split_python_units, split_rust_units};
use eval_v2_support::variant::{
    self, Answered, Prediction, RawAnswer, RawAnswers, Variant, wording_violations,
};
use eval_v2_support::variants::exceeds::{
    self, E5, NECESSARY_KEY, NecessityReading, RelationOutcome, TrivialReason, UNIT_TEXT_FIELD,
    assess_necessity, necessity_outcome, trivial_reason,
};
use eval_v2_support::variants::intent::{
    ASSERTION_CHOICE, ASSERTIONS_FIELD, T3, derive_assertion_selection, extract_assertions,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// One synthetic row of `mode` (the shared fixture row), with its test body
/// replaced by `test_body` when given.
fn row_with(id: &str, mode: Mode, test_body: Option<&str>) -> Row {
    let mut value = fixtures::row(id, mode, "dev", &json!({}));
    if let (Some(body), Some(test)) = (
        test_body,
        value.get_mut("test").and_then(Value::as_object_mut),
    ) {
        test.insert("body".to_owned(), Value::from(body));
    }
    fixtures::parse(&[value]).unwrap().rows.remove(0)
}

fn four_modes() -> Vec<Row> {
    fixtures::four_modes().unwrap().rows
}

fn close(actual: Option<f64>, expected: f64) -> bool {
    actual.is_some_and(|actual| (actual - expected).abs() < 1e-9)
}

/// A fake Jev. T3's choice gets `P(none) = none_p` spread evenly over the
/// other labels; E5's noul is 0.2 for a unit whose text holds `4096` and 0.9
/// for any other. Every call is counted.
struct FakeJev {
    none_p: f64,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for FakeJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let mut answers = serde_json::Map::new();
        for (key, question) in body["questions"].as_object().unwrap() {
            let answer = if key == NECESSARY_KEY {
                let unit = body["state"][UNIT_TEXT_FIELD].as_str().unwrap();
                json!({"type": "noul", "noul": if unit.contains("4096") { 0.2 } else { 0.9 }})
            } else if key == ASSERTION_CHOICE {
                let labels: Vec<String> = question["criteria"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect();
                let others = f64::from(u32::try_from(labels.len() - 1).unwrap());
                let probabilities: serde_json::Map<String, Value> = labels
                    .iter()
                    .map(|label| {
                        let p = if label == "none" {
                            self.none_p
                        } else {
                            (1.0 - self.none_p) / others
                        };
                        (label.clone(), json!(p))
                    })
                    .collect();
                json!({"type": "choice", "choice": "none", "confidence": self.none_p,
                       "probabilities": probabilities})
            } else {
                panic!("the fake has no answer for {key}")
            };
            answers.insert(key.clone(), answer);
        }
        Ok(RawResponse {
            status: 200,
            headers: Headers::new(),
            body: json!({"model": "jev-fake", "answers": answers,
                         "usage": {"input_tokens": 1, "output_tokens": 1}})
            .to_string(),
        })
    }
}

fn fake_client(none_p: f64) -> (typesafe_sdk_client::Client, Arc<FakeJev>) {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(FakeJev {
        none_p,
        calls: AtomicUsize::new(0),
    });
    (
        quoin_jev::client::with_transport(config, fake.clone()),
        fake,
    )
}

// ---------------------------------------------------------------------------
// T3: extraction
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1024, MP-242 round 2. Every Rust assertion form is one
/// statement, whole and in order: an `.unwrap_err()` statement from its
/// `let`, multi-line macros, `assert!(matches!(..))`, `prop_assert*!`,
/// `.expect_err(..)`, a tail `debug_assert!` with no `;`. An assertion inside
/// another is part of it; a commented-out one and one inside a string are
/// not assertions.
#[test]
fn tc_1024_t3_extracts_every_rust_assertion_statement() {
    let body = "#[test]\nfn t() {\n    // assert!(commented_out);\n    \
                let msg = \"assert_eq!(in_a_string, 1)\";\n    \
                let err = parse(\"\")\n        .unwrap_err();\n    \
                assert_eq!(err.kind, Kind::Empty);\n    \
                assert!(matches!(run(), Ok(Outcome { code: 0, .. })));\n    \
                assert_ne!(\n        a,\n        b\n    );\n    \
                prop_assert_eq!(x, y);\n    \
                let e = f().expect_err(\"must fail\");\n    \
                assert_eq!(g().unwrap_err(), E::G);\n    \
                debug_assert!(z)\n}";
    assert_eq!(
        extract_assertions("tests/t.rs", body),
        [
            "let err = parse(\"\") .unwrap_err();",
            "assert_eq!(err.kind, Kind::Empty);",
            "assert!(matches!(run(), Ok(Outcome { code: 0, .. })));",
            "assert_ne!( a, b );",
            "prop_assert_eq!(x, y);",
            "let e = f().expect_err(\"must fail\");",
            "assert_eq!(g().unwrap_err(), E::G);",
            "debug_assert!(z)",
        ]
    );
    assert_eq!(
        extract_assertions("tests/tc_001.rs", TEST_BODY),
        [
            "let error = check(\"x\".repeat(5000)).unwrap_err();",
            "assert_eq!(error.code, Code::Refused);",
        ]
    );
}

/// Provenance: PLAT-1024, MP-242 round 2. Python: `assert` statements
/// (with a `\` continuation), unittest `self.assert*(..)` across lines, and
/// `pytest.raises` from its `with`; a comment or a string is not one.
#[test]
fn tc_1024_t3_extracts_every_python_assertion_statement() {
    let body = "def test_x(self):\n    # assert commented\n    \
                s = \"assert in string\"\n    assert parse(\"\") == []\n    \
                self.assertEqual(\n        total, 3)\n    \
                with pytest.raises(ValueError):\n        parse(None)\n    \
                assert ok, \\\n        \"message\"\n";
    assert_eq!(
        extract_assertions("tests/test_x.py", body),
        [
            "assert parse(\"\") == []",
            "self.assertEqual( total, 3)",
            "with pytest.raises(ValueError):",
            "assert ok, \\ \"message\"",
        ]
    );
}

// ---------------------------------------------------------------------------
// T3: the derive rule and the empty case
// ---------------------------------------------------------------------------

fn selection(label: &str, probabilities: &[(&str, f64)]) -> RawAnswers {
    RawAnswers::from([(
        ASSERTION_CHOICE.to_owned(),
        RawAnswer::Choice {
            label: label.to_owned(),
            confidence: 0.5,
            probabilities: probabilities
                .iter()
                .map(|(label, p)| ((*label).to_owned(), *p))
                .collect(),
        },
    )])
}

/// `(label probabilities, expected answer, confidence, ordinal)`.
type SelectionCase = (&'static [(&'static str, f64)], &'static str, f64, f64);

/// Provenance: PLAT-1024, MP-242 round 2. `P(any) = 1 - P(none)`; `yes` iff
/// `P(any) >= 0.5`; confidence `P(any)` for `yes`, `1 - P(any)` for `no`;
/// the ordinal is `P(any)`. Mass split across several assertions still
/// counts: 0.3 + 0.2 is 0.5, `yes`.
#[test]
fn tc_1024_t3_yes_is_one_minus_p_none() {
    let cases: [SelectionCase; 4] = [
        (&[("A1", 0.3), ("A2", 0.2), ("none", 0.5)], "yes", 0.5, 0.5),
        (
            &[("A1", 0.1), ("A2", 0.39), ("none", 0.51)],
            "no",
            0.51,
            0.49,
        ),
        (&[("A1", 0.7), ("A2", 0.2), ("none", 0.1)], "yes", 0.9, 0.9),
        (
            &[("A1", 0.0), ("A2", 0.05), ("none", 0.95)],
            "no",
            0.95,
            0.05,
        ),
    ];
    for (probabilities, answer, confidence, ordinal) in cases {
        let prediction = derive_assertion_selection(2, &selection("A1", probabilities)).unwrap();
        assert_eq!(prediction.answer, answer, "{probabilities:?}");
        assert!(
            close(prediction.confidence, confidence),
            "{probabilities:?}: {prediction:?}"
        );
        assert!(
            close(prediction.ordinal, ordinal),
            "{probabilities:?}: {prediction:?}"
        );
    }
}

/// Provenance: PLAT-1024, MP-242 rule 6 (malformed answers fail loudly). A
/// missing choice, a noul in its place, a label outside the offered ones, a
/// distribution missing `none`, and one that does not total 1 are errors.
#[test]
fn tc_1024_t3_a_malformed_answer_is_an_error() {
    let fine = [("A1", 0.6), ("A2", 0.1), ("none", 0.3)];
    let noul = RawAnswers::from([(ASSERTION_CHOICE.to_owned(), RawAnswer::Noul(0.4))]);
    let cases = [
        (RawAnswers::new(), "no answer in the response"),
        (noul, "expected a choice"),
        (selection("A3", &fine), "unknown label \"A3\""),
        (
            selection("A1", &[("A1", 0.6), ("A2", 0.4)]),
            "no probability for \"none\"",
        ),
        (
            selection("A1", &[("A1", 0.2), ("A2", 0.1), ("none", 0.1)]),
            "totals",
        ),
    ];
    for (answers, expected) in cases {
        let error = derive_assertion_selection(2, &answers).unwrap_err();
        assert!(error.contains(expected), "{expected:?}: got {error:?}");
    }
}

/// Provenance: PLAT-1024, MP-242 round 2. A test with no assertion is `no`
/// in code: nothing is asked, `P(any) = 0` and the confidence is 1, and a
/// run over it sends no request.
#[tokio::test]
async fn tc_1024_t3_a_test_with_no_assertion_is_no_without_a_call() {
    let row = row_with(
        "EV2-0011",
        Mode::ReqTest,
        Some("#[test]\nfn t() {\n    let _ = run(); // assert!(later)\n}"),
    );
    assert!((T3.asks)(&row).is_empty());
    assert_eq!(
        (T3.derive)(&row, &[]).get("test_asserts_intent"),
        Some(&Prediction {
            answer: "no".to_owned(),
            confidence: Some(1.0),
            ordinal: Some(0.0),
        })
    );
    let (client, fake) = fake_client(0.1);
    let output = variant::run(&client, std::slice::from_ref(&row), &[&T3])
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        output.results[0].predictions["test_asserts_intent"].answer,
        "no"
    );

    // Not vacuous: the fixture's test, with two assertions, is asked, and a
    // low P(none) makes it `yes`.
    let asserted = row_with("EV2-0012", Mode::ReqTest, None);
    let output = variant::run(&client, &[asserted], &[&T3]).await.unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        output.results[0].predictions["test_asserts_intent"].answer,
        "yes"
    );
}

// ---------------------------------------------------------------------------
// T3: wording per mode
// ---------------------------------------------------------------------------

fn question_text(variant: &Variant, row: &Row) -> String {
    (variant.asks)(row)
        .iter()
        .map(|ask| serde_json::to_string(&ask.request.questions).unwrap())
        .collect()
}

/// Provenance: PLAT-1024, MP-242 round 2 (RT never mentions code). T3 runs
/// on RT and RTC only and asks the same thing in both; its question text
/// never names code; its state lists the assertions and carries code only
/// in RTC. Misapplied to an RC row, the wording rule flags it.
#[test]
fn tc_1024_t3_never_mentions_code() {
    let rows = four_modes();
    let (r, rt, rc, rtc) = (&rows[0], &rows[1], &rows[2], &rows[3]);
    assert!(!T3.applies_to(r) && T3.applies_to(rt) && !T3.applies_to(rc) && T3.applies_to(rtc));
    let code_words = Regex::new(r"(?i)\bcode\b|symbol_|implementation").unwrap();
    for row in [rt, rtc] {
        assert!(wording_violations(&T3, row).is_empty());
        let text = question_text(&T3, row);
        assert!(!code_words.is_match(&text), "{text}");
    }
    assert_eq!(question_text(&T3, rt), question_text(&T3, rtc));

    let state = |row: &Row| serde_json::to_value(&(T3.asks)(row)[0].request.state).unwrap();
    let rt_state = state(rt);
    assert_eq!(
        rt_state[ASSERTIONS_FIELD],
        json!({"A1": "let error = check(\"x\".repeat(5000)).unwrap_err();",
               "A2": "assert_eq!(error.code, Code::Refused);"})
    );
    assert!(rt_state.get("symbol_body").is_none(), "{rt_state}");
    assert_eq!(state(rtc)["symbol_body"], CODE_BODY);

    let misapplied = Variant {
        modes: &Mode::ALL,
        ..T3
    };
    let found = wording_violations(&misapplied, rc);
    assert!(found.iter().any(|v| v.contains("the test")), "{found:#?}");
}

// ---------------------------------------------------------------------------
// E5: the structural triviality filter
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1024, MP-241 round 2. E5 skips, with no call: E1's
/// pass-through units; branches that only log; branches that only pass on
/// an error they were given; and size caps (an upper bound whose body
/// refuses). A refusal without a bound, a bound whose body is not a refusal,
/// a counter and a function unit are all asked about.
#[test]
fn tc_1024_e5_skips_only_trivial_units() {
    let arms = split_rust_units(
        "fn f(x: Result<u8, E>, name: &str) -> Result<u8, E> {\n    match x {\n        \
         Err(e) => return Err(e.into()),\n        \
         Ok(0) => { log::debug!(\"zero\"); tracing::info!(n = 0); }\n        \
         Ok(n) if n > 64 => Err(E::too_big(n)),\n        \
         Ok(n) if name == \"__reserved__\" => Err(E::reserved()),\n        \
         Ok(n) if name.len() > 64 => Ok(shorten(n)),\n        \
         Ok(n) => { COUNTER.fetch_add(1, Relaxed); Ok(n) }\n        \
         _ => Ok(()),\n    }\n}",
    );
    let reasons: Vec<Option<TrivialReason>> = arms
        .iter()
        .map(|unit| trivial_reason("src/f.rs", unit))
        .collect();
    assert_eq!(
        reasons,
        [
            Some(TrivialReason::ErrorPlumbing),
            Some(TrivialReason::Logging),
            Some(TrivialReason::SizeCap),
            None,
            None,
            None,
            Some(TrivialReason::PassThrough),
        ]
    );

    let blocks = split_rust_units(
        "fn g(x: &[u8]) -> Result<(), Error> {\n    if x.len() >= MAX_BYTES {\n        \
         eprintln!(\"big\");\n        return Err(Error::refused());\n    } else {\n        \
         eprintln!(\"ok\")\n    }\n}",
    );
    assert_eq!(
        blocks
            .iter()
            .map(|unit| trivial_reason("src/g.rs", unit))
            .collect::<Vec<_>>(),
        [Some(TrivialReason::SizeCap), Some(TrivialReason::Logging)]
    );

    let functions = split_rust_units("fn a() { eprintln!(\"a\"); }\nfn b() { eprintln!(\"b\"); }");
    assert!(
        functions
            .iter()
            .all(|unit| trivial_reason("src/a.rs", unit).is_none()),
        "a function unit is never trivial beyond E1's rule"
    );

    let clauses = split_python_units(
        "def f(x):\n    try:\n        return parse(x)\n    except ValueError as e:\n        \
         raise ConfigError(\"bad\") from e\n    if len(x) > 4096:\n        raise TooBig()\n    \
         elif x == \"__reserved__\":\n        raise Reserved()\n    else:\n        \
         logger.debug(\"fine\")\n",
    );
    assert_eq!(
        clauses
            .iter()
            .map(|unit| trivial_reason("src/f.py", unit))
            .collect::<Vec<_>>(),
        [
            None,
            Some(TrivialReason::ErrorPlumbing),
            Some(TrivialReason::SizeCap),
            None,
            Some(TrivialReason::Logging),
        ]
    );
}

// ---------------------------------------------------------------------------
// E5: the derive rule and the breaker
// ---------------------------------------------------------------------------

fn readings(necessary: &[f64]) -> Vec<NecessityReading> {
    necessary
        .iter()
        .enumerate()
        .map(|(unit, necessary)| NecessityReading {
            unit,
            necessary: *necessary,
        })
        .collect()
}

fn decided(outcome: &RelationOutcome) -> &Prediction {
    match outcome {
        RelationOutcome::Decided(prediction) => prediction,
        other @ RelationOutcome::TraceSuspect { .. } => panic!("expected decided, got {other:?}"),
    }
}

/// Provenance: PLAT-1024, MP-241 round 2. `yes` iff some asked unit has
/// `P(necessary) < 0.5` (0.5 itself is necessary); the ordinal is the
/// highest `1 - P(necessary)`, the confidence that for `yes` and one minus
/// it for `no`; nothing asked is `no` at ordinal 0. With two or more units
/// asked and all unnecessary, the breaker abstains; one unnecessary unit
/// alone is `yes`, not the breaker.
#[test]
fn tc_1024_e5_yes_when_a_unit_is_unnecessary() {
    let cases: [(&[f64], &str, f64, f64); 5] = [
        (&[0.9, 0.3], "yes", 0.7, 0.7),
        (&[0.9, 0.6], "no", 0.6, 0.4),
        (&[0.5, 0.8], "no", 0.5, 0.5),
        (&[0.3], "yes", 0.7, 0.7),
        (&[], "no", 1.0, 0.0),
    ];
    for (necessary, answer, confidence, ordinal) in cases {
        let outcome = necessity_outcome(&readings(necessary));
        let prediction = decided(&outcome);
        assert_eq!(prediction.answer, answer, "{necessary:?}");
        assert!(
            close(prediction.confidence, confidence),
            "{necessary:?}: {prediction:?}"
        );
        assert!(
            close(prediction.ordinal, ordinal),
            "{necessary:?}: {prediction:?}"
        );
    }
    assert_eq!(
        necessity_outcome(&readings(&[0.2, 0.4])),
        RelationOutcome::TraceSuspect {
            unrelated: 2,
            considered: 2,
        }
    );
}

fn unit_noul(unit: usize, answer: RawAnswer) -> Answered {
    Answered {
        unit: Some(unit),
        answers: RawAnswers::from([(NECESSARY_KEY.to_owned(), answer)]),
    }
}

/// Provenance: PLAT-1024, MP-241 round 2, and rule 6 (malformed answers
/// fail loudly). A missing, mis-shaped or out-of-range answer names the row
/// and the unit; E5's derive stops the run on it.
#[test]
fn tc_1024_e5_a_malformed_answer_is_an_error() {
    let row = row_with("EV2-0021", Mode::ReqCode, None);
    let good = unit_noul(0, RawAnswer::Noul(0.9));
    let error = |answer: Answered| assess_necessity(&row, &[good.clone(), answer]).unwrap_err();
    assert_eq!(
        error(Answered {
            unit: Some(1),
            answers: RawAnswers::new(),
        }),
        "EV2-0021 unit 1: `unit_necessary` is missing from the answers"
    );
    assert!(error(unit_noul(1, RawAnswer::Noul(1.2))).contains("not a probability"));
    let choice = RawAnswer::Choice {
        label: "yes".to_owned(),
        confidence: 1.0,
        probabilities: std::collections::BTreeMap::new(),
    };
    assert!(error(unit_noul(1, choice)).starts_with("EV2-0021 unit 1:"));
    let result =
        std::panic::catch_unwind(|| (E5.derive)(&row, &[unit_noul(0, RawAnswer::Noul(-0.1))]));
    assert!(
        result.is_err(),
        "E5's derive must stop on an unreadable answer"
    );
}

// ---------------------------------------------------------------------------
// E5: asks, wording, and the report
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1024, MP-241 round 2. E5 asks one question per unit it
/// does not skip, with the whole body in `symbol_body` and the unit in
/// `code_unit_text`; it runs on RC and RTC, declares the code, and never
/// names a test; misapplied to R or RT, the wording rule flags it.
#[test]
fn tc_1024_e5_asks_each_unit_against_the_whole_body() {
    let rows = four_modes();
    let rc = &rows[2];
    let asks = (E5.asks)(rc);
    assert_eq!(
        asks.iter().map(|ask| ask.unit).collect::<Vec<_>>(),
        [Some(0), Some(1)],
        "`_ => Ok(())` is skipped"
    );
    let state = serde_json::to_value(&asks[1].request.state).unwrap();
    assert_eq!(state["symbol_body"], CODE_BODY);
    assert_eq!(
        state[UNIT_TEXT_FIELD],
        "n if n > 4096 => { log(\"too big }\"); Err(Error::refused()) }"
    );
    assert!(
        state["code_unit"]
            .as_str()
            .unwrap()
            .starts_with("match arm")
    );

    for row in &rows {
        assert_eq!(E5.applies_to(row), row.mode.has_code());
        let misapplied = Variant {
            modes: &Mode::ALL,
            ..E5
        };
        let violations = wording_violations(&misapplied, row);
        assert_eq!(
            violations.is_empty(),
            row.mode.has_code(),
            "{}: {violations:#?}",
            row.mode.as_str()
        );
        assert!(!violations.iter().any(|v| v.contains("the test")));
    }
}

/// Provenance: PLAT-1024, MP-241 round 2. E5 runs end to end: two calls on
/// the fixture's RC row, the `4096` arm read as unnecessary, the row `yes`,
/// and the diagnostics carry E5's unit table and its Bar D line.
#[tokio::test]
async fn tc_1024_e5_runs_end_to_end() {
    let value = fixtures::row(
        "EV2-0031",
        Mode::ReqCode,
        "dev",
        &json!({"code_exceeds_requirement": fixtures::truth(&json!(true), "by_construction", &[])}),
    );
    let rows = fixtures::parse(&[value]).unwrap().rows;
    let (client, fake) = fake_client(0.5);
    let output = variant::run(&client, &rows, &[&E5]).await.unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 2);
    let prediction = &output.results[0].predictions[exceeds::KEY];
    assert_eq!(prediction.answer, "yes");
    assert!(close(prediction.ordinal, 0.8), "{prediction:?}");

    let report = exceeds::render_diagnostics(&rows, &output);
    for needle in [
        "#### E5@v1: units and the circuit breaker",
        "| 1 | 1 | 0 | pass-through 1 | 2 | 1 |",
        "#### E5@v1: on the rows it answered (bars A and B)",
        "Bar D, E5@v1:",
    ] {
        assert!(report.contains(needle), "missing {needle:?} in:\n{report}");
    }
}
