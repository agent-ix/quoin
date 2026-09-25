// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The criterion-soundness checklist variants K1 and K2 and MP-243's bars,
//! offline (PLAT-1031).
//!
//! Checks the derivation (each defect at τ, `criterion_sound` from the
//! five), loud failure on a missing or mis-keyed answer, the request shapes,
//! the wording rule, bars A-D (natural-only rows, fractional contested
//! credit, mutant/source pairing by `mutation.source_id`, the δ/τ pair rule
//! and the sign test), the jointly-answered C0 comparison, and K1/K2 end to
//! end through the harness runner over a fake Jev. No network, no key.
//!
//! Provenance: PLAT-1031, PLAT-1024, MP-243.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::float_cmp,
    reason = "the asserted credits are exact sums of halves and ones"
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
use eval_v2_support::keys::{self, Mode};
use eval_v2_support::metrics::scored;
use eval_v2_support::variant::{
    self, Answered, Prediction, RawAnswer, RawAnswers, RowResult, RunOutput, Variant,
    wording_violations,
};
use eval_v2_support::variants::soundness::{
    self, BarC, BarD, Bars, CHECKS, GRADES, K1, K2, KeyMargin, PairOutcome, SOUND, TAU, bars,
    credit, derive_checklist, joint, pair_outcome, pairs, render_bars, sign_test_p,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const STATEMENT: &str = "The system shall refuse oversized uploads.";
const PLAIN: &str = "A 5000-byte upload is refused with status 413.";

/// One truth entry.
fn truth(answer: &str, kind: &str, alternatives: &[&str]) -> Value {
    json!({"answer": answer, "kind": kind, "alternatives": alternatives, "rationale": "stated"})
}

/// A mode-R row. `source` makes it a mutant of that row.
fn row_value(id: &str, ac_text: &str, truth: &Value, source: Option<&str>) -> Value {
    let fr = format!("FR-{}", &id[id.len() - 3..]);
    let mutation = source.map(|source| {
        json!({"id": format!("M-{id}"), "target": "requirement", "kind": "criterion",
               "description": "d", "patch": "p", "source_id": source})
    });
    json!({
        "id": id, "mode": "R", "split": "dev",
        "strata": {"fr_id": fr, "req_kind": "functional", "test_kind": null,
                   "crate": "quoin-core", "ears_pattern": null},
        "requirement": {"fr_id": fr, "ac_id": format!("{fr}-AC-1"),
                        "statement": STATEMENT, "ac_text": ac_text, "context": null},
        "test": null, "code": null, "ref": null, "mutation": mutation,
        "truth": truth,
    })
}

fn parse(values: &[Value]) -> Vec<Row> {
    let text = serde_json::to_string(&json!({
        "schema": corpus::SCHEMA, "sampling_rule": "synthetic", "seed": 1,
        "source_commit": "0000000", "rows": values,
    }))
    .unwrap();
    corpus::parse(&text).unwrap().rows
}

/// Requirement-only natural rows, one per `(id, ac_text)`, each labelled
/// sound.
fn rows(criteria: &[(&str, &str)]) -> Vec<Row> {
    parse(
        &criteria
            .iter()
            .map(|(id, ac)| {
                row_value(
                    id,
                    ac,
                    &json!({"criterion_sound": truth("yes", "agent_dual", &[])}),
                    None,
                )
            })
            .collect::<Vec<Value>>(),
    )
}

fn answers(pairs: &[(&str, f64)]) -> Vec<Answered> {
    let answers: RawAnswers = pairs
        .iter()
        .map(|(key, p)| ((*key).to_owned(), RawAnswer::Noul(*p)))
        .collect();
    vec![Answered {
        unit: None,
        answers,
    }]
}

fn five(p: [f64; 5]) -> Vec<(&'static str, f64)> {
    CHECKS.iter().map(|check| check.key).zip(p).collect()
}

fn answer(prediction: &Prediction) -> (&str, f64) {
    (prediction.answer.as_str(), prediction.confidence.unwrap())
}

fn close(left: (&str, f64), right: (&str, f64)) -> bool {
    left.0 == right.0 && (left.1 - right.1).abs() < 1e-9
}

// ---------------------------------------------------------------------------
// The derivation
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1031, MP-243. All five checks below τ: every defect key
/// is `no` and `criterion_sound` is `yes` with confidence `1 - max`.
#[test]
fn tc_1031_no_defect_below_tau_is_sound() {
    let row = &rows(&[("EV2-0001", PLAIN)])[0];
    let out = (K1.derive)(row, &answers(&five([0.1, 0.2, 0.05, 0.3, 0.45])));
    assert!(close(answer(&out[SOUND]), ("yes", 0.55)), "{out:?}");
    assert!((out[SOUND].ordinal.unwrap() - 0.55).abs() < 1e-9);
    assert!(close(answer(&out["missing_trigger"]), ("no", 0.55)));
    assert!(close(answer(&out["vague_term"]), ("no", 0.9)));
    assert_eq!(out.len(), 6);
}

/// Provenance: PLAT-1031, MP-243. τ is inclusive: one check at exactly 0.5
/// is a defect, and one defect makes the criterion unsound with that
/// check's probability as the confidence.
#[test]
fn tc_1031_one_defect_at_tau_is_unsound() {
    assert!((TAU - 0.5).abs() < f64::EPSILON);
    let row = &rows(&[("EV2-0001", PLAIN)])[0];
    let out = (K1.derive)(row, &answers(&five([0.1, 0.5, 0.05, 0.3, 0.2])));
    assert!(close(answer(&out["no_measurable_threshold"]), ("yes", 0.5)));
    assert!(close(answer(&out[SOUND]), ("no", 0.5)), "{out:?}");
    let out = derive_checklist(&[0.9, 0.7, 0.05, 0.3, 0.2]);
    assert!(close(answer(&out[SOUND]), ("no", 0.9)), "{out:?}");
}

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 12). A missing
/// answer to an asked check aborts the run naming the row and the key.
#[test]
#[should_panic(expected = "EV2-0001 / untestable: asked but not answered")]
fn tc_1031_a_missing_check_answer_fails_loudly() {
    let row = &rows(&[("EV2-0001", PLAIN)])[0];
    let mut pairs = five([0.1; 5]);
    pairs.retain(|(key, _)| *key != "untestable");
    let _ = (K1.derive)(row, &answers(&pairs));
}

