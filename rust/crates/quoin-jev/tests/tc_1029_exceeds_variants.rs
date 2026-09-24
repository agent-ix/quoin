// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1029's `code_exceeds_requirement` variants, offline.
//!
//! The clause splitter, the trivial-unit pre-filter, E1's and E4's roll-up
//! (mass thresholds, parking, the circuit breaker), E2's roll-up, the tau
//! sweep and E3's gated curve, each checked against hand-computed answers,
//! then the variants end to end over a fake Jev. No network, no key.
//!
//! Provenance: PLAT-1029, PLAT-1024, MP-241.

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

use eval_v2_support::corpus::{self, Row, TruthKind};
use eval_v2_support::keys::Mode;
use eval_v2_support::metrics::{
    ContrastSpec, Direction, PairOutcome, PairVerdict, PairedContrast, SOURCE_ID_FIELD, Scored,
    paired_contrast, sign_test_p,
};
use eval_v2_support::units::split_rust_units;
use eval_v2_support::variant::{
    self, Answered, Prediction, RawAnswer, RawAnswers, Variant, wording_violations,
};
use eval_v2_support::variants::exceeds::{
    self, BAR_C_COVERAGE, CLAUSE_KEY, CLAUSES_FIELD, E1, E2, E4, KEY, KIND_KEY, MAX_CLAUSES,
    RELATION_KEY, RelationOutcome, UnitReading, assess_relation, assess_selection, gated_curve,
    is_trivial, requirement_clauses, tau_sweep,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const TEST_BODY: &str = "#[test]\nfn tc_001_refuses_oversize() {\n    \
    let error = check(\"x\".repeat(5000)).unwrap_err();\n    \
    assert_eq!(error.code, Code::Refused);\n}";

/// Three arms: two that do something, and `_ => Ok(())`, a pass-through.
const CODE_BODY: &str = "pub fn check(input: String) -> Result<(), Error> {\n    \
    match input.len() {\n        0 => Err(Error::empty()),\n        \
    n if n > 4096 => { log(\"too big }\"); Err(Error::refused()) }\n        \
    _ => Ok(()),\n    }\n}";

fn row_value(id: &str, mode: Mode, truth: &Value) -> Value {
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
                        "ac_text": "A 5000-byte request is refused with CORE_REFUSED.",
                        "context": null},
        "test": test,
        "code": code,
        "ref": null,
        "mutation": null,
        "truth": truth,
    })
}

fn exceeds_truth(answer: bool, kind: &str) -> Value {
    let alternatives: Vec<&str> = if kind == "agent_contested" {
        vec![if answer { "no" } else { "yes" }]
    } else {
        Vec::new()
    };
    json!({KEY: {"answer": answer, "kind": kind, "alternatives": alternatives,
                 "rationale": "stated"}})
}

fn rows(values: &[Value]) -> Vec<Row> {
    let text = serde_json::to_string(&json!({
        "schema": corpus::SCHEMA,
        "sampling_rule": "every row, synthetic",
        "seed": 7,
        "source_commit": "0000000",
        "rows": values,
    }))
    .unwrap();
    corpus::parse(&text).expect("synthetic corpus parses").rows
}

/// One RC row with code, for derive tests.
fn rc_row() -> Row {
    rows(&[row_value(
        "EV2-0001",
        Mode::ReqCode,
        &exceeds_truth(true, "by_construction"),
    )])
    .remove(0)
}

/// A unit's answers: `relation` is the `task_relation` distribution over
/// levels 0..=3, `kind` the `unit_kind` choice's probabilities.
fn unit_answer(unit: usize, relation: [f64; 4], kind: &[(&str, f64)]) -> Answered {
    let mut answers = RawAnswers::new();
    let probabilities: BTreeMap<String, f64> = relation
        .iter()
        .enumerate()
        .map(|(level, p)| (level.to_string(), *p))
        .collect();
    answers.insert(
        RELATION_KEY.to_owned(),
        RawAnswer::Score {
            score: 1.5,
            confidence: 0.5,
            probabilities,
        },
    );
    answers.insert(
        KIND_KEY.to_owned(),
        RawAnswer::Choice {
            label: kind
                .first()
                .map(|(l, _)| (*l).to_owned())
                .unwrap_or_default(),
            confidence: 0.5,
            probabilities: kind.iter().map(|(l, p)| ((*l).to_owned(), *p)).collect(),
        },
    );
    Answered {
        unit: Some(unit),
        answers,
    }
}

fn decided(outcome: &RelationOutcome) -> &Prediction {
    match outcome {
        RelationOutcome::Decided(prediction) => prediction,
        other @ RelationOutcome::TraceSuspect { .. } => {
            panic!("expected a decided row, got {other:?}")
        }
    }
}

fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("a value");
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

// ---------------------------------------------------------------------------
// The clause splitter
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1029, MP-241. Sentences split after `.`/`;`, not inside
/// `4.5` or after `e.g.`; `and` never splits; a piece under three words joins
/// the clause before it; an AC restating its statement adds no clause.
#[test]
fn tc_1029_clauses_follow_the_documented_rule() {
    let clauses = requirement_clauses(
        "The system shall accept version 4.5 requests, e.g. from the CLI and the API; \
         it shall refuse the rest. Always.",
        Some("The system shall accept version 4.5 requests,  e.g. from the CLI and the API."),
    );
    assert_eq!(
        clauses,
        [
            "The system shall accept version 4.5 requests, e.g. from the CLI and the API",
            "it shall refuse the rest. Always.",
        ]
    );
}

/// Provenance: PLAT-1029, MP-241. A list item starts a clause; a wrapped
/// line continues one; a blank line ends one.
#[test]
fn tc_1029_list_items_are_clauses() {
    let clauses = requirement_clauses(
        "The service shall:",
        Some(
            "- reject an empty body\n- log each refusal with\n  its request id\n\n\
             2) retry a transient failure",
        ),
    );
    assert_eq!(
        clauses,
        [
            "The service shall:",
            "reject an empty body",
            "log each refusal with its request id",
            "retry a transient failure",
        ]
    );
}

/// Provenance: PLAT-1029, MP-241. At most `MAX_CLAUSES`; the overflow joins
/// the last, so no requirement text is dropped.
#[test]
fn tc_1029_clauses_are_capped_without_dropping_text() {
    let statement: String = (1..=15)
        .map(|n| format!("Rule number {n} holds."))
        .collect::<Vec<_>>()
        .join(" ");
    let clauses = requirement_clauses(&statement, None);
    assert_eq!(clauses.len(), MAX_CLAUSES);
    assert_eq!(clauses[10], "Rule number 11 holds.");
    assert_eq!(
        clauses[11],
        "Rule number 12 holds. Rule number 13 holds. Rule number 14 holds. Rule number 15 holds."
    );
    assert!(requirement_clauses("  \n ", Some("...")).is_empty());
}

// ---------------------------------------------------------------------------
// The pre-filter
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1029, MP-241. A pass-through arm or block is trivial; an
/// arm that calls something is not; a function never is.
#[test]
fn tc_1029_only_pass_through_branches_are_trivial() {
    let arms = split_rust_units(
        "fn f(x: Option<u8>) -> Result<u8, E> {\n    match x {\n        \
         None => return Err(E::Missing),\n        Some(0) => Ok(()),\n        \
         Some(1) => { /* keep */ Err(\"literal\") }\n        Some(2) => Ok(double(2)),\n        \
         Some(n) => Ok(n + 1),\n    }\n}",
    );
    let trivial: Vec<(&str, bool)> = arms
        .iter()
        .map(|unit| (unit.label.as_str(), is_trivial(unit)))
        .collect();
    assert_eq!(
        trivial,
        [
            ("match arm None", true),
            ("match arm Some(0)", true),
            ("match arm Some(1)", true),
            ("match arm Some(2)", false),
            ("match arm Some(n)", false),
        ]
    );
    let blocks = split_rust_units("fn g(x: bool) -> u8 { if x { 1 } else { compute() } }");
    assert_eq!(
        blocks.iter().map(is_trivial).collect::<Vec<_>>(),
        [true, false]
    );
    let functions = split_rust_units("fn a() -> u8 { 1 }\nfn b() -> u8 { 2 }");
    assert!(functions.iter().all(|unit| !is_trivial(unit)));
}

