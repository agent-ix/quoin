// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1028's severity variants, offline: S1's rubric rule branch by branch
//! and its fact distribution, S2/S2M's reading of a score distribution, S3's
//! outcome-to-level mapping, the wording rule in every mode each variant
//! claims, and the runner end to end over a fake Jev. No network, no key.
//!
//! Provenance: PLAT-1028, PLAT-1024, MP-240.

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

use eval_v2_support::corpus::{self, Row};
use eval_v2_support::keys::Mode;
use eval_v2_support::metrics::scored;
use eval_v2_support::severity::{
    self, Branch, EXAMPLES, Fact, HIGH_MASS_FLOOR, Level, S3_LEVELS, SEVERITY, expected_level,
    expected_prediction, high_mass_prediction, level_distribution, rule,
};
use eval_v2_support::variant::{
    self, Answered, Prediction, RawAnswer, RawAnswers, Variant, wording_violations,
};
use gap_semantic_support::SEVERITY_RUBRIC;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// One synthetic row of `mode`, carrying a severity truth of `high`.
fn row_value(id: &str, mode: Mode) -> Value {
    let test = mode.has_test().then(|| {
        json!({"path": "tests/tc_orders.rs", "fn_name": "tc_refunds_are_capped",
               "body": "#[test]\nfn tc_refunds_are_capped() { assert!(refund(5, 3).is_err()); }"})
    });
    let code = mode.has_code().then(|| {
        json!({"path": "src/refund.rs", "symbol": "refund",
               "body": "pub fn refund(amount: u32, paid: u32) -> Result<u32, E> { if amount > paid { Err(E) } else { Ok(amount) } }"})
    });
    json!({
        "id": id,
        "mode": mode.as_str(),
        "split": "dev",
        "strata": {"fr_id": format!("FR-{}", &id[id.len() - 3..]), "req_kind": "functional",
                   "test_kind": null, "crate": "orders", "ears_pattern": null},
        "requirement": {"fr_id": format!("FR-{}", &id[id.len() - 3..]), "ac_id": null,
                        "statement": "A refund shall never exceed the amount paid.",
                        "ac_text": null, "context": null},
        "test": test,
        "code": code,
        "ref": null,
        "mutation": null,
        "truth": {"severity": {"answer": "high", "kind": "by_construction",
                               "alternatives": [], "rationale": "stated"}},
    })
}

/// Rows `EV2-0001..3` in `RT`, `RC`, `RTC`.
fn rows() -> Vec<Row> {
    let text = serde_json::to_string(&json!({
        "schema": corpus::SCHEMA,
        "sampling_rule": "synthetic",
        "seed": 1,
        "source_commit": "0000000",
        "rows": [
            row_value("EV2-0001", Mode::ReqTest),
            row_value("EV2-0002", Mode::ReqCode),
            row_value("EV2-0003", Mode::ReqTestCode),
        ],
    }))
    .unwrap();
    corpus::parse(&text).unwrap().rows
}

fn row_in(mode: Mode) -> Row {
    rows().into_iter().find(|row| row.mode == mode).unwrap()
}

fn facts(pairs: &[(Fact, bool)]) -> BTreeMap<Fact, bool> {
    pairs.iter().copied().collect()
}

fn answered(answers: RawAnswers) -> [Answered; 1] {
    [Answered {
        unit: None,
        answers,
    }]
}

fn nouls(pairs: &[(Fact, f64)]) -> RawAnswers {
    pairs
        .iter()
        .map(|(fact, p)| (fact.key().to_owned(), RawAnswer::Noul(*p)))
        .collect()
}