/// Provenance: PLAT-1031, MP-243. A check answered as another question type
/// is mis-keyed, and fails loudly.
#[test]
#[should_panic(expected = "EV2-0001 / compound: asked as a noul, answered as Choice")]
fn tc_1031_a_mis_keyed_check_answer_fails_loudly() {
    let row = &rows(&[("EV2-0001", PLAIN)])[0];
    let mut answered = answers(&five([0.1; 5]));
    answered[0].answers.insert(
        "compound".to_owned(),
        RawAnswer::Choice {
            label: "yes".to_owned(),
            confidence: 0.9,
            probabilities: BTreeMap::new(),
        },
    );
    let _ = (K1.derive)(row, &answered);
}

/// Provenance: PLAT-1031, MP-243. A probability outside `[0, 1]` fails
/// loudly.
#[test]
#[should_panic(expected = "EV2-0001 / vague_term: noul 1.5 is not a probability")]
fn tc_1031_an_out_of_range_probability_fails_loudly() {
    let row = &rows(&[("EV2-0001", PLAIN)])[0];
    let _ = (K1.derive)(row, &answers(&five([1.5, 0.1, 0.1, 0.1, 0.1])));
}

/// Provenance: PLAT-1031, MP-243. K2 derives exactly as K1.
#[test]
fn tc_1031_k2_derives_as_k1() {
    let row = &rows(&[("EV2-0001", PLAIN)])[0];
    for p in [[0.1, 0.2, 0.05, 0.3, 0.45], [0.6, 0.2, 0.05, 0.3, 0.45]] {
        let answered = answers(&five(p));
        assert_eq!((K1.derive)(row, &answered), (K2.derive)(row, &answered));
    }
}

// ---------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------

fn request_json(variant: &Variant, row: &Row) -> Value {
    let asks = (variant.asks)(row);
    assert_eq!(asks.len(), 1);
    serde_json::to_value(&asks[0].request).unwrap()
}

/// Provenance: PLAT-1031, MP-243. Each check is a defect key in the corpus
/// schema; the graded keys are read off `CHECKS`; K1 and K2 run in mode R
/// only, declare no artifact and are registered. K3 is not registered.
#[test]
fn tc_1031_the_checks_match_the_corpus_keys() {
    for check in CHECKS {
        let spec = keys::spec(check.key).unwrap();
        assert_eq!(spec.no_defect, "no", "{}", check.key);
        assert_eq!(spec.needs, keys::Needs::Requirement);
        assert!(
            check
                .instruction
                .starts_with("Judge the text in `criterion`, read literally."),
            "{}",
            check.key
        );
    }
    assert_eq!(
        GRADES,
        [
            "criterion_sound",
            "vague_term",
            "no_measurable_threshold",
            "untestable",
            "compound",
            "missing_trigger"
        ]
    );
    for variant in [K1, K2] {
        assert_eq!(variant.modes, [Mode::Req]);
        assert!(variant.references.is_empty());
        assert_eq!(variant.grades, GRADES);
        assert!(variant::REGISTRY.iter().any(|v| v.id == variant.id));
    }
    assert!(!variant::REGISTRY.iter().any(|v| v.id == "K3"));
}

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 9). The state is
/// the criterion plus the requirement's statement as context, as C0 sends
/// it. With no AC the statement is also the criterion.
#[test]
fn tc_1031_the_state_is_the_criterion_with_its_statement() {
    let mut rows = rows(&[("EV2-0001", PLAIN)]);
    assert_eq!(
        request_json(&K1, &rows[0])["state"],
        json!({"criterion_id": "FR-001-AC-1", "criterion": PLAIN,
               "requirement_id": "FR-001", "requirement_statement": STATEMENT})
    );
    rows[0].requirement.ac_id = None;
    rows[0].requirement.ac_text = None;
    assert_eq!(
        request_json(&K1, &rows[0])["state"],
        json!({"criterion_id": "FR-001", "criterion": STATEMENT,
               "requirement_id": "FR-001", "requirement_statement": STATEMENT})
    );
}

