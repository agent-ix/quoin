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
}
