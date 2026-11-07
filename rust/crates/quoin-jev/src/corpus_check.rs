// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pre-flight adequacy checks a lens's own fixture corpus should pass before
//! a live evaluation spends real tokens against it (PLAT-933).
//!
//! Two real incidents motivate this, both from the criterion-strength
//! lens's own dogfood (PLAT-917):
//!
//! 1. The corpus's own `governing_ruling_on_disagreement` field said a
//!    second reader "disputed 5 of 14 (agreed 9)"; the fixtures underneath
//!    actually held 9 of 15 contested. Nobody compared the prose to the data
//!    until a live evaluation was already mid-run.
//! 2. The FR context each fixture's label was made against left `statement`
//!    empty on 10 of 11 criteria -- a field the lens's request shape needs
//!    just to run -- and this was also only discovered mid-run.
//!
//! [`check_stated_counts`] and [`check_required_fields`] turn both into a
//! function a corpus's own default-gate test can call, so either mismatch
//! fails the build instead of surfacing during a run that spends real
//! tokens. A third check, [`check_answerability`] (PLAT-982), catches a
//! label that is not a legal answer to the question meant to produce it --
//! a corpus asserting a ground truth the lens could never emit. All three
//! operate on plain [`Value`] rather than a lens's own typed
//! context so any lens's corpus can call them without this crate knowing
//! that lens's shape.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

/// One way a corpus failed an adequacy check.
///
/// A plain message rather than a coded [`crate::error::JevError`]: these are
/// advisory findings a test asserts `is_empty()` on, not an operational
/// failure this crate returns to a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdequacyFinding(pub String);

impl std::fmt::Display for AdequacyFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Matches the English phrasing that stated the stale count in the real
/// incident: `"disputed 5 of 14"`. Capture 1 is the claimed *contested*
/// count (`5`, "disputed N"); capture 2 is the claimed *total* (`14`, "of
/// M"). The two are checked against different actual counts -- see
/// [`check_stated_counts`] -- never pooled into one set either could match.
#[allow(
    clippy::expect_used,
    reason = "a fixed, hand-written literal: a bad pattern here is a compile-time-discoverable \
              bug in this module, not a runtime condition to propagate as an error"
)]
static OF_CLAIM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d+)\s+of\s+(\d+)\b").expect("static pattern compiles"));

/// Matches the companion claim the same sentence made: `"(agreed 9)"`. Its
/// one capture is the claimed *agreed* (non-contested) count, checked only
/// against the actual agreed count.
#[allow(
    clippy::expect_used,
    reason = "a fixed, hand-written literal: a bad pattern here is a compile-time-discoverable \
              bug in this module, not a runtime condition to propagate as an error"
)]
static AGREED_CLAIM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bagreed\s+(\d+)\b").expect("static pattern compiles"));

/// Every `(id, entry)` pair in a fixtures container, whichever of the two
/// shapes this crate's own corpora already use: a JSON array (id from the
/// entry's own `fixture_id` field, falling back to its index) such as
/// `criterion-strength-fixtures.json`'s fixture lists, or a JSON object
/// keyed by id such as `criterion-strength-fr-context.json`'s `fixtures`
/// map.
fn entries(value: &Value) -> Vec<(String, &Value)> {
    match value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let id = item
                    .get("fixture_id")
                    .and_then(Value::as_str)
                    .map_or_else(|| index.to_string(), ToOwned::to_owned);
                (id, item)
            })
            .collect(),
        Value::Object(map) => map.iter().map(|(id, item)| (id.clone(), item)).collect(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => Vec::new(),
    }
}

/// Whether a JSON value counts as *present*, for both [`is_contested`] and
/// [`check_required_fields`]: not `null`, and not an empty string (after
/// trimming), empty array, or empty object. A `false` or a `0` is a real,
/// meaningful answer -- only `null` and emptiness read as "nothing was
/// recorded here".
fn is_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

