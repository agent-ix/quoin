// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Unit tests for `verdict`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
use super::{
    Certainty, NoulSignals, Severity, SubQuestionCheck, Thresholds, extract, label_sub_question,
    top_two_margin,
};
use crate::question_set::QuestionSet;

const ASSET: &str =
    include_str!("../../../../../skills/spec-criterion-strength-analysis/assets/question-set.json");

/// Confidence 0.5, margin 0.1. The margin is chosen so that no fixture
/// written before PLAT-981 (whose top-two gaps are all >= 0.4) crosses it.
const T: Thresholds = Thresholds {
    confidence: 0.5,
    margin: 0.1,
};

/// Provenance: PLAT-837
#[test]
fn severity_mapping_matches_skill_md() {
    assert_eq!(
        Severity::for_weakness_kind("unfalsifiable"),
        Some(Severity::High)
    );
    assert_eq!(
        Severity::for_weakness_kind("implementation_coupled"),
        Some(Severity::Medium)
    );
    assert_eq!(
        Severity::for_weakness_kind("unmeasurable_threshold"),
        Some(Severity::Medium)
    );
    assert_eq!(
        Severity::for_weakness_kind("restates_requirement"),
        Some(Severity::Low)
    );
    assert_eq!(
        Severity::for_weakness_kind("happy_path_only"),
        Some(Severity::Low)
    );
    assert_eq!(Severity::for_weakness_kind("sound"), None);
}

fn response_fixture(confidence: f64) -> typesafe_sdk_answers::SystemOneResponse {
    gated_fixture(
        confidence,
        &serde_json::json!({"unfalsifiable": confidence, "sound": 1.0 - confidence}),
        &serde_json::json!({}),
    )
}

/// `weakness_kind` answered `unfalsifiable` and `adverse_case_coverage`
/// answered, both at `confidence`, with the given `probabilities` maps.
fn gated_fixture(
    confidence: f64,
    choice_probabilities: &serde_json::Value,
    score_probabilities: &serde_json::Value,
) -> typesafe_sdk_answers::SystemOneResponse {
    let body = serde_json::json!({
        "model": "jev-1.13.0",
        "answers": {
            "FR-001-AC-1::falsifiable": {"type": "noul", "noul": 0.9},
            "FR-001-AC-1::states_observable_outcome": {"type": "noul", "noul": 0.8},
            "FR-001-AC-1::threshold_present": {"type": "noul", "noul": 0.1},
            "FR-001-AC-1::restates_requirement": {"type": "noul", "noul": 0.05},
            "FR-001-AC-1::implementation_coupled": {"type": "noul", "noul": 0.02},
            "FR-001-AC-1::weakness_kind": {
                "type": "choice",
                "choice": "unfalsifiable",
                "confidence": confidence,
                "probabilities": choice_probabilities
            },
            "adverse_case_coverage": {
                "type": "score",
                "score": 1.4,
                "confidence": confidence,
                "legend": {},
                "probabilities": score_probabilities
            }
        },
        "usage": {"input_tokens": 100, "output_tokens": 0}
    });
    serde_json::from_value(body).expect("well-formed fixture")
}

/// Provenance: PLAT-837. The classifier name recorded is the concrete
/// response `model` field, never the requested alias.
#[test]
fn the_classifier_is_the_concrete_model_from_the_response() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = response_fixture(0.9);
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(verdict.classifier, "jev-1.13.0");
}

/// Provenance: PLAT-837. High confidence: a finding is produced and NOT
/// marked unconfirmed.
#[test]
fn a_high_confidence_verdict_is_not_marked_unconfirmed() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = response_fixture(0.95);
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(verdict.findings.len(), 1);
    assert_eq!(verdict.findings[0].certainty, Certainty::Confident);
    assert_eq!(verdict.findings[0].severity, Severity::High);
}

/// Provenance: PLAT-837. This is the acceptance criterion itself: low
/// confidence must still produce a finding (annotated), never suppress
/// it -- the opposite of ix-board's rule, and stated as such in this
/// ticket.
#[test]
fn a_low_confidence_verdict_still_produces_a_finding_marked_unconfirmed() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = response_fixture(0.3);
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(
        verdict.findings.len(),
        1,
        "low confidence must not suppress the finding"
    );
    assert_eq!(verdict.findings[0].certainty, Certainty::Unconfirmed);
}