/// Provenance: PLAT-1031 (review of #624, finding 12). K2's worked examples
/// are in the instruction text, 3-5 per check with both outcomes present;
/// its criteria are exactly K1's.
#[test]
fn tc_1031_k2_puts_worked_examples_in_the_instruction() {
    let row = &rows(&[("EV2-0001", PLAIN)])[0];
    let k1 = request_json(&K1, row);
    let k2 = request_json(&K2, row);
    for check in CHECKS {
        let total = check.defect_examples.len() + check.clear_examples.len();
        assert!((3..=5).contains(&total), "{}: {total}", check.key);
        let (one, two) = (&k1["questions"][check.key], &k2["questions"][check.key]);
        assert_eq!(one["criteria"], two["criteria"], "{}", check.key);
        // K1@v2 asks `compound` in the canonical definition's words.
        let k1_instruction = if check.key == "compound" {
            soundness::COMPOUND_V2
        } else {
            check.instruction
        };
        assert_eq!(one["instructions"], k1_instruction);
        let text = two["instructions"].as_str().unwrap();
        assert!(text.starts_with(check.instruction), "{}", check.key);
        for example in check.defect_examples {
            assert!(text.contains(&format!("\"{}\" -> yes", example.criterion)));
        }
        for example in check.clear_examples {
            assert!(text.contains(&format!("\"{}\" -> no", example.criterion)));
        }
    }
}

/// Provenance: PLAT-1031, PLAT-1024 (Peter's scope ruling). No K variant's
/// wording refers to a test or code, including every K2 example.
#[test]
fn tc_1031_the_wording_names_no_absent_artifact() {
    let rows = rows(&[("EV2-0001", PLAIN), ("EV2-0002", "When full, it refuses.")]);
    for variant in [K1, K2] {
        for row in &rows {
            assert_eq!(wording_violations(&variant, row), Vec::<String>::new());
        }
    }
}

// ---------------------------------------------------------------------------
// Bars A-C
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 9). Credit is
/// fractional on a contested row: `1/k` for an answer among `k` readings.
#[test]
fn tc_1031_contested_credit_is_fractional() {
    let rows = parse(&[row_value(
        "EV2-0001",
        PLAIN,
        &json!({"criterion_sound": truth("yes", "agent_contested", &["no"]),
                "compound": truth("no", "agent_dual", &[])}),
        None,
    )]);
    let contested = &rows[0].truth[SOUND];
    assert_eq!(credit(contested, Some("yes")), 0.5);
    assert_eq!(credit(contested, Some("no")), 0.5);
    assert_eq!(credit(contested, None), 0.0);
    let clean = &rows[0].truth["compound"];
    assert_eq!(credit(clean, Some("no")), 1.0);
    assert_eq!(credit(clean, Some("yes")), 0.0);
}

/// A run output where `label` answered each row with the given predictions.
/// One fake prediction: key, answer, probability.
type Fake = (&'static str, &'static str, f64);

