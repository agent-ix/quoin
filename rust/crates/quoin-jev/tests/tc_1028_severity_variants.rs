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

use eval_v2_support::corpus::TruthKind;
use eval_v2_support::corpus::{self, Row};
use eval_v2_support::keys::Mode;
use eval_v2_support::metrics::{Scored, concordance, paired_concordance, scored};
use eval_v2_support::severity::{
    self, Branch, EXAMPLES, Fact, HIGH_MASS_FLOOR, Level, S3_LEVELS, SEVERITY, expected_level,
    expected_prediction, high_mass_prediction, level_distribution, most_probable_level, rule,
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
const CLEAN: [(Fact, bool); 6] = [
    (Fact::TraceCorrect, true),
    (Fact::TestPassesWhenBroken, false),
    (Fact::AssertionVacuous, false),
    (Fact::TestChecksSomeClauses, false),
    (Fact::CodeContradicts, false),
    (Fact::CodeMissesStatedCase, false),
];

/// `CLEAN` with one fact flipped to its defect answer.
fn clean_but(fact: Fact) -> BTreeMap<Fact, bool> {
    let mut map = facts(&CLEAN);
    let value = map.get_mut(&fact).unwrap();
    *value = !*value;
    map
}

/// Provenance: PLAT-1028, MP-240, PR #620 review finding 2. Each branch of
/// the step-5 rule fires on its own defect, with the level the rubric line it
/// quotes assigns. Each axis's two tiers are separate facts: the code
/// contradicting the requirement is `high`, missing a stated case is
/// `medium`; the test passing with the behaviour broken is `high`, checking
/// only some clauses is `medium`. Written out literally, one row per branch.
#[test]
fn tc_1028_s1_rule_each_branch_fires_on_its_own_defect() {
    let table = [
        (
            Fact::TraceCorrect,
            Branch::TraceMismatch,
            "high",
            "A test tagged `FR-007-AC-1` that asserts something unrelated",
        ),
        (
            Fact::TestPassesWhenBroken,
            Branch::TestMissesIntent,
            "high",
            "test does not validate intent",
        ),
        (
            Fact::AssertionVacuous,
            Branch::TestHollow,
            "high",
            "does not exercise code",
        ),
        (
            Fact::CodeContradicts,
            Branch::CodeContradicts,
            "high",
            "code contradicts the requirement",
        ),
        (
            Fact::TestChecksSomeClauses,
            Branch::TestPartial,
            "medium",
            "partial validation",
        ),
        (
            Fact::CodeMissesStatedCase,
            Branch::CodeMissesCase,
            "medium",
            "meaningful edge cases unchecked",
        ),
    ];
    for (fact, branch, level, quoted) in table {
        let fired = rule(&clean_but(fact));
        assert_eq!(fired, branch, "{fact:?}");
        assert_eq!(fired.level().label(), level, "{fact:?}");
        assert!(fired.rubric().contains(quoted), "{fired:?}");
    }
    assert_eq!(rule(&facts(&CLEAN)), Branch::Clean);
    assert_eq!(Branch::Clean.level(), Level::None);
}

/// Provenance: PR #620 review finding 1. S1 asks no "code exceeds the
/// requirement" fact in any mode: MP-234 measured that question as a coin
/// flip, and at 0.5 it turned clean rows `medium`.
#[test]
fn tc_1028_s1_asks_no_exceeds_fact() {
    for mode in Mode::ALL {
        let keys: Vec<&str> = Fact::asked_in(mode).into_iter().map(Fact::key).collect();
        assert!(
            !keys.contains(&"code_exceeds_requirement"),
            "{mode:?}: {keys:?}"
        );
        let text = serde_json::to_string(&severity::s1_questions(mode)).unwrap();
        assert!(!text.contains("no requirement states"), "{mode:?}: {text}");
    }
}

/// Provenance: PLAT-1028, MP-240. The worst failing axis sets the level: a
/// high defect beside a medium one is high; `low` is never produced, because
/// step 5 names no condition for it.
#[test]
fn tc_1028_s1_rule_the_worst_axis_wins_and_low_is_unreachable() {
    let mut both = clean_but(Fact::TestChecksSomeClauses);
    both.insert(Fact::CodeContradicts, true);
    assert_eq!(rule(&both), Branch::CodeContradicts);

    let mut medium_pair = clean_but(Fact::CodeMissesStatedCase);
    medium_pair.insert(Fact::TestChecksSomeClauses, true);
    assert_eq!(rule(&medium_pair), Branch::TestPartial);

    // Every one of the 2^6 fact combinations: none reaches `low`.
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
            "test_passes_when_broken",
            "assertion_vacuous",
            "test_checks_only_some_clauses"
        ]
    );
    assert_eq!(
        keys(Mode::ReqCode),
        [
            "trace_correct",
            "code_contradicts_requirement",
            "code_misses_stated_case"
        ]
    );
    assert_eq!(keys(Mode::ReqTestCode).len(), 6);
    assert!(keys(Mode::Req).is_empty());

    // An RT row: clean test facts and no code facts at all is `none`.
    let rt = facts(&[
        (Fact::TraceCorrect, true),
        (Fact::TestPassesWhenBroken, false),
        (Fact::AssertionVacuous, false),
        (Fact::TestChecksSomeClauses, false),
    ]);
    assert_eq!(rule(&rt), Branch::Clean);
    // An RC row with a trace mismatch is still high.
    let rc = facts(&[(Fact::TraceCorrect, false), (Fact::CodeContradicts, false)]);
    assert_eq!(rule(&rc), Branch::TraceMismatch);

    let asked = |mode: Mode| -> Vec<String> {
        let row = row_in(mode);
        let asks = (severity::S1.asks)(&row);
        asks[0].request.questions.keys().cloned().collect()
    };
    assert_eq!(asked(Mode::ReqTest), keys(Mode::ReqTest));
    assert_eq!(asked(Mode::ReqCode), keys(Mode::ReqCode));
}

