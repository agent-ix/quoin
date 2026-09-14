// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Catalog-driven verification-method recommendation (FR-031).
//!
//! The proto-advisor was a **skill-local prose table** — `test | analysis |
//! inspection | demonstration` plus a handful of evidence kinds, declared in
//! no manifest, read by no code. The result was that `Verification` columns
//! defaulted to `Test` by habit, and nothing ever advised DAST for an attack
//! surface, monitors for a temporal property, or fault injection for a
//! reliability NFR.
//!
//! This module is the deterministic half. Rules match or they do not; where
//! they are inconclusive it says so and stops, rather than guessing. An LLM
//! may then judge the residue — labelled as judgement, never as a verdict (the
//! FR-042 / ADR-0010 discipline: no verdict-by-LLM).

pub mod characteristics;
pub mod compound;
pub mod facts;
pub mod pipeline;
pub mod table;
pub mod uncatalogued;

use std::collections::BTreeSet;

use quoin_combinatorial::js;

pub use characteristics::{characteristics_of, mintable_characteristics};
pub use compound::{matches_outside_compound, prose};
pub use facts::{Advice, MatchReason, ObligationEvidence, ObligationFacts, Recommendation};
pub use pipeline::{PropertyShape, advise_all, archetype_of, evidence_for, facts_for};
pub use table::STATEMENT_CHARACTERISTICS;
pub use uncatalogued::{
    UNCATALOGUED_METHOD_REASON, UncataloguedMethods, uncatalogued_authored_methods,
};

use crate::catalog::{MethodCatalog, VerificationMethod};

/// The four applicability axes this advisor can observe.
///
/// A rule naming any other axis is **skipped, not failed**: the engine
/// deliberately leaves the axis set open (FR-054-CON-2), so a module may
/// declare rules this advisor has no facts for — that is a gap in what can be
/// observed, not a reason to reject the method.
struct ObservedAxes {
    characteristics: BTreeSet<String>,
    property_shapes: BTreeSet<String>,
    object_types: BTreeSet<String>,
    archetypes: BTreeSet<String>,
}

impl ObservedAxes {
    /// The facts on one named axis, or [`None`] when the axis is unobservable.
    fn axis(&self, rule: &str) -> Option<&BTreeSet<String>> {
        match rule {
            "characteristics" => Some(&self.characteristics),
            "property_shapes" => Some(&self.property_shapes),
            "object_types" => Some(&self.object_types),
            "archetypes" => Some(&self.archetypes),
            _ => None,
        }
    }
}

/// Recommend methods for one obligation.
///
/// A method is recommended when **any** of its applicability rules matches a
/// fact about the obligation. Ranking is by number of matching rules — a
/// method two axes agree on outranks one a single axis suggested — with the
/// method id as a deterministic tiebreak, so the same input always yields the
/// same order.
#[must_use]
pub fn advise(catalog: &MethodCatalog, facts: &ObligationFacts) -> Advice {
    let observed = ObservedAxes {
        characteristics: characteristics_of(
            &facts.statement,
            facts.criticality.as_deref(),
            facts.evidence.as_ref(),
            facts.parameters.as_ref(),
        )
        .into_iter()
        .collect(),
        // `new Set(facts.propertyShape ? [facts.propertyShape] : [])`: the
        // empty string is falsy in JavaScript, so an empty shape is no shape.
        property_shapes: singleton(facts.property_shape.as_deref()),
        object_types: facts.object_types.iter().cloned().collect(),
        archetypes: singleton(facts.archetype.as_deref()),
    };

    let mut recommended: Vec<Recommendation> = Vec::new();
    for method in &catalog.methods {
        let reasons = match_rules(method, &observed);
        if reasons.is_empty() {
            continue;
        }
        recommended.push(Recommendation {
            method: method.id.clone(),
            class: method.class.clone(),
            evidence_kind: method.evidence_kind.clone(),
            reasons,
        });
    }

    recommended.sort_by(|left, right| {
        right
            .reasons
            .len()
            .cmp(&left.reasons.len())
            .then_with(|| js::compare(&left.method, &right.method))
    });

    let authored = normalize_authored(facts.authored_method.as_deref());
    let inconclusive = recommended.is_empty();
    // The engine already diagnosed this value as a word the catalog never
    // declared, so there is no choice here to disagree with (quoin#168).
    let uncatalogued = authored.is_some() && facts.uncatalogued_method == Some(true);
    // A mismatch is only meaningful when the advisor had something to say.
    // With no rules matched, "the author chose Test and we recommend nothing"
    // is not a disagreement — it is silence, and reporting it as a mismatch
    // would bury the real ones. And an uncatalogued value can never mismatch:
    // it is outside both the method set and the closed class set, so "not
    // among the recommendations" is vacuously true of it.
    let mismatch = !inconclusive
        && !uncatalogued
        && authored.as_deref().is_some_and(|authored| {
            let authored = authored.to_lowercase();
            !recommended.iter().any(|recommendation| {
                recommendation.method.to_lowercase() == authored
                    || recommendation.class.to_lowercase() == authored
            })
        });

    Advice {
        obligation: facts.id.clone(),
        recommended,
        authored,
        mismatch,
        uncatalogued,
        inconclusive,
    }
}

/// `new Set(value ? [value] : [])`.
fn singleton(value: Option<&str>) -> BTreeSet<String> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| BTreeSet::from([value.to_owned()]))
        .unwrap_or_default()
}