fn output(label: &str, answers: &[(&str, &[Fake])]) -> RunOutput {
    RunOutput {
        results: answers
            .iter()
            .map(|(row, predictions)| RowResult {
                answered: Vec::new(),
                row_id: (*row).to_owned(),
                variant: label.to_owned(),
                predictions: predictions
                    .iter()
                    .map(|(key, answer, p)| {
                        (
                            *key,
                            Prediction {
                                answer: (*answer).to_owned(),
                                confidence: Some(0.9),
                                ordinal: Some(*p),
                            },
                        )
                    })
                    .collect(),
            })
            .collect(),
        ..RunOutput::default()
    }
}

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 9). Bars A and B
/// read natural rows only: a mutant's `criterion_sound = no` never enters A,
/// and a tie made of half credits is not a win.
#[test]
fn tc_1031_bars_a_and_b_read_natural_rows_only() {
    let rows = parse(&[
        row_value(
            "EV2-0001",
            PLAIN,
            &json!({"criterion_sound": truth("yes", "agent_dual", &[])}),
            None,
        ),
        row_value(
            "EV2-0002",
            PLAIN,
            &json!({"criterion_sound": truth("no", "agent_dual", &[])}),
            None,
        ),
        row_value(
            "EV2-0003",
            PLAIN,
            &json!({"criterion_sound": truth("yes", "agent_contested", &["no"])}),
            None,
        ),
        // A mutant labelled unsound, which the variant gets right: if it
        // counted, A would pass.
        row_value(
            "EV2-0004",
            PLAIN,
            &json!({"criterion_sound": truth("no", "by_construction", &[])}),
            None,
        ),
    ]);
    let mut rows = rows;
    rows[3].mutation = parse(&[row_value("EV2-0009", PLAIN, &json!({}), Some("EV2-0001"))])[0]
        .mutation
        .clone();
    // Natural: sound row cleared (1), unsound row cleared wrongly (0),
    // contested (0.5) = 1.5; constant `yes` = 1 + 0 + 0.5 = 1.5. A tie.
    let run = output(
        "K1@v1",
        &[
            ("EV2-0001", &[(SOUND, "yes", 0.9)]),
            ("EV2-0002", &[(SOUND, "yes", 0.9)]),
            ("EV2-0003", &[(SOUND, "no", 0.1)]),
            ("EV2-0004", &[(SOUND, "no", 0.1)]),
        ],
    );
    let result = bars(&rows, &run, "K1@v1");
    assert_eq!(result.a.rows, 3);
    assert_eq!((result.a.credit, result.a.constant), (1.5, 1.5));
    assert!(result.a.both_classes);
    assert!(!result.a_passes(), "a tie is not a win");
    assert_eq!(result.b, (2, 1));
    assert!(result.b_passes());
}

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 2). Bar C counts
/// only checks with both classes on natural rows and needs a strict
/// majority of them; fewer than two qualifying is not gateable.
#[test]
fn tc_1031_bar_c_needs_a_majority_of_qualifying_checks() {
    let labels = |vague: &str, compound: &str, trigger: &str| {
        json!({"vague_term": truth(vague, "agent_dual", &[]),
               "compound": truth(compound, "agent_dual", &[]),
               "missing_trigger": truth(trigger, "agent_dual", &[])})
    };
    let rows = parse(&[
        row_value("EV2-0001", PLAIN, &labels("yes", "yes", "no"), None),
        row_value("EV2-0002", PLAIN, &labels("no", "no", "no"), None),
    ]);
    // vague_term right on both (beats constant 1), compound wrong on both,
    // missing_trigger has one class so does not qualify.
    let run = output(
        "K1@v1",
        &[
            (
                "EV2-0001",
                &[
                    ("vague_term", "yes", 0.9),
                    ("compound", "no", 0.1),
                    ("missing_trigger", "no", 0.1),
                ],
            ),
            (
                "EV2-0002",
                &[
                    ("vague_term", "no", 0.1),
                    ("compound", "yes", 0.9),
                    ("missing_trigger", "no", 0.1),
                ],
            ),
        ],
    );
    let c = bars(&rows, &run, "K1@v1").c;
    assert_eq!(c.qualifying, ["vague_term", "compound"]);
    assert_eq!(c.beating, ["vague_term"]);
    assert!(c.gateable());
    assert!(!c.passes(), "1 of 2 is not a strict majority");
    let one = parse(&[row_value(
        "EV2-0001",
        PLAIN,
        &labels("yes", "yes", "no"),
        None,
    )]);
    assert!(!bars(&one, &run, "K1@v1").c.gateable());
}

/// Provenance: PLAT-1031, MP-243 (review of #624, G1). Exactly one
/// qualifying check, which beats its constant, is not gateable, so C does
/// not pass: a `>= 1` gate, or a `passes` that skips `gateable`, would pass.
#[test]
fn tc_1031_bar_c_with_one_qualifying_check_is_not_gateable() {
    let labels = |vague: &str| {
        json!({"vague_term": truth(vague, "agent_dual", &[]),
               "compound": truth("no", "agent_dual", &[])})
    };
    let rows = parse(&[
        row_value("EV2-0001", PLAIN, &labels("yes"), None),
        row_value("EV2-0002", PLAIN, &labels("no"), None),
    ]);
    let run = output(
        "K1@v1",
        &[
            (
                "EV2-0001",
                &[("vague_term", "yes", 0.9), ("compound", "no", 0.1)],
            ),
            (
                "EV2-0002",
                &[("vague_term", "no", 0.1), ("compound", "no", 0.1)],
            ),
        ],
    );
    let c = bars(&rows, &run, "K1@v1").c;
    assert_eq!(c.qualifying, ["vague_term"]);
    assert_eq!(c.beating, ["vague_term"]);
    assert!(!c.gateable(), "one qualifying check is not gateable");
    assert!(!c.passes(), "an ungateable C never passes");
}

