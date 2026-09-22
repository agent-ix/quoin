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
//! tokens. Both operate on plain [`Value`] rather than a lens's own typed
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
/// incident: `"disputed 5 of 14"`.
#[allow(
    clippy::expect_used,
    reason = "a fixed, hand-written literal: a bad pattern here is a compile-time-discoverable \
              bug in this module, not a runtime condition to propagate as an error"
)]
static OF_CLAIM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d+)\s+of\s+(\d+)\b").expect("static pattern compiles"));

/// Matches the companion claim the same sentence made: `"(agreed 9)"`.
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

/// Whether a fixture entry carries a recorded second reading -- any key
/// ending in `_contested` whose value is present and not `null`, at any
/// depth. Recursive because the real corpus nests these under a `labels`
/// object (`fixture.labels.weakness_kind_contested`), not at the fixture's
/// own top level.
fn is_contested(item: &Value) -> bool {
    match item {
        Value::Object(fields) => fields.iter().any(|(key, value)| {
            (key.ends_with("_contested") && !value.is_null()) || is_contested(value)
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
            let present = entry.get(*field).is_some_and(|value| match value {
                Value::Null => false,
                Value::String(text) => !text.trim().is_empty(),
                Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => true,
            });
            if !present {
                findings.push(AdequacyFinding(format!(
                    "{id}: required field `{field}` is missing, null, or empty"
                )));
            }
        }
    }
    findings
}

/// Fails one finding per number a corpus's own top-level prose claims that
/// matches none of the counts the corpus actually holds.
///
/// Derives the actual total, contested, and agreed (non-contested) fixture
/// counts from every top-level array field whose key ends in `_fixtures`
/// ([`is_contested`] decides which items count as contested), then scans
/// every top-level *string* field for a claim in the two forms the real
/// incident used (`"N of M"`, `"agreed N"`) and flags any captured number
/// that is none of the three actual counts.
///
/// Deliberately not a general prose parser: it reads only metadata fields at
/// the corpus root, never fixture content (`criterion_text`, `rationale`),
/// so a number inside a real criterion's own text is never in scope.
///
/// PLAT-933: this is what would have caught the corpus's own
/// `governing_ruling_on_disagreement` saying "disputed 5 of 14 (agreed 9)"
/// while the fixtures held 9 of 15 contested.
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
    let agreed = total.saturating_sub(contested);
    let plausible = [total, contested, agreed];

    let mut findings = Vec::new();
    for (field, value) in root {
        let Value::String(text) = value else {
            continue;
        };
        for capture in OF_CLAIM.captures_iter(text) {
            check_claim(&capture[1], field, text, &plausible, &mut findings);
            check_claim(&capture[2], field, text, &plausible, &mut findings);
        }
        for capture in AGREED_CLAIM.captures_iter(text) {
            check_claim(&capture[1], field, text, &plausible, &mut findings);
        }
    }
    findings
}

/// Flags one captured number when it matches none of the corpus's actual
/// counts. Silently ignores a capture that does not parse as `usize` --
/// [`OF_CLAIM`] and [`AGREED_CLAIM`] only ever capture digit runs, so this is
/// unreachable rather than a validation this function needs to report.
fn check_claim(
    captured: &str,
    field: &str,
    text: &str,
    plausible: &[usize; 3],
    findings: &mut Vec<AdequacyFinding>,
) {
    let Ok(claimed) = captured.parse::<usize>() else {
        return;
    };
    if !plausible.contains(&claimed) {
        findings.push(AdequacyFinding(format!(
            "{field}: claims {claimed}, which matches none of the corpus's actual counts \
             (total {}, contested {}, agreed {}) -- text: {text:?}",
            plausible[0], plausible[1], plausible[2]
        )));
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use serde_json::json;

    use super::{check_required_fields, check_stated_counts};

    /// Provenance: PLAT-933. Reproduces the real incident's exact phrasing
    /// ("disputed 5 of 14 (agreed 9)") over data shaped like the real
    /// corpus (9 of 15 actually contested) and asserts both stale numbers
    /// are flagged.
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
        assert!(
            messages.iter().any(|m| m.contains("claims 14")),
            "expected the stale total (14) to be flagged, got: {messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("claims 5")),
            "expected the stale disputed count (5) to be flagged, got: {messages:?}"
        );
        assert!(
            !messages.iter().any(|m| m.contains("claims 9")),
            "9 is the corpus's actual contested count and must not be flagged, got: {messages:?}"
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

    /// Provenance: PLAT-933. A corpus with no recognizable fixture array is
    /// itself a finding rather than a silent no-op -- a typo in a corpus's
    /// own key should not pass this check by accident.
    #[test]
    fn a_corpus_with_no_fixture_array_is_flagged_rather_than_skipped() {
        let corpus = json!({ "$comment": "nothing here" });
        assert_eq!(check_stated_counts(&corpus).len(), 1);
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
}
