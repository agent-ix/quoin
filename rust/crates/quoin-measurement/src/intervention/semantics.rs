// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The checks the JSON Schema cannot state.
//!
//! A port of `semanticFindings` (`intervention.ts:208-344`). These are the
//! cross-field agreements — a randomized design needs a randomized assignment,
//! a causal conclusion needs attributed confidence, a `treatment_id` must name
//! an arm that exists — and JSON Schema can express none of them.
//!
//! # Why this runs on the untyped document
//!
//! `validateInterventionRecord` collects schema errors and semantic findings
//! into **one** refusal (`intervention.ts:47-63`), so the semantic pass runs on
//! records the schema has already rejected. Reading those through the typed
//! record is impossible — it would not deserialize — so this pass reads
//! [`serde_json::Value`] directly and the typed record is built afterwards, in
//! [`crate::intervention::validate`]. That ordering is what keeps the finding
//! set identical to the oracle's for a malformed record.

use serde_json::{Map, Value};

use crate::common::scalar::js_string;
use crate::date_time::Rfc3339DateTime;

/// The `conclusion.kind` values that assert an observed comparison.
const OBSERVED_EFFECT_CONCLUSIONS: [&str; 2] = ["causal_effect_established", "no_effect_observed"];

/// The `assignment.method` values that are random.
const RANDOM_ASSIGNMENTS: [&str; 2] = ["randomized", "blocked_randomized"];

/// The `disposition` values that leave a qualifier unaccounted for.
const OPEN_DISPOSITIONS: [&str; 2] = ["uncontrolled", "unknown"];

/// Every semantic finding this document carries, in the order the oracle
/// emits them.
///
/// The caller sorts and deduplicates; this preserves emission order so a
/// reviewer can follow it against the TypeScript.
#[must_use]
pub fn semantic_findings(value: &Map<String, Value>) -> Vec<String> {
    let mut findings = Vec::new();
    check_observed_at(value, &mut findings);
    check_design(value, &mut findings);
    let treatments = records(value.get("treatments"));
    check_arm_ids(value, &treatments, &mut findings);
    let arm_ids: Vec<&str> = treatments
        .iter()
        .filter_map(|arm| text(arm, "id"))
        .collect();
    check_linked_keys(
        value.get("changed_variables"),
        "name",
        "/changed_variables",
        &arm_ids,
        &mut findings,
    );
    check_linked_keys(
        value.get("measured_effects"),
        "metric",
        "/measured_effects",
        &arm_ids,
        &mut findings,
    );
    check_conclusion(value, &treatments, &mut findings);
    findings
}

/// `intervention.ts:212-217`. The grammar is this crate's one RFC 3339 reader,
/// which is the permissive one; see `DIVERGENCE.md` §11.1.
fn check_observed_at(value: &Map<String, Value>, findings: &mut Vec<String>) {
    if let Some(observed) = value.get("observed_at").and_then(Value::as_str)
        && Rfc3339DateTime::parse(observed).is_err()
    {
        findings.push("/observed_at: must be a valid date-time".to_owned());
    }
}

/// `intervention.ts:218-238` — the design and its assignment must agree.
fn check_design(value: &Map<String, Value>, findings: &mut Vec<String>) {
    let design = object(value.get("design"));
    let assignment = design.and_then(|found| object(found.get("assignment")));
    let method = assignment.and_then(|found| text(found, "method"));
    if design.and_then(|found| text(found, "kind")) == Some("randomized")
        && !method.is_some_and(|found| RANDOM_ASSIGNMENTS.contains(&found))
    {
        findings.push(
            "/design/assignment/method: randomized design requires randomized assignment"
                .to_owned(),
        );
    }
    if method.is_some_and(|found| RANDOM_ASSIGNMENTS.contains(&found))
        && assignment
            .and_then(|found| text(found, "seed"))
            .is_none_or(str::is_empty)
    {
        findings.push("/design/assignment/seed: randomized assignment requires a seed".to_owned());
    }
}

/// `intervention.ts:240-252` — no arm id is used twice, baseline included.
fn check_arm_ids<'a>(
    value: &'a Map<String, Value>,
    treatments: &[&'a Map<String, Value>],
    findings: &mut Vec<String>,
) {
    let mut seen: Vec<&str> = Vec::new();
    // `asRecord` first, then the `typeof … === "string"` test: a baseline that
    // is not an object contributes no id rather than refusing here.
    if let Some(baseline) = object(value.get("baseline")).and_then(|found| text(found, "id")) {
        seen.push(baseline);
    }
    for (index, treatment) in treatments.iter().enumerate() {
        let Some(id) = text(treatment, "id") else {
            continue;
        };
        if seen.contains(&id) {
            findings.push(format!("/treatments/{index}/id: duplicate arm id {id}"));
        }
        seen.push(id);
    }
}

