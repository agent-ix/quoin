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
    self, ASSERTIONS_FIELD, CLAUSES_FIELD, T3, T4, criterion_clauses, derive_assertion_selection,
    derive_clause_coverage, extract_assertions,
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

/// Provenance: PLAT-1024, MP-242 round 2, T3 v4 (RES-31). TypeScript and
/// JavaScript: an `expect(..)` chain ending in a matcher is one statement,
/// on one line or continued on the next with no `;`, with the `await`
/// before it; each `node:assert` form is one statement; an assertion inside
/// a callback is listed on its own, and one nested in another is part of
/// it. A bare `expect(x)`, a member `.expect(..)`, and `expect(` inside a
/// string, a template literal or a comment are not assertions.
#[test]
fn tc_1024_t3_extracts_script_assertions() {
    let body = "it('reads the config', async () => {\n  \
                // expect(commented).toBe(1);\n  \
                const s = \"expect(in_a_string).toBe(1)\";\n  \
                const t = `assert(${name}) expect(x)`;\n  \
                expect(parse('a b')).toEqual({ a: 1 });\n  \
                await expect(load('x')).rejects.toThrow('missing');\n  \
                expect(result)\n    .not.toBeNull()\n  \
                expect(flag).to.be.true;\n  \
                expect(bare);\n  \
                request(app).expect(200);\n  \
                assert(ok);\n  \
                assert.equal(a, 1);\n  \
                assert.strictEqual(b, 'two');\n  \
                assert.deepStrictEqual(c, [1, 2]);\n  \
                assert.throws(() => parse(''), /empty/);\n  \
                items.forEach((item) => {\n    assert.ok(item.valid);\n  });\n  \
                [1, 2].map((n) => expect(n).toBeGreaterThan(0));\n  \
                expect(() => { assert.fail('inner'); }).toThrow();\n\
                });";
    let expected = [
        "expect(parse('a b')).toEqual({ a: 1 });",
        "await expect(load('x')).rejects.toThrow('missing');",
        "expect(result) .not.toBeNull()",
        "expect(flag).to.be.true;",
        "assert(ok);",
        "assert.equal(a, 1);",
        "assert.strictEqual(b, 'two');",
        "assert.deepStrictEqual(c, [1, 2]);",
        "assert.throws(() => parse(''), /empty/);",
        "assert.ok(item.valid);",
        "expect(n).toBeGreaterThan(0)",
        "expect(() => { assert.fail('inner'); }).toThrow();",
    ];
    for path in [
        "src/config.test.ts",
        "ui/App.test.tsx",
        "test/config.test.js",
        "test/config.test.mjs",
        "test/config.test.cjs",
    ] {
        assert_eq!(extract_assertions(path, body), expected, "{path}");
    }
    let sample = "expect(run().problems).toEqual([]); assert.equal(a, 1);";
    assert_eq!(
        extract_assertions("src/x.test.ts", sample),
        ["expect(run().problems).toEqual([]);", "assert.equal(a, 1);"]
    );
    // A string holding `expect(` lists nothing, so the row is `no` with no
    // call.
    let in_string = "test('x', () => {\n  log(\"expect(a).toBe(1)\");\n});";
    assert!(extract_assertions("src/x.test.ts", in_string).is_empty());
}

