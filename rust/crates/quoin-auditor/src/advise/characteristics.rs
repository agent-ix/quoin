// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Characteristics readable from an obligation's facts.
//!
//! Mostly prose regexes ([`super::table`]), plus three signals that are not
//! lexical at all: a **structural** one (a statement that parses as a declared
//! configuration space is a configuration matrix by construction, whatever
//! words it contains), a **field** one (the obligation's own criticality), and
//! two **evidence** ones.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use quoin_combinatorial::{js, parse_space};
use regex::Regex;

use super::compound::{matches_outside_compound, prose};
use super::facts::ObligationEvidence;
use super::table::STATEMENT_CHARACTERISTICS;

/// `/^(p0|high|critical)$/i` — read from the value, never inferred.
static HIGH_CRITICALITY: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(
        clippy::expect_used,
        reason = "a literal pattern that fails to compile is a build defect"
    )]
    Regex::new(r"(?i-u)^(?:p0|high|critical)$").expect("the criticality pattern is a literal")
});

/// Every characteristic the advisor can read off one obligation's facts.
///
/// Returned sorted and deduplicated — `[...new Set(matched)].sort()` — so the
/// order the signals are gathered in is unobservable.
#[must_use]
pub fn characteristics_of(
    statement: &str,
    criticality: Option<&str>,
    evidence: Option<&ObligationEvidence>,
    parameters: Option<&BTreeMap<String, String>>,
) -> Vec<String> {
    let text = prose(statement);
    let mut matched: Vec<&str> = STATEMENT_CHARACTERISTICS
        .iter()
        .filter(|entry| matches_outside_compound(&entry.pattern, &text))
        .map(|entry| entry.name)
        .collect();

    // ── Structural signal ──
    // A minted space reads `2-way over features(default|python|wasm)` — no
    // "configuration", no "feature flag" — so the very obligations that most
    // need the combinatorial method would be the ones no regex advised for.
    // Note it reads the RAW statement, not the prose: a link target has never
    // parsed as a space, and reading the raw string is what the retained code
    // does.
    if parse_space(statement).is_some() {
        matched.push("configuration-matrix");
    }
    // ── Structured signal (agent-ix/quoin#166) ──
    // A `target` or `threshold` key in the obligation's own `parameters` IS a
    // quantified threshold, whatever the statement happens to say.
    if parameters.is_some_and(|parameters| {
        parameters.contains_key("target") || parameters.contains_key("threshold")
    }) {
        matched.push("quantified-threshold");
    }
    // Read from the value, never inferred from a threshold this code chose.
    // `if (criticality && …)`: the empty string is falsy in JavaScript, so an
    // empty cell never reaches the pattern.
    if criticality
        .is_some_and(|value| !value.is_empty() && HIGH_CRITICALITY.is_match(js::js_trim(value)))
    {
        matched.push("high-criticality");
    }

    // ── Evidence-side characteristics (agent-ix/quoin#158) ──
    //
    // These say something about the TESTS, not about the requirement, and that
    // is the point: concolic execution and mutation testing are escalations
    // reached when a cheap search stalls, not choices made from a sentence.
    //
    // Both require a binding. An obligation nothing is bound to is
    // `undischarged` — a finding the auditor already reports — and conflating
    // the two would recommend a solver for code nobody has tested yet.
    if let Some(evidence) = evidence.filter(|evidence| evidence.bound) {
        if evidence.fault_detection_scores.is_empty() {
            // Exercised, and nothing measures whether the exercise discriminates.
            matched.push("fault-detection-unmeasured");
        } else if math_min(&evidence.fault_detection_scores) < 1.0 {
            // Measured, and a seeded fault survived. The weakest bound symbol
            // decides, for the reason FR-039 gives: averaging lets a
            // well-tested symbol carry one whose mutants all survive.
            matched.push("fault-detection-failed");
        }
    }

    let distinct: BTreeSet<&str> = matched.into_iter().collect();
    let mut names: Vec<String> = distinct.into_iter().map(ToOwned::to_owned).collect();
    names.sort_by(|left, right| js::compare(left, right));
    names
}

/// `Math.min(...values)` over a non-empty list.
///
/// `Math.min` propagates `NaN`, and `NaN < 1` is false — so a score the store
/// recorded as a non-number leaves `fault-detection-failed` unminted rather
/// than minting it by accident. `f64::min` would swallow the `NaN` instead,
/// which is why this is a fold and not `fold(f64::INFINITY, f64::min)`.
fn math_min(values: &[f64]) -> f64 {
    values.iter().fold(f64::INFINITY, |smallest, &value| {
        if value.is_nan() || smallest.is_nan() {
            f64::NAN
        } else if value < smallest {
            value
        } else {
            smallest
        }
    })
}