fn score_answer(score: f64, probabilities: &[(&str, f64)]) -> RawAnswer {
    RawAnswer::Score {
        score,
        confidence: 0.55,
        probabilities: probabilities
            .iter()
            .map(|(level, p)| ((*level).to_owned(), *p))
            .collect(),
    }
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

const SEVERITY_VARIANTS: [&Variant; 10] = [
    &severity::S1,
    &severity::S1_RT,
    &severity::S1_RC,
    &severity::S2,
    &severity::S2_RT,
    &severity::S2_RC,
    &severity::S2M,
    &severity::S2M_RT,
    &severity::S2M_RC,
    &severity::S3,
];

// ---------------------------------------------------------------------------
// Levels
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1028. The level enum is the rubric the corpus labels
/// with, in the same order, and `nearest` rounds as S0's grader does.
#[test]
fn tc_1028_levels_are_the_rubric_in_order() {
    let labels: Vec<&str> = Level::ALL.into_iter().map(Level::label).collect();
    assert_eq!(labels, SEVERITY_RUBRIC);
    assert_eq!(Level::nearest(2.49), Some(Level::Medium));
    assert_eq!(Level::nearest(2.5), Some(Level::High));
    assert_eq!(Level::nearest(-0.4), Some(Level::None));
    assert_eq!(Level::nearest(3.5), None);
    assert_eq!(expected_level(&[0.0; 4]), None);
    assert!(close(expected_level(&[0.5, 0.0, 0.0, 0.5]).unwrap(), 1.5));
}

// ---------------------------------------------------------------------------
// S1: the rule, branch by branch
// ---------------------------------------------------------------------------

/// Every fact answered the no-defect way.
const CLEAN: [(Fact, bool); 7] = [
    (Fact::TraceCorrect, true),
    (Fact::TestAssertsIntent, true),
    (Fact::AssertionVacuous, false),
    (Fact::TestComplete, true),
    (Fact::CodeImplementsIntent, true),
    (Fact::CodeComplete, true),
    (Fact::CodeExceedsRequirement, false),
];

/// `CLEAN` with one fact flipped to its defect answer.
fn clean_but(fact: Fact) -> BTreeMap<Fact, bool> {
    let mut map = facts(&CLEAN);
    let value = map.get_mut(&fact).unwrap();
    *value = !*value;
    map
}

/// Provenance: PLAT-1028, MP-240. Each branch of the step-5 rule fires on its
/// own defect, with the level the rubric line it quotes assigns. Written out
/// literally, one row per branch, so the table is the check and not a
/// re-derivation of it.
#[test]
fn tc_1028_s1_rule_each_branch_fires_on_its_own_defect() {
    let table = [
        (Fact::TraceCorrect, Branch::TraceMismatch, "high"),
        (Fact::TestAssertsIntent, Branch::TestMissesIntent, "high"),
        (Fact::AssertionVacuous, Branch::TestHollow, "high"),
        (Fact::CodeImplementsIntent, Branch::CodeMissesIntent, "high"),
        (Fact::TestComplete, Branch::TestPartial, "medium"),
        (Fact::CodeComplete, Branch::CodeDrift, "medium"),
        (Fact::CodeExceedsRequirement, Branch::CodeExceeds, "medium"),
    ];
    for (fact, branch, level) in table {
        let fired = rule(&clean_but(fact));
        assert_eq!(fired, branch, "{fact:?}");
        assert_eq!(fired.level().label(), level, "{fact:?}");
        assert!(fired.rubric().starts_with("step "), "{fired:?}");
    }
    assert_eq!(rule(&facts(&CLEAN)), Branch::Clean);
    assert_eq!(Branch::Clean.level(), Level::None);
}

/// Provenance: PLAT-1028, MP-240. The worst failing axis sets the level: a
/// high defect beside a medium one is high; `low` is never produced, because
/// step 5 names no condition for it.
#[test]
fn tc_1028_s1_rule_the_worst_axis_wins_and_low_is_unreachable() {
    let mut both = clean_but(Fact::TestComplete);
    both.insert(Fact::CodeImplementsIntent, false);
    assert_eq!(rule(&both), Branch::CodeMissesIntent);

    let mut medium_pair = clean_but(Fact::CodeExceedsRequirement);
    medium_pair.insert(Fact::TestComplete, false);
    assert_eq!(rule(&medium_pair), Branch::TestPartial);

    // Every one of the 2^7 fact combinations: none reaches `low`.
    let all: Vec<(Fact, f64)> = CLEAN.iter().map(|(fact, _)| (*fact, 0.5)).collect();
    let distribution = level_distribution(&all);
    assert!(close(distribution[Level::Low.index()], 0.0));
    assert!(close(distribution.iter().sum::<f64>(), 1.0));
}

/// Provenance: PLAT-1028, MP-240. In `RT` and `RC` only the facts of the
/// present artifact are asked, and a branch whose fact was not asked is
/// skipped rather than read as clean or as a defect.
#[test]
fn tc_1028_s1_asks_only_present_artifacts_and_skips_absent_branches() {
    let keys = |mode: Mode| -> Vec<&str> {
        Fact::asked_in(mode)
            .into_iter()
            .map(Fact::key)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(Mode::ReqTest),
        [
            "trace_correct",
            "test_asserts_intent",
            "assertion_vacuous",
            "test_complete"
        ]
    );
    assert_eq!(
        keys(Mode::ReqCode),
        [
            "trace_correct",
            "code_implements_intent",
            "code_complete",
            "code_exceeds_requirement"
        ]
    );
    assert_eq!(keys(Mode::ReqTestCode).len(), 7);
    assert!(keys(Mode::Req).is_empty());

    // An RT row: clean test facts and no code facts at all is `none`.
    let rt = facts(&[
        (Fact::TraceCorrect, true),
        (Fact::TestAssertsIntent, true),
        (Fact::AssertionVacuous, false),
        (Fact::TestComplete, true),
    ]);
    assert_eq!(rule(&rt), Branch::Clean);
    // An RC row with a trace mismatch is still high.
    let rc = facts(&[
        (Fact::TraceCorrect, false),
        (Fact::CodeImplementsIntent, true),
    ]);
    assert_eq!(rule(&rc), Branch::TraceMismatch);

    let asked = |mode: Mode| -> Vec<String> {
        let row = row_in(mode);
        let asks = (severity::S1.asks)(&row);
        asks[0].request.questions.keys().cloned().collect()
    };
    assert_eq!(asked(Mode::ReqTest), keys(Mode::ReqTest));
    assert_eq!(asked(Mode::ReqCode), keys(Mode::ReqCode));
}

/// Provenance: PLAT-1028, MP-240. S1's answer is the rule over the facts
/// thresholded at 0.5; its ordinal is the expected level under the fact
/// distribution; its confidence is the answered level's mass. By hand, RT:
/// P(asserts intent) = 0.8, P(vacuous) = 0, P(complete) = 0.5, trace certain.
/// High = 0.2; medium = 0.8 x 0.5 = 0.4; none = 0.4. Expected level
/// 0.2 x 3 + 0.4 x 2 = 1.4. Thresholded, every fact is clean: `none`, 0.4.
#[test]
fn tc_1028_s1_derive_reads_the_fact_distribution() {
    let row = row_in(Mode::ReqTest);
    let answers = nouls(&[
        (Fact::TraceCorrect, 1.0),
        (Fact::TestAssertsIntent, 0.8),
        (Fact::AssertionVacuous, 0.0),
        (Fact::TestComplete, 0.5),
    ]);
    let predictions = (severity::S1_RT.derive)(&row, &answered(answers));
    let prediction = &predictions[SEVERITY];
    assert_eq!(prediction.answer, "none");
    assert!(close(prediction.confidence.unwrap(), 0.4), "{prediction:?}");
    assert!(close(prediction.ordinal.unwrap(), 1.4), "{prediction:?}");

    // A fact left unanswered leaves the row unanswered, not guessed.
    let partial = nouls(&[(Fact::TraceCorrect, 1.0), (Fact::TestAssertsIntent, 0.8)]);
    assert!((severity::S1_RT.derive)(&row, &answered(partial)).is_empty());
}

// ---------------------------------------------------------------------------
// S2, S2M, S3
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1028, MP-240. S2 grades the expected level and reports
/// the answered level's mass; S2M answers `high` once P(high) reaches the
/// pre-registered floor of 0.25 and orders by P(high).
#[test]
fn tc_1028_s2_and_s2m_read_the_score_distribution() {
    assert!(close(HIGH_MASS_FLOOR, 0.25));
    let spread = score_answer(1.9, &[("0", 0.1), ("1", 0.2), ("2", 0.4), ("3", 0.3)]);
    let expected = expected_prediction(&spread).unwrap();
    assert_eq!(expected.answer, "medium");
    assert!(close(expected.confidence.unwrap(), 0.4));
    assert!(close(expected.ordinal.unwrap(), 1.9));
    let mass = high_mass_prediction(&spread).unwrap();
    assert_eq!(mass.answer, "high");
    assert!(close(mass.confidence.unwrap(), 0.3));
    assert!(close(mass.ordinal.unwrap(), 0.3));

    // Below the floor, S2M falls back to the nearest level. Keys spelled as
    // decimals read the same.
    let low_high = score_answer(
        1.6,
        &[("0.0", 0.2), ("1.0", 0.2), ("2.0", 0.4), ("3.0", 0.2)],
    );
    let mass = high_mass_prediction(&low_high).unwrap();
    assert_eq!(mass.answer, "medium");
    assert!(close(mass.ordinal.unwrap(), 0.2));

    // No distribution: S2 keeps the service's confidence; S2M has nothing to
    // read. An out-of-range level makes the distribution unreadable.
    let bare = score_answer(3.0, &[]);
    assert_eq!(
        expected_prediction(&bare),
        Some(Prediction {
            answer: "high".to_owned(),
            confidence: Some(0.55),
            ordinal: Some(3.0),
        })
    );
    assert_eq!(high_mass_prediction(&bare), None);
    assert_eq!(
        high_mass_prediction(&score_answer(2.0, &[("4", 1.0)])),
        None
    );
    assert_eq!(expected_prediction(&RawAnswer::Noul(0.5)), None);
}

/// Provenance: PLAT-1028, MP-240. S3's four action levels map to the rubric
/// by position: block merge is `high`, no action is `none`.
#[test]
fn tc_1028_s3_actions_map_to_the_rubric_by_position() {
    assert!(S3_LEVELS[3].starts_with("block merge"));
    assert!(S3_LEVELS[0].starts_with("no action"));
    let row = row_in(Mode::ReqCode);
    let asks = (severity::S3.asks)(&row);
    let question = serde_json::to_value(&asks[0].request.questions[SEVERITY]).unwrap();
    assert_eq!(question["type"], "score");
    assert_eq!(question["criteria"], json!(S3_LEVELS));
    let answer = |score: f64| {
        let mut answers = RawAnswers::new();
        answers.insert(SEVERITY.to_owned(), score_answer(score, &[]));
        (severity::S3.derive)(&row, &answered(answers))[SEVERITY]
            .answer
            .clone()
    };
    assert_eq!(
        [answer(0.1), answer(1.2), answer(2.4), answer(2.6)],
        ["none", "low", "medium", "high"]
    );
}

/// Provenance: PLAT-1028. S2's worked examples: at least two per level in
/// each mode, three for `high`; and each mode's prompt carries only its own.
#[test]
fn tc_1028_s2_has_worked_examples_per_level_in_each_mode() {
    for mode in [Mode::ReqTest, Mode::ReqCode, Mode::ReqTestCode] {
        for level in Level::ALL {
            let count = EXAMPLES
                .iter()
                .filter(|example| example.mode == mode && example.level == level)
                .count();
            let floor = if level == Level::High { 3 } else { 2 };
            assert!(count >= floor, "{mode:?} {level:?}: {count} example(s)");
        }
        let row = row_in(mode);
        let question =
            serde_json::to_string(&(severity::S2.asks)(&row)[0].request.questions[SEVERITY])
                .unwrap();
        for example in EXAMPLES {
            assert_eq!(
                question.contains(example.text),
                example.mode == mode,
                "{mode:?}: {}",
                example.text
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The wording rule and the runner
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1028, PLAT-1024 (Peter's scope ruling). Every severity
/// variant asks something in each mode it claims and names no absent
/// artifact; the RTC wording sent to an RT or RC row is caught.
#[test]
fn tc_1028_every_severity_variant_passes_the_wording_rule() {
    for variant in SEVERITY_VARIANTS {
        assert!(!variant.modes.is_empty(), "{}", variant.id);
        for row in rows().iter().filter(|row| variant.applies_to(row)) {
            assert!(
                !(variant.asks)(row).is_empty(),
                "{} asks nothing",
                variant.id
            );
            let violations = wording_violations(variant, row);
            assert!(violations.is_empty(), "{violations:#?}");
        }
    }
    for rtc in [severity::S1, severity::S2] {
        let misapplied = Variant {
            modes: &Mode::ALL,
            ..rtc
        };
        let rt = wording_violations(&misapplied, &row_in(Mode::ReqTest));
        assert!(rt.iter().any(|v| v.contains("the code")), "{rt:#?}");
        let rc = wording_violations(&misapplied, &row_in(Mode::ReqCode));
        assert!(rc.iter().any(|v| v.contains("the test")), "{rc:#?}");
    }
    for id in [
        "S1", "S1-RT", "S1-RC", "S2", "S2-RT", "S2-RC", "S2M", "S2M-RT", "S2M-RC", "S3",
    ] {
        assert_eq!(variant::resolve(id).unwrap()[0].id, id);
    }
}

/// A fake Jev: every `noul` answers 0.9 except the keys in `nouls`; every
/// `score` answers 2.2 with a fixed distribution. Counts requests.
struct FakeJev {
    nouls: BTreeMap<&'static str, f64>,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for FakeJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let mut answers = serde_json::Map::new();
        for (key, question) in body["questions"].as_object().unwrap() {
            let answer = match question["type"].as_str().unwrap() {
                "noul" => {
                    json!({"type": "noul", "noul": self.nouls.get(key.as_str()).unwrap_or(&0.9)})
                }
                "score" => json!({"type": "score", "score": 2.2, "confidence": 0.5, "legend": {},
                                  "probabilities": {"0": 0.1, "1": 0.1, "2": 0.4, "3": 0.4}}),
                other => panic!("no severity variant asks a {other}"),
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

/// Provenance: PLAT-1028. Every severity variant runs end to end over one row
/// per mode: S2 and S2M share one request per row, S1 derives from facts, and
/// each is graded on `severity`. The fake says the test would still pass
/// against a stub (P = 0.9), so S1 is `high` wherever a test is shown, and
/// `none` on the RC row, where code facts are clean.
#[tokio::test]
async fn tc_1028_the_severity_variants_run_end_to_end() {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(FakeJev {
        nouls: BTreeMap::from([("code_exceeds_requirement", 0.1)]),
        calls: AtomicUsize::new(0),
    });
    let client = quoin_jev::client::with_transport(config, fake.clone());
    let rows = rows();
    let output = variant::run(&client, &rows, &SEVERITY_VARIANTS)
        .await
        .unwrap();

    // Per row: one S1 request, one S2 request (S2M reuses it), one S3.
    assert_eq!(fake.calls.load(Ordering::SeqCst), 9);
    assert_eq!((output.requests_sent, output.requests_reused), (9, 3));

    let answer = |label: &str, row: &str| -> Prediction {
        scored(&rows, &output, label, SEVERITY)
            .into_iter()
            .find(|scored| scored.row_id == row)
            .and_then(|scored| scored.prediction)
            .unwrap_or_else(|| panic!("{label} has no answer on {row}"))
    };
    assert_eq!(answer("S1-RT@v1", "EV2-0001").answer, "high");
    assert_eq!(answer("S1-RC@v1", "EV2-0002").answer, "none");
    assert_eq!(answer("S1@v1", "EV2-0003").answer, "high");
    for (label, row) in [
        ("S2-RT@v1", "EV2-0001"),
        ("S2-RC@v1", "EV2-0002"),
        ("S2@v1", "EV2-0003"),
    ] {
        assert_eq!(answer(label, row).answer, "medium");
    }
    for (label, row) in [
        ("S2M-RT@v1", "EV2-0001"),
        ("S2M-RC@v1", "EV2-0002"),
        ("S2M@v1", "EV2-0003"),
    ] {
        let prediction = answer(label, row);
        assert_eq!(prediction.answer, "high");
        assert!(close(prediction.ordinal.unwrap(), 0.4));
    }
    assert_eq!(answer("S3@v1", "EV2-0002").answer, "medium");
}