/// Provenance: PLAT-1031, MP-243 (review of #624, G4). Bar C reads natural
/// rows only: a mutant's by-construction `yes` would give `vague_term` both
/// classes, and the variant is right on both rows, yet no check qualifies.
#[test]
fn tc_1031_bar_c_reads_natural_rows_only() {
    let rows = parse(&[
        row_value(
            "EV2-0100",
            PLAIN,
            &json!({"vague_term": truth("no", "agent_dual", &[])}),
            None,
        ),
        row_value(
            "EV2-0200",
            "A large upload is refused promptly.",
            &json!({"vague_term": truth("yes", "by_construction", &[])}),
            Some("EV2-0100"),
        ),
    ]);
    let run = output(
        "K1@v1",
        &[
            ("EV2-0100", &[("vague_term", "no", 0.1)]),
            ("EV2-0200", &[("vague_term", "yes", 0.9)]),
        ],
    );
    let c = bars(&rows, &run, "K1@v1").c;
    assert!(c.qualifying.is_empty(), "{c:?}");
    assert!(c.beating.is_empty(), "{c:?}");
}

/// Bars where A, B and D pass, with `c` as given.
fn bars_with_c(qualifying: &[&'static str], beating: &[&'static str]) -> Bars {
    let a = KeyMargin {
        rows: 2,
        both_classes: true,
        credit: 2.0,
        constant: 1.0,
        constant_label: "yes",
        abstentions: 0,
    };
    Bars {
        a,
        b: (1, 1),
        c: BarC {
            qualifying: qualifying.to_vec(),
            beating: beating.to_vec(),
        },
        d: BarD {
            pairs: 10,
            successes: 10,
            ..BarD::default()
        },
    }
}

/// Provenance: PLAT-1031, MP-243 (review of #624, G3). Selectable needs A,
/// B and D, and C either passing or not gateable: a failing gateable C
/// blocks selection, and an ungateable C does not.
#[test]
fn tc_1031_selectable_reads_the_c_clause() {
    let all = ["vague_term", "compound", "missing_trigger"];
    let passing = bars_with_c(&all, &["vague_term", "compound"]);
    assert!(passing.c.gateable() && passing.c.passes());
    assert!(passing.selectable(), "A, B, C and D all pass");
    let failing = bars_with_c(&all, &["vague_term"]);
    assert!(failing.c.gateable() && !failing.c.passes());
    assert!(
        !failing.selectable(),
        "a gateable C that fails blocks selection"
    );
    let ungateable = bars_with_c(&["vague_term"], &[]);
    assert!(!ungateable.c.gateable());
    assert!(ungateable.selectable(), "an ungateable C makes no claim");
    let mut no_d = bars_with_c(&all, &["vague_term", "compound"]);
    no_d.d.successes = 9;
    assert!(!no_d.selectable(), "D must be gateable");
}

// ---------------------------------------------------------------------------
// Bar D
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 5 and B1). A
/// success crosses τ, the source below and the mutant at or above, with a
/// rise of at least δ; a failure is a fall of at least δ; anything else is a
/// tie.
#[test]
fn tc_1031_a_pair_needs_tau_and_delta() {
    assert_eq!(pair_outcome(0.30, 0.60), PairOutcome::Success);
    assert_eq!(pair_outcome(0.40, 0.50), PairOutcome::Success);
    assert_eq!(pair_outcome(0.20, 0.45), PairOutcome::Tie, "below τ");
    assert_eq!(pair_outcome(0.55, 0.70), PairOutcome::Tie, "no crossing");
    assert_eq!(pair_outcome(0.50, 0.65), PairOutcome::Tie, "source at τ");
    assert_eq!(pair_outcome(0.55, 0.60), PairOutcome::Tie, "rose < δ");
    assert_eq!(pair_outcome(0.60, 0.50), PairOutcome::Failure);
    assert_eq!(pair_outcome(0.60, 0.55), PairOutcome::Tie, "fell < δ");
}

/// Provenance: PLAT-1031, MP-243. The one-sided sign test at α = 0.05: 9 of
/// 10 is `11/1024`, significant; 8 of 10 is `56/1024`, not.
#[test]
fn tc_1031_the_sign_test_is_one_sided_binomial() {
    assert!((sign_test_p(9, 10) - 11.0 / 1024.0).abs() < 1e-12);
    assert!((sign_test_p(8, 10) - 56.0 / 1024.0).abs() < 1e-12);
    assert!((sign_test_p(0, 10) - 1.0).abs() < 1e-12);
    assert!((sign_test_p(10, 10) - 1.0 / 1024.0).abs() < 1e-12);
}