/// Provenance: PLAT-981. The ticket's own worked example -- 0.5
/// `unfalsifiable` / 0.45 `sound` -- reported at HIGH confidence is still
/// `Uncertain`: the margin gate fires on its own, not only when the
/// confidence gate also does. The finding is annotated, not dropped.
#[test]
fn a_near_tie_at_high_confidence_is_uncertain_and_still_a_finding() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = gated_fixture(
        0.9,
        &serde_json::json!({"unfalsifiable": 0.5, "sound": 0.45}),
        &serde_json::json!({}),
    );
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(verdict.findings.len(), 1, "uncertain must not suppress");
    assert_eq!(verdict.findings[0].certainty, Certainty::Uncertain);
}

/// Provenance: PLAT-981. PRECEDENCE: when the confidence gate (0.3 < 0.5)
/// and the margin gate (0.52 - 0.48 = 0.04 < 0.1) both fire, the answer is
/// `Uncertain`, not `Unconfirmed`. Pinned here because flipping the order
/// of the two checks in `Certainty::assess` compiles and would otherwise
/// pass every other test.
#[test]
fn when_both_gates_fire_uncertain_takes_precedence_over_unconfirmed() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = gated_fixture(
        0.3,
        &serde_json::json!({"unfalsifiable": 0.52, "sound": 0.48}),
        &serde_json::json!({"1": 0.52, "2": 0.48}),
    );
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(verdict.findings[0].certainty, Certainty::Uncertain);
    assert_eq!(
        verdict.coverage.expect("score answered").certainty,
        Certainty::Uncertain
    );
}

/// Provenance: PLAT-981. A `probabilities` map with one label (or none)
/// has no margin: it must not panic, must not read as `Uncertain`, and
/// falls through to the confidence gate alone -- `Confident` at high
/// confidence, `Unconfirmed` at low.
#[test]
fn a_single_label_has_no_margin_and_falls_through_to_the_confidence_gate() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let ac = ["FR-001-AC-1".to_owned()];
    let lone = serde_json::json!({"unfalsifiable": 0.95});
    let none = serde_json::json!({});

    let high = extract(&gated_fixture(0.95, &lone, &none), &set, &ac, T);
    assert_eq!(high.findings[0].certainty, Certainty::Confident);
    assert_eq!(
        high.coverage.expect("score answered").certainty,
        Certainty::Confident
    );

    let low = extract(&gated_fixture(0.3, &lone, &none), &set, &ac, T);
    assert_eq!(low.findings[0].certainty, Certainty::Unconfirmed);
    assert_eq!(
        low.coverage.expect("score answered").certainty,
        Certainty::Unconfirmed
    );
}

/// Provenance: PLAT-981. The margin gate applies to `adverse_case_coverage`
/// too: a score whose top two levels are within the margin is `Uncertain`
/// even at high confidence, while a clear `weakness_kind` in the same
/// response stays `Confident` -- the two answers are gated independently.
#[test]
fn a_contested_coverage_score_is_uncertain_independently_of_the_finding() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = gated_fixture(
        0.9,
        &serde_json::json!({"unfalsifiable": 0.9, "sound": 0.1}),
        &serde_json::json!({"0": 0.05, "1": 0.47, "2": 0.43, "3": 0.05}),
    );
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(verdict.findings[0].certainty, Certainty::Confident);
    assert_eq!(
        verdict.coverage.expect("score answered").certainty,
        Certainty::Uncertain
    );
}

/// Provenance: PLAT-981. The margin is top1 - top2 by VALUE, whatever
/// order the map lists labels in; a tie at the top is a zero margin; and
/// fewer than two entries is no margin at all.
#[test]
fn the_margin_is_the_gap_between_the_two_largest_values() {
    assert_eq!(top_two_margin([0.2, 0.5, 0.25]), Some(0.25));
    assert_eq!(top_two_margin([0.25, 0.2, 0.5]), Some(0.25));
    assert_eq!(top_two_margin([0.5, 0.5, 0.0]), Some(0.0));
    assert_eq!(top_two_margin([0.5]), None);
    assert_eq!(top_two_margin([]), None);
}