fn s1_answer(variant: &Variant, mode: Mode, pairs: &[(Fact, f64)]) -> Prediction {
    let predictions = (variant.derive)(&row_in(mode), &answered(nouls(pairs)));
    predictions[SEVERITY].clone()
}

/// Provenance: PLAT-1028, MP-240, PR #620 review finding 6. S1's level is the
/// most probable level under the fact distribution, not the rule over each
/// fact thresholded alone. By hand, RC: P(trace correct) = 0.55,
/// P(contradicts) = 0.45, P(misses a case) = 0.45. Thresholded, every fact
/// reads clean (`none`). The distribution: high = 0.45 + 0.55 x 0.45 =
/// 0.6975; medium = 0.55 x 0.55 x 0.45 = 0.136125; none = 0.55 x 0.55 x
/// 0.55 = 0.166375. So `high`, at 0.6975, with expected level
/// 3 x 0.6975 + 2 x 0.136125 = 2.36475.
#[test]
fn tc_1028_s1_level_is_the_most_probable_level() {
    let prediction = s1_answer(
        &severity::S1_RC,
        Mode::ReqCode,
        &[
            (Fact::TraceCorrect, 0.55),
            (Fact::CodeContradicts, 0.45),
            (Fact::CodeMissesStatedCase, 0.45),
        ],
    );
    assert_eq!(prediction.answer, "high");
    assert!(
        close(prediction.confidence.unwrap(), 0.6975),
        "{prediction:?}"
    );
    assert!(
        close(prediction.ordinal.unwrap(), 2.36475),
        "{prediction:?}"
    );

    // A tie goes to the more severe level: high 0.5, medium 0.5.
    let tie = s1_answer(
        &severity::S1_RC,
        Mode::ReqCode,
        &[
            (Fact::TraceCorrect, 1.0),
            (Fact::CodeContradicts, 0.5),
            (Fact::CodeMissesStatedCase, 1.0),
        ],
    );
    assert_eq!(tie.answer, "high");
    assert!(close(tie.confidence.unwrap(), 0.5), "{tie:?}");
    assert_eq!(
        most_probable_level(&[0.25, 0.25, 0.25, 0.25]),
        Some(Level::High)
    );
    assert_eq!(most_probable_level(&[0.0; 4]), None);

    // A fact left unanswered leaves the row unanswered, not guessed.
    let partial = nouls(&[(Fact::TraceCorrect, 1.0)]);
    assert!((severity::S1_RC.derive)(&row_in(Mode::ReqCode), &answered(partial)).is_empty());
}

