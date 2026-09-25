// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1024 round 2, offline: T3 (assertion selection, MP-242 "Round 2")
//! and E5 (per-unit outcome necessity, MP-241 "Round 2").
//!
//! No network and no key. T3's assertion extraction and derive rule, the
//! empty-assertion case with no call, and T3's wording per mode; E5's
//! structural triviality filter and statement units, its derive rule, its asks, its
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
    self, E5, NECESSARY_KEY, NecessityReading, TrivialReason, UNIT_TEXT_FIELD, assess_necessity,
    e5_units, necessity_outcome, trivial_reason,
};
use eval_v2_support::variants::intent::{
    ASSERTIONS_FIELD, T3, derive_assertion_selection, extract_assertions,
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

/// A fake Jev. Each of T3's per-assertion nouls (`A1` ..) answers
/// `assert_p`; E5's noul is 0.2 for a unit whose text holds `4096` and 0.9
/// for any other. Every call is counted.
struct FakeJev {
    assert_p: f64,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for FakeJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let mut answers = serde_json::Map::new();
        for key in body["questions"].as_object().unwrap().keys() {
            let p = if key == NECESSARY_KEY {
                let unit = body["state"][UNIT_TEXT_FIELD].as_str().unwrap();
                if unit.contains("4096") { 0.2 } else { 0.9 }
            } else if key.starts_with('A') {
                self.assert_p
            } else {
                panic!("the fake has no answer for {key}")
            };
            answers.insert(key.clone(), json!({"type": "noul", "noul": p}));
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

fn fake_client(assert_p: f64) -> (typesafe_sdk_client::Client, Arc<FakeJev>) {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(FakeJev {
        assert_p,
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
    // v2: two arms' assertions stay two statements.
    let arms = "match found {\n    Some(d) => assert_eq!(&d, want, \"{f}\"),\n    \
                None => assert_eq!(got, Value::Null),\n}";
    assert_eq!(
        extract_assertions("tests/t.rs", arms),
        [
            "assert_eq!(&d, want, \"{f}\")",
            "assert_eq!(got, Value::Null)"
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

/// Provenance: PLAT-1024, MP-242 round 2 (PR #630 F1). A brace-delimited
/// assert macro needs no `;`, so it ends at its own closing brace; it does
/// not run on into the statement after it.
#[test]
fn tc_1024_t3_a_brace_macro_statement_ends_at_its_brace() {
    let body = "fn t() {\n    assert_matches! { value, Some(_) }\n    let next = 1;\n    \
                assert!(next == 1);\n}";
    assert_eq!(
        extract_assertions("tests/t.rs", body),
        ["assert_matches! { value, Some(_) }", "assert!(next == 1);"]
    );
}

/// Provenance: PLAT-1024, MP-242 round 2, T3 v3 (PR #630 F2). A `panic!` is
/// an assertion where it is the failure a check leads to: a match arm, with
/// or without a block, a `let .. else`, an `if` block, or a statement of its
/// own. A proptest `return Err(TestCaseError::fail(..))` likewise. A `panic!`
/// inside a closure is an `.expect(..)` spelled out, and neither it nor
/// `.unwrap()` / `.expect(..)` is listed.
#[test]
fn tc_1024_t3_extracts_failure_points() {
    let body = "fn t() {\n    let parsed = parse(\"x\").expect(\"parses\");\n    \
                let n = count().unwrap();\n    \
                let v = load().unwrap_or_else(|e| panic!(\"load: {e}\"));\n    \
                match parsed {\n        Ok(Kind::A) => {}\n        \
                Err(e) => panic!(\"unexpected {e}\"),\n        \
                _ => { panic!(\"wrong kind\") }\n    }\n    \
                let Some(first) = v.first() else { panic!(\"empty\") };\n    \
                if n != 3 {\n        return Err(TestCaseError::fail(\"n\"));\n    }\n    \
                panic!(\"always\");\n}";
    assert_eq!(
        extract_assertions("tests/t.rs", body),
        [
            "Err(e) => panic!(\"unexpected {e}\")",
            "_ => { panic!(\"wrong kind\") }",
            "let Some(first) = v.first() else { panic!(\"empty\") };",
            "if n != 3 { return Err(TestCaseError::fail(\"n\")); }",
            "panic!(\"always\");",
        ]
    );
    // A test whose only check is `.expect(..)` has no assertion: `no`, no
    // call.
    let expect_only = "fn t() {\n    validator().expect(\"the schema compiles\");\n}";
    assert!(extract_assertions("tests/t.rs", expect_only).is_empty());
    let row = row_with("EV2-0013", Mode::ReqTest, Some(expect_only));
    assert!((T3.asks)(&row).is_empty());
    assert_eq!((T3.derive)(&row, &[])["test_asserts_intent"].answer, "no");
}

/// Provenance: PLAT-1024, MP-242 round 2, T3 v3 (PR #630 F2). Python: a
/// mock's `assert_called*` / `assert_not_called`, `np.testing.assert_*`,
/// and an `assert` after a `:` on the same line are assertions; a dict
/// value named `assert_x` is not.
#[test]
fn tc_1024_t3_extracts_python_mock_numpy_and_inline_asserts() {
    let body = "def test_x(mock_send):\n    cfg = {\"k\": assert_x}\n    run()\n    \
                mock_send.assert_called_once_with(\"a\", 1)\n    \
                self.client.post.assert_not_called()\n    \
                np.testing.assert_allclose(\n        got, want)\n    \
                for item in items: assert item.ok\n";
    assert_eq!(
        extract_assertions("tests/test_x.py", body),
        [
            "mock_send.assert_called_once_with(\"a\", 1)",
            "self.client.post.assert_not_called()",
            "np.testing.assert_allclose( got, want)",
            "for item in items: assert item.ok",
        ]
    );
}

/// Provenance: PLAT-1024, MP-242 round 2, T3 v3 (PR #630 F3). Whitespace is
/// collapsed only in code: a string literal's spaces and line breaks stay
/// verbatim, and a line comment keeps the line break that ends it.
#[test]
fn tc_1024_t3_keeps_literals_verbatim() {
    let body = "fn t() {\n    assert_eq!(\n        render(),\n        \
                \"a  b\\n    c\", // two spaces\n        \"\"\n    );\n}";
    assert_eq!(
        extract_assertions("tests/t.rs", body),
        ["assert_eq!( render(), \"a  b\\n    c\", // two spaces\n\"\" );"]
    );
    let multiline = "fn t() {\n    assert_eq!(out, \"line one\n    line two\");\n}";
    assert_eq!(
        extract_assertions("tests/t.rs", multiline),
        ["assert_eq!(out, \"line one\n    line two\");"]
    );
    let python = "def test_x():\n    assert out == \"a   b\"  # spaced\n";
    assert_eq!(
        extract_assertions("tests/test_x.py", python),
        ["assert out == \"a   b\" # spaced"]
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

fn nouls(answers: &[(&str, f64)]) -> RawAnswers {
    answers
        .iter()
        .map(|(label, p)| ((*label).to_owned(), RawAnswer::Noul(*p)))
        .collect()
}

/// `(per-assertion P, expected answer, confidence, ordinal)`.
type SelectionCase = (&'static [(&'static str, f64)], &'static str, f64, f64);

/// Provenance: PLAT-1024, MP-242 round 2 (T3 v2). `P(any)` is the highest
/// per-assertion `P`; `yes` iff `P(any) >= 0.5`; confidence `P(any)` for
/// `yes`, `1 - P(any)` for `no`; the ordinal is `P(any)`. Two middling
/// assertions do not add up: 0.4 and 0.4 is `no`.
#[test]
fn tc_1024_t3_yes_is_the_strongest_assertion() {
    let cases: [SelectionCase; 4] = [
        (&[("A1", 0.3), ("A2", 0.5)], "yes", 0.5, 0.5),
        (&[("A1", 0.4), ("A2", 0.4)], "no", 0.6, 0.4),
        (&[("A1", 0.9), ("A2", 0.2)], "yes", 0.9, 0.9),
        (&[("A1", 0.49), ("A2", 0.05)], "no", 0.51, 0.49),
    ];
    for (answers, answer, confidence, ordinal) in cases {
        let prediction = derive_assertion_selection(2, &nouls(answers)).unwrap();
        assert_eq!(prediction.answer, answer, "{answers:?}");
        assert!(
            close(prediction.confidence, confidence),
            "{answers:?}: {prediction:?}"
        );
        assert!(
            close(prediction.ordinal, ordinal),
            "{answers:?}: {prediction:?}"
        );
    }
}

/// Provenance: PLAT-1024, MP-242 rule 6 (malformed answers fail loudly). A
/// missing assertion answer, a choice in its place, a probability outside
/// [0, 1], and an answer to an assertion that was not asked are errors.
#[test]
fn tc_1024_t3_a_malformed_answer_is_an_error() {
    let mut as_choice = nouls(&[("A1", 0.6)]);
    as_choice.insert(
        "A2".to_owned(),
        RawAnswer::Choice {
            label: "yes".to_owned(),
            confidence: 0.5,
            probabilities: std::collections::BTreeMap::new(),
        },
    );
    let cases = [
        (nouls(&[("A1", 0.6)]), "`A2`: no answer in the response"),
        (as_choice, "`A2`: expected a noul"),
        (
            nouls(&[("A1", 0.6), ("A2", 1.2)]),
            "`A2`: probability 1.2 is outside [0, 1]",
        ),
        (
            nouls(&[("A1", 0.6), ("A2", 0.1), ("A3", 0.9)]),
            "`A3`: answered, but no such assertion was asked",
        ),
    ];
    for (answers, expected) in cases {
        let error = derive_assertion_selection(2, &answers).unwrap_err();
        assert!(error.starts_with(expected), "{expected:?}: got {error:?}");
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
    let (client, fake) = fake_client(0.9);
    let output = variant::run(&client, std::slice::from_ref(&row), &[&T3])
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        output.results[0].predictions["test_asserts_intent"].answer,
        "no"
    );

    // Not vacuous: the fixture's test, with two assertions, is asked, and a
    // high per-assertion P makes it `yes`.
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

/// Provenance: PLAT-1024, MP-241 round 2, E5 v4 (PR #630 F4). A lone `if`
/// is one statement unit, and the triviality checks read it as the branch it
/// is: MP-241's own size-cap example is skipped, and so is a lone `if` that
/// only logs; a lone `if` that refuses without a bound, and one bounded that
/// does something else, are asked about. A statement that passes on an
/// error is skipped as a branch body would be.
#[test]
fn tc_1024_e5_skips_a_lone_if_size_cap() {
    let body = "fn f(x: &[u8]) -> Result<(), Error> {\n    \
                if x.len() > MAX { return Err(Error::too_big()) }\n    \
                if x.is_empty() { return Err(Error::empty()); }\n    \
                if x.len() >= 4096 { shrink(x); }\n    \
                if verbose { tracing::debug!(\"parsed\"); }\n    \
                let e = check(x);\n    \
                return Err(e);\n}";
    let units = e5_units("src/f.rs", body);
    let texts: Vec<&str> = units.iter().map(|unit| unit.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "if x.len() > MAX { return Err(Error::too_big()) }",
            "if x.is_empty() { return Err(Error::empty()); }",
            "if x.len() >= 4096 { shrink(x); }",
            "if verbose { tracing::debug!(\"parsed\"); }",
            "let e = check(x);",
            "return Err(e);",
        ]
    );
    assert_eq!(
        units
            .iter()
            .map(|unit| trivial_reason("src/f.rs", unit))
            .collect::<Vec<_>>(),
        [
            Some(TrivialReason::SizeCap),
            None,
            None,
            Some(TrivialReason::Logging),
            None,
            Some(TrivialReason::ErrorPlumbing),
        ]
    );
}

// ---------------------------------------------------------------------------
// E5: the derive rule
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

/// Provenance: PLAT-1024, MP-241 round 2, exp2. v5: `yes` iff some asked
/// unit has `P(necessary) < 0.3` (0.3 itself is not); the ordinal is the
/// highest `1 - P(necessary)` minus 0.2, so `yes` is `ordinal > 0.5`; the
/// confidence is the ordinal for `yes` and `1 - ordinal` (capped at 1) for
/// `no`, so never below 0.5 on the side answered, even for a unit in
/// [0.3, 0.5); nothing asked is `no` at ordinal -0.2, confidence 1. Every unit unnecessary is `yes`
/// too: v3 has no breaker.
#[test]
fn tc_1024_e5_yes_when_a_unit_is_unnecessary() {
    let cases: [(&[f64], &str, f64, f64); 6] = [
        (&[0.9, 0.2], "yes", 0.6, 0.6),
        (&[0.9, 0.3], "no", 0.5, 0.5),
        (&[0.9, 0.45], "no", 0.65, 0.35),
        (&[0.5, 0.8], "no", 0.7, 0.3),
        (&[0.1], "yes", 0.7, 0.7),
        (&[], "no", 1.0, -0.2),
    ];
    for (necessary, answer, confidence, ordinal) in cases {
        let prediction = necessity_outcome(&readings(necessary));
        assert_eq!(prediction.answer, answer, "{necessary:?}");
        assert!(
            close(prediction.confidence, confidence),
            "{necessary:?}: {prediction:?}"
        );
        assert!(
            close(prediction.ordinal, ordinal),
            "{necessary:?}: {prediction:?}"
        );
        assert!(
            prediction
                .confidence
                .is_some_and(|c| (0.5..=1.0).contains(&c)),
            "{necessary:?}: {prediction:?}"
        );
    }
    // v3 has no breaker: every asked unit unnecessary is still `yes`.
    let prediction = necessity_outcome(&readings(&[0.2, 0.1]));
    assert_eq!(prediction.answer, "yes");
    assert!(close(prediction.ordinal, 0.7), "{prediction:?}");
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

/// Provenance: PLAT-1024, MP-241 round 2 (E5 v2). A Rust function is cut
/// into its top-level statements: an added counter or `if` is its own unit,
/// a statement holding a `match` or an `if`/`else` chain is cut into its
/// branches, a lone `if` stays one unit, and a statement that only logs is
/// skipped. Python keeps the v1 split.
#[test]
fn tc_1024_e5_cuts_a_function_into_statements() {
    let body = "pub fn current() -> Self {\n    \
                static CALLS: AtomicUsize = AtomicUsize::new(0);\n    \
                CALLS.fetch_add(1, Ordering::Relaxed);\n    \
                eprintln!(\"called\");\n    \
                if name.len() > 64 {\n        return Some(\"long\");\n    }\n    \
                let kind = match raw { 0 => Kind::A, _ => Kind::B };\n    \
                if a { one() } else if b { two() } else { three() }\n    \
                Self { kind }\n}";
    let units = e5_units("src/engine.rs", body);
    let texts: Vec<&str> = units.iter().map(|unit| unit.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "static CALLS: AtomicUsize = AtomicUsize::new(0);",
            "CALLS.fetch_add(1, Ordering::Relaxed);",
            "eprintln!(\"called\");",
            "if name.len() > 64 {\n        return Some(\"long\");\n    }",
            "0 => Kind::A",
            "_ => Kind::B",
            "if a { one() }",
            "else if b { two() }",
            "else { three() }",
            "Self { kind }",
        ]
    );
    let skipped: Vec<Option<TrivialReason>> = units
        .iter()
        .map(|unit| trivial_reason("src/engine.rs", unit))
        .collect();
    let logging = Some(TrivialReason::Logging);
    let pass = Some(TrivialReason::PassThrough);
    assert_eq!(
        skipped,
        [
            None, None, logging, None, pass, pass, None, None, None, None
        ]
    );

    let two_fns = e5_units("src/a.rs", "fn a() { x(); y() }\nfn b() { z(); }");
    assert_eq!(
        two_fns.iter().map(|u| u.text.as_str()).collect::<Vec<_>>(),
        ["x();", "y()", "z();"]
    );
    let python = "def f(x):\n    if x:\n        return 1\n    else:\n        return 2\n";
    assert_eq!(e5_units("src/f.py", python), split_python_units(python));
}

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
    let (client, fake) = fake_client(0.9);
    let output = variant::run(&client, &rows, &[&E5]).await.unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 2);
    let prediction = &output.results[0].predictions[exceeds::KEY];
    assert_eq!(prediction.answer, "yes");
    assert!(close(prediction.ordinal, 0.6), "{prediction:?}");

    let report = exceeds::render_diagnostics(&rows, &output);
    for needle in [
        "#### E5@v5: units",
        "| 1 | pass-through 1 | 2 | 1 |",
        "#### E5@v5: on the rows it answered (bars A and B)",
        "Bar D, E5@v5:",
    ] {
        assert!(report.contains(needle), "missing {needle:?} in:\n{report}");
    }
}

/// Provenance: PLAT-1024 exp2. `e_bar_d` scores each additive pair on the
/// combiner's own score: at `any unit P < 0.5` a source already holding an
/// unnecessary unit cannot cross (a tie), while at `>= 2 units` the added
/// unit makes that same pair cross; a source labelled `yes` is excluded.
#[test]
fn tc_1024_exp2_e_bar_d_reads_the_combiner_score() {
    use eval_v2_support::exp2::{ERow, ERule, e_bar_d};
    let natural = |id: &str, exceeds: bool| {
        fixtures::row(
            id,
            Mode::ReqCode,
            "dev",
            &json!({"code_exceeds_requirement": fixtures::truth(&json!(exceeds), "agent_dual", &[])}),
        )
    };
    let mutant = |id: &str, source: &str| {
        let mut value = fixtures::row(
            id,
            Mode::ReqCode,
            "dev",
            &json!({"code_exceeds_requirement": fixtures::truth(&json!(true), "by_construction", &[])}),
        );
        value["mutation"] = json!({"id": format!("M-{id}"), "target": "code",
            "kind": "additive_code", "description": "adds a branch", "patch": "p",
            "source_id": source});
        value
    };
    let rows = fixtures::parse(&[
        natural("EV2-0001", false),
        natural("EV2-0002", false),
        natural("EV2-0003", true),
        mutant("EV2-0011", "EV2-0001"),
        mutant("EV2-0012", "EV2-0002"),
        mutant("EV2-0013", "EV2-0003"),
    ])
    .unwrap()
    .rows;
    let units: [(&str, &[f64]); 6] = [
        ("EV2-0001", &[0.9, 0.9]),
        ("EV2-0011", &[0.9, 0.9, 0.2]),
        ("EV2-0002", &[0.9, 0.2]),
        ("EV2-0012", &[0.9, 0.2, 0.2]),
        ("EV2-0003", &[0.9]),
        ("EV2-0013", &[0.9, 0.2]),
    ];
    let erows: Vec<ERow<'_>> = units
        .iter()
        .map(|(id, p)| ERow {
            row: rows.iter().find(|row| row.id == *id).unwrap(),
            units: p.iter().copied().enumerate().collect(),
        })
        .collect();
    let rule = |at_least, tau| ERule {
        name: format!("{at_least}@{tau}"),
        at_least,
        tau,
    };
    let any = e_bar_d(&rows, &erows, &rule(1, 0.5)).unwrap();
    assert_eq!(any.excluded_source_on_defect_side, 1);
    assert_eq!((any.successes(), any.failures(), any.ties()), (1, 0, 1));
    let two = e_bar_d(&rows, &erows, &rule(2, 0.5)).unwrap();
    assert_eq!((two.successes(), two.failures(), two.ties()), (1, 0, 1));
    let pairs: Vec<(&str, bool)> = two
        .pairs
        .iter()
        .map(|pair| {
            (
                pair.mutant.as_str(),
                pair.verdict == eval_v2_support::metrics::PairVerdict::Success,
            )
        })
        .collect();
    assert_eq!(pairs, [("EV2-0011", false), ("EV2-0012", true)]);
}

// ---------------------------------------------------------------------------
// Experiment 3: the requirement statement's own checks (F0, F1)
// ---------------------------------------------------------------------------

/// A requirement row whose statement section is `statement`.
fn statement_row(id: &str, fr: &str, statement: &str) -> Row {
    let mut value = fixtures::row(id, Mode::Req, "dev", &json!({}));
    value["requirement"]["fr_id"] = json!(fr);
    value["requirement"]["statement"] = json!(statement);
    serde_json::from_value(value).unwrap()
}

/// Provenance: PLAT-1024 exp3. The unit is the statement section's SHALL
/// sentences only, one per line: prose without SHALL, headings, tables,
/// block quotes and fenced blocks are dropped; a paragraph wrapped over lines
/// is one sentence; a list item is its own block.
#[test]
fn tc_1024_statement_unit_keeps_only_the_shall_sentences() {
    use eval_v2_support::variants::statement::shall_sentences;
    let section = "Some background with no obligation. The CLI SHALL print\n\
                   the usage text, falling back to `help` for version 1.2.\n\
                   \n\
                   ### A heading that SHALL be ignored\n\
                   \n\
                   | Table | SHALL be ignored |\n\
                   > A quote that SHALL be ignored.\n\
                   \n\
                   ```\n\
                   fenced text SHALL be ignored\n\
                   ```\n\
                   - `valid`: a list item that shall hold.\n\
                   - another item.\n\
                   \n\
                   Afterwards, `quoin` SHALL exit 0. Nothing else.";
    assert_eq!(
        shall_sentences(section).as_deref(),
        Some(
            "The CLI SHALL print the usage text, falling back to `help` for version 1.2.\n\
             - `valid`: a list item that shall hold.\n\
             Afterwards, `quoin` SHALL exit 0."
        )
    );
    assert_eq!(shall_sentences("No obligation here. None at all."), None);
}

/// Provenance: PLAT-1024 exp3. A stakeholder need or a statement with no
/// SHALL sentence has no unit, so F0 and F1 ask nothing about it; a
/// requirement with one gets one request whose state is its unit.
#[test]
fn tc_1024_statement_variants_ask_only_about_a_unit() {
    use eval_v2_support::variants::statement::{F0, F1, STATEMENT_FIELD, statement_unit};
    let need = statement_row("EV2-0001", "StR-005", "Authors require that it shall run.");
    let prose = statement_row("EV2-0002", "FR-036", "Specs and ADRs declare boundaries.");
    let fr = statement_row("EV2-0003", "FR-041", "`quoin` SHALL read SBOMs.");
    for row in [&need, &prose] {
        assert_eq!(statement_unit(row), None);
        assert!((F0.asks)(row).is_empty() && (F1.asks)(row).is_empty());
    }
    for variant in [&F0, &F1] {
        let asks = (variant.asks)(&fr);
        assert_eq!(asks.len(), 1);
        let state = serde_json::to_value(&asks[0].request.state).unwrap();
        assert_eq!(state[STATEMENT_FIELD], "`quoin` SHALL read SBOMs.");
        assert!(wording_violations(variant, &fr).is_empty());
    }
}

/// Provenance: PLAT-1024 exp3. The combiner: `any >= 0.5` flags on one
/// check, `>= 2` needs a second, and `any >= 0.7` raises the bar; the sound
/// prediction's ordinal is one minus the score.
#[test]
fn tc_1024_statement_combiners() {
    use eval_v2_support::variants::statement::{COMBINERS, F1_COMBINER};
    let [(_, any), (_, two), (_, strict)] = COMBINERS;
    assert_eq!(any, F1_COMBINER);
    let one = [0.6, 0.2, 0.1];
    let both = [0.8, 0.55, 0.0];
    let none = [0.4, 0.3, 0.49];
    assert_eq!(
        [one, both, none].map(|p| [any.flags(&p), two.flags(&p), strict.flags(&p)]),
        [
            [true, false, false],
            [true, true, true],
            [false, false, false]
        ]
    );
    assert!((two.score(&both) - 0.55).abs() < 1e-12);
    let sound = any.sound(&one);
    assert_eq!(sound.answer, "no");
    assert!((sound.ordinal.unwrap() - 0.4).abs() < 1e-12);
    assert_eq!(any.sound(&none).answer, "yes");
}

/// Provenance: PLAT-1024 exp3. F1 grades each check at 0.5 and derives
/// `fr_statement_sound` from them; F0 reads its one noul; a row with no unit
/// gets no answer at all.
#[test]
fn tc_1024_statement_derive() {
    use eval_v2_support::variants::statement::{F0, F1, SOUND, WELL_FORMED};
    let fr = statement_row("EV2-0003", "FR-041", "`quoin` SHALL read SBOMs.");
    let answered = |pairs: &[(&str, f64)]| {
        vec![Answered {
            unit: None,
            answers: pairs
                .iter()
                .map(|(key, p)| ((*key).to_owned(), RawAnswer::Noul(*p)))
                .collect::<RawAnswers>(),
        }]
    };
    let f1 = (F1.derive)(
        &fr,
        &answered(&[
            ("compound_obligation", 0.2),
            ("multiple_readings", 0.7),
            ("names_internal_symbol", 0.1),
        ]),
    );
    let answer = |key: &str| f1[key].answer.clone();
    assert_eq!(answer("multiple_readings"), "yes");
    assert_eq!(answer("compound_obligation"), "no");
    assert_eq!(answer(SOUND), "no");
    let f0 = (F0.derive)(&fr, &answered(&[(WELL_FORMED, 0.8)]));
    assert_eq!(f0[SOUND].answer, "yes");
    assert!((F1.derive)(&fr, &[]).is_empty());
}

/// Provenance: PLAT-1024 exp3. Every row labelled `fr_statement_sound` has a
/// unit, and every statement mutant changes its source's unit: an edit to a
/// sentence without SHALL would be invisible to F0 and F1.
#[test]
fn tc_1024_every_statement_mutant_changes_the_unit() {
    use eval_v2_support::variants::statement::{SOUND, injected, statement_unit};
    let rows = eval_v2_support::corpus::load_in_repo()
        .unwrap()
        .unwrap()
        .file
        .rows;
    let by_id: std::collections::BTreeMap<&str, &Row> =
        rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut mutants = 0;
    for row in rows.iter().filter(|row| row.truth.contains_key(SOUND)) {
        let unit = statement_unit(row).unwrap_or_else(|| panic!("{}: no unit", row.id));
        if injected(row).is_some() {
            mutants += 1;
            let source = by_id[row.mutation.as_ref().unwrap().source_id.as_deref().unwrap()];
            assert_ne!(Some(unit), statement_unit(source), "{}", row.id);
            assert_eq!(row.split, eval_v2_support::corpus::Split::Dev, "{}", row.id);
        }
    }
    assert!(mutants >= 30, "{mutants} statement mutants");
}

/// Provenance: PLAT-1024 exp3. Bar D pairs each statement mutant with its
/// source: a rise across the rule's threshold is a success, a source already
/// labelled defective is dropped, and an unanswered side is a failure.
#[test]
fn tc_1024_statement_bar_d() {
    use eval_v2_support::variants::statement::{CHECKS, SOUND, bar_d, injected};
    let rows = eval_v2_support::corpus::load_in_repo()
        .unwrap()
        .unwrap()
        .file
        .rows;
    let keys: Vec<&str> = CHECKS.iter().map(|check| check.key).collect();
    let mutants: Vec<&Row> = rows.iter().filter(|row| injected(row).is_some()).collect();
    let scores: std::collections::BTreeMap<&str, f64> = rows
        .iter()
        .filter(|row| row.truth.contains_key(SOUND))
        .map(|row| {
            (
                row.id.as_str(),
                if injected(row).is_some() { 0.9 } else { 0.1 },
            )
        })
        .collect();
    let d = bar_d(&rows, &scores, 0.5, &keys, SOUND);
    assert_eq!(d.pairs, mutants.len());
    assert_eq!(d.successes + d.source_already_yes, d.pairs);
    // At a threshold above every score nothing crosses: all ties.
    let high = bar_d(&rows, &scores, 0.95, &keys, SOUND);
    assert_eq!(high.ties, high.pairs - high.source_already_yes);
    // With no answers every kept pair is a failure.
    let none = bar_d(&rows, &std::collections::BTreeMap::new(), 0.5, &keys, SOUND);
    assert_eq!(none.failures, none.pairs - none.source_already_yes);
    assert_eq!(none.abstained, none.failures);
}

/// Provenance: PLAT-1024 exp3, PR #633 review finding 1. The statement
/// mutants run only on a variant graded on a statement key: K1 and E5 see
/// exactly the dev rows they saw before those mutants existed (the counts
/// measured on origin/main at 93112a22: 248 dev rows, 105 in R, 100 in RC or
/// RTC), and a run without an F variant drops the mutants from its rows.
#[test]
fn tc_1024_statement_mutants_cost_other_variants_nothing() {
    use eval_v2_support::corpus::Split;
    use eval_v2_support::variant::rows_for;
    use eval_v2_support::variants::soundness::K1;
    use eval_v2_support::variants::statement::{F0, F1, is_statement_mutant};
    let dev: Vec<Row> = eval_v2_support::corpus::load_in_repo()
        .unwrap()
        .unwrap()
        .file
        .rows
        .into_iter()
        .filter(|row| row.split == Split::Dev)
        .collect();
    let statement_mutants = dev.iter().filter(|row| is_statement_mutant(row)).count();
    assert_eq!(statement_mutants, 40);
    let count = |variant: &Variant| dev.iter().filter(|row| variant.applies_to(row)).count();
    assert_eq!((count(&K1), count(&E5)), (105, 100));
    assert_eq!(rows_for(dev.clone(), &[&K1, &E5]).len(), 248);
    assert_eq!(rows_for(dev.clone(), &[&K1, &F0]).len(), 248 + 40);
    assert!(
        dev.iter()
            .filter(|row| is_statement_mutant(row))
            .all(|row| F1.applies_to(row) && F0.applies_to(row) && !K1.applies_to(row))
    );
}

/// Provenance: PLAT-1024 exp3, PR #633 review finding 6. Every committed
/// statement label carries all four keys with `fr_statement_sound` = `no`
/// exactly when a check fires; a missing key, an inconsistent aggregate, an
/// unknown id or a held-out id is refused, never silently dropped.
#[test]
fn tc_1024_statement_labels_are_complete_or_refused() {
    use eval_v2_support::assemble::{fixture_dir, statement_labels, statement_truth};
    let read = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(fixture_dir().join(name)).unwrap()).unwrap()
    };
    let sample = read("sample.json");
    let labels = read("labels-fr-statement.json")["labels"].clone();
    let checked = statement_labels(&labels, &sample).unwrap();
    assert_eq!(checked.len(), 41);
    assert!(checked.values().all(|truth| truth.len() == 4));
    let entry = |sound: &str, compound: &str| {
        json!({
            "fr_statement_sound": {"answer": sound, "why": "w"},
            "compound_obligation": {"answer": compound, "why": "w"},
            "multiple_readings": {"answer": "no", "why": "w"},
            "names_internal_symbol": {"answer": "no", "why": "w"},
        })
    };
    assert!(statement_truth("N-1", &entry("no", "yes")).is_ok());
    assert!(statement_truth("N-1", &entry("yes", "no")).is_ok());
    assert!(statement_truth("N-1", &entry("yes", "yes")).is_err());
    assert!(statement_truth("N-1", &entry("no", "no")).is_err());
    let mut missing = entry("yes", "no");
    missing.as_object_mut().unwrap().remove("multiple_readings");
    assert!(
        statement_truth("N-1", &missing)
            .unwrap_err()
            .contains("multiple_readings")
    );
    let heldout = sample["split"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, split)| *split == "heldout")
        .map(|(id, _)| id.clone())
        .unwrap();
    for id in ["N-999", heldout.as_str()] {
        let mut bad = labels.clone();
        bad[id] = entry("yes", "no");
        assert!(statement_labels(&bad, &sample).is_err(), "{id} accepted");
    }
}

/// Provenance: PLAT-1024 exp3, PR #633 review finding 5. With one pair per
/// source, a source's pairs count once, by their majority outcome.
#[test]
fn tc_1024_statement_bar_d_counts_each_source_once() {
    use eval_v2_support::variants::statement::{CHECKS, SOUND, bar_d, bar_d_per_source, injected};
    let rows = eval_v2_support::corpus::load_in_repo()
        .unwrap()
        .unwrap()
        .file
        .rows;
    let keys: Vec<&str> = CHECKS.iter().map(|check| check.key).collect();
    let mutants: Vec<&Row> = rows.iter().filter(|row| injected(row).is_some()).collect();
    let sources: std::collections::BTreeSet<&str> = mutants
        .iter()
        .filter_map(|row| row.mutation.as_ref().unwrap().source_id.as_deref())
        .collect();
    // Every mutant rises from 0.1 to 0.9, except the compound ones, which fall.
    let scores: std::collections::BTreeMap<&str, f64> = rows
        .iter()
        .filter(|row| row.truth.contains_key(SOUND))
        .map(|row| {
            let score = match injected(row) {
                None => 0.3,
                Some("compound_obligation") => 0.1,
                Some(_) => 0.9,
            };
            (row.id.as_str(), score)
        })
        .collect();
    let every = bar_d(&rows, &scores, 0.5, &keys, SOUND);
    let once = bar_d_per_source(&rows, &scores, 0.5, &keys, SOUND);
    assert_eq!(every.pairs, mutants.len());
    assert_eq!(once.pairs, sources.len());
    // A source with one compound and two other mutants: 2 wins beat 1 loss.
    assert!(once.successes > 0 && once.successes + once.failures + once.ties <= sources.len());
    assert!(once.decided() < every.decided());
}

/// Provenance: PLAT-1024 exp3, PR #633 review finding 3. A statement mutant
/// whose source is missing stops the run rather than leaving bar D's
/// denominator.
#[test]
#[should_panic(expected = "is not among this run's rows")]
fn tc_1024_statement_bar_d_refuses_a_missing_source() {
    use eval_v2_support::variants::statement::{CHECKS, SOUND, bar_d, injected};
    let rows: Vec<Row> = eval_v2_support::corpus::load_in_repo()
        .unwrap()
        .unwrap()
        .file
        .rows
        .into_iter()
        .filter(|row| row.mutation.is_some() && injected(row).is_some())
        .collect();
    let keys: Vec<&str> = CHECKS.iter().map(|check| check.key).collect();
    let _ = bar_d(&rows, &std::collections::BTreeMap::new(), 0.5, &keys, SOUND);
}