/// `n` natural sources, each with a `vague_term` mutant; the source of
/// pair 0 is already labelled `yes` on `vague_term`.
fn paired_rows(n: usize) -> Vec<Row> {
    let mut values = Vec::new();
    for i in 0..n {
        let source = format!("EV2-{:04}", 100 + i);
        let mutant = format!("EV2-{:04}", 200 + i);
        let source_label = if i == 0 { "yes" } else { "no" };
        values.push(row_value(
            &source,
            PLAIN,
            &json!({"vague_term": truth(source_label, "agent_dual", &[])}),
            None,
        ));
        values.push(row_value(
            &mutant,
            "A large upload is refused promptly.",
            &json!({"vague_term": truth("yes", "by_construction", &[]),
                    "criterion_sound": truth("no", "by_construction", &[])}),
            Some(&source),
        ));
    }
    parse(&values)
}

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 5). Bar D pairs
/// by `mutation.source_id`, drops a pair whose source is already `yes`,
/// and passes on the sign test with at least 10 non-tie pairs.
#[test]
fn tc_1031_bar_d_pairs_by_source_id_and_sign_tests() {
    let rows = paired_rows(11);
    let found = pairs(&rows);
    assert_eq!(found.len(), 11);
    assert!(found.iter().all(|pair| pair.key == "vague_term"
        && pair.mutant.mutation.as_ref().unwrap().source_id.as_deref()
            == Some(pair.source.id.as_str())));
    let d = bars(&rows, &paired_run(), "K1@v1").d;
    assert_eq!(
        (
            d.pairs,
            d.source_already_yes,
            d.successes,
            d.failures,
            d.abstained,
            d.ties
        ),
        (11, 1, 9, 1, 0, 0)
    );
    assert!(d.gateable());
    assert!(d.passes(), "p = {}", d.p_value());
}

/// Provenance: PLAT-1031, MP-243 (the shared paired bar D). A pair the
/// variant did not answer on one row is a failure, not a tie: dropping one
/// successful mutant's answer turns 9-1 (p = 11/1024) into 8-2 (p = 56/1024),
/// which no longer passes.
#[test]
fn tc_1031_an_abstained_pair_is_a_failure() {
    let rows = paired_rows(11);
    let mut run = paired_run();
    run.results.retain(|result| result.row_id != "EV2-0205");
    let d = bars(&rows, &run, "K1@v1").d;
    assert_eq!((d.successes, d.failures, d.abstained, d.ties), (8, 2, 1, 0));
    assert!(!d.passes(), "p = {}", d.p_value());
}

/// K1 answers for [`paired_rows`]`(11)`: pairs 1..=9 succeed (0.1 -> 0.8),
/// pair 10 fails (0.8 -> 0.1); pair 0 is dropped (source already `yes`).
fn paired_run() -> RunOutput {
    let mut answers: Vec<(String, f64)> = Vec::new();
    for i in 0..11 {
        let (source_p, mutant_p) = if i == 10 { (0.8, 0.1) } else { (0.1, 0.8) };
        answers.push((format!("EV2-{:04}", 100 + i), source_p));
        answers.push((format!("EV2-{:04}", 200 + i), mutant_p));
    }
    RunOutput {
        results: answers
            .iter()
            .map(|(row, p)| RowResult {
                answered: Vec::new(),
                row_id: row.clone(),
                variant: "K1@v1".to_owned(),
                predictions: [("vague_term", soundness::defect_prediction(*p))]
                    .into_iter()
                    .collect(),
            })
            .collect(),
        ..RunOutput::default()
    }
}

fn broken_pair(edit: impl Fn(&mut Vec<Row>)) {
    let mut rows = paired_rows(1);
    edit(&mut rows);
    let _ = pairs(&rows);
}

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 5). A missing
/// source panics.
#[test]
#[should_panic(expected = "EV2-0200: source EV2-0100 is not among this run's rows")]
fn tc_1031_a_missing_source_panics() {
    broken_pair(|rows| {
        rows.remove(0);
    });
}

/// Provenance: PLAT-1031, MP-243. A source that is itself a mutant panics.
#[test]
#[should_panic(expected = "EV2-0200: source EV2-0100 is itself a mutant")]
fn tc_1031_a_mutant_source_panics() {
    broken_pair(|rows| rows[0].mutation = rows[1].mutation.clone());
}

/// Provenance: PLAT-1031, MP-243. A source not in mode R panics.
#[test]
#[should_panic(expected = "EV2-0200: source EV2-0100 is mode RT, not R")]
fn tc_1031_a_source_in_another_mode_panics() {
    broken_pair(|rows| rows[0].mode = Mode::ReqTest);
}

/// Provenance: PLAT-1031, MP-243. A source in another split panics.
#[test]
#[should_panic(expected = "EV2-0200: source EV2-0100 is in heldout, the mutant in dev")]
fn tc_1031_a_source_in_another_split_panics() {
    broken_pair(|rows| rows[0].split = corpus::Split::Heldout);
}