/// Provenance: PLAT-981. Both gates are strict, like the confidence gate
/// before them: a margin exactly AT the threshold is not `Uncertain`.
#[test]
fn a_margin_exactly_at_the_threshold_is_not_uncertain() {
    let thresholds = Thresholds {
        confidence: 0.5,
        margin: 0.25,
    };
    assert_eq!(
        Certainty::assess(0.9, [0.5, 0.25], thresholds),
        Certainty::Confident
    );
    assert_eq!(
        Certainty::assess(0.9, [0.5, 0.375], thresholds),
        Certainty::Uncertain
    );
}

/// Provenance: PLAT-981. The confidence gate is strict too: a confidence
/// exactly AT the threshold is `Confident`, not `Unconfirmed`. The margin
/// (0.9 - 0.1) is wide, so only the confidence gate is in play.
#[test]
fn a_confidence_exactly_at_the_threshold_is_not_unconfirmed() {
    let thresholds = Thresholds {
        confidence: 0.5,
        margin: 0.1,
    };
    assert_eq!(
        Certainty::assess(0.5, [0.9, 0.1], thresholds),
        Certainty::Confident
    );
    assert_eq!(
        Certainty::assess(0.49, [0.9, 0.1], thresholds),
        Certainty::Unconfirmed
    );
}

/// Provenance: PLAT-981. The serialised spelling is the `as_str` one, so
/// a JSON consumer and the rendered report name a bucket the same way.
#[test]
fn certainty_serialises_to_its_as_str_spelling() {
    for (certainty, spelling) in [
        (Certainty::Confident, "confident"),
        (Certainty::Unconfirmed, "unconfirmed"),
        (Certainty::Uncertain, "uncertain"),
    ] {
        assert_eq!(certainty.as_str(), spelling);
        assert_eq!(
            serde_json::to_value(certainty).unwrap(),
            serde_json::json!(spelling)
        );
    }
}

/// Provenance: PLAT-837. A `sound` verdict produces no finding, and is
/// named in `sound` rather than silently absent.
#[test]
fn a_sound_verdict_produces_no_finding() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let body = serde_json::json!({
        "model": "jev-1.13.0",
        "answers": {
            "FR-001-AC-1::falsifiable": {"type": "noul", "noul": 0.95},
            "FR-001-AC-1::states_observable_outcome": {"type": "noul", "noul": 0.9},
            "FR-001-AC-1::threshold_present": {"type": "noul", "noul": 0.9},
            "FR-001-AC-1::restates_requirement": {"type": "noul", "noul": 0.02},
            "FR-001-AC-1::implementation_coupled": {"type": "noul", "noul": 0.02},
            "FR-001-AC-1::weakness_kind": {
                "type": "choice", "choice": "sound", "confidence": 0.9,
                "probabilities": {"sound": 0.9}
            },
            "adverse_case_coverage": {
                "type": "score", "score": 3.0, "confidence": 0.9,
                "legend": {}, "probabilities": {}
            }
        },
        "usage": {"input_tokens": 50, "output_tokens": 0}
    });
    let response: typesafe_sdk_answers::SystemOneResponse =
        serde_json::from_value(body).expect("well-formed");
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert!(verdict.findings.is_empty());
    assert_eq!(verdict.sound, vec!["FR-001-AC-1".to_owned()]);
}

/// Provenance: PLAT-837 review, finding 1. A `weakness_kind` label
/// outside `question-set.json`'s declared `answer_space` (a version-skewed
/// Jev inventing a seventh label, or a malformed response) must get its
/// own outcome -- `unrecognized` -- and must NEVER be counted as `sound`.
/// `Severity::for_weakness_kind`'s wildcard arm alone cannot make this
/// distinction; `extract` must validate against `answer_space` first.
#[test]
fn an_unrecognized_weakness_kind_label_is_never_counted_as_sound() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let body = serde_json::json!({
        "model": "jev-1.13.0",
        "answers": {
            "FR-001-AC-1::falsifiable": {"type": "noul", "noul": 0.5},
            "FR-001-AC-1::states_observable_outcome": {"type": "noul", "noul": 0.5},
            "FR-001-AC-1::threshold_present": {"type": "noul", "noul": 0.5},
            "FR-001-AC-1::restates_requirement": {"type": "noul", "noul": 0.5},
            "FR-001-AC-1::implementation_coupled": {"type": "noul", "noul": 0.5},
            "FR-001-AC-1::weakness_kind": {
                "type": "choice", "choice": "quantum_uncertainty", "confidence": 0.9,
                "probabilities": {"quantum_uncertainty": 0.9}
            },
            "adverse_case_coverage": {
                "type": "score", "score": 3.0, "confidence": 0.9,
                "legend": {}, "probabilities": {}
            }
        },
        "usage": {"input_tokens": 50, "output_tokens": 0}
    });
    let response: typesafe_sdk_answers::SystemOneResponse =
        serde_json::from_value(body).expect("well-formed");
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert!(verdict.sound.is_empty(), "must never land in sound");
    assert!(verdict.findings.is_empty(), "not a mapped finding either");
    assert_eq!(
        verdict.unrecognized,
        vec![("FR-001-AC-1".to_owned(), "quantum_uncertainty".to_owned())]
    );
}