/// Every applicability rule of one method that an observable fact satisfies.
fn match_rules(method: &VerificationMethod, facts: &ObservedAxes) -> Vec<MatchReason> {
    let mut reasons: Vec<MatchReason> = Vec::new();
    for (rule, values) in &method.applicability {
        let Some(known) = facts.axis(rule) else {
            continue;
        };
        for value in values {
            if known.contains(value) {
                reasons.push(MatchReason {
                    rule: rule.clone(),
                    value: value.clone(),
                });
            }
        }
    }
    reasons.sort_by(|left, right| {
        js::compare(&left.rule, &right.rule).then_with(|| js::compare(&left.value, &right.value))
    });
    reasons
}

/// `Test (TC-707)` → `Test`; an empty cell → [`None`].
#[must_use]
pub fn normalize_authored(cell: Option<&str>) -> Option<String> {
    let cell = cell.filter(|cell| !cell.is_empty())?;
    let head = cell.split_once('(').map_or(cell, |(before, _)| before);
    let trimmed = js::js_trim(head);
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
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

    use super::{advise, normalize_authored};
    use crate::advise::facts::ObligationFacts;
    use crate::catalog::{MethodCatalog, VerificationMethod};

    fn method(id: &str, class: &str, rules: &[(&str, &[&str])]) -> VerificationMethod {
        VerificationMethod {
            id: id.to_owned(),
            name: id.to_owned(),
            class: class.to_owned(),
            definition: String::new(),
            evidence_kind: None,
            applicability: rules
                .iter()
                .map(|(rule, values)| {
                    (
                        (*rule).to_owned(),
                        values.iter().map(|value| (*value).to_owned()).collect(),
                    )
                })
                .collect(),
            tooling: Vec::new(),
            module_name: "m".to_owned(),
        }
    }

    fn catalog() -> MethodCatalog {
        MethodCatalog {
            methods: vec![
                method(
                    "model-checking",
                    "Analysis",
                    &[("characteristics", &["temporal", "invariance"])],
                ),
                method(
                    "unit-testing",
                    "Test",
                    &[("characteristics", &["temporal"])],
                ),
                method(
                    "unreachable-method",
                    "Analysis",
                    &[("phase_of_the_moon", &["waxing"])],
                ),
            ],
            duplicates: Vec::new(),
            unreadable: Vec::new(),
        }
    }

    fn facts(statement: &str) -> ObligationFacts {
        ObligationFacts {
            id: "FR-1-AC-1".to_owned(),
            statement: statement.to_owned(),
            ..ObligationFacts::default()
        }
    }

    #[test]
    fn more_matching_rules_outrank_fewer_and_the_id_breaks_the_tie() {
        let advice = advise(&catalog(), &facts("the invariant always holds"));
        assert_eq!(
            advice
                .recommended
                .iter()
                .map(|r| r.method.as_str())
                .collect::<Vec<_>>(),
            ["model-checking", "unit-testing"],
            "two matched axes outrank one"
        );
        assert_eq!(advice.recommended[0].reasons.len(), 2);
        assert!(!advice.inconclusive);
    }

    #[test]
    fn a_rule_on_an_unobservable_axis_is_skipped_and_never_fails_the_method() {
        let advice = advise(&catalog(), &facts("the invariant always holds"));
        assert!(
            !advice
                .recommended
                .iter()
                .any(|r| r.method == "unreachable-method"),
            "an unobservable axis contributes no reason"
        );
    }

    #[test]
    fn silence_is_not_a_mismatch() {
        let mut quiet = facts("nothing lexical here at all");
        quiet.authored_method = Some("Test".to_owned());
        let advice = advise(&catalog(), &quiet);
        assert!(advice.inconclusive);
        assert!(!advice.mismatch, "no recommendation is not a disagreement");
    }

    #[test]
    fn an_uncatalogued_value_is_its_own_state_and_never_a_mismatch() {
        let mut authored = facts("the invariant always holds");
        authored.authored_method = Some("Smoke".to_owned());
        authored.uncatalogued_method = Some(true);
        let advice = advise(&catalog(), &authored);
        assert!(advice.uncatalogued);
        assert!(!advice.mismatch);

        authored.uncatalogued_method = Some(false);
        let advice = advise(&catalog(), &authored);
        assert!(!advice.uncatalogued);
        assert!(
            advice.mismatch,
            "a catalogued value the advisor disagrees with"
        );
    }

    #[test]
    fn an_authored_class_counts_as_agreement_case_insensitively() {
        let mut authored = facts("the invariant always holds");
        authored.authored_method = Some("analysis (TC-7)".to_owned());
        let advice = advise(&catalog(), &authored);
        assert_eq!(advice.authored.as_deref(), Some("analysis"));
        assert!(
            !advice.mismatch,
            "the recommended method's CLASS was authored"
        );
    }

    #[test]
    fn the_parameters_axis_reaches_the_characteristic_set() {
        let mut parameterised = facts("the thing works");
        parameterised.parameters = Some(BTreeMap::from([(
            "threshold".to_owned(),
            "< 5 min".to_owned(),
        )]));
        let mut with_threshold = catalog();
        with_threshold.methods.push(method(
            "benchmark",
            "Test",
            &[("characteristics", &["quantified-threshold"])],
        ));
        let advice = advise(&with_threshold, &parameterised);
        assert_eq!(advice.recommended.len(), 1);
        assert_eq!(advice.recommended[0].method, "benchmark");
    }

    #[test]
    fn an_authored_cell_is_sliced_at_its_first_parenthesis_and_trimmed() {
        assert_eq!(
            normalize_authored(Some("Test (TC-707)")).as_deref(),
            Some("Test")
        );
        assert_eq!(normalize_authored(Some("   ")), None);
        assert_eq!(normalize_authored(Some("(TC-707)")), None);
        assert_eq!(normalize_authored(None), None);
        assert_eq!(normalize_authored(Some("")), None);
    }
}