/// Provenance: PLAT-1031, MP-243. A checklist mutant without a source id
/// panics.
#[test]
#[should_panic(expected = "EV2-0200: vague_term mutant has no mutation.source_id")]
fn tc_1031_a_mutant_without_a_source_id_panics() {
    broken_pair(|rows| {
        if let Some(mutation) = rows[1].mutation.as_mut() {
            mutation.source_id = None;
        }
    });
}

/// Provenance: PLAT-1031, MP-243 (review of #624, G2). A mutant whose truth
/// marks two checks as injected panics: a mutation injects one defect.
#[test]
#[should_panic(
    expected = "EV2-0200: injects [\"vague_term\", \"compound\"]; a mutation injects one"
)]
fn tc_1031_a_mutant_injecting_two_checks_panics() {
    broken_pair(|rows| {
        let second = rows[1].truth["vague_term"].clone();
        rows[1].truth.insert("compound".to_owned(), second);
    });
}

/// Provenance: PLAT-1031, MP-243 (review of #624, G5). Exactly 9 decided
/// pairs is one short of gateable, so D does not pass even at 9 of 9
/// (p = 1/512).
#[test]
fn tc_1031_nine_decided_pairs_are_not_gateable() {
    let d = BarD {
        pairs: 9,
        successes: 9,
        ..BarD::default()
    };
    assert_eq!(d.decided(), 9);
    assert!(d.p_value() <= 0.05, "p = {}", d.p_value());
    assert!(!d.gateable());
    assert!(!d.passes());
    // paired_run's answers for pair 10 (the failure) have no rows here.
    let d = bars(&paired_rows(10), &paired_run(), "K1@v1").d;
    // Pair 0 is dropped (source already yes), pairs 1..=9 succeed: 9 decided.
    assert_eq!((d.pairs, d.source_already_yes, d.decided()), (10, 1, 9));
    assert!(!d.gateable() && !d.passes());
}

/// Provenance: PLAT-1031, MP-243 (review of #624, G5). Only mode-R mutants
/// are paired: a checklist mutant in another mode is not a pair.
#[test]
fn tc_1031_only_mode_r_mutants_are_paired() {
    let mut rows = paired_rows(3);
    let rt = rows.iter_mut().find(|row| row.id == "EV2-0201").unwrap();
    rt.mode = Mode::ReqTest;
    let found: Vec<&str> = pairs(&rows).iter().map(|p| p.mutant.id.as_str()).collect();
    assert_eq!(found, ["EV2-0200", "EV2-0202"]);
}

/// Provenance: PLAT-1031, MP-243 (review of #624, N1; the shared paired
/// rule). One pair per mutation id: a second mode-R row of the same
/// mutation is not a second pair, and the first in corpus order is kept.
#[test]
fn tc_1031_one_pair_per_mutation_id() {
    let mut rows = paired_rows(3);
    let first = rows[1].mutation.as_ref().unwrap().id.clone();
    let dup = rows.iter_mut().find(|row| row.id == "EV2-0202").unwrap();
    dup.mutation.as_mut().unwrap().id = first;
    let found: Vec<&str> = pairs(&rows).iter().map(|p| p.mutant.id.as_str()).collect();
    assert_eq!(found, ["EV2-0200", "EV2-0201"]);
}

