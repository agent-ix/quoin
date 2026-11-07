// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Unit tests for `corpus_check`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
use serde_json::json;

use serde_json::Value;

use super::{check_answerability, check_required_fields, check_stated_counts};

/// A `choice` answer space shaped like `question-set.json`'s
/// `weakness_kind`, as JSON values.
fn kinds() -> Vec<Value> {
    ["sound", "unfalsifiable", "happy_path_only"]
        .into_iter()
        .map(Value::from)
        .collect()
}

/// Provenance: PLAT-982. A label outside the closed set is flagged by
/// fixture id and names the offending value; in-set labels are not.
#[test]
fn a_label_outside_the_answer_space_is_flagged_by_id() {
    let fixtures = json!([
        { "fixture_id": "A", "labels": { "weakness_kind": "sound" } },
        { "fixture_id": "B", "labels": { "weakness_kind": "vague" } },
    ]);
    let findings = check_answerability(&fixtures, &["labels", "weakness_kind"], &kinds());
    assert_eq!(findings.len(), 1, "got: {findings:?}");
    assert!(
        findings[0]
            .0
            .starts_with("B: label `labels.weakness_kind` = \"vague\"")
    );
}

/// Provenance: PLAT-982. Membership is exact JSON equality: a case
/// variant is not the member, and a string `"2"` or float `2.0` is not
/// score level `2`.
#[test]
fn membership_is_exact_not_case_or_type_folded() {
    let kind_fixtures = json!({ "A": { "labels": { "weakness_kind": "Sound" } } });
    let kind = check_answerability(&kind_fixtures, &["labels", "weakness_kind"], &kinds());
    assert_eq!(kind.len(), 1, "got: {kind:?}");

    let levels: Vec<Value> = (0..4).map(Value::from).collect();
    let score_fixtures = json!({
        "B": { "labels": { "adverse_case_coverage": "2" } },
        "C": { "labels": { "adverse_case_coverage": 2.0 } },
        "D": { "labels": { "adverse_case_coverage": 2 } },
    });
    let score = check_answerability(
        &score_fixtures,
        &["labels", "adverse_case_coverage"],
        &levels,
    );
    let ids: Vec<&str> = score
        .iter()
        .map(|f| f.0.split(':').next().unwrap())
        .collect();
    assert_eq!(ids, vec!["B", "C"], "got: {score:?}");
}

/// Provenance: PLAT-982. A contested second reading is a claimed answer
/// too, so each of its members must be in the answer space.
#[test]
fn a_contested_reading_outside_the_answer_space_is_flagged() {
    let fixtures = json!([
        {
            "fixture_id": "A",
            "labels": {
                "weakness_kind": "sound",
                "weakness_kind_contested": ["sound", "vague"],
            },
        },
        {
            "fixture_id": "B",
            "labels": {
                "weakness_kind": "sound",
                "weakness_kind_contested": ["sound", "unfalsifiable"],
            },
        },
    ]);
    let findings = check_answerability(&fixtures, &["labels", "weakness_kind"], &kinds());
    assert_eq!(findings.len(), 1, "got: {findings:?}");
    assert!(findings[0].0.starts_with("A: contested reading \"vague\""));
}

/// Provenance: PLAT-982. A `_contested` value that is not an array
/// cannot be checked member by member, so it is itself a finding.
#[test]
fn a_non_array_contested_reading_is_flagged() {
    let fixtures = json!([
        {
            "fixture_id": "A",
            "labels": { "weakness_kind": "sound", "weakness_kind_contested": "sound" },
        },
    ]);
    let findings = check_answerability(&fixtures, &["labels", "weakness_kind"], &kinds());
    assert_eq!(findings.len(), 1, "got: {findings:?}");
    assert!(findings[0].0.contains("not an array of readings"));
}

/// Provenance: PLAT-982. A fixture with no label -- absent, `null`, or
/// with the parent object missing -- answers nothing and is flagged.
#[test]
fn a_missing_or_null_label_is_flagged() {
    let fixtures = json!([
        { "fixture_id": "A", "labels": {} },
        { "fixture_id": "B", "labels": { "weakness_kind": null } },
        { "fixture_id": "C" },
        { "fixture_id": "D", "labels": { "weakness_kind": "sound" } },
    ]);
    let findings = check_answerability(&fixtures, &["labels", "weakness_kind"], &kinds());
    let ids: Vec<&str> = findings
        .iter()
        .map(|f| f.0.split(':').next().unwrap())
        .collect();
    assert_eq!(ids, vec!["A", "B", "C"], "got: {findings:?}");
}