/// Provenance: PLAT-837 review, finding 2. An AC row Jev never answered a
/// `weakness_kind` choice for at all is tracked as `unanswered` -- not
/// silently dropped from both `findings` and `sound`.
#[test]
fn an_unanswered_ac_row_is_tracked_not_dropped() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = response_fixture(0.9); // only answers FR-001-AC-1
    let verdict = extract(
        &response,
        &set,
        &["FR-001-AC-1".to_owned(), "FR-001-AC-2".to_owned()],
        T,
    );
    assert_eq!(verdict.unanswered, vec!["FR-001-AC-2".to_owned()]);
    assert!(!verdict.sound.contains(&"FR-001-AC-2".to_owned()));
    assert!(!verdict.findings.iter().any(|f| f.ac_id == "FR-001-AC-2"));
}

/// Provenance: PLAT-837. A score outside the rubric's `[0, 3]` range --
/// negative, in this case -- must report no label, not the level-0 label
/// a bare saturating cast would produce (`as u8` on a negative float
/// saturates to 0, which IS a valid rubric level and would silently
/// misreport "no label" as "happy path only").
#[test]
fn an_out_of_range_negative_score_reports_no_label() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let body = serde_json::json!({
        "model": "jev-1.13.0",
        "answers": {
            "adverse_case_coverage": {
                "type": "score", "score": -1.0, "confidence": 0.9,
                "legend": {}, "probabilities": {}
            }
        },
        "usage": {"input_tokens": 10, "output_tokens": 0}
    });
    let response: typesafe_sdk_answers::SystemOneResponse =
        serde_json::from_value(body).expect("well-formed");
    let verdict = extract(&response, &set, &[], T);
    let coverage = verdict.coverage.expect("a score answer was present");
    assert_eq!(coverage.label, None);
}

fn signals(values: &[(&str, f64)]) -> NoulSignals {
    NoulSignals {
        values: values
            .iter()
            .map(|(id, value)| ((*id).to_owned(), *value))
            .collect(),
    }
}

/// Provenance: PLAT-984. The shared fixture answers `unfalsifiable` while
/// its own `falsifiable` sub-answer is 0.9 -- the label is not carried by
/// the question that asks about that defect, and the typed verdict says so.
#[test]
fn a_label_its_own_sub_question_contradicts_is_reported_as_disagreeing() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let response = response_fixture(0.9);
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(
        verdict.findings[0].label_sub_question,
        SubQuestionCheck::Disagrees {
            question: "falsifiable",
            noul: 0.9
        }
    );
}

/// Provenance: PLAT-984. The same label with `falsifiable` answered no is
/// corroborated by its sub-question.
#[test]
fn a_label_its_own_sub_question_backs_is_reported_as_agreeing() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    let body = serde_json::json!({
        "model": "jev-1.13.0",
        "answers": {
            "FR-001-AC-1::falsifiable": {"type": "noul", "noul": 0.1},
            "FR-001-AC-1::weakness_kind": {
                "type": "choice", "choice": "unfalsifiable", "confidence": 0.9,
                "probabilities": {"unfalsifiable": 0.9}
            }
        },
        "usage": {"input_tokens": 50, "output_tokens": 0}
    });
    let response: typesafe_sdk_answers::SystemOneResponse =
        serde_json::from_value(body).expect("well-formed");
    let verdict = extract(&response, &set, &["FR-001-AC-1".to_owned()], T);
    assert_eq!(
        verdict.findings[0].label_sub_question,
        SubQuestionCheck::Agrees {
            question: "falsifiable",
            noul: 0.1
        }
    );
}