// ---------------------------------------------------------------------------
// C0 comparison
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1031, MP-243 (review of #624, finding 5). The comparison
/// with C0 uses only rows both answered, and reports each one's abstentions
/// against the 5% ceiling.
#[test]
fn tc_1031_the_c0_comparison_uses_jointly_answered_rows() {
    let rows = rows(&[("EV2-0001", PLAIN), ("EV2-0002", PLAIN)]);
    let mut run = output(
        "K1@v1",
        &[
            ("EV2-0001", &[(SOUND, "yes", 0.9)]),
            ("EV2-0002", &[(SOUND, "yes", 0.9)]),
        ],
    );
    run.results.extend(
        output(
            "C0@v1",
            &[("EV2-0001", &[(SOUND, "no", 0.2)]), ("EV2-0002", &[])],
        )
        .results,
    );
    let j = joint(&rows, &run, "K1@v1", "C0@v1", SOUND);
    assert_eq!((j.labelled, j.both, j.abstentions), (2, 1, (0, 1)));
    assert_eq!((j.left.credit, j.right.credit), (1.0, 0.0));
    assert!(!j.interpretable(), "1 of 2 abstained, over 5%");
}

// ---------------------------------------------------------------------------
// End to end over a fake Jev
// ---------------------------------------------------------------------------

/// A fake Jev: answers each `noul` with the probability given for its key,
/// or `default`. Counts requests.
struct KeyedJev {
    by_key: BTreeMap<&'static str, f64>,
    default: f64,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for KeyedJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let answers: serde_json::Map<String, Value> = body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .map(|key| {
                let p = self
                    .by_key
                    .get(key.as_str())
                    .copied()
                    .unwrap_or(self.default);
                (key.clone(), json!({"type": "noul", "noul": p}))
            })
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

/// Provenance: PLAT-1031, MP-243. K1 and K2 run end to end through the
/// harness runner, one request each per row. A fake that clears every
/// check is graded sound on every row, the case PLAT-917's lens never
/// produced; one that flags `compound` makes every row unsound. The bars
/// render for both.
#[tokio::test]
async fn tc_1031_the_variants_run_end_to_end() {
    let rows = rows(&[
        ("EV2-0001", PLAIN),
        ("EV2-0002", "Every audit record has a UTC time."),
    ]);
    for (flag, expected) in [(0.1, "yes"), (0.8, "no")] {
        let config =
            quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]))
                .unwrap();
        let fake = Arc::new(KeyedJev {
            by_key: BTreeMap::from([("compound", flag)]),
            default: 0.1,
            calls: AtomicUsize::new(0),
        });
        let client = quoin_jev::client::with_transport(config, fake.clone());
        let output = variant::run(&client, &rows, &[&K1, &K2]).await.unwrap();
        assert_eq!(fake.calls.load(Ordering::SeqCst), 4);
        for label in ["K1@v2", "K2@v1"] {
            let sound: Vec<String> = scored(&rows, &output, label, SOUND)
                .iter()
                .map(|row| row.prediction.as_ref().unwrap().answer.clone())
                .collect();
            assert_eq!(sound, [expected, expected], "{label}");
        }
        let report = render_bars(&rows, &output, &[&K1, &K2]);
        assert!(report.contains("## MP-243 bars: K1@v2 (2 natural R rows; one row = 50.0 pp)"));
        assert!(
            report.contains("- D [by-construction pairs]: 0 pairs"),
            "{report}"
        );
        assert!(report.contains("not selectable"), "{report}");
    }
}

/// Provenance: PLAT-1024 exp2. The aggregate bar D reads the combiner's own
/// score: a source already flagged by another check cannot cross (a tie), a
/// source labelled `yes` on the injected check is dropped, a count rule
/// needs a second check to fire, and a stricter threshold is read by the
/// shifted crossing.
#[test]
fn tc_1024_exp2_k_aggregate_bar_d_reads_the_combiner_score() {
    use eval_v2_support::exp2::{KRow, KRule, k_aggregate_d};
    let rows = paired_rows(3);
    // CHECKS order: vague_term, no_measurable_threshold, untestable,
    // compound, missing_trigger. Mutants inject vague_term.
    let p: [(&str, [f64; 5]); 6] = [
        ("EV2-0100", [0.1, 0.1, 0.1, 0.1, 0.1]),
        ("EV2-0200", [0.8, 0.1, 0.1, 0.1, 0.1]),
        ("EV2-0101", [0.1, 0.1, 0.1, 0.1, 0.1]),
        ("EV2-0201", [0.8, 0.1, 0.1, 0.1, 0.1]),
        ("EV2-0102", [0.1, 0.1, 0.1, 0.7, 0.1]),
        ("EV2-0202", [0.8, 0.1, 0.1, 0.7, 0.1]),
    ];
    let krows: Vec<KRow<'_>> = p
        .iter()
        .map(|(id, p)| KRow {
            row: rows.iter().find(|row| row.id == *id).unwrap(),
            p: p.to_vec(),
        })
        .collect();
    let by_id: BTreeMap<&str, &KRow<'_>> = krows.iter().map(|k| (k.row.id.as_str(), k)).collect();
    let rule = |at_least, tau| KRule {
        name: String::new(),
        dropped: None,
        at_least,
        tau,
    };
    let any = k_aggregate_d(&rows, &by_id, &rule(1, 0.5));
    assert_eq!(
        (
            any.pairs,
            any.source_already_yes,
            any.successes,
            any.failures,
            any.ties
        ),
        (3, 1, 1, 0, 1)
    );
    // >= 2 checks: EV2-0201's second-highest stays 0.1 (tie); EV2-0202's
    // rises from 0.1 to 0.7 and crosses (success).
    let two = k_aggregate_d(&rows, &by_id, &rule(2, 0.5));
    assert_eq!((two.successes, two.failures, two.ties), (1, 0, 1));
    // At 0.75, EV2-0102's 0.7 is below the threshold, so both pairs cross.
    let strict = k_aggregate_d(&rows, &by_id, &rule(1, 0.75));
    assert_eq!((strict.successes, strict.failures, strict.ties), (2, 0, 0));
}