/// Every `characteristics` value any code path in quoin can produce.
///
/// This exists so the **join** between the catalog and the fact set is
/// checkable. The catalog declares the values that trigger a method; this is
/// the set that can ever be produced to match them. Nothing compared the two,
/// and the consequence was silent: `match_rules` skips an unknown *axis* by
/// design (FR-054-CON-2 leaves the axis set open) but an unknown *value* on a
/// known axis simply never matches, and `inconclusive` is already a legitimate
/// outcome. A systematically under-advising advisor was indistinguishable from
/// a quiet one.
///
/// Measured when this was added: the installed catalog declared **60** values
/// and 20 were producible, leaving **7 methods** that no statement could ever
/// reach (agent-ix/quoin#128).
///
/// Derived from the regex table rather than hand-listed, because a hand-listed
/// copy is the failure mode this whole check exists to catch.
#[must_use]
pub fn mintable_characteristics() -> BTreeSet<String> {
    let mut names: BTreeSet<String> = STATEMENT_CHARACTERISTICS
        .iter()
        .map(|entry| entry.name.to_owned())
        .collect();
    // Not lexical: read from the obligation's own criticality field.
    names.insert("high-criticality".to_owned());
    // Not lexical: read from the evidence store.
    names.insert("fault-detection-unmeasured".to_owned());
    names.insert("fault-detection-failed".to_owned());
    names
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::collections::BTreeMap;

    use super::super::facts::ObligationEvidence;
    use super::{characteristics_of, math_min, mintable_characteristics};

    fn of(statement: &str) -> Vec<String> {
        characteristics_of(statement, None, None, None)
    }

    #[test]
    fn the_result_is_sorted_and_deduplicated() {
        // `malformed` is an alternative of BOTH `untrusted-input` and
        // `input-validation`, so this is not a one-entry list.
        let found = of("rejects malformed user-supplied input");
        let mut sorted = found.clone();
        sorted.sort();
        assert_eq!(found, sorted);
        assert!(
            found.len() >= 2,
            "expected several characteristics, got {found:?}"
        );
        let mut distinct = found.clone();
        distinct.dedup();
        assert_eq!(found, distinct);
    }

    #[test]
    fn a_link_target_no_longer_mints_a_characteristic() {
        assert!(
            !of("holds ([FR-1](../stakeholder/StR-005.md))")
                .contains(&"stakeholder-facing".to_owned()),
            "a directory name inside a link target is not prose"
        );
        assert!(of("the stakeholder signs off").contains(&"stakeholder-facing".to_owned()));
    }

    #[test]
    fn a_declared_space_is_a_configuration_matrix_whatever_the_words_are() {
        let found = of("2-way over features(default|python|wasm) target(linux|wasm32)");
        assert!(
            found.contains(&"configuration-matrix".to_owned()),
            "a structural signal must not need the word `configuration`: {found:?}"
        );
    }

    #[test]
    fn a_threshold_parameter_mints_without_the_statement_saying_so() {
        let parameters = BTreeMap::from([("target".to_owned(), "< 4 min".to_owned())]);
        let found = characteristics_of("the thing works", None, None, Some(&parameters));
        assert!(found.contains(&"quantified-threshold".to_owned()));
        assert!(!of("the thing works").contains(&"quantified-threshold".to_owned()));
    }

    #[test]
    fn criticality_is_read_from_the_value_and_never_judged() {
        assert!(
            characteristics_of("x", Some("  P0 "), None, None)
                .contains(&"high-criticality".to_owned()),
            "the value is trimmed with JavaScript's trim, then matched case-insensitively"
        );
        assert!(
            !characteristics_of("x", Some("medium"), None, None)
                .contains(&"high-criticality".to_owned())
        );
        assert!(
            !characteristics_of("x", Some(""), None, None).contains(&"high-criticality".to_owned())
        );
    }

    #[test]
    fn unbound_evidence_mints_nothing_at_all() {
        let unbound = ObligationEvidence {
            bound: false,
            fault_detection_scores: vec![0.2],
        };
        let found = characteristics_of("x", None, Some(&unbound), None);
        assert!(!found.contains(&"fault-detection-unmeasured".to_owned()));
        assert!(!found.contains(&"fault-detection-failed".to_owned()));
    }

    #[test]
    fn a_bound_obligation_with_no_score_is_unmeasured_and_with_a_low_one_failed() {
        let unmeasured = ObligationEvidence {
            bound: true,
            fault_detection_scores: Vec::new(),
        };
        assert!(
            characteristics_of("x", None, Some(&unmeasured), None)
                .contains(&"fault-detection-unmeasured".to_owned())
        );
        let failed = ObligationEvidence {
            bound: true,
            fault_detection_scores: vec![1.0, 0.4],
        };
        assert!(
            characteristics_of("x", None, Some(&failed), None)
                .contains(&"fault-detection-failed".to_owned())
        );
        let clean = ObligationEvidence {
            bound: true,
            fault_detection_scores: vec![1.0],
        };
        let found = characteristics_of("x", None, Some(&clean), None);
        assert!(!found.contains(&"fault-detection-failed".to_owned()));
        assert!(!found.contains(&"fault-detection-unmeasured".to_owned()));
    }

    #[test]
    fn math_min_propagates_nan_the_way_javascript_does() {
        assert!(math_min(&[1.0, f64::NAN, 0.1]).is_nan());
        assert!((math_min(&[1.0, 0.1]) - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn every_lexical_name_is_mintable_and_the_three_non_lexical_ones_too() {
        let mintable = mintable_characteristics();
        assert!(mintable.len() >= 53, "got {}", mintable.len());
        for name in [
            "high-criticality",
            "fault-detection-unmeasured",
            "fault-detection-failed",
            "temporal",
            "latency",
            "degradation",
            "quantified-threshold",
        ] {
            assert!(mintable.contains(name), "`{name}` is not mintable");
        }
    }
}