/// Provenance: PLAT-982. Degenerate inputs are findings, not vacuous
/// passes: an empty answer space, an empty label path, an empty
/// container.
#[test]
fn degenerate_inputs_are_flagged_rather_than_passed() {
    let fixtures = json!([{ "fixture_id": "A", "labels": { "weakness_kind": "sound" } }]);
    assert_eq!(
        check_answerability(&fixtures, &["labels", "weakness_kind"], &[]).len(),
        1
    );
    assert_eq!(check_answerability(&fixtures, &[], &kinds()).len(), 1);
    assert_eq!(
        check_answerability(&json!([]), &["labels", "weakness_kind"], &kinds()).len(),
        1
    );
}

/// Provenance: PLAT-933. Reproduces the real incident's exact phrasing
/// ("disputed 5 of 14 (agreed 9)") over data shaped like the real
/// corpus (9 of 15 actually contested, agreed 6) and asserts all three
/// stale numbers are flagged -- including "agreed 9", which review
/// found the first version of this check silently passed because 9
/// happens to equal the corpus's actual *contested* count, not because
/// 9 is a correct agreed count (it is not: actual agreed is 6).
#[test]
fn a_stale_prose_count_is_flagged_against_the_real_incidents_own_numbers() {
    let mut weakness = Vec::new();
    for i in 0..11 {
        let mut fixture = json!({ "fixture_id": format!("CS-FIX-{i:03}") });
        if i < 7 {
            fixture["weakness_kind_contested"] = json!(["sound", "unfalsifiable"]);
        }
        weakness.push(fixture);
    }
    let mut coverage = Vec::new();
    for i in 0..4 {
        let mut fixture = json!({ "fixture_id": format!("CS-FIX-{:03}", i + 11) });
        if i < 2 {
            fixture["adverse_case_coverage_contested"] = json!([1, 2]);
        }
        coverage.push(fixture);
    }
    let corpus = json!({
        "governing_ruling_on_disagreement":
            "A second, independent reader re-derived all labels in this corpus and \
             disputed 5 of 14 (agreed 9).",
        "weakness_kind_fixtures": weakness,
        "adverse_case_coverage_fixtures": coverage,
    });

    let findings = check_stated_counts(&corpus);
    let messages: Vec<String> = findings.iter().map(ToString::to_string).collect();
    assert_eq!(
        messages.len(),
        3,
        "expected all three stale numbers (5, 14, 9) flagged, got: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("claims contested 5")),
        "expected the stale contested count (5) to be flagged, got: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("claims total 14")),
        "expected the stale total (14) to be flagged, got: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("claims agreed 9")),
        "expected the stale agreed count (9) to be flagged even though 9 equals the \
         actual *contested* count, got: {messages:?}"
    );
}

/// Provenance: PLAT-933. When the prose already agrees with the data,
/// there is nothing to flag.
#[test]
fn a_correct_stated_count_is_clean() {
    let corpus = json!({
        "governing_ruling_on_disagreement": "disputed 2 of 3 (agreed 1)",
        "weakness_kind_fixtures": [
            { "fixture_id": "A", "weakness_kind_contested": ["x", "y"] },
            { "fixture_id": "B", "weakness_kind_contested": ["x", "y"] },
            { "fixture_id": "C" },
        ],
    });
    assert_eq!(check_stated_counts(&corpus), Vec::new());
}

/// Provenance: PLAT-933 review. The exact regression the pooled-set
/// design missed: total 3, contested 2, agreed 1, but the prose claims
/// "agreed 2" -- 2 is a real count in this corpus (it's `contested`),
/// so a pooled "matches any actual count" check passed it. Checked
/// against `agreed` specifically, it must fail.
#[test]
fn an_agreed_claim_matching_a_different_fields_actual_count_is_still_flagged() {
    let corpus = json!({
        "governing_ruling_on_disagreement": "disputed 2 of 3 (agreed 2)",
        "weakness_kind_fixtures": [
            { "fixture_id": "A", "weakness_kind_contested": ["x", "y"] },
            { "fixture_id": "B", "weakness_kind_contested": ["x", "y"] },
            { "fixture_id": "C" },
        ],
    });
    let findings = check_stated_counts(&corpus);
    assert_eq!(findings.len(), 1, "got: {findings:?}");
    assert!(findings[0].0.contains("claims agreed 2"));
}