/// Whether a fixture entry carries a recorded second reading -- any key
/// ending in `_contested` whose value [`is_present`], at any depth.
/// Recursive because the real corpus nests these under a `labels` object
/// (`fixture.labels.weakness_kind_contested`), not at the fixture's own top
/// level. An empty `_contested` array (present as a key but recording no
/// second reading) does not count -- only a genuinely present value does.
fn is_contested(item: &Value) -> bool {
    match item {
        Value::Object(fields) => fields.iter().any(|(key, value)| {
            (key.ends_with("_contested") && is_present(value)) || is_contested(value)
        }),
        Value::Array(items) => items.iter().any(is_contested),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

/// Fails one finding per `(fixture, field)` where a lens-declared required
/// input is missing, `null`, or an all-whitespace string.
///
/// `fixtures` is either shape [`entries`] reads. Call it with the field
/// names a lens's own context type sends as non-optional -- e.g.
/// `FrContext::statement` -- over the corpus itself, or over a separate
/// context sidecar keyed the same way, whichever actually carries the
/// field. Nothing here knows a lens's Rust type; it only knows the JSON
/// field name that type is built from.
///
/// PLAT-933: this is what would have caught `statement` going out empty on
/// 10 of 11 criteria before the live run that discovered it instead.
#[must_use]
pub fn check_required_fields(fixtures: &Value, required: &[&str]) -> Vec<AdequacyFinding> {
    let items = entries(fixtures);
    if items.is_empty() {
        return vec![AdequacyFinding(
            "no fixture entries found (expected a JSON array or an object of entries)".to_owned(),
        )];
    }
    let mut findings = Vec::new();
    for (id, entry) in &items {
        for field in required {
            let present = entry.get(*field).is_some_and(is_present);
            if !present {
                findings.push(AdequacyFinding(format!(
                    "{id}: required field `{field}` is missing, null, or empty"
                )));
            }
        }
    }
    findings
}

/// The counts a corpus's own data actually holds, derived once and then
/// checked field-by-field against a prose claim -- never pooled into one set
/// a claim for any field could match. Pooling them was PLAT-933 review's
/// finding: a stale "agreed 9" passed because 9 happened to equal the actual
/// *contested* count, not because it was a correct agreed count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActualCounts {
    /// Every fixture, across all `*_fixtures` arrays.
    total: usize,
    /// Fixtures [`is_contested`] finds a recorded second reading on.
    contested: usize,
    /// `total - contested`: fixtures with a single, uncontested reading.
    agreed: usize,
}

/// Fails one finding per number a corpus's own top-level prose claims that
/// does not match the actual count that claim is about.
///
/// Derives [`ActualCounts`] from every top-level array field whose key ends
/// in `_fixtures` ([`is_contested`] decides which items count as
/// contested), then scans every top-level *string* field for a claim in the
/// two forms the real incident used and checks each capture against the one
/// actual count it claims to be: [`OF_CLAIM`]'s first capture ("disputed
/// N") against `contested`, its second ("of M") against `total`, and
/// [`AGREED_CLAIM`]'s capture ("agreed N") against `agreed`.
///
/// Deliberately not a general prose parser: it reads only metadata fields at
/// the corpus root, never fixture content (`criterion_text`, `rationale`),
/// so a number inside a real criterion's own text is never in scope.
///
/// PLAT-933: this is what would have caught the corpus's own
/// `governing_ruling_on_disagreement` saying "disputed 5 of 14 (agreed 9)"
/// while the fixtures held 9 of 15 contested (agreed 6) -- all three of 5,
/// 14 and 9 are wrong, and checking each against its own actual count
/// catches all three, including the 9 that happened to equal a *different*
/// actual count.
#[must_use]
pub fn check_stated_counts(corpus: &Value) -> Vec<AdequacyFinding> {
    let Value::Object(root) = corpus else {
        return vec![AdequacyFinding(
            "corpus root is not a JSON object".to_owned(),
        )];
    };

    let fixture_arrays: Vec<&Vec<Value>> = root
        .iter()
        .filter_map(|(key, value)| match value {
            Value::Array(items) if key.ends_with("_fixtures") => Some(items),
            _ => None,
        })
        .collect();

    if fixture_arrays.is_empty() {
        return vec![AdequacyFinding(
            "no top-level array field ending in `_fixtures`; nothing to check".to_owned(),
        )];
    }

    let total = fixture_arrays
        .iter()
        .map(|items| items.len())
        .sum::<usize>();
    let contested = fixture_arrays
        .iter()
        .flat_map(|items| items.iter())
        .filter(|item| is_contested(item))
        .count();
    let actual = ActualCounts {
        total,
        contested,
        agreed: total.saturating_sub(contested),
    };

    let mut findings = Vec::new();
    for (field, value) in root {
        let Value::String(text) = value else {
            continue;
        };
        for capture in OF_CLAIM.captures_iter(text) {
            check_claim(
                &capture[1],
                "contested",
                actual.contested,
                field,
                text,
                &mut findings,
            );
            check_claim(
                &capture[2],
                "total",
                actual.total,
                field,
                text,
                &mut findings,
            );
        }
        for capture in AGREED_CLAIM.captures_iter(text) {
            check_claim(
                &capture[1],
                "agreed",
                actual.agreed,
                field,
                text,
                &mut findings,
            );
        }
    }
    findings
}

/// Fails one finding per labeled answer that is not a member of the closed
/// answer space of the question that is supposed to produce it (PLAT-982).
///
/// `fixtures` is either shape [`entries`] reads. `label_path` is the chain of
/// object keys from a fixture entry down to its label -- `["labels",
/// "weakness_kind"]` in the criterion-strength corpus. `answer_space` is the
/// question's closed set, as JSON values so one function serves a `choice`
/// (string members, e.g. `question-set.json`'s `choice.answer_space`) and a
/// `score` (integer levels, e.g. its `score.rubric[].level`) alike; a member
/// matches only by exact JSON equality, so `"Sound"` is not `"sound"` and
/// `2.0` is not level `2`.
///
/// A recorded second reading is checked too: when the label's parent object
/// carries `<last key>_contested`, that value must be an array and every
/// element in it must also be in `answer_space`. A contested alternative is
/// as much a claimed answer as the primary one -- an evaluation that scores
/// a reply against either reading reads both.
///
/// Every entry must carry the label: a missing or `null` label is a finding,
/// since a fixture that answers nothing contributes nothing to an accuracy
/// number but still counts toward its denominator. An empty `answer_space`
/// or an empty container is a finding rather than a vacuous pass.
///
/// PLAT-982: a mislabeled item should be a loud failure at corpus-check
/// time, not a silent wrong answer contributing to a bad accuracy number --
/// the same defect class as PLAT-917's false provenance claim, a corpus
/// asserting something about itself that nothing checked.
#[must_use]
pub fn check_answerability(
    fixtures: &Value,
    label_path: &[&str],
    answer_space: &[Value],
) -> Vec<AdequacyFinding> {
    let Some((last, parents)) = label_path.split_last() else {
        return vec![AdequacyFinding("label path is empty".to_owned())];
    };
    if answer_space.is_empty() {
        return vec![AdequacyFinding(
            "answer space is empty; no label could be answerable".to_owned(),
        )];
    }
    let items = entries(fixtures);
    if items.is_empty() {
        return vec![AdequacyFinding(
            "no fixture entries found (expected a JSON array or an object of entries)".to_owned(),
        )];
    }

    let path = label_path.join(".");
    let space = answer_space
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let contested_key = format!("{last}_contested");
    let mut findings = Vec::new();
    for (id, entry) in &items {
        let parent = parents
            .iter()
            .try_fold(*entry, |value, key| value.get(*key));
        let label = parent.and_then(|value| value.get(*last));
        match label {
            None | Some(Value::Null) => findings.push(AdequacyFinding(format!(
                "{id}: label `{path}` is missing or null"
            ))),
            Some(answer) if !answer_space.contains(answer) => {
                findings.push(AdequacyFinding(format!(
                    "{id}: label `{path}` = {answer} is not in the question's answer space \
                     [{space}]"
                )));
            }
            Some(_) => {}
        }
        match parent.and_then(|value| value.get(&contested_key)) {
            None => {}
            Some(Value::Array(readings)) => {
                for reading in readings.iter().filter(|r| !answer_space.contains(r)) {
                    findings.push(AdequacyFinding(format!(
                        "{id}: contested reading {reading} in `{path}_contested` is not in the \
                         question's answer space [{space}]"
                    )));
                }
            }
            Some(other) => findings.push(AdequacyFinding(format!(
                "{id}: `{path}_contested` is {other}, not an array of readings"
            ))),
        }
    }
    findings
}

/// Flags one captured number against the single actual count it claims to
/// be. Silently ignores a capture that does not parse as `usize` --
/// [`OF_CLAIM`] and [`AGREED_CLAIM`] only ever capture digit runs, so this is
/// unreachable rather than a validation this function needs to report.
fn check_claim(
    captured: &str,
    kind: &str,
    actual: usize,
    field: &str,
    text: &str,
    findings: &mut Vec<AdequacyFinding>,
) {
    let Ok(claimed) = captured.parse::<usize>() else {
        return;
    };
    if claimed != actual {
        findings.push(AdequacyFinding(format!(
            "{field}: claims {kind} {claimed}, but the corpus actually holds {kind} {actual} \
             -- text: {text:?}"
        )));
    }
}

#[cfg(test)]
mod tests;
