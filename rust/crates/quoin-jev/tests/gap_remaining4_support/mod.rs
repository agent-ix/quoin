// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Agent-labelled ground truth for the four gap-analysis battery questions
//! PLAT-839 never graded on the shipped corpus, and the glue that turns a
//! live response into rows the shared grading maths reads (PLAT-1014).
//!
//! The four are `code_implements_intent`, `code_exceeds_requirement`,
//! `divergence_kind` and `severity`. MP-233 graded the first three against
//! targeted MUTANTS (and `severity` as mutant-vs-shipped discrimination); this
//! module grades them on the 29 SHIPPED triples, against labels an agent wrote
//! by reading each triple.
//!
//! # These labels are agent-labelled
//!
//! `../fixtures/gap-semantic-remaining4-labels.json` was written by one Claude
//! agent session reading the corpus. It is not human ground truth and nothing
//! in it was checked against the compiler or a test runner. Its `provenance`
//! field says so, and `../gap_remaining4_labels.rs` fails the default gate if
//! that sentence is ever removed.
//!
//! # No new maths
//!
//! Every number comes from `../support/grading.rs` — tally, the
//! constant-predictor baseline, defect / no-defect recall, per-class
//! precision/recall and calibration — which PLAT-917 and PLAT-838 already
//! use. This module only builds [`Graded`] rows: a binary question's truth is
//! the label `yes`/`no`, `divergence_kind` is the six-way label, and `severity`
//! is graded at the verdict the ticket maps it to (`high` -> `FAIL`,
//! `medium`/`low` -> `CONDITIONAL`, else `PASS`) through
//! `gap_semantic_support::severity_verdict`.

#![allow(
    dead_code,
    reason = "each test binary links this module separately, so items only one binary uses read as dead in the other"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

#[path = "../support/grading.rs"]
pub(crate) mod grading;

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::gap_battery_support::BatteryAnswers;
use crate::gap_semantic_support::{
    DIVERGENCE_KINDS, SEVERITY_RUBRIC, nearest_rubric_label, severity_verdict,
};
use grading::{Graded, Tier, Verdict};

/// The labels, compiled in so the fixture and this grader cannot drift.
pub(crate) const LABELS: &str = include_str!("../fixtures/gap-semantic-remaining4-labels.json");

/// The whole labels file.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LabelFile {
    /// Who labelled, how, and what it is not. Must say `AGENT-LABELLED`.
    pub(crate) provenance: String,
    /// The unit a label judges.
    pub(crate) unit: String,
    /// The labelling rule per question, fixed before the first live call.
    pub(crate) rules: BTreeMap<String, String>,
    pub(crate) labels: Vec<Label>,
}

/// One triple's four labels.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Label {
    /// The corpus id, e.g. `GAP-01`.
    pub(crate) id: String,
    /// `None` = excluded from this question's population, per the rule.
    pub(crate) code_implements_intent: Option<bool>,
    /// `None` = excluded from this question's population, per the rule.
    pub(crate) code_exceeds_requirement: Option<bool>,
    /// The primary six-way reading.
    pub(crate) divergence_kind: String,
    /// Other readings the labeller recorded as defensible.
    #[serde(default)]
    pub(crate) divergence_kind_contested: Vec<String>,
    /// The primary rubric level: `none` / `low` / `medium` / `high`.
    pub(crate) severity: String,
    /// Other levels the labeller recorded as defensible.
    #[serde(default)]
    pub(crate) severity_contested: Vec<String>,
    /// Why, citing the triple's own text.
    pub(crate) rationale: String,
}

/// Parses the compiled-in labels.
///
/// # Panics
/// If the fixture stops matching this shape.
pub(crate) fn label_file() -> LabelFile {
    serde_json::from_str(LABELS).expect("the remaining-4 labels parse")
}

/// The labels alone.
pub(crate) fn labels() -> Vec<Label> {
    label_file().labels
}

/// The two binary answers' labels.
pub(crate) const YES: &str = "yes";
pub(crate) const NO: &str = "no";

/// The severity verdicts the ticket's rubric maps to.
pub(crate) const VERDICTS: [&str; 3] = ["PASS", "CONDITIONAL", "FAIL"];

const fn yes_no(value: bool) -> &'static str {
    if value { YES } else { NO }
}

/// How `actual` compares to a primary reading and its contested alternatives.
pub(crate) fn verdict_of(
    expected: &str,
    contested: &[String],
    actual: Option<&str>,
    space: &[&str],
) -> Verdict {
    match actual {
        None => Verdict::Unanswered,
        Some(actual) if !space.contains(&actual) => Verdict::Unrecognized,
        Some(actual) if actual == expected => Verdict::Primary,
        Some(actual) if contested.iter().any(|reading| reading == actual) => Verdict::Contested,
        Some(_) => Verdict::Wrong,
    }
}