/// Provenance: PLAT-933. A corpus with no recognizable fixture array is
/// itself a finding rather than a silent no-op -- a typo in a corpus's
/// own key should not pass this check by accident.
#[test]
fn a_corpus_with_no_fixture_array_is_flagged_rather_than_skipped() {
    let corpus = json!({ "$comment": "nothing here" });
    assert_eq!(check_stated_counts(&corpus).len(), 1);
}

/// Provenance: PLAT-933 review. An empty `_contested` array is a key
/// that exists but records no second reading -- it must not count as
/// contested, or a fixture that merely carries the key (perhaps written
/// defensively, or left over from a prior edit) would inflate the
/// actual contested count against real, non-empty ones.
#[test]
fn an_empty_contested_array_does_not_count_as_contested() {
    let corpus = json!({
        "governing_ruling_on_disagreement": "disputed 0 of 2 (agreed 2)",
        "weakness_kind_fixtures": [
            { "fixture_id": "A", "weakness_kind_contested": [] },
            { "fixture_id": "B" },
        ],
    });
    assert_eq!(check_stated_counts(&corpus), Vec::new());
}

/// Provenance: PLAT-933. Reproduces the real incident: a required field
/// (`statement`) present as an empty string on most fixtures, over a
/// map-shaped container matching `criterion-strength-fr-context.json`'s
/// own `fixtures` field.
#[test]
fn missing_or_empty_required_fields_are_flagged_by_id() {
    let contexts = json!({
        "CS-FIX-001": { "statement": "The system SHALL do a thing." },
        "CS-FIX-002": { "statement": "" },
        "CS-FIX-003": { "statement": "   " },
        "CS-FIX-004": {},
    });

    let findings = check_required_fields(&contexts, &["statement"]);
    let ids: Vec<&str> = findings
        .iter()
        .map(|f| f.0.split(':').next().unwrap())
        .collect();
    assert_eq!(
        ids.len(),
        3,
        "expected 3 of 4 fixtures flagged, got {findings:?}"
    );
    assert!(ids.contains(&"CS-FIX-002"));
    assert!(ids.contains(&"CS-FIX-003"));
    assert!(ids.contains(&"CS-FIX-004"));
    assert!(!ids.contains(&"CS-FIX-001"));
}

/// Provenance: PLAT-933 review. An empty array or object is as absent
/// as an empty string -- `acceptance_criteria: []` is not a populated
/// required field any more than `statement: ""` is.
#[test]
fn empty_array_and_object_required_fields_are_flagged_too() {
    let fixtures = json!({
        "A": { "acceptance_criteria": [] },
        "B": { "acceptance_criteria": {} },
        "C": { "acceptance_criteria": ["FR-001-AC-1"] },
    });
    let findings = check_required_fields(&fixtures, &["acceptance_criteria"]);
    let ids: Vec<&str> = findings
        .iter()
        .map(|f| f.0.split(':').next().unwrap())
        .collect();
    assert_eq!(ids.len(), 2, "got: {findings:?}");
    assert!(ids.contains(&"A"));
    assert!(ids.contains(&"B"));
}

/// Provenance: PLAT-933. The array shape (`fixture_id` per entry) is
/// checked the same way as the map shape.
#[test]
fn required_fields_are_checked_over_the_array_shape_too() {
    let fixtures = json!([
        { "fixture_id": "A", "criterion_text": "has text" },
        { "fixture_id": "B" },
    ]);
    let findings = check_required_fields(&fixtures, &["criterion_text"]);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].0.starts_with("B:"));
}

/// Provenance: PLAT-933. An empty fixtures container is a finding, not a
/// vacuous pass.
#[test]
fn an_empty_fixtures_container_is_flagged() {
    assert_eq!(check_required_fields(&json!([]), &["statement"]).len(), 1);
    assert_eq!(check_required_fields(&json!({}), &["statement"]).len(), 1);
}