/// `intervention.ts:320-344` — every linked row names one existing arm, and no
/// `(arm, key)` pair appears twice.
fn check_linked_keys(
    value: Option<&Value>,
    field: &str,
    path: &str,
    arm_ids: &[&str],
    findings: &mut Vec<String>,
) {
    let mut seen: Vec<(String, String)> = Vec::new();
    for (index, item) in records(value).into_iter().enumerate() {
        let Some(arm) = text(item, "treatment_id").filter(|found| arm_ids.contains(found)) else {
            findings.push(format!(
                "{path}/{index}/treatment_id: does not resolve to one treatment"
            ));
            continue;
        };
        // The `\0` join in `intervention.ts:337` exists so that no pair of
        // members can be concatenated two ways; a tuple key cannot be, so the
        // separator has no counterpart here.
        let key = (arm.to_owned(), js_string(item.get(field)));
        if seen.contains(&key) {
            findings.push(format!("{path}/{index}: duplicate treatment/{field} key"));
        }
        seen.push(key);
    }
}

/// `intervention.ts:254-318` — the conclusion must be supported by what was
/// measured.
fn check_conclusion(
    value: &Map<String, Value>,
    treatments: &[&Map<String, Value>],
    findings: &mut Vec<String>,
) {
    let conclusion = object(value.get("conclusion"));
    let kind = conclusion.and_then(|found| text(found, "kind"));
    let status = value.get("status").and_then(Value::as_str);
    if matches!(status, Some("failed" | "inconclusive")) && kind != Some("cause_not_established") {
        findings.push(
            "/conclusion/kind: failed or inconclusive records require cause_not_established"
                .to_owned(),
        );
    }
    if !kind.is_some_and(|found| OBSERVED_EFFECT_CONCLUSIONS.contains(&found)) {
        return;
    }
    // `arms = [baseline, ...treatments]` includes the baseline **even when it
    // is not an object**: `arm?.sample_size` is then `undefined`, which is not
    // a number, so the finding fires. An absent baseline is a failing arm here
    // for the same reason.
    let positive = |arm: &Map<String, Value>| {
        arm.get("sample_size")
            .and_then(Value::as_f64)
            .is_some_and(|size| size >= 1.0)
    };
    let arms_hold = object(value.get("baseline")).is_some_and(positive)
        && treatments.iter().all(|arm| positive(arm));
    if !arms_hold {
        findings.push(
            "/conclusion/kind: observed-effect conclusions require positive arm samples".to_owned(),
        );
    }
    let effects = records(value.get("measured_effects"));
    if effects.is_empty()
        || effects.iter().any(|effect| {
            effect.get("baseline_value") == Some(&Value::Null)
                || effect.get("treatment_value") == Some(&Value::Null)
        })
    {
        findings.push(
            "/measured_effects: observed-effect conclusions require observed comparisons"
                .to_owned(),
        );
    }
    if kind != Some("causal_effect_established") {
        return;
    }
    if effects
        .iter()
        .any(|effect| effect.get("effect") == Some(&Value::Null))
    {
        findings.push("/measured_effects: causal conclusions require non-null effects".to_owned());
    }
    if conclusion.and_then(|found| text(found, "attribution_confidence")) == Some("none") {
        findings.push(
            "/conclusion/attribution_confidence: causal conclusion requires attributed confidence"
                .to_owned(),
        );
    }
    let open = records(value.get("interactions"))
        .into_iter()
        .chain(records(value.get("confounders")))
        .any(|item| {
            text(item, "disposition").is_some_and(|found| OPEN_DISPOSITIONS.contains(&found))
        });
    if open {
        findings.push(
            "/conclusion/kind: causal conclusion conflicts with uncontrolled or unknown qualifier"
                .to_owned(),
        );
    }
}

/// `isRecord` (`intervention.ts:365-367`) — an object, not an array, not null.
fn object(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value.and_then(Value::as_object)
}

/// `records` (`intervention.ts:357-359`) — the object members of an array.
fn records(value: Option<&Value>) -> Vec<&Map<String, Value>> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_object).collect())
        .unwrap_or_default()
}

/// A member read as a string, the way `typeof x === "string"` reads it.
fn text<'a>(value: &'a Map<String, Value>, member: &str) -> Option<&'a str> {
    value.get(member).and_then(Value::as_str)
}