/// Provenance: PLAT-1029, MP-241. Trivial units are never asked about: E1
/// and E2 send one request per non-trivial unit, E4 sends E1's request, and
/// E2's state lists the clauses its choice labels name.
#[test]
fn tc_1029_trivial_units_are_not_asked_about() {
    let row = rc_row();
    let units = |variant: &Variant| -> Vec<Option<usize>> {
        (variant.asks)(&row).iter().map(|ask| ask.unit).collect()
    };
    assert_eq!(units(&E1), [Some(0), Some(1)]);
    assert_eq!(units(&E2), [Some(0), Some(1)]);
    let e1 = serde_json::to_string(&(E1.asks)(&row)[0].request).unwrap();
    let e4 = serde_json::to_string(&(E4.asks)(&row)[0].request).unwrap();
    assert_eq!(e1, e4, "E4 must reuse E1's request so it costs no call");

    let ask = &(E2.asks)(&row)[1];
    let state = serde_json::to_value(&ask.request.state).unwrap();
    assert_eq!(
        state[CLAUSES_FIELD],
        json!({"C1": "The system shall refuse a request larger than 4096 bytes.",
               "C2": "A 5000-byte request is refused with CORE_REFUSED."})
    );
    assert_eq!(
        state["symbol_body"],
        "n if n > 4096 => { log(\"too big }\"); Err(Error::refused()) }"
    );
    let question = serde_json::to_value(&ask.request.questions[CLAUSE_KEY]).unwrap();
    let mut labels: Vec<&str> = question["criteria"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    labels.sort_unstable();
    assert_eq!(labels, ["C1", "C2", "none"]);
}

/// Provenance: PLAT-1029, PLAT-1024 (Peter's scope ruling). E1, E2 and E4
/// run on RC and RTC only, declare the code, and never mention a test.
#[test]
fn tc_1029_the_variants_respect_their_modes() {
    let all = rows(&[
        row_value("EV2-0001", Mode::Req, &json!({})),
        row_value("EV2-0002", Mode::ReqTest, &json!({})),
        row_value("EV2-0003", Mode::ReqCode, &json!({})),
        row_value("EV2-0004", Mode::ReqTestCode, &json!({})),
    ]);
    for variant in [E1, E2, E4] {
        let applies: Vec<Mode> = all
            .iter()
            .filter(|row| variant.applies_to(row))
            .map(|row| row.mode)
            .collect();
        assert_eq!(
            applies,
            [Mode::ReqCode, Mode::ReqTestCode],
            "{}",
            variant.id
        );
        for row in &all {
            let misapplied = Variant {
                modes: &Mode::ALL,
                ..variant
            };
            let violations = wording_violations(&misapplied, row);
            assert_eq!(
                violations.is_empty(),
                row.mode.has_code(),
                "{} on {}: {violations:#?}",
                variant.id,
                row.mode.as_str()
            );
            assert!(
                !violations.iter().any(|v| v.contains("the test")),
                "{} mentions a test: {violations:#?}",
                variant.id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E1 and E4
// ---------------------------------------------------------------------------

const ADDS: [(&str, f64); 4] = [
    ("adds_behaviour", 0.7),
    ("states_behaviour", 0.3),
    ("supports_requirement", 0.0),
    ("unrelated", 0.0),
];
const STATES: [(&str, f64); 4] = [
    ("states_behaviour", 0.8),
    ("unrelated", 0.2),
    ("supports_requirement", 0.0),
    ("adds_behaviour", 0.0),
];
const NEEDED: [f64; 4] = [0.05, 0.1, 0.25, 0.6];

/// Provenance: PLAT-1029, MP-241. A unit with a strict majority of BOTH
/// answers on "not needed" makes the row `yes`, at the weaker answer's mass.
#[test]
fn tc_1029_e1_says_yes_when_both_answers_say_not_needed() {
    let row = rc_row();
    let answered = [
        unit_answer(0, NEEDED, &STATES),
        unit_answer(1, [0.1, 0.7, 0.1, 0.1], &ADDS),
    ];
    let assessment = assess_relation(&row, &answered, true).unwrap();
    assert_eq!(assessment.trivial, 1, "`_ => Ok(())` is skipped");
    assert_eq!((assessment.exceeding(), assessment.parked()), (1, 0));
    let prediction = decided(&assessment.outcome);
    assert_eq!(prediction.answer, "yes");
    assert_close(prediction.confidence, 0.7);
    assert_close(prediction.ordinal, 0.7);
}

/// Provenance: PLAT-1029, MP-241. When the two answers disagree the unit is
/// parked: E1 says `no` at a confidence capped at 0.5, while E4, which reads
/// the score alone, says `yes`.
#[test]
fn tc_1029_a_conflict_parks_the_unit_and_caps_the_no() {
    let row = rc_row();
    let answered = [
        unit_answer(0, NEEDED, &STATES),
        unit_answer(1, [0.1, 0.8, 0.05, 0.05], &STATES),
    ];
    let e1 = assess_relation(&row, &answered, true).unwrap();
    assert_eq!((e1.exceeding(), e1.parked()), (0, 1));
    let no = decided(&e1.outcome);
    assert_eq!(no.answer, "no");
    // Strength = min(0.9, 0.2) = 0.2, so 1 - 0.2 = 0.8, capped at 0.5.
    assert_close(no.confidence, 0.5);

    let e4 = assess_relation(&row, &answered, false).unwrap();
    assert_eq!(e4.parked(), 0);
    let yes = decided(&e4.outcome);
    assert_eq!(yes.answer, "yes");
    assert_close(yes.confidence, 0.9);
}

/// Provenance: PLAT-1029, MP-241. With no unit exceeding and none parked,
/// `no` carries one minus the strongest unit's strength.
#[test]
fn tc_1029_a_clean_row_says_no() {
    let row = rc_row();
    let answered = [
        unit_answer(0, NEEDED, &STATES),
        unit_answer(1, [0.1, 0.2, 0.3, 0.4], &STATES),
    ];
    let prediction = decided(&assess_relation(&row, &answered, true).unwrap().outcome).clone();
    assert_eq!(prediction.answer, "no");
    // Unit 1: min(0.3, 0.2) = 0.2; unit 0: min(0.15, 0.2) = 0.15.
    assert_close(prediction.ordinal, 0.2);
    assert_close(prediction.confidence, 0.8);
}

/// Provenance: PLAT-1029, MP-241. The breaker trips on a strict majority of
/// units with most mass on "no visible connection": the row is unanswered and
/// reported as trace-suspect, even when a unit would exceed. Exactly half does
/// not trip it.
#[test]
fn tc_1029_the_circuit_breaker_reports_a_trace_problem() {
    let row = rc_row();
    let unrelated = [0.6, 0.3, 0.05, 0.05];
    let tripped = assess_relation(
        &row,
        &[
            unit_answer(0, unrelated, &ADDS),
            unit_answer(1, unrelated, &ADDS),
            unit_answer(2, NEEDED, &STATES),
        ],
        true,
    )
    .unwrap();
    assert_eq!(
        tripped.outcome,
        RelationOutcome::TraceSuspect {
            unrelated: 2,
            considered: 3
        }
    );
    assert!(
        (E1.derive)(
            &row,
            &[
                unit_answer(0, unrelated, &ADDS),
                unit_answer(1, unrelated, &ADDS)
            ]
        )
        .is_empty()
    );

    let half = assess_relation(
        &row,
        &[
            unit_answer(0, unrelated, &ADDS),
            unit_answer(1, NEEDED, &STATES),
        ],
        true,
    )
    .unwrap();
    assert_eq!(decided(&half.outcome).answer, "yes");
}

/// Provenance: PLAT-1029, MP-241 (review of #620). A missing or mis-keyed
/// distribution fails loudly, naming the row and the unit. It never reads as
/// zero mass and never turns the row into a quiet "unanswered".
#[test]
fn tc_1029_an_unreadable_answer_fails_loudly() {
    let row = rc_row();
    let with_relation = |probabilities: &[(&str, f64)]| {
        let mut answer = unit_answer(1, NEEDED, &STATES);
        answer.answers.insert(
            RELATION_KEY.to_owned(),
            RawAnswer::Score {
                score: 3.0,
                confidence: 0.9,
                probabilities: probabilities
                    .iter()
                    .map(|(k, p)| ((*k).to_owned(), *p))
                    .collect(),
            },
        );
        assess_relation(&row, &[unit_answer(0, NEEDED, &STATES), answer], true).unwrap_err()
    };
    assert_eq!(
        with_relation(&[]),
        "EV2-0001 unit 1: the score came back with no distribution"
    );
    assert_eq!(
        with_relation(&[("1", 0.5), ("7", 0.5)]),
        "EV2-0001 unit 1: score key \"7\" is not a level 0..4"
    );
    assert_eq!(
        with_relation(&[("1", 0.2), ("3", 0.2)]),
        "EV2-0001 unit 1: the distribution totals 0.4, not 1"
    );
    let kind = |kind: &[(&str, f64)]| {
        assess_relation(&row, &[unit_answer(0, NEEDED, kind)], true).unwrap_err()
    };
    assert_eq!(
        kind(&[("adds_behaviour", 1.0)]),
        "EV2-0001 unit 0: the choice gave no probability for \"states_behaviour\""
    );
    assert!(
        kind(&[("adds", 1.0)]).contains("choice key \"adds\" is not one of"),
        "a mis-keyed label is refused"
    );
    let mut missing = unit_answer(0, NEEDED, &STATES);
    missing.answers.remove(RELATION_KEY);
    assert_eq!(
        assess_relation(&row, &[missing], false).unwrap_err(),
        "EV2-0001 unit 0: `task_relation` is missing from the answers"
    );

    let reading = UnitReading {
        unit: 0,
        relation_low: 0.5,
        unrelated: 0.5,
        kind_low: Some(0.9),
    };
    assert!(
        !reading.exceeds() && reading.parked() && !reading.looks_unrelated(),
        "0.5 is not a strict majority"
    );
}

/// Provenance: PLAT-1029, MP-241. The variant stops the run rather than
/// score an unreadable answer.
#[test]
#[should_panic(expected = "PLAT-1029: unreadable answer: EV2-0001 unit 0")]
fn tc_1029_e1_stops_on_an_unreadable_answer() {
    let row = rc_row();
    let mut answer = unit_answer(0, NEEDED, &STATES);
    answer.answers.remove(KIND_KEY);
    let _ = (E1.derive)(&row, &[answer]);
}

// ---------------------------------------------------------------------------
// E2
// ---------------------------------------------------------------------------

fn clause_answer(unit: usize, none: f64) -> Answered {
    let mut answers = RawAnswers::new();
    answers.insert(
        CLAUSE_KEY.to_owned(),
        RawAnswer::Choice {
            label: "C1".to_owned(),
            confidence: 1.0 - none,
            probabilities: [
                ("C1".to_owned(), 1.0 - none),
                ("C2".to_owned(), 0.0),
                ("none".to_owned(), none),
            ]
            .into_iter()
            .collect(),
        },
    );
    Answered {
        unit: Some(unit),
        answers,
    }
}

/// Provenance: PLAT-1029, MP-241. E2 says `yes` when any unit's `P(none)`
/// is at least tau (0.5, inclusive), at that probability; otherwise `no` at
/// one minus the highest. A unit with no distribution, or one keyed off the
/// row's clauses, is an error.
#[test]
fn tc_1029_e2_reads_the_none_probability() {
    let row = rc_row();
    let yes = assess_selection(&row, &[clause_answer(0, 0.2), clause_answer(1, 0.5)])
        .unwrap()
        .unwrap();
    assert_eq!(yes.answer, "yes");
    assert_close(yes.confidence, 0.5);
    let no = assess_selection(&row, &[clause_answer(0, 0.2), clause_answer(1, 0.49)])
        .unwrap()
        .unwrap();
    assert_eq!(no.answer, "no");
    assert_close(no.confidence, 0.51);
    assert_close(no.ordinal, 0.49);

    let mut blank = clause_answer(1, 0.9);
    blank.answers.insert(
        CLAUSE_KEY.to_owned(),
        RawAnswer::Choice {
            label: "none".to_owned(),
            confidence: 0.9,
            probabilities: BTreeMap::new(),
        },
    );
    assert_eq!(
        assess_selection(&row, &[clause_answer(0, 0.1), blank]).unwrap_err(),
        "EV2-0001 unit 1: the choice came back with no distribution"
    );
    let mut third = clause_answer(1, 0.2);
    if let Some(RawAnswer::Choice { probabilities, .. }) = third.answers.get_mut(CLAUSE_KEY) {
        probabilities.insert("C3".to_owned(), 0.0);
    }
    assert!(
        assess_selection(&row, &[third])
            .unwrap_err()
            .contains("choice key \"C3\" is not one of")
    );
}

// ---------------------------------------------------------------------------
// The tau sweep and E3
// ---------------------------------------------------------------------------

fn scored(id: &str, expected: &str, answer: Option<(&str, f64, f64)>, kind: TruthKind) -> Scored {
    Scored {
        row_id: id.to_owned(),
        mode: Mode::ReqCode,
        kind,
        expected: expected.to_owned(),
        alternatives: Vec::new(),
        prediction: answer.map(|(answer, confidence, ordinal)| Prediction {
            answer: answer.to_owned(),
            confidence: Some(confidence),
            ordinal: Some(ordinal),
        }),
    }
}

/// Provenance: PLAT-1029, MP-241. The sweep re-thresholds stored ordinals.
/// Ordinals 0.35 (yes), 0.55 (no), 0.85 (yes), 0.2 (no), plus one unanswered
/// yes. At 0.3: yes,yes,yes,no -> 3/5, yes recall 2/3, no recall 1/2. At 0.6:
/// no,no,yes,no -> 3/5, yes recall 1/3, no recall 2/2.
#[test]
fn tc_1029_the_tau_sweep_rethresholds_stored_ordinals() {
    let k = TruthKind::ByConstruction;
    let rows = [
        scored("a", "yes", Some(("no", 0.65, 0.35)), k),
        scored("b", "no", Some(("yes", 0.55, 0.55)), k),
        scored("c", "yes", Some(("yes", 0.85, 0.85)), k),
        scored("d", "no", Some(("no", 0.8, 0.2)), k),
        scored("e", "yes", None, k),
    ];
    let sweep = tau_sweep(&rows, &[0.3, 0.6]);
    assert_close(sweep[0].agreement, 60.0);
    assert_close(sweep[0].yes_recall, 200.0 / 3.0);
    assert_close(sweep[0].no_recall, 50.0);
    assert_close(sweep[1].agreement, 60.0);
    assert_close(sweep[1].yes_recall, 100.0 / 3.0);
    assert_close(sweep[1].no_recall, 100.0);
    assert_close(Some(sweep[0].baseline), 60.0);
}

/// Provenance: PLAT-1029, MP-241 bar C. E3's floor is the confidence of the
/// row at rank ceil(target% x total); ties at the floor are kept; accuracy
/// leaves abstentions out; an unanswered row is never kept; a target past
/// the rows with a confidence is unreachable.
#[test]
fn tc_1029_the_gated_curve_keeps_the_most_confident_rows() {
    let k = TruthKind::Mechanical;
    let rows = [
        scored("a", "yes", Some(("yes", 0.9, 0.9)), k),
        scored("b", "no", Some(("no", 0.8, 0.2)), k),
        scored("c", "no", Some(("yes", 0.7, 0.7)), k),
        scored("d", "yes", Some(("yes", 0.7, 0.7)), k),
        scored("e", "no", None, k),
    ];
    let curve = gated_curve(&rows, &[BAR_C_COVERAGE, 40, 100]);
    // 60% of 5 -> rank 3 -> floor 0.7, and the tie at 0.7 keeps 4 rows.
    assert_eq!(
        (
            curve[0].floor,
            curve[0].kept,
            curve[0].correct,
            curve[0].total
        ),
        (Some(0.7), 4, 3, 5)
    );
    assert_close(curve[0].accuracy(), 75.0);
    assert_close(curve[0].coverage(), 80.0);
    assert_eq!(
        (curve[1].floor, curve[1].kept, curve[1].correct),
        (Some(0.8), 2, 2)
    );
    assert_eq!((curve[2].floor, curve[2].kept), (None, 0));
    assert_eq!(curve[2].accuracy(), None);

    // The abstention ceiling is 20%: 1 of 5 is at it, 2 of 5 above it.
    assert!(!exceeds::above_ceiling(1, 5));
    assert!(exceeds::above_ceiling(2, 5));
    assert_eq!(exceeds::answered_only(&rows).len(), 4);
}

// ---------------------------------------------------------------------------
// End to end over a fake Jev
// ---------------------------------------------------------------------------

/// A fake Jev leaning one way. With `yes`, every `task_relation` puts 0.8 on
/// level 1, every `unit_kind` 0.7 on `adds_behaviour`, every `serves_clause`
/// 0.7 on `none`, and every `noul` answers 0.8. Without it: level 3,
/// `states_behaviour`, `C1`, and 0.2.
struct FakeJev {
    yes: bool,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for FakeJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let mut answers = serde_json::Map::new();
        let pick = |a: &str, b: &str| if self.yes { a.to_owned() } else { b.to_owned() };
        for (key, question) in body["questions"].as_object().unwrap() {
            let answer = match question["type"].as_str().unwrap() {
                "noul" => json!({"type": "noul", "noul": if self.yes { 0.8 } else { 0.2 }}),
                "choice" => {
                    let (top, other) = if key == CLAUSE_KEY {
                        (pick("none", "C1"), pick("C1", "none"))
                    } else if key == KIND_KEY {
                        (
                            pick("adds_behaviour", "states_behaviour"),
                            pick("states_behaviour", "adds_behaviour"),
                        )
                    } else {
                        let first = question["criteria"]
                            .as_object()
                            .unwrap()
                            .keys()
                            .min()
                            .unwrap()
                            .clone();
                        (first.clone(), first)
                    };
                    let mut probabilities = serde_json::Map::new();
                    for label in question["criteria"].as_object().unwrap().keys() {
                        probabilities.insert(label.clone(), json!(0.0));
                    }
                    probabilities.insert(other.clone(), json!(0.3));
                    probabilities.insert(top.clone(), json!(if top == other { 1.0 } else { 0.7 }));
                    json!({"type": "choice", "choice": top, "confidence": 0.7,
                           "probabilities": probabilities})
                }
                _ if key == RELATION_KEY => {
                    let probabilities = if self.yes {
                        json!({"0": 0.05, "1": 0.8, "2": 0.1, "3": 0.05})
                    } else {
                        json!({"0": 0.05, "1": 0.05, "2": 0.1, "3": 0.8})
                    };
                    json!({"type": "score", "score": 1.0, "confidence": 0.8,
                           "legend": {}, "probabilities": probabilities})
                }
                _ => json!({"type": "score", "score": 3.0, "confidence": 0.6,
                            "legend": {}, "probabilities": {"3": 1.0}}),
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

fn fake_client(yes: bool) -> (typesafe_sdk_client::Client, Arc<FakeJev>) {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(FakeJev {
        yes,
        calls: AtomicUsize::new(0),
    });
    (
        quoin_jev::client::with_transport(config, fake.clone()),
        fake,
    )
}

/// Provenance: PLAT-1029, MP-241. E0, E1, E2 and E4 run end to end: two
/// non-trivial units per code row, E4 reusing E1's requests, answers rolled
/// up and graded, and the diagnostics report every section with the
/// agent-labelled slice named.
#[tokio::test]
async fn tc_1029_the_variants_run_end_to_end() {
    let rows = rows(&[
        row_value(
            "EV2-0001",
            Mode::ReqCode,
            &exceeds_truth(true, "by_construction"),
        ),
        row_value(
            "EV2-0002",
            Mode::ReqTestCode,
            &exceeds_truth(true, "agent_dual"),
        ),
    ]);
    let (client, fake) = fake_client(true);
    let variants: Vec<&Variant> = variant::resolve("E0,E0-RC,E1,E2,E4").unwrap();
    let output = variant::run(&client, &rows, &variants).await.unwrap();

    // E0: 1 (RTC only). E0-RC: 1 (RC only). E1: 2 units x 2 rows. E4:
    // reuses E1's 4. E2: 4.
    assert_eq!(fake.calls.load(Ordering::SeqCst), 10);
    assert_eq!((output.requests_sent, output.requests_reused), (10, 4));

    for label in ["E0@v1", "E0-RC@v1", "E1@v1", "E2@v1", "E4@v1"] {
        let graded = eval_v2_support::metrics::scored(&rows, &output, label, KEY);
        assert!(
            graded
                .iter()
                .all(|row| row.prediction.as_ref().unwrap().answer == "yes"),
            "{label}: {graded:#?}"
        );
    }

    let report = exceeds::render_diagnostics(&rows, &output);
    println!("{report}");
    for needle in [
        "#### E1@v1: units and the circuit breaker",
        "| 2 | 2 | 0 | 2 | 4 | 0 | 4 |",
        "#### E1@v1: on the rows it answered (bars A and B)",
        "E1@v1 vs E0@v1 on the 1 RTC row(s) both answered (incl. AGENT-LABELLED): 100.0% vs 100.0%.",
        "#### E2@v1: tau sweep",
        "#### E3 over E0@v1",
        "#### E3 over E4@v1",
        "mode RTC [AGENT-LABELLED]",
        "E1@v1 vs E0-RC@v1 on the 1 RC row(s) both answered: 100.0% vs 100.0%.",
        "Bar C, mode RC: accuracy at >= 60% coverage 100.0% vs E0-RC@v1 overall (rows it answered) 100.0% -> fails",
        "Bar D, E1@v1: 0 succeeded, 0 failed (0 of them abstentions), 0 tied, 0 excluded (source already `yes`); one-sided sign test p = n/a -> not gateable (< 10 decided pairs): no claim",
        "Bar C, mode RTC [AGENT-LABELLED]: accuracy at >= 60% coverage 100.0% vs E0@v1 overall (rows it answered) 100.0% -> fails",
    ] {
        assert!(report.contains(needle), "missing {needle:?} in:\n{report}");
    }

    let (client, _) = fake_client(false);
    let output = variant::run(&client, &rows, &variants).await.unwrap();
    for label in ["E1@v1", "E2@v1", "E4@v1"] {
        let graded = eval_v2_support::metrics::scored(&rows, &output, label, KEY);
        assert!(
            graded
                .iter()
                .all(|row| row.prediction.as_ref().unwrap().answer == "no"),
            "{label}: {graded:#?}"
        );
    }
}

/// Provenance: PLAT-1029, MP-241. A run with no E-variant adds no report.
#[test]
fn tc_1029_no_e_variant_no_report() {
    assert_eq!(
        exceeds::render_diagnostics(&[], &variant::RunOutput::default()),
        ""
    );
}

// ---------------------------------------------------------------------------
// E0-RC, bar D and the breaker cross-tab
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1029, MP-241. E0-RC sends exactly `FullBatteryV1`'s
/// `code_exceeds_requirement` question, byte for byte, and nothing else; it
/// runs on RC only, with no test field in its state.
#[test]
fn tc_1029_e0_rc_is_e0s_question_alone() {
    let battery = gap_semantic_support::question_set(gap_semantic_support::Variant::FullBatteryV1);
    let asked = exceeds::e0_rc_questions();
    assert_eq!(asked.keys().collect::<Vec<_>>(), [KEY]);
    assert_eq!(
        serde_json::to_value(&asked[KEY]).unwrap(),
        serde_json::to_value(&battery[KEY]).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&asked[KEY]).unwrap()["instructions"],
        "Does the code implement behaviour no requirement states?"
    );
    let row = rc_row();
    assert!(exceeds::E0_RC.applies_to(&row));
    assert!(wording_violations(&exceeds::E0_RC, &row).is_empty());
    let state = serde_json::to_value(&(exceeds::E0_RC.asks)(&row)[0].request.state).unwrap();
    assert!(state.get("test_body").is_none() && state.get("symbol_body").is_some());
}

/// A mutant row of `kind`: the RC fixture with a mutation block.
fn mutant(id: &str, kind: &str) -> Value {
    let mut value = row_value(id, Mode::ReqCode, &exceeds_truth(true, "by_construction"));
    value["mutation"] = json!({"id": format!("M-{id}"), "target": "code", "kind": kind,
                               "description": "adds a branch", "patch": "p"});
    value
}

/// A mutant of `mode` carrying mutation id `mutation`, so one mutation can
/// appear in several modes.
fn same_mutation(id: &str, mode: Mode, mutation: &str) -> Value {
    let mut value = row_value(id, mode, &exceeds_truth(true, "by_construction"));
    value["mutation"] = json!({"id": mutation, "target": "code", "kind": "additive_code",
                               "description": "adds a branch", "patch": "p"});
    value
}

/// A run output with one result per `(row, ordinal)` for `variant` on
/// [`KEY`]; `None` is a row the variant ran on and left unanswered.
fn output_of(variant: &str, ordinals: &[(&str, Option<f64>)]) -> variant::RunOutput {
    output_on(variant, KEY, ordinals)
}

/// [`output_of`] on any key.
fn output_on(
    variant: &str,
    key: &'static str,
    ordinals: &[(&str, Option<f64>)],
) -> variant::RunOutput {
    variant::RunOutput {
        results: ordinals
            .iter()
            .map(|(row, ordinal)| variant::RowResult {
                row_id: (*row).to_owned(),
                variant: variant.to_owned(),
                predictions: ordinal
                    .map(|o| {
                        variant::Predictions::from([(
                            key,
                            Prediction {
                                answer: if o >= 0.5 { "yes" } else { "no" }.to_owned(),
                                confidence: Some(o.max(1.0 - o)),
                                ordinal: Some(o),
                            },
                        )])
                    })
                    .unwrap_or_default(),
                answered: Vec::new(),
            })
            .collect(),
        ..variant::RunOutput::default()
    }
}

/// A synthetic pairing standing in for `mutation.source_id` (pending in
/// PLAT-1025): each `(mutant, source)` in `pairs`.
fn pairing(pairs: &[(&'static str, &'static str)]) -> impl Fn(&Row) -> Option<String> {
    let pairs = pairs.to_vec();
    move |row: &Row| {
        pairs
            .iter()
            .find(|(mutant, _)| *mutant == row.id)
            .map(|(_, source)| (*source).to_owned())
    }
}

/// Bar D's spec for `code_exceeds_requirement`, as MP-241 registers it.
fn up_spec() -> ContrastSpec<'static> {
    ContrastSpec {
        variant: "E1@v1",
        modes: E1.modes,
        key: KEY,
        kinds: &["additive_code"],
        direction: Direction::Up,
        tau: exceeds::PAIRED_TAU,
        delta: exceeds::PAIRED_DELTA,
        defect_side: &["yes"],
    }
}

/// A natural source row labelled `yes`/`no` on [`KEY`].
fn source(id: &str, exceeds: bool) -> Value {
    row_value(id, Mode::ReqCode, &exceeds_truth(exceeds, "agent_dual"))
}

/// Each pair's `(mutant, source, verdict)`.
fn verdicts(contrast: &PairedContrast) -> Vec<(&str, &str, PairVerdict)> {
    contrast
        .pairs
        .iter()
        .map(|p| (p.mutant.as_str(), p.source.as_str(), p.verdict))
        .collect()
}

/// A contrast with the given verdict counts, for the sign test.
fn counted(successes: usize, failures: usize, ties: usize) -> PairedContrast {
    PairedContrast {
        pairs: (0..successes + failures + ties)
            .map(|index| PairOutcome {
                mutant: format!("M{index}"),
                source: "S".to_owned(),
                shift: Some(0.0),
                verdict: if index < successes {
                    PairVerdict::Success
                } else if index < successes + failures {
                    PairVerdict::Failure
                } else {
                    PairVerdict::Tie
                },
            })
            .collect(),
        excluded_source_on_defect_side: 0,
    }
}

/// Provenance: PLAT-1029, MP-241 bar D. A mutant is paired with the source
/// row its `mutation.source_id` names, and no other: the same mutant scores
/// a success against one source and a tie against another.
#[test]
fn tc_1029_bar_d_pairs_by_source_id() {
    assert_eq!(SOURCE_ID_FIELD, "mutation.source_id");
    let all = rows(&[
        source("EV2-0001", false),
        source("EV2-0002", false),
        mutant("EV2-0011", "additive_code"),
    ]);
    let output = output_of(
        "E1@v1",
        &[
            ("EV2-0001", Some(0.20)),
            ("EV2-0002", Some(0.45)),
            ("EV2-0011", Some(0.52)),
        ],
    );
    let to_first = paired_contrast(
        &all,
        &output,
        &up_spec(),
        &pairing(&[("EV2-0011", "EV2-0001")]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&to_first),
        [("EV2-0011", "EV2-0001", PairVerdict::Success)]
    );
    let shift = to_first.pairs[0].shift.unwrap();
    assert!((shift - 0.32).abs() < 1e-9, "{shift}");
    let to_second = paired_contrast(
        &all,
        &output,
        &up_spec(),
        &pairing(&[("EV2-0011", "EV2-0002")]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&to_second),
        [("EV2-0011", "EV2-0002", PairVerdict::Tie)]
    );
}

/// Provenance: PLAT-1029, MP-241 bar D. One pair per mutation id: for a
/// variant on every mode, a mutation seen in RC, RTC and RT is paired once,
/// from its RTC row, whatever the row order; without an RTC row, RC is used
/// before RT. Only listed kinds are paired.
#[test]
fn tc_1029_bar_d_takes_one_pair_per_mutation_preferring_rtc() {
    let all = rows(&[
        source("EV2-0001", false),
        same_mutation("EV2-0011", Mode::ReqCode, "M-X"),
        same_mutation("EV2-0012", Mode::ReqTestCode, "M-X"),
        same_mutation("EV2-0013", Mode::ReqTest, "M-X"),
        same_mutation("EV2-0014", Mode::ReqTest, "M-Y"),
        same_mutation("EV2-0015", Mode::ReqCode, "M-Y"),
        mutant("EV2-0016", "test_weakening"),
    ]);
    let output = output_of(
        "E1@v1",
        &[
            ("EV2-0001", Some(0.30)),
            ("EV2-0011", Some(0.90)), // M-X in RC: would succeed
            ("EV2-0012", Some(0.10)), // M-X in RTC: the pair, fails
            ("EV2-0013", Some(0.95)), // M-X in RT: would succeed
            ("EV2-0014", Some(0.90)), // M-Y in RT: would succeed
            ("EV2-0015", Some(0.05)), // M-Y in RC: the pair, fails
            ("EV2-0016", Some(0.90)), // not a listed kind
        ],
    );
    let every_mutant = |row: &Row| row.mutation.as_ref().map(|_| "EV2-0001".to_owned());
    let every_mode = ContrastSpec {
        modes: &Mode::ALL,
        ..up_spec()
    };
    let contrast = paired_contrast(&all, &output, &every_mode, &every_mutant).unwrap();
    assert_eq!(
        verdicts(&contrast),
        [
            ("EV2-0012", "EV2-0001", PairVerdict::Failure),
            ("EV2-0015", "EV2-0001", PairVerdict::Failure),
        ]
    );
}

/// Provenance: PLAT-1029, MP-241 bar D. An abstention on an in-scope row is
/// a failure, on either side of the pair and whether the row came back
/// unanswered or with no result at all; abstentions are also counted apart.
/// (A row outside the variant's modes is never paired: see
/// `tc_1029_bar_d_pairs_only_rows_in_the_variants_modes`.)
#[test]
fn tc_1029_bar_d_counts_an_abstention_as_a_failure() {
    let all = rows(&[
        source("EV2-0001", false),
        source("EV2-0002", false),
        mutant("EV2-0011", "additive_code"),
        mutant("EV2-0012", "additive_code"),
        mutant("EV2-0013", "additive_code"),
    ]);
    // EV2-0013 is an RC row, in E1's modes, with no result at all.
    let output = output_of(
        "E1@v1",
        &[
            ("EV2-0001", Some(0.20)),
            ("EV2-0002", None),       // source unanswered
            ("EV2-0011", None),       // mutant unanswered
            ("EV2-0012", Some(0.90)), // over the unanswered source
        ],
    );
    let contrast = paired_contrast(
        &all,
        &output,
        &up_spec(),
        &pairing(&[
            ("EV2-0011", "EV2-0001"),
            ("EV2-0012", "EV2-0002"),
            ("EV2-0013", "EV2-0001"),
        ]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&contrast),
        [
            ("EV2-0011", "EV2-0001", PairVerdict::Failure),
            ("EV2-0012", "EV2-0002", PairVerdict::Failure),
            ("EV2-0013", "EV2-0001", PairVerdict::Failure),
        ]
    );
    assert_eq!(
        (contrast.failures(), contrast.unanswered(), contrast.ties()),
        (3, 3, 0)
    );
}

/// Provenance: PLAT-1029, MP-241 bar D. The helper pairs only rows in the
/// variant's modes. For RC-only E0-RC, a mutation with both an RTC and an RC
/// row is paired from its RC row, never the RTC row E0-RC did not run on, and
/// a mutation with only an RTC row is not paired at all. A source outside the
/// variant's modes is refused.
#[test]
fn tc_1029_bar_d_pairs_only_rows_in_the_variants_modes() {
    assert_eq!(exceeds::E0_RC.modes, [Mode::ReqCode]);
    let rtc_source = row_value(
        "EV2-0002",
        Mode::ReqTestCode,
        &exceeds_truth(false, "agent_dual"),
    );
    let all = rows(&[
        source("EV2-0001", false),
        rtc_source,
        same_mutation("EV2-0011", Mode::ReqTestCode, "M-X"),
        same_mutation("EV2-0012", Mode::ReqCode, "M-X"),
        same_mutation("EV2-0013", Mode::ReqTestCode, "M-Z"),
    ]);
    // E0-RC ran on the RC rows only.
    let output = output_of(
        "E0-RC@v1",
        &[("EV2-0001", Some(0.20)), ("EV2-0012", Some(0.80))],
    );
    let contrast = exceeds::bar_d(
        &all,
        &output,
        &exceeds::E0_RC,
        &pairing(&[
            ("EV2-0011", "EV2-0002"),
            ("EV2-0012", "EV2-0001"),
            ("EV2-0013", "EV2-0002"),
        ]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&contrast),
        [("EV2-0012", "EV2-0001", PairVerdict::Success)]
    );
    assert_eq!(contrast.unanswered(), 0);

    // The RC mutant pointing at an RTC source, as this corpus's RC mutants
    // do, is refused rather than scored as an abstention.
    assert_eq!(
        exceeds::bar_d(
            &all,
            &output,
            &exceeds::E0_RC,
            &pairing(&[("EV2-0012", "EV2-0002")]),
        )
        .unwrap_err(),
        "EV2-0012: source EV2-0002 is in mode RTC, which E0-RC@v1 does not run on"
    );
}

/// Provenance: PLAT-1029, MP-241 bar D. The crossing is strict on the
/// source's side: for "the mutant is at or above tau", a source already at
/// tau is on the defect side, so a move from exactly tau is a tie, however
/// large; a mutant landing exactly on tau has crossed.
#[test]
fn tc_1029_bar_d_a_source_at_tau_has_not_crossed() {
    let all = rows(&[
        source("EV2-0001", false),
        source("EV2-0002", false),
        mutant("EV2-0011", "additive_code"),
        mutant("EV2-0012", "additive_code"),
    ]);
    let output = output_of(
        "E1@v1",
        &[
            ("EV2-0001", Some(0.50)), // exactly tau
            ("EV2-0002", Some(0.35)),
            ("EV2-0011", Some(0.90)), // 0.50 -> 0.90: +0.40, source at tau
            ("EV2-0012", Some(0.50)), // 0.35 -> 0.50: lands on tau
        ],
    );
    let contrast = paired_contrast(
        &all,
        &output,
        &up_spec(),
        &pairing(&[("EV2-0011", "EV2-0001"), ("EV2-0012", "EV2-0002")]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&contrast),
        [
            ("EV2-0011", "EV2-0001", PairVerdict::Tie),
            ("EV2-0012", "EV2-0002", PairVerdict::Success),
        ]
    );
}

/// Provenance: PLAT-1029, MP-241 bar D, for a key whose defect lowers the
/// answer. With `Direction::Down` the shift is source minus mutant and the
/// crossing is source at or above tau, mutant below it: 0.80 -> 0.30
/// succeeds (shift +0.50), 0.50 -> 0.35 succeeds from exactly tau, 0.30 ->
/// 0.60 fails (shift -0.30), and 0.80 -> 0.60 moves the right way without
/// crossing, a tie.
#[test]
fn tc_1029_bar_d_reads_a_downward_contrast() {
    let all = rows(&[
        source("EV2-0001", true),
        source("EV2-0002", true),
        source("EV2-0003", true),
        mutant("EV2-0011", "additive_code"),
        mutant("EV2-0012", "additive_code"),
        mutant("EV2-0013", "additive_code"),
        mutant("EV2-0014", "additive_code"),
    ]);
    let output = output_of(
        "E1@v1",
        &[
            ("EV2-0001", Some(0.80)),
            ("EV2-0002", Some(0.30)),
            ("EV2-0003", Some(0.50)),
            ("EV2-0011", Some(0.30)),
            ("EV2-0012", Some(0.60)),
            ("EV2-0013", Some(0.60)),
            ("EV2-0014", Some(0.35)),
        ],
    );
    let down = ContrastSpec {
        direction: Direction::Down,
        defect_side: &["no"],
        ..up_spec()
    };
    let contrast = paired_contrast(
        &all,
        &output,
        &down,
        &pairing(&[
            ("EV2-0011", "EV2-0001"),
            ("EV2-0012", "EV2-0002"),
            ("EV2-0013", "EV2-0001"),
            ("EV2-0014", "EV2-0003"),
        ]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&contrast),
        [
            ("EV2-0011", "EV2-0001", PairVerdict::Success),
            ("EV2-0012", "EV2-0002", PairVerdict::Failure),
            ("EV2-0013", "EV2-0001", PairVerdict::Tie),
            ("EV2-0014", "EV2-0003", PairVerdict::Success),
        ]
    );
    let shifts: Vec<f64> = contrast.pairs.iter().map(|p| p.shift.unwrap()).collect();
    for (actual, expected) in shifts.iter().zip([0.50, -0.30, 0.20, 0.15]) {
        assert!((actual - expected).abs() < 1e-9, "{shifts:?}");
    }
    assert_eq!(contrast.excluded_source_on_defect_side, 0);
}

/// Provenance: PLAT-1029, MP-241 bar D. An ordinal that is not a finite
/// number, on either side of a pair, is refused and names its row, never
/// read as a tie.
#[test]
fn tc_1029_bar_d_refuses_a_nan_ordinal() {
    let all = rows(&[
        source("EV2-0001", false),
        mutant("EV2-0011", "additive_code"),
    ]);
    let pairs = pairing(&[("EV2-0011", "EV2-0001")]);
    let on_mutant = output_of(
        "E1@v1",
        &[("EV2-0001", Some(0.20)), ("EV2-0011", Some(f64::NAN))],
    );
    assert_eq!(
        paired_contrast(&all, &on_mutant, &up_spec(), &pairs).unwrap_err(),
        format!("EV2-0011: E1@v1 ordinal on {KEY} is NaN, not a finite number")
    );
    let on_source = output_of(
        "E1@v1",
        &[("EV2-0001", Some(f64::NAN)), ("EV2-0011", Some(0.90))],
    );
    assert_eq!(
        paired_contrast(&all, &on_source, &up_spec(), &pairs).unwrap_err(),
        format!("EV2-0001: E1@v1 ordinal on {KEY} is NaN, not a finite number")
    );
}

/// Provenance: PLAT-1029, MP-241 bar D. A success must both cross tau and
/// move at least delta; a fall of at least delta fails; everything else is a
/// tie, and ties are left out of the sign test: 9 successes, 1 failure and 3
/// ties is 9 of 10 (p = 11/1024, holds), not 9 of 13.
#[test]
fn tc_1029_bar_d_excludes_ties() {
    let all = rows(&[
        source("EV2-0001", false),
        source("EV2-0002", false),
        source("EV2-0003", false),
        mutant("EV2-0011", "additive_code"),
        mutant("EV2-0012", "additive_code"),
        mutant("EV2-0013", "additive_code"),
        mutant("EV2-0014", "additive_code"),
        mutant("EV2-0015", "additive_code"),
        mutant("EV2-0016", "additive_code"),
    ]);
    let output = output_of(
        "E1@v1",
        &[
            ("EV2-0001", Some(0.30)),
            ("EV2-0002", Some(0.45)),
            ("EV2-0003", Some(0.40)),
            ("EV2-0011", Some(0.55)), // 0.30 -> 0.55: crosses, +0.25
            ("EV2-0012", Some(0.45)), // 0.30 -> 0.45: +0.15, no cross
            ("EV2-0013", Some(0.52)), // 0.45 -> 0.52: crosses, +0.07
            ("EV2-0014", Some(0.25)), // 0.30 -> 0.25: -0.05
            ("EV2-0015", Some(0.18)), // 0.30 -> 0.18: -0.12
            ("EV2-0016", Some(0.50)), // 0.40 -> 0.50: crosses, exactly +0.10
        ],
    );
    let contrast = paired_contrast(
        &all,
        &output,
        &up_spec(),
        &pairing(&[
            ("EV2-0011", "EV2-0001"),
            ("EV2-0012", "EV2-0001"),
            ("EV2-0013", "EV2-0002"),
            ("EV2-0014", "EV2-0001"),
            ("EV2-0015", "EV2-0001"),
            ("EV2-0016", "EV2-0003"),
        ]),
    )
    .unwrap();
    let only: Vec<PairVerdict> = verdicts(&contrast).into_iter().map(|v| v.2).collect();
    assert_eq!(
        only,
        [
            PairVerdict::Success,
            PairVerdict::Tie,
            PairVerdict::Tie,
            PairVerdict::Tie,
            PairVerdict::Failure,
            PairVerdict::Success,
        ]
    );
    assert_eq!((contrast.decided(), contrast.ties()), (3, 3));

    let with_ties = counted(9, 1, 3);
    assert_eq!(with_ties.decided(), 10);
    assert_close(with_ties.p_value(), 11.0 / 1024.0);
    assert!(with_ties.passes());
}

/// Provenance: PLAT-1029, MP-241 bar D. A pair whose source's truth is
/// already on the defect side is excluded and counted, not scored.
#[test]
fn tc_1029_bar_d_excludes_sources_already_on_the_defect_side() {
    let all = rows(&[
        source("EV2-0001", false),
        source("EV2-0002", true),
        mutant("EV2-0011", "additive_code"),
        mutant("EV2-0012", "additive_code"),
    ]);
    let output = output_of(
        "E1@v1",
        &[
            ("EV2-0001", Some(0.20)),
            ("EV2-0002", Some(0.20)),
            ("EV2-0011", Some(0.90)),
            ("EV2-0012", Some(0.90)),
        ],
    );
    let contrast = paired_contrast(
        &all,
        &output,
        &up_spec(),
        &pairing(&[("EV2-0011", "EV2-0001"), ("EV2-0012", "EV2-0002")]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&contrast),
        [("EV2-0011", "EV2-0001", PairVerdict::Success)]
    );
    assert_eq!(contrast.excluded_source_on_defect_side, 1);
}

/// Provenance: PLAT-1029, MP-241 bar D, as the severity experiment reads
/// it. The helper is scale-generic: on raw severity levels with tau = 2
/// (`medium`) and delta = 0.5, low -> medium succeeds, low -> 1.8 is a tie,
/// medium -> low fails, and a source already `medium` or `high` is excluded.
#[test]
fn tc_1029_bar_d_reads_severity_levels() {
    let severity = |id: &str, label: &str| {
        row_value(
            id,
            Mode::ReqTestCode,
            &json!({"severity": {"answer": label, "kind": "agent_dual",
                                  "alternatives": [], "rationale": "stated"}}),
        )
    };
    let mut injected = severity("EV2-0011", "high");
    injected["mutation"] = json!({"id": "M-1", "target": "code", "kind": "drop_check",
                                  "description": "drops a check", "patch": "p"});
    let mutant_of = |id: &str| {
        let mut value = injected.clone();
        value["id"] = json!(id);
        value["mutation"]["id"] = json!(format!("M-{id}"));
        value
    };
    let all = rows(&[
        severity("EV2-0001", "low"),
        severity("EV2-0002", "none"),
        severity("EV2-0003", "medium"),
        mutant_of("EV2-0011"),
        mutant_of("EV2-0012"),
        mutant_of("EV2-0013"),
        mutant_of("EV2-0014"),
    ]);
    let output = output_on(
        "S1@v1",
        "severity",
        &[
            ("EV2-0001", Some(1.0)),
            ("EV2-0002", Some(2.1)), // mis-scored, but its truth is `none`
            ("EV2-0003", Some(2.0)),
            ("EV2-0011", Some(2.0)), // 1.0 -> 2.0: success
            ("EV2-0012", Some(1.8)), // 1.0 -> 1.8: below medium, tie
            ("EV2-0013", Some(1.4)), // 2.1 -> 1.4: fell 0.7, failure
            ("EV2-0014", Some(3.0)), // over a `medium` source: excluded
        ],
    );
    let contrast = paired_contrast(
        &all,
        &output,
        &ContrastSpec {
            variant: "S1@v1",
            modes: &[Mode::ReqTestCode],
            key: "severity",
            kinds: &["drop_check"],
            direction: Direction::Up,
            tau: 2.0,
            delta: 0.5,
            defect_side: &["medium", "high"],
        },
        &pairing(&[
            ("EV2-0011", "EV2-0001"),
            ("EV2-0012", "EV2-0001"),
            ("EV2-0013", "EV2-0002"),
            ("EV2-0014", "EV2-0003"),
        ]),
    )
    .unwrap();
    assert_eq!(
        verdicts(&contrast),
        [
            ("EV2-0011", "EV2-0001", PairVerdict::Success),
            ("EV2-0012", "EV2-0001", PairVerdict::Tie),
            ("EV2-0013", "EV2-0002", PairVerdict::Failure),
        ]
    );
    assert_eq!(contrast.excluded_source_on_defect_side, 1);
}

/// Provenance: PLAT-1029, MP-241 bar D. A pairing the corpus does not
/// support is refused, never skipped: no source, a source not loaded, a
/// source that is itself a mutant, or a source in another split.
#[test]
fn tc_1029_an_unsupported_pairing_is_refused() {
    let mut heldout = mutant("EV2-0014", "additive_code");
    heldout["split"] = json!("heldout");
    let all = rows(&[
        row_value(
            "EV2-0001",
            Mode::ReqCode,
            &exceeds_truth(false, "agent_dual"),
        ),
        mutant("EV2-0011", "additive_code"),
        heldout,
    ]);
    let output = variant::RunOutput::default();
    let spec = ContrastSpec {
        variant: "E1@v1",
        modes: E1.modes,
        key: KEY,
        kinds: &["additive_code"],
        direction: Direction::Up,
        tau: 0.5,
        delta: 0.1,
        defect_side: &["yes"],
    };
    let only = |id: &'static str, to: Option<&'static str>| {
        move |row: &Row| -> Option<String> {
            if row.id == id {
                to.map(str::to_owned)
            } else if row.mutation.is_some() {
                Some("EV2-0001".to_owned())
            } else {
                None
            }
        }
    };
    let refuse = |pairing: &dyn Fn(&Row) -> Option<String>| {
        paired_contrast(&all, &output, &spec, pairing).unwrap_err()
    };
    assert_eq!(
        refuse(&only("EV2-0011", None)),
        "EV2-0011: additive_code mutant names no mutation.source_id"
    );
    assert_eq!(
        refuse(&only("EV2-0011", Some("EV2-0999"))),
        "EV2-0011: source EV2-0999 is not among the rows loaded"
    );
    assert_eq!(
        refuse(&only("EV2-0011", Some("EV2-0014"))),
        "EV2-0011: source EV2-0014 is itself a mutant"
    );
    assert_eq!(
        refuse(&only("EV2-0000", None)),
        "EV2-0014: source EV2-0001 is in split dev, the mutant in heldout"
    );
    // The shipped stub names no source, so a run with an additive mutant
    // stops at bar D instead of reporting zero pairs.
    assert!(
        exceeds::bar_d(&all, &output, &E1, &exceeds::source_id)
            .unwrap_err()
            .contains("names no mutation.source_id")
    );
}

/// Provenance: PLAT-1029, MP-241 bar D. Gateable only at 10 or more non-tie
/// pairs: 9 of 9 is p = 1/512, far under alpha, and still makes no claim.
#[test]
fn tc_1029_bar_d_needs_ten_decided_pairs() {
    let nine = counted(9, 0, 5);
    assert_close(nine.p_value(), 1.0 / 512.0);
    assert!(!nine.gateable() && !nine.passes());
    let ten = counted(10, 0, 0);
    assert!(ten.gateable() && ten.passes());

    let render = |c: &PairedContrast| {
        let mut out = String::new();
        exceeds::render_bar_d(&mut out, "E1@v1", c);
        out
    };
    assert!(render(&nine).contains("not gateable (< 10 decided pairs): no claim"));
}

/// Provenance: PLAT-1029, MP-241 bar D. The test is a one-sided sign test
/// at alpha = 0.05: `P(X >= successes)`, `X ~ Binomial(n, 1/2)`, checked
/// against hand-computed tails. 20 of 30 (p = 0.0494) holds and 8 of 10
/// (p = 0.0547) fails, which pins alpha between them; 1 of 10 fails, where a
/// two-sided test would reject.
#[test]
fn tc_1029_bar_d_is_a_one_sided_sign_test() {
    assert_close(sign_test_p(10, 10), 1.0 / 1024.0);
    assert_close(sign_test_p(9, 10), 11.0 / 1024.0);
    assert_close(sign_test_p(8, 10), 56.0 / 1024.0);
    assert_close(sign_test_p(1, 10), 1023.0 / 1024.0);
    assert_close(sign_test_p(0, 4), 1.0);
    assert_eq!(sign_test_p(0, 0), None);
    // P(X >= 20 | n = 30) = 53_009_102 / 2^30.
    assert_close(sign_test_p(20, 30), 53_009_102.0 / 1_073_741_824.0);
    // Past n = 1074, 0.5^n underflows f64 to zero; the tail must not. Exact
    // values from integer binomial sums.
    assert_close(sign_test_p(1000, 2000), 0.508_919_505_572_927_2);
    assert_close(sign_test_p(0, 2000), 1.0);
    let far = sign_test_p(1100, 2000).unwrap();
    assert!((far / 4.228_544_767_751_963e-6 - 1.0).abs() < 1e-9, "{far}");

    assert!(counted(20, 10, 0).passes());
    assert!(counted(9, 1, 0).passes());
    assert!(!counted(8, 2, 0).passes());
    assert!(!counted(1, 9, 0).passes());
    assert!(!counted(0, 10, 0).passes());

    let render = |c: &PairedContrast| {
        let mut out = String::new();
        exceeds::render_bar_d(&mut out, "E1@v1", c);
        out
    };
    let holds = render(&counted(9, 1, 3));
    assert!(
        holds.contains("9 succeeded, 1 failed (0 of them abstentions), 3 tied")
            && holds.contains("p = 0.0107 -> HOLDS"),
        "{holds}"
    );
    assert!(render(&counted(8, 2, 0)).contains("p = 0.0547 -> fails"));
}

/// Provenance: PLAT-1029 (coordinator's Q2 ruling). When a trace-check
/// variant answered `trace_correct` on the same rows, E1's breaker rows are
/// cross-tabbed against its answers.
#[test]
fn tc_1029_breaker_rows_are_crosstabbed_with_the_trace_check() {
    let all = rows(&[
        row_value(
            "EV2-0001",
            Mode::ReqCode,
            &exceeds_truth(true, "agent_dual"),
        ),
        row_value(
            "EV2-0002",
            Mode::ReqCode,
            &exceeds_truth(false, "agent_dual"),
        ),
    ]);
    let unrelated = [0.7, 0.2, 0.05, 0.05];
    let result =
        |row: &str, answered: Vec<Answered>, variant: &str, key: &'static str, answer: &str| {
            variant::RowResult {
                row_id: row.to_owned(),
                variant: variant.to_owned(),
                predictions: variant::Predictions::from([(
                    key,
                    Prediction {
                        answer: answer.to_owned(),
                        confidence: Some(0.8),
                        ordinal: Some(0.8),
                    },
                )]),
                answered,
            }
        };
    let output = variant::RunOutput {
        results: vec![
            result(
                "EV2-0001",
                vec![
                    unit_answer(0, unrelated, &ADDS),
                    unit_answer(1, unrelated, &ADDS),
                ],
                "E1@v1",
                KEY,
                "yes",
            ),
            result(
                "EV2-0002",
                vec![
                    unit_answer(0, NEEDED, &STATES),
                    unit_answer(1, NEEDED, &STATES),
                ],
                "E1@v1",
                KEY,
                "no",
            ),
            result("EV2-0001", Vec::new(), "TC@v1", "trace_correct", "no"),
            result("EV2-0002", Vec::new(), "TC@v1", "trace_correct", "yes"),
        ],
        ..variant::RunOutput::default()
    };
    let report = exceeds::render_diagnostics(&all, &output);
    for needle in [
        "E1@v1 breaker vs `trace_correct`",
        "| TC@v1 | yes | no | 1 |",
        "| TC@v1 | no | yes | 1 |",
    ] {
        assert!(report.contains(needle), "missing {needle:?} in:\n{report}");
    }
}