/// Provenance: PR #620 review finding 6. Every RTC fact at 0.6. No high
/// defect needs trace correct (0.6) and all three high facts `no` (0.4
/// each): 0.6 x 0.4^3 = 0.0384, so high = 0.9616. Medium needs that and not
/// both medium facts `no`: 0.0384 x (1 - 0.4^2) = 0.032256. None = 0.0384 x
/// 0.16 = 0.006144. Expected level 3 x 0.9616 + 2 x 0.032256 = 2.949312.
#[test]
fn tc_1028_s1_all_facts_at_0_6() {
    let pairs: Vec<(Fact, f64)> = CLEAN.iter().map(|(fact, _)| (*fact, 0.6)).collect();
    let distribution = level_distribution(&pairs);
    assert!(close(distribution[Level::High.index()], 0.9616));
    assert!(close(distribution[Level::Medium.index()], 0.032_256));
    assert!(close(distribution[Level::None.index()], 0.006_144));
    let prediction = s1_answer(&severity::S1, Mode::ReqTestCode, &pairs);
    assert_eq!(prediction.answer, "high");
    assert!(
        close(prediction.confidence.unwrap(), 0.9616),
        "{prediction:?}"
    );
    assert!(
        close(prediction.ordinal.unwrap(), 2.949_312),
        "{prediction:?}"
    );
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
/// `score` answers 2.2 with `score_probabilities`. Counts requests.
struct FakeJev {
    nouls: BTreeMap<&'static str, f64>,
    score_probabilities: Value,
    calls: AtomicUsize,
}

/// The distribution the fake gives every score by default.
fn default_score_probabilities() -> Value {
    json!({"0": 0.1, "1": 0.1, "2": 0.4, "3": 0.4})
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
                                  "probabilities": self.score_probabilities}),
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
/// with the behaviour broken and against a stub (P = 0.9), so S1 is `high`
/// wherever a test is shown, and `none` on the RC row, where both code facts
/// are at 0.1 (high 0.19, medium 0.081, none 0.729).
#[tokio::test]
async fn tc_1028_the_severity_variants_run_end_to_end() {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(FakeJev {
        nouls: BTreeMap::from([
            ("code_contradicts_requirement", 0.1),
            ("code_misses_stated_case", 0.1),
        ]),
        score_probabilities: default_score_probabilities(),
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

// ---------------------------------------------------------------------------
// PR #620 review findings
// ---------------------------------------------------------------------------

fn fake_client(score_probabilities: Value) -> (typesafe_sdk_client::Client, Arc<FakeJev>) {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(FakeJev {
        nouls: BTreeMap::new(),
        score_probabilities,
        calls: AtomicUsize::new(0),
    });
    (
        quoin_jev::client::with_transport(config, fake.clone()),
        fake,
    )
}

/// Provenance: PR #620 review finding 9. A score whose probabilities are not
/// keyed by its levels fails the run with the keys named, rather than
/// leaving the row unanswered; so does one with no distribution at all.
#[tokio::test]
async fn tc_1028_score_probabilities_off_the_levels_fail_the_run() {
    let rows = vec![row_in(Mode::ReqTest)];
    for probabilities in [
        json!({"low": 0.5, "high": 0.5}),
        json!({"4": 1.0}),
        json!({}),
    ] {
        let (client, _) = fake_client(probabilities.clone());
        let error = variant::run(&client, &rows, &[&severity::S2M_RT])
            .await
            .unwrap_err();
        assert!(error.contains("score `severity` has 4 levels"), "{error}");
        assert!(error.contains("EV2-0001 / S2M-RT@v1"), "{error}");
        for key in probabilities.as_object().unwrap().keys() {
            assert!(error.contains(&format!("{key:?}")), "{error}");
        }
    }
    let (client, _) = fake_client(json!({"0.0": 0.2, "3": 0.8}));
    let output = variant::run(&client, &rows, &[&severity::S2M_RT])
        .await
        .unwrap();
    let answer = &output.results[0].predictions[SEVERITY];
    assert_eq!(answer.answer, "high");
}

fn rtc_questions_on_any_row(row: &Row) -> Vec<variant::Ask> {
    vec![variant::Ask {
        unit: None,
        request: variant::request(
            variant::state(row),
            severity::s1_questions(Mode::ReqTestCode),
        ),
    }]
}

/// Provenance: PR #620 review finding 10. RTC question text sent to an RT
/// row, by a variant whose declaration (`references: [Test]`) and state are
/// both truthful for RT, is caught by the question-TEXT check alone.
#[test]
fn tc_1028_the_text_check_catches_rtc_wording_on_an_rt_row() {
    let leaky = Variant {
        id: "LEAKY-RT",
        modes: &[Mode::ReqTest],
        references: &[variant::Artifact::Test],
        asks: rtc_questions_on_any_row,
        ..severity::S1_RT
    };
    let violations = wording_violations(&leaky, &row_in(Mode::ReqTest));
    assert!(!violations.is_empty());
    assert!(
        violations
            .iter()
            .all(|v| v.contains("question text refers to the code")
                && v.contains("which the mode lacks")),
        "{violations:#?}"
    );
    assert!(
        !violations
            .iter()
            .any(|v| v.contains("declares") || v.contains("state field")),
        "{violations:#?}"
    );
}

/// Provenance: PR #620 review finding 11. S3's single wording says an
/// artifact missing from the row's mode is expected and not a defect.
#[test]
fn tc_1028_s3_says_a_missing_artifact_is_not_a_defect() {
    let asks = (severity::S3.asks)(&row_in(Mode::ReqCode));
    let text = serde_json::to_string(&asks[0].request.questions[SEVERITY]).unwrap();
    assert!(
        text.contains("a missing artifact is not itself a mismatch"),
        "{text}"
    );
}

/// Provenance: PR #620 review findings 1 and 7. S2 teaches no "adds
/// behaviour the requirement does not state" medium, in its levels or its
/// examples: step 5 and the corpus rule have no such clause, and the old RC
/// example (deleting stored files) is `high` under step 4 A.
#[test]
fn tc_1028_s2_teaches_no_exceeds_medium() {
    for mode in [Mode::ReqTest, Mode::ReqCode, Mode::ReqTestCode] {
        let text = serde_json::to_string(&(severity::S2.asks)(&row_in(mode))[0].request.questions)
            .unwrap();
        for phrase in [
            "does not state",
            "nothing states",
            "adds behaviour",
            "deletes",
        ] {
            assert!(!text.contains(phrase), "{mode:?} teaches {phrase:?}");
        }
    }
}

/// The scenario of a worked example: its requirement sentence.
fn scenario(text: &str) -> &str {
    let text = text.strip_prefix("Requirement: ").unwrap_or(text);
    text.split(" Test")
        .next()
        .and_then(|head| head.split(" Code:").next())
        .unwrap_or(text)
}

/// Provenance: PR #620 review finding 8, PLAT-1028. No S2 worked-example
/// scenario appears in the eval-v2 corpus, in-repo or external: a prompt
/// must never carry a row it is graded on. Ignored until PLAT-1025 lands the
/// corpus; with `--ignored` it fails when there is no corpus to check.
#[test]
#[ignore = "PLAT-1025: in-repo corpus not landed"]
fn tc_1028_no_worked_example_is_in_the_corpus() {
    let sources: Vec<corpus::Source> = [
        corpus::load_in_repo().unwrap(),
        corpus::load_external_from_env().unwrap(),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert!(!sources.is_empty(), "no corpus found: nothing was checked");
    for source in &sources {
        let corpus_text = source.text.to_lowercase();
        for example in EXAMPLES {
            let scenario = scenario(example.text).to_lowercase();
            assert!(scenario.len() > 20, "{scenario}");
            assert!(
                !corpus_text.contains(&scenario),
                "{}: carries the worked example {scenario:?}",
                source.path.display()
            );
        }
    }
}

fn selection_file(entries: &Value) -> Result<corpus::SelectionFile, String> {
    corpus::parse_selection(
        &json!({"schema": corpus::SELECTION_SCHEMA, "selections": entries}).to_string(),
    )
}

fn selection_entry(mp: &str, variant: &str, version: u32) -> Value {
    json!({"mp": mp, "variant": variant, "version": version,
           "selected_at_commit": "358870db", "dev_evidence": "reviews/dev.md",
           "baselines": ["S0@v1"]})
}

/// Provenance: PR #620 review finding 3a. The committed selection file
/// parses; an MP may select only once; and a held-out run is refused for any
/// variant version not covered by an entry (a family id covers its per-mode
/// ids, and a listed baseline label is covered).
#[test]
fn tc_1028_heldout_runs_only_the_committed_selection() {
    let committed = std::fs::read_to_string(corpus::heldout_selection_path()).unwrap();
    corpus::parse_selection(&committed).unwrap();

    let twice = selection_file(&json!([
        selection_entry("MP-240", "S1", 1),
        selection_entry("MP-240", "S2", 1)
    ]))
    .unwrap_err();
    assert!(
        twice.contains("MP-240 has more than one held-out selection"),
        "{twice}"
    );
    let empty = selection_file(&json!([selection_entry("", "S1", 1)])).unwrap_err();
    assert!(empty.contains("empty mp"), "{empty}");

    let file = selection_file(&json!([selection_entry("MP-240", "S2", 1)])).unwrap();
    corpus::check_heldout_selected(&file, &[("S2", 1), ("S2-RT", 1), ("S2-RC", 1), ("S0", 1)])
        .unwrap();
    for refused in [("S2", 2), ("S2M", 1), ("S1", 1), ("S0", 2)] {
        let error = corpus::check_heldout_selected(&file, &[refused]).unwrap_err();
        assert!(
            error.contains(&format!("{}@v{}", refused.0, refused.1)),
            "{error}"
        );
    }
}

fn level_row(id: &str, expected: &str, ordinal: Option<f64>) -> Scored {
    Scored {
        row_id: id.to_owned(),
        mode: Mode::ReqTestCode,
        kind: TruthKind::Mechanical,
        expected: expected.to_owned(),
        alternatives: Vec::new(),
        prediction: ordinal.map(|ordinal| Prediction {
            answer: "medium".to_owned(),
            confidence: Some(0.5),
            ordinal: Some(ordinal),
        }),
    }
}

/// Provenance: PR #620 review finding 4, MP-240 Bar C. A variant that
/// abstains on the hard row looks perfect unpaired (1.0 against S0's 0.8,
/// where S0 answered the hard row wrongly: C=4, D=1). Paired over the three
/// rows both answered, the two tie at 1.0, so it is not above S0; and one of
/// four rows unanswered (25%) exceeds the 10% cap on its own.
#[test]
fn tc_1028_bar_c_compares_over_rows_both_answered() {
    let spec = eval_v2_support::keys::spec("severity").unwrap();
    let abstainer = [
        level_row("a", "none", Some(0.0)),
        level_row("b", "medium", Some(2.0)),
        level_row("c", "high", Some(3.0)),
        level_row("d", "high", None),
    ];
    let s0 = [
        level_row("a", "none", Some(0.0)),
        level_row("b", "medium", Some(2.0)),
        level_row("c", "high", Some(3.0)),
        level_row("d", "high", Some(0.5)),
    ];
    assert!(close(
        concordance(&abstainer, spec).unwrap().index().unwrap(),
        1.0
    ));
    let s0_alone = concordance(&s0, spec).unwrap();
    assert_eq!((s0_alone.concordant, s0_alone.discordant), (4, 1));
    assert!(close(s0_alone.index().unwrap(), 0.8));

    let paired = paired_concordance(&abstainer, &s0, spec).unwrap();
    assert_eq!(paired.shared, 3);
    assert_eq!(
        (paired.variant_abstained, paired.baseline_abstained),
        (1, 0)
    );
    assert!(close(paired.variant.index().unwrap(), 1.0));
    assert!(close(paired.baseline.index().unwrap(), 1.0));
    assert!(close(paired.variant_abstention().unwrap(), 25.0));
    assert_eq!(paired.beats_baseline(), Some(false));

    // Answering the hard row correctly beats S0 on the same four rows.
    let mut answers_all = abstainer.clone();
    answers_all[3] = level_row("d", "high", Some(2.9));
    let paired = paired_concordance(&answers_all, &s0, spec).unwrap();
    assert_eq!(paired.shared, 4);
    assert_eq!(paired.beats_baseline(), Some(true));
}