/// One row for the shared grader. `contested` excludes `expected`; the shared
/// `Graded::contested` includes it, so it is added here.
fn row(
    id: &str,
    expected: &str,
    contested: &[String],
    actual: Option<String>,
    space: &[&str],
    confidence: Option<f64>,
) -> Graded {
    let verdict = verdict_of(expected, contested, actual.as_deref(), space);
    let mut readings = vec![expected.to_owned()];
    readings.extend(contested.iter().cloned());
    Graded {
        fixture_id: id.to_owned(),
        tier: if contested.is_empty() {
            Tier::Clean
        } else {
            Tier::Ambiguous
        },
        expected: expected.to_owned(),
        contested: readings,
        actual: actual.clone().unwrap_or_else(|| "<unanswered>".to_owned()),
        actual_class: actual.unwrap_or_else(|| "<unanswered>".to_owned()),
        verdict,
        confidence,
    }
}

/// A `noul` question's row, or `None` when the label excludes this triple.
///
/// `yes` = the lens's probability for `key` is at least 0.5, the same
/// threshold MP-232 and MP-233 use. Confidence is the probability of the
/// label it picked, `max(p, 1 - p)`.
fn binary(
    id: &str,
    truth: Option<bool>,
    said: Option<&BatteryAnswers>,
    key: &str,
) -> Option<Graded> {
    let truth = truth?;
    let p = said.and_then(|answers| answers.p(key));
    let actual = p.map(|p| yes_no(p >= 0.5).to_owned());
    let confidence = p.map(|p| p.max(1.0 - p));
    Some(row(id, yes_no(truth), &[], actual, &[YES, NO], confidence))
}

/// Every graded population, from one pass's answers keyed by triple id.
#[derive(Debug, Clone, Default)]
pub(crate) struct Grades {
    /// `code_implements_intent`: defect class `no`.
    pub(crate) implements: Vec<Graded>,
    /// `code_exceeds_requirement`: defect class `yes`.
    pub(crate) exceeds: Vec<Graded>,
    /// `divergence_kind` as asked: no-defect class `aligned`.
    pub(crate) divergence_asked: Vec<Graded>,
    /// `divergence_kind` derived from the same response's own `noul`
    /// answers by MP-233's pre-registered rule. Reported, not gated.
    pub(crate) divergence_derived: Vec<Graded>,
    /// `severity` at the verdict it maps to: no-defect class `PASS`.
    pub(crate) severity_verdict: Vec<Graded>,
    /// `severity` at its exact rubric level. Reported, not gated.
    pub(crate) severity_level: Vec<Graded>,
}

/// Grades one pass. A triple with no entry in `said` is graded `Unanswered`
/// on every question, never skipped.
pub(crate) fn grade(labels: &[Label], said: &BTreeMap<String, BatteryAnswers>) -> Grades {
    let mut grades = Grades::default();
    for label in labels {
        let id = label.id.as_str();
        let answers = said.get(id);
        grades.implements.extend(binary(
            id,
            label.code_implements_intent,
            answers,
            "code_implements_intent",
        ));
        grades.exceeds.extend(binary(
            id,
            label.code_exceeds_requirement,
            answers,
            "code_exceeds_requirement",
        ));
        grades.divergence_asked.push(row(
            id,
            &label.divergence_kind,
            &label.divergence_kind_contested,
            answers.and_then(|answers| answers.divergence_kind.clone()),
            &DIVERGENCE_KINDS,
            None,
        ));
        grades.divergence_derived.push(row(
            id,
            &label.divergence_kind,
            &label.divergence_kind_contested,
            answers
                .and_then(BatteryAnswers::derived_divergence_kind)
                .map(ToOwned::to_owned),
            &DIVERGENCE_KINDS,
            None,
        ));
        let level = answers
            .and_then(|answers| answers.severity)
            .and_then(nearest_rubric_label);
        let contested_verdicts: Vec<String> = label
            .severity_contested
            .iter()
            .map(|level| severity_verdict(Some(level)).to_owned())
            .filter(|verdict| verdict != severity_verdict(Some(&label.severity)))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        grades.severity_verdict.push(row(
            id,
            severity_verdict(Some(&label.severity)),
            &contested_verdicts,
            level
                .as_deref()
                .map(|level| severity_verdict(Some(level)).to_owned()),
            &VERDICTS,
            None,
        ));
        grades.severity_level.push(row(
            id,
            &label.severity,
            &label.severity_contested,
            level,
            &SEVERITY_RUBRIC,
            None,
        ));
    }
    grades
}

/// The four gated questions: display name, rows, and the no-defect label the
/// shared defect / no-defect recall is keyed on.
pub(crate) fn gated(grades: &Grades) -> [(&'static str, &[Graded], &'static str); 4] {
    [
        ("code_implements_intent", &grades.implements, YES),
        ("code_exceeds_requirement", &grades.exceeds, NO),
        (
            "divergence_kind (asked)",
            &grades.divergence_asked,
            "aligned",
        ),
        ("severity (verdict)", &grades.severity_verdict, "PASS"),
    ]
}