/// Provenance: PLAT-1024, MP-242 round 2, T3 v4 (RES-31). A TypeScript row
/// is asked about its listed assertions, and `QUOIN_JEV_INTENT_OUT`'s dump
/// writes one line per row: the variant, the row, each listed assertion
/// with its `P`, the derived prediction and the truth label.
#[tokio::test]
async fn tc_1024_t3_script_row_is_asked_and_dumped() {
    let mut value = fixtures::row(
        "EV2-0021",
        Mode::ReqTest,
        "dev",
        &json!({"test_asserts_intent": fixtures::truth(&json!("yes"), "mechanical", &[])}),
    );
    value["test"] = json!({
        "path": "src/check.test.ts",
        "fn_name": "refuses oversize",
        "body": "it('refuses oversize', () => {\n  const r = check('x'.repeat(5000));\n  \
                 expect(r.code).toBe('CORE_REFUSED');\n  assert.ok(r);\n});",
    });
    let row = fixtures::parse(&[value]).unwrap().rows.remove(0);
    let (client, fake) = fake_client(0.8);
    let output = variant::run(&client, std::slice::from_ref(&row), &[&T3])
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let lines = intent::dump(std::slice::from_ref(&row), &output, &[&T3]);
    assert_eq!(lines.len(), 1);
    let line: Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(
        line,
        json!({
            "variant": "T3@v4",
            "row_id": "EV2-0021",
            "mode": "RT",
            "assertions": [
                {"label": "A1", "text": "expect(r.code).toBe('CORE_REFUSED');", "p": 0.8},
                {"label": "A2", "text": "assert.ok(r);", "p": 0.8},
            ],
            "clauses": [],
            "prediction": {"answer": "yes", "confidence": 0.8, "p_yes": 0.8},
            "truth": {"answer": "yes", "kind": "mechanical", "alternatives": []},
        })
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
// T4: clause coverage (RES-31)
// ---------------------------------------------------------------------------

/// Provenance: RES-31. A criterion is cut at `;`, at sentence ends, and at a
/// serial list's `, and ` with the list's earlier items; nothing inside
/// backticks or parentheses is a cut point; a criterion with no cut point is
/// one clause; past eight the overflow joins the last.
#[test]
fn tc_1024_t4_cuts_a_criterion_into_clauses() {
    assert_eq!(
        criterion_clauses(
            "Cargo publication is disabled, both license texts are present, and CI has no \
             automatic trigger."
        ),
        [
            "Cargo publication is disabled",
            "both license texts are present",
            "CI has no automatic trigger",
        ]
    );
    assert_eq!(
        criterion_clauses("Library source contains no `unsafe` block."),
        ["Library source contains no `unsafe` block"]
    );
    assert_eq!(criterion_clauses("A; B. C"), ["A", "B", "C"]);
    // Backticks and parentheses hide every cut point; a sentence may open
    // with a backtick.
    assert_eq!(
        criterion_clauses(
            "`quoin check` prints `a; b. C, and d` (one line; no colour, and no bell). `quoin` \
             exits 0."
        ),
        [
            "`quoin check` prints `a; b. C, and d` (one line; no colour, and no bell)",
            "`quoin` exits 0",
        ]
    );
    // `; and` closes a serial list too; a lowercase word after `. ` and a
    // comma with no `, and ` are not cut points.
    assert_eq!(
        criterion_clauses("The file is read, parsed; and cached. e.g. twice, once."),
        ["The file is read", "parsed", "cached. e.g. twice, once"]
    );
    let many = (1..=10)
        .map(|n| format!("P{n}"))
        .collect::<Vec<_>>()
        .join("; ");
    assert_eq!(
        criterion_clauses(&many),
        ["P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8; P9; P10"]
    );
    assert!(criterion_clauses(" ; . ").is_empty());
}

/// `(per-clause P, expected answer, confidence, ordinal)`.
type CoverageCase = (&'static [(&'static str, f64)], &'static str, f64, f64);

/// Provenance: RES-31. `P(yes)` is the lowest per-clause `P`: every clause
/// must be covered, and one uncovered clause makes the row `no`. Nothing
/// asked is `no` at confidence 1; a missing, stray or out-of-range answer is
/// an error.
#[test]
fn tc_1024_t4_yes_needs_every_clause() {
    let cases: [CoverageCase; 3] = [
        (&[("C1", 0.9), ("C2", 0.6), ("C3", 0.8)], "yes", 0.6, 0.6),
        (&[("C1", 0.95), ("C2", 0.1), ("C3", 0.9)], "no", 0.9, 0.1),
        (&[("C1", 0.5), ("C2", 0.5), ("C3", 0.5)], "yes", 0.5, 0.5),
    ];
    for (answers, answer, confidence, ordinal) in cases {
        let prediction = derive_clause_coverage(3, &nouls(answers)).unwrap();
        assert_eq!(prediction.answer, answer, "{answers:?}");
        assert!(close(prediction.confidence, confidence), "{prediction:?}");
        assert!(close(prediction.ordinal, ordinal), "{prediction:?}");
    }
    let nothing = derive_clause_coverage(0, &RawAnswers::new()).unwrap();
    assert_eq!(
        nothing,
        Prediction {
            answer: "no".to_owned(),
            confidence: Some(1.0),
            ordinal: Some(0.0),
        }
    );
    for (answers, expected) in [
        (nouls(&[("C1", 0.6)]), "`C2`: no answer in the response"),
        (
            nouls(&[("C1", 0.6), ("C2", -0.1)]),
            "`C2`: probability -0.1 is outside [0, 1]",
        ),
        (
            nouls(&[("C1", 0.6), ("C2", 0.7), ("A1", 0.9)]),
            "`A1`: answered, but no such clause was asked",
        ),
    ] {
        let error = derive_clause_coverage(2, &answers).unwrap_err();
        assert!(error.starts_with(expected), "{expected:?}: got {error:?}");
    }
}

/// A fake Jev for T4: clause `Ck` answers the k-th of `clause_p`.
struct ClauseJev {
    clause_p: Vec<f64>,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for ClauseJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let mut answers = serde_json::Map::new();
        for key in body["questions"].as_object().unwrap().keys() {
            let k: usize = key.strip_prefix('C').unwrap().parse().unwrap();
            answers.insert(
                key.clone(),
                json!({"type": "noul", "noul": self.clause_p[k - 1]}),
            );
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

/// Provenance: RES-31. A T4 row is asked once, one `noul` per clause, with
/// the assertions and the clauses in its state; one clause left uncovered
/// makes it `no`; `QUOIN_JEV_INTENT_OUT`'s dump lists the assertions and
/// each clause with its `P`. A test with no assertion is `no` with no call,
/// and the statement stands in for a missing criterion.
#[tokio::test]
async fn tc_1024_t4_row_is_asked_per_clause_and_dumped() {
    let mut value = fixtures::row(
        "EV2-0041",
        Mode::ReqTest,
        "dev",
        &json!({"test_asserts_intent": fixtures::truth(&json!("no"), "mechanical", &[])}),
    );
    value["requirement"]["ac_text"] = json!(
        "Cargo publication is disabled, both license texts are present, and CI has no \
         automatic trigger."
    );
    value["test"] = json!({
        "path": "src/release.test.ts",
        "fn_name": "release settings",
        "body": "it('release settings', () => {\n  expect(manifest.publish).toBe(false);\n  \
                 expect(exists('LICENSE-AGPL')).toBe(true);\n});",
    });
    let row = fixtures::parse(&[value]).unwrap().rows.remove(0);

    let asks = (T4.asks)(&row);
    assert_eq!(asks.len(), 1);
    let state = serde_json::to_value(&asks[0].request.state).unwrap();
    assert_eq!(
        state[CLAUSES_FIELD],
        json!({"C1": "Cargo publication is disabled",
               "C2": "both license texts are present",
               "C3": "CI has no automatic trigger"})
    );
    assert_eq!(
        state[ASSERTIONS_FIELD],
        json!({"A1": "expect(manifest.publish).toBe(false);",
               "A2": "expect(exists('LICENSE-AGPL')).toBe(true);"})
    );
    let keys: Vec<&String> = asks[0].request.questions.keys().collect();
    assert_eq!(keys, ["C1", "C2", "C3"]);
    assert!(wording_violations(&T4, &row).is_empty());

    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(ClauseJev {
        clause_p: vec![0.9, 0.7, 0.2],
        calls: AtomicUsize::new(0),
    });
    let client = quoin_jev::client::with_transport(config, fake.clone());
    let output = variant::run(&client, std::slice::from_ref(&row), &[&T4])
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let lines = intent::dump(std::slice::from_ref(&row), &output, &[&T4]);
    assert_eq!(lines.len(), 1);
    let line: Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(
        line,
        json!({
            "variant": "T4@v1",
            "row_id": "EV2-0041",
            "mode": "RT",
            "assertions": [
                {"label": "A1", "text": "expect(manifest.publish).toBe(false);", "p": null},
                {"label": "A2", "text": "expect(exists('LICENSE-AGPL')).toBe(true);", "p": null},
            ],
            "clauses": [
                {"label": "C1", "text": "Cargo publication is disabled", "p": 0.9},
                {"label": "C2", "text": "both license texts are present", "p": 0.7},
                {"label": "C3", "text": "CI has no automatic trigger", "p": 0.2},
            ],
            "prediction": {"answer": "no", "confidence": 0.8, "p_yes": 0.2},
            "truth": {"answer": "no", "kind": "mechanical", "alternatives": []},
        })
    );

    // No assertion: `no`, no call, no clauses dumped.
    let bare = row_with(
        "EV2-0042",
        Mode::ReqTest,
        Some("#[test]\nfn t() {\n    run();\n}"),
    );
    assert!((T4.asks)(&bare).is_empty());
    let output = variant::run(&client, std::slice::from_ref(&bare), &[&T4])
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        output.results[0].predictions["test_asserts_intent"].answer,
        "no"
    );
    let line: Value =
        serde_json::from_str(&intent::dump(std::slice::from_ref(&bare), &output, &[&T4])[0])
            .unwrap();
    assert_eq!(line["clauses"], json!([]));

    // No criterion: the statement's clauses are asked about.
    let mut value = fixtures::row("EV2-0043", Mode::ReqTestCode, "dev", &json!({}));
    value["requirement"]["ac_text"] = Value::Null;
    let no_ac = fixtures::parse(&[value]).unwrap().rows.remove(0);
    let state = serde_json::to_value(&(T4.asks)(&no_ac)[0].request.state).unwrap();
    assert_eq!(
        state[CLAUSES_FIELD],
        json!({"C1": "The system shall refuse a request larger than 4096 bytes"})
    );
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

// ---------------------------------------------------------------------------
// Experiment 4: acceptance-criterion refusal reasons (R0, R1)
// ---------------------------------------------------------------------------

/// A row in `mode` whose criterion text is `ac` (`None` for no criterion).
fn criterion_row(id: &str, mode: Mode, ac: Option<&str>) -> Row {
    let mut value = fixtures::row(id, mode, "dev", &json!({}));
    value["requirement"]["ac_text"] = json!(ac);
    serde_json::from_value(value).unwrap()
}

/// The in-repo dev rows.
fn dev_rows() -> Vec<Row> {
    eval_v2_support::corpus::load_in_repo()
        .unwrap()
        .unwrap()
        .file
        .rows
        .into_iter()
        .filter(|row| row.split == eval_v2_support::corpus::Split::Dev)
        .collect()
}

/// Provenance: PLAT-1024 exp4. The unit is the criterion's own text: a row
/// with no criterion text is not asked; a row with one gets one request whose
/// state is the criterion and its id only, in every mode, with no test or
/// code vocabulary in the questions.
#[test]
fn tc_1024_refusal_variants_ask_only_about_a_criterion() {
    use eval_v2_support::variants::refusal::{CRITERION_FIELD, R0, R1};
    for mode in Mode::ALL {
        let none = criterion_row("EV2-0001", mode, None);
        let blank = criterion_row("EV2-0002", mode, Some("  "));
        for row in [&none, &blank] {
            assert!((R0.asks)(row).is_empty() && (R1.asks)(row).is_empty());
        }
        let row = criterion_row("EV2-0003", mode, Some("A 5000-byte request is refused."));
        for variant in [&R0, &R1] {
            let asks = (variant.asks)(&row);
            assert_eq!(asks.len(), 1);
            let state = serde_json::to_value(&asks[0].request.state).unwrap();
            assert_eq!(state[CRITERION_FIELD], "A 5000-byte request is refused.");
            assert_eq!(state["criterion_id"], "FR-003-AC-1");
            assert_eq!(state.as_object().unwrap().len(), 2);
            assert!(wording_violations(variant, &row).is_empty(), "{mode:?}");
        }
    }
}

/// Provenance: PLAT-1024 exp4. Both combiners fixed before the first live
/// call are pre-registered: any check at 0.5 (R1's graded answer) and any
/// check at 0.7; `>= 2` is post hoc.
#[test]
fn tc_1024_refusal_pre_registered_combiners() {
    use eval_v2_support::variants::refusal::{POST_HOC, PRE_REGISTERED, R1_COMBINER};
    let [(any_name, any), (strict_name, strict)] = PRE_REGISTERED;
    assert_eq!((any_name, strict_name), ("any >= 0.5", "any >= 0.7"));
    assert_eq!(any, R1_COMBINER);
    let [(_, two)] = POST_HOC;
    let one = [0.6, 0.2, 0.1, 0.0, 0.0];
    let high = [0.75, 0.55, 0.0, 0.0, 0.0];
    let none = [0.4, 0.3, 0.49, 0.1, 0.0];
    assert_eq!(
        [one, high, none].map(|p| [any.flags(&p), strict.flags(&p), two.flags(&p)]),
        [
            [true, false, false],
            [true, true, true],
            [false, false, false]
        ]
    );
}

/// Provenance: PLAT-1024 exp4. R1 grades each check at 0.5 and derives
/// `criterion_groundable` from them; R0 reads its one noul; a row with no
/// unit gets no answer.
#[test]
fn tc_1024_refusal_derive() {
    use eval_v2_support::variants::refusal::{CHECKABLE, CHECKS, R0, R1, SOUND};
    let row = criterion_row(
        "EV2-0003",
        Mode::Req,
        Some("A 5000-byte request is refused."),
    );
    let answered = |pairs: &[(&str, f64)]| {
        vec![Answered {
            unit: None,
            answers: pairs
                .iter()
                .map(|(key, p)| ((*key).to_owned(), RawAnswer::Noul(*p)))
                .collect::<RawAnswers>(),
        }]
    };
    let mut pairs: Vec<(&str, f64)> = CHECKS.iter().map(|check| (check.key, 0.1)).collect();
    let r1 = (R1.derive)(&row, &answered(&pairs));
    assert_eq!(r1[SOUND].answer, "yes");
    pairs[2].1 = 0.6;
    let r1 = (R1.derive)(&row, &answered(&pairs));
    assert_eq!(r1["names_single_witness"].answer, "yes");
    assert_eq!(r1["oracle_is_adjectival"].answer, "no");
    assert_eq!(r1[SOUND].answer, "no");
    assert!((r1[SOUND].ordinal.unwrap() - 0.4).abs() < 1e-12);
    let r0 = (R0.derive)(&row, &answered(&[(CHECKABLE, 0.3)]));
    assert_eq!(r0[SOUND].answer, "no");
    assert!((R1.derive)(&row, &[]).is_empty());
}

/// Provenance: PLAT-1024 exp4. The refusal mutants: 40, eight per reason,
/// all dev, each shown in its source's mode, each changing the source's
/// criterion text, over at least 20 distinct sources (so per-source bar D
/// can gate) with at most two mutants per source, every source labelled
/// groundable.
#[test]
fn tc_1024_refusal_mutants_are_dev_and_change_the_criterion() {
    use eval_v2_support::variants::refusal::{CHECKS, REFUSAL, SOUND, criterion_unit};
    let rows = dev_rows();
    let by_id: std::collections::BTreeMap<&str, &Row> =
        rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut per_reason: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut per_source: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for mutant in rows.iter().filter(|row| REFUSAL.is_mutant(row)) {
        let injected = REFUSAL.injected(mutant).unwrap();
        *per_reason.entry(injected).or_default() += 1;
        let source_id = mutant
            .mutation
            .as_ref()
            .unwrap()
            .source_id
            .as_deref()
            .unwrap();
        *per_source.entry(source_id).or_default() += 1;
        let source = by_id[source_id];
        assert_eq!(source.mode, mutant.mode, "{}", mutant.id);
        assert!(source.mutation.is_none(), "{}", mutant.id);
        assert_eq!(source.truth[SOUND].answer.label(), "yes", "{}", mutant.id);
        assert_ne!(
            criterion_unit(mutant),
            criterion_unit(source),
            "{}",
            mutant.id
        );
        assert_eq!(mutant.requirement.statement, source.requirement.statement);
    }
    assert_eq!(per_reason.len(), CHECKS.len());
    assert!(per_reason.values().all(|n| *n == 8), "{per_reason:?}");
    assert!(per_source.len() >= 20, "{} sources", per_source.len());
    assert!(per_source.values().all(|n| *n <= 2), "{per_source:?}");
}

/// Provenance: PLAT-1024 exp4. Every committed refusal label carries all six
/// keys with `criterion_groundable` = `no` exactly when a check fires; a
/// missing key, an inconsistent aggregate, an unknown id or a held-out id is
/// refused, never silently dropped.
#[test]
fn tc_1024_refusal_labels_are_complete_or_refused() {
    use eval_v2_support::assemble::{fixture_dir, refusal_labels, refusal_truth};
    let read = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(fixture_dir().join(name)).unwrap()).unwrap()
    };
    let sample = read("sample.json");
    let labels = read("labels-ac-refusal.json")["labels"].clone();
    let checked = refusal_labels(&labels, &sample).unwrap();
    assert_eq!(checked.len(), 48);
    assert!(checked.values().all(|truth| truth.len() == 6));
    let entry = |sound: &str, adjectival: &str| {
        json!({
            "criterion_groundable": {"answer": sound, "why": "w"},
            "oracle_is_adjectival": {"answer": adjectival, "why": "w"},
            "domain_unbounded": {"answer": "no", "why": "w"},
            "names_single_witness": {"answer": "no", "why": "w"},
            "describes_its_own_test": {"answer": "no", "why": "w"},
            "static_or_demonstration": {"answer": "no", "why": "w"},
        })
    };
    assert!(refusal_truth("N-1", &entry("no", "yes")).is_ok());
    assert!(refusal_truth("N-1", &entry("yes", "no")).is_ok());
    assert!(refusal_truth("N-1", &entry("yes", "yes")).is_err());
    assert!(refusal_truth("N-1", &entry("no", "no")).is_err());
    let mut missing = entry("yes", "no");
    missing.as_object_mut().unwrap().remove("domain_unbounded");
    assert!(
        refusal_truth("N-1", &missing)
            .unwrap_err()
            .contains("domain_unbounded")
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
        assert!(refusal_labels(&bad, &sample).is_err(), "{id} accepted");
    }
}

/// Provenance: PLAT-1024 exp4. The refusal mutants run only on R0 and R1:
/// K1, E5, F0 and F1 see exactly the dev rows they saw before (the counts
/// measured on origin/main at 0e274484: 248 dev rows plus 40 statement
/// mutants, 105 in R for K1, 100 in RC or RTC for E5), and a run without an
/// R variant drops them from its rows.
#[test]
fn tc_1024_refusal_mutants_cost_other_variants_nothing() {
    use eval_v2_support::variant::rows_for;
    use eval_v2_support::variants::refusal::{R0, R1, is_refusal_mutant};
    use eval_v2_support::variants::soundness::K1;
    use eval_v2_support::variants::statement::{F0, F1, is_statement_mutant};
    let dev = dev_rows();
    assert_eq!(dev.iter().filter(|row| is_refusal_mutant(row)).count(), 40);
    let count = |variant: &Variant| dev.iter().filter(|row| variant.applies_to(row)).count();
    assert_eq!((count(&K1), count(&E5)), (105, 100));
    assert_eq!((count(&F0), count(&F1)), (288, 288));
    // R0 and R1 ask only the 48 labelled natural rows and the 40 mutants:
    // not the two unlabelled projections, nor the row whose criterion text
    // the table parser cut off (PR #634 review).
    assert_eq!((count(&R0), count(&R1)), (88, 88));
    for id in ["EV2-0181", "EV2-0182", "EV2-0152"] {
        let row = dev.iter().find(|row| row.id == id).unwrap();
        assert!(!(R1.asks)(row).is_empty(), "{id} has a criterion");
        assert!(!R0.applies_to(row) && !R1.applies_to(row), "{id}");
    }
    assert_eq!(rows_for(dev.clone(), &[&K1, &E5]).len(), 248);
    assert_eq!(rows_for(dev.clone(), &[&K1, &F0]).len(), 248 + 40);
    assert_eq!(rows_for(dev.clone(), &[&R1]).len(), 248 + 40);
    assert_eq!(rows_for(dev.clone(), &[&F1, &R0]).len(), 248 + 80);
    for row in &dev {
        if is_refusal_mutant(row) {
            assert!(R0.applies_to(row) && R1.applies_to(row));
            assert!(!F0.applies_to(row) && !F1.applies_to(row) && !K1.applies_to(row));
        }
        if is_statement_mutant(row) {
            assert!(!R0.applies_to(row) && !R1.applies_to(row));
        }
    }
}

/// Provenance: PLAT-1024 exp4. Refusal bar D pairs each mutant with its
/// source; with one pair per source a source counts once, and a perfect rule
/// decides one pair per source, enough to gate.
#[test]
fn tc_1024_refusal_bar_d_counts_each_source_once() {
    use eval_v2_support::variants::refusal::{CHECKS, REFUSAL, SOUND};
    let rows = dev_rows();
    let keys: Vec<&str> = CHECKS.iter().map(|check| check.key).collect();
    let mutants = rows.iter().filter(|row| REFUSAL.is_mutant(row)).count();
    let perfect: std::collections::BTreeMap<&str, f64> = REFUSAL
        .labelled(&rows)
        .into_iter()
        .map(|row| {
            let defect = REFUSAL.injected(row).is_some();
            (row.id.as_str(), if defect { 0.9 } else { 0.1 })
        })
        .collect();
    let every = REFUSAL.bar_d(&rows, &perfect, 0.5, &keys, SOUND);
    let once = REFUSAL.bar_d_per_source(&rows, &perfect, 0.5, &keys, SOUND);
    assert_eq!((every.pairs, every.successes), (mutants, mutants));
    assert_eq!(once.successes, once.pairs);
    assert!(once.pairs >= 20 && once.gateable() && once.passes());
}

/// Provenance: PLAT-1024 exp4, PR #634 review. At exactly `P = 0.5` the
/// holistic baselines' graded answer and the report's flag agree: both use
/// `battery::flags` on the defect score `1 - P`, so 0.5 is flagged; just
/// above it is not.
#[test]
fn tc_1024_holistic_answer_and_report_flag_agree_at_one_half() {
    use eval_v2_support::variants::battery::{flags, holistic_predictions};
    use eval_v2_support::variants::refusal::{CHECKABLE, R0, SOUND};
    use eval_v2_support::variants::soundness::TAU;
    use eval_v2_support::variants::statement::{self, F0, WELL_FORMED};
    let row = criterion_row(
        "EV2-0003",
        Mode::Req,
        Some("A 5000-byte request is refused."),
    );
    let answered = |key: &str, p: f64| {
        vec![Answered {
            unit: None,
            answers: [(key.to_owned(), RawAnswer::Noul(p))]
                .into_iter()
                .collect::<RawAnswers>(),
        }]
    };
    for (p, expected) in [(0.5, "no"), (0.51, "yes"), (0.49, "no")] {
        let report_flags = flags(1.0 - p, TAU);
        assert_eq!(report_flags, expected == "no", "report at {p}");
        assert_eq!(holistic_predictions(SOUND, p)[SOUND].answer, expected);
        assert_eq!(
            (R0.derive)(&row, &answered(CHECKABLE, p))[SOUND].answer,
            expected
        );
        assert_eq!(
            (F0.derive)(&row, &answered(WELL_FORMED, p))[statement::SOUND].answer,
            expected
        );
    }
}

/// Provenance: PLAT-1024 exp4, PR #634 review. A refusal mutant whose source
/// is missing stops the run rather than leaving bar D's denominator.
#[test]
#[should_panic(expected = "is not among this run's rows")]
fn tc_1024_refusal_bar_d_refuses_a_missing_source() {
    use eval_v2_support::variants::refusal::{CHECKS, REFUSAL, SOUND};
    let rows: Vec<Row> = dev_rows()
        .into_iter()
        .filter(|row| REFUSAL.is_mutant(row))
        .collect();
    let keys: Vec<&str> = CHECKS.iter().map(|check| check.key).collect();
    let _ = REFUSAL.bar_d(&rows, &std::collections::BTreeMap::new(), 0.5, &keys, SOUND);
}

/// Provenance: PLAT-1024 exp4, PR #634 review. A refusal mutant edits only
/// its criterion: its test and code bodies, and every other requirement
/// field, are its source's.
#[test]
fn tc_1024_refusal_mutant_leaves_test_and_code_unchanged() {
    use eval_v2_support::variants::refusal::REFUSAL;
    let rows = dev_rows();
    let by_id: std::collections::BTreeMap<&str, &Row> =
        rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut seen = 0;
    for mutant in rows.iter().filter(|row| REFUSAL.is_mutant(row)) {
        let source = by_id[mutant
            .mutation
            .as_ref()
            .unwrap()
            .source_id
            .as_deref()
            .unwrap()];
        let body = |row: &Row| {
            (
                row.test
                    .as_ref()
                    .map(|t| (t.path.clone(), t.fn_name.clone(), t.body.clone())),
                row.code
                    .as_ref()
                    .map(|c| (c.path.clone(), c.symbol.clone(), c.body.clone())),
            )
        };
        assert_eq!(body(mutant), body(source), "{}", mutant.id);
        assert_eq!(mutant.requirement.fr_id, source.requirement.fr_id);
        assert_eq!(mutant.requirement.ac_id, source.requirement.ac_id);
        assert_eq!(mutant.requirement.context, source.requirement.context);
        assert_ne!(mutant.requirement.ac_text, source.requirement.ac_text);
        seen += usize::from(mutant.test.is_some() || mutant.code.is_some());
    }
    assert!(seen > 0, "no refusal mutant shows a test or code body");
}