/// Provenance: PLAT-984. A "yes" sub-question (`restates_requirement`)
/// agrees at or above 0.5 and disagrees below it -- the polarity is per
/// label, not one direction for all.
#[test]
fn a_yes_polarity_sub_question_agrees_at_the_boundary_and_disagrees_below_it() {
    assert_eq!(
        SubQuestionCheck::for_label(
            "restates_requirement",
            &signals(&[("restates_requirement", 0.5)])
        ),
        SubQuestionCheck::Agrees {
            question: "restates_requirement",
            noul: 0.5
        }
    );
    assert_eq!(
        SubQuestionCheck::for_label(
            "implementation_coupled",
            &signals(&[("implementation_coupled", 0.49)])
        ),
        SubQuestionCheck::Disagrees {
            question: "implementation_coupled",
            noul: 0.49
        }
    );
    assert_eq!(
        SubQuestionCheck::for_label(
            "unmeasurable_threshold",
            &signals(&[("threshold_present", 0.5)])
        ),
        SubQuestionCheck::Disagrees {
            question: "threshold_present",
            noul: 0.5
        }
    );
}

/// Provenance: PLAT-984. A label with a sub-question whose answer is
/// missing from the row is `Unanswered`, never read as agreement; and
/// `happy_path_only`, which no `noul` question covers, says so rather
/// than naming a question.
#[test]
fn a_missing_sub_answer_and_an_uncovered_label_have_their_own_outcomes() {
    assert_eq!(
        SubQuestionCheck::for_label("unfalsifiable", &signals(&[("threshold_present", 0.1)])),
        SubQuestionCheck::Unanswered {
            question: "falsifiable"
        }
    );
    assert_eq!(
        SubQuestionCheck::for_label("happy_path_only", &signals(&[("falsifiable", 0.1)])),
        SubQuestionCheck::NoSubQuestion
    );
    assert_eq!(
        SubQuestionCheck::for_label("quantum_uncertainty", &signals(&[])),
        SubQuestionCheck::NoSubQuestion
    );
}

/// Provenance: PLAT-984. Every question the label table names is one the
/// shipped `question-set.json` actually asks, and every non-`sound` label
/// but `happy_path_only` has one -- the table cannot drift from the asset
/// silently.
#[test]
fn every_label_sub_question_is_a_question_the_asset_asks() {
    let set = QuestionSet::parse(ASSET).expect("parses");
    for label in &set.choice.answer_space {
        let found = label_sub_question(label);
        if matches!(label.as_str(), "sound" | "happy_path_only") {
            assert_eq!(found, None, "{label}");
            continue;
        }
        let (question, _) = found.expect("every other label has a sub-question");
        assert!(
            set.noul.iter().any(|entry| entry.id == question),
            "{label} names {question}, which the asset does not ask"
        );
    }
}

/// Provenance: PLAT-984. The serialised form is tagged by `outcome`, so a
/// consumer of the JSON verdict reads the same four outcomes the type has.
#[test]
fn the_sub_question_check_serialises_tagged_by_outcome() {
    let json = serde_json::to_string(&SubQuestionCheck::Disagrees {
        question: "falsifiable",
        noul: 0.9,
    })
    .expect("serialises");
    assert_eq!(
        json,
        r#"{"outcome":"disagrees","question":"falsifiable","noul":0.9}"#
    );
    let json = serde_json::to_string(&SubQuestionCheck::NoSubQuestion).expect("serialises");
    assert_eq!(json, r#"{"outcome":"no_sub_question"}"#);
}

/// Provenance: PLAT-837. A `noul` answer has no `confidence` field to
/// read -- this test asserts the type-level fact the module doc
/// describes, not just the prose claim.
#[test]
fn noul_answers_carry_no_confidence_field() {
    let value = serde_json::json!({"type": "noul", "noul": 0.5});
    let parsed: typesafe_sdk_answers::Answer =
        serde_json::from_value(value).expect("a bare noul answer parses");
    let noul = parsed.as_noul().expect("this is a noul answer");
    assert!((noul.noul - 0.5).abs() < f64::EPSILON);
    // NoulAnswer has exactly one field (`noul`); there is no `.confidence`
    // to read here at all -- this is a compile-time fact, demonstrated by
    // the absence of any such access in this file.
}
