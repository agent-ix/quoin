// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Assembling the discharge report: `build_discharge_report` (FR-046).
//!
//! Walks the clause binding, spends each fact against the clause it names,
//! and reports the ones nothing asked for. The shape it emits is in
//! [`super::report`]; the readers it admits facts through are in
//! [`super::parse`].

use std::collections::BTreeMap;

use quoin_quire_types::{ClauseBinding, ClauseBindingOutcome};

use super::parse::{current_attestation, instant, parse_fact};
use super::report::{
    BuildDischargeRequest, Checked, ClauseDischarge, DischargeBinding, DischargeFact,
    DischargeReport, DischargeSchemaVersion, DischargeState, UnusedDischargeFact, UnusedFactReason,
    reject,
};

/// Build a complete, non-scored discharge partition.
///
/// # Errors
///
/// [`crate::DischargeError`] when `asOf` is not an instant, when any supplied fact
/// fails `parseFact`/`parseAttestation`, or when two facts name the same
/// clause. FR-046-AC-5: a duplicate is rejected, never resolved by ordering.
#[expect(
    clippy::too_many_lines,
    reason = "the retained buildDischargeReport is one function, and splitting \
              it would put the partition's reading order somewhere other than \
              the order the oracle decides in, which is what a reader compares"
)]
pub fn build_discharge_report(request: &BuildDischargeRequest) -> Checked<DischargeReport> {
    let as_of = instant("asOf", &request.as_of)?;

    // Every fact is parsed BEFORE the duplicate check, because the retained
    // code is `request.facts.map(parseFact)` followed by the loop that fills
    // the map. A malformed second fact therefore beats a duplicate first one,
    // and reversing that would change which message a caller sees.
    let mut parsed_facts = Vec::with_capacity(request.facts.len());
    for value in &request.facts {
        parsed_facts.push(parse_fact(value)?);
    }
    let mut facts: BTreeMap<&str, &DischargeFact> = BTreeMap::new();
    for fact in &parsed_facts {
        if facts.contains_key(fact.clause_id()) {
            return reject(format!(
                "duplicate discharge fact for clause {}",
                fact.clause_id()
            ));
        }
        facts.insert(fact.clause_id(), fact);
    }

    let mut direct = Vec::new();
    let mut dispositions = Vec::new();
    let mut open = Vec::new();
    let mut unresolved = Vec::new();
    let mut not_binding = Vec::new();
    let mut unused_facts = Vec::new();
    let mut known: Vec<&str> = Vec::with_capacity(request.binding.clauses.len());

    for clause in &request.binding.clauses {
        known.push(clause.clause_id.as_str());
        let fact = facts.get(clause.clause_id.as_str()).copied();
        match clause.outcome {
            ClauseBindingOutcome::Unresolved => {
                unresolved.push(entry(
                    clause,
                    DischargeState::Unresolved,
                    Some(reason_for(clause)),
                    None,
                ));
                if let Some(fact) = fact {
                    unused_facts.push(UnusedDischargeFact {
                        clause_id: clause.clause_id.clone(),
                        kind: fact.kind(),
                        reason: UnusedFactReason::Unresolved,
                    });
                }
            }
            ClauseBindingOutcome::NotBinding => {
                not_binding.push(entry(clause, DischargeState::NotBinding, None, None));
                if let Some(fact) = fact {
                    unused_facts.push(UnusedDischargeFact {
                        clause_id: clause.clause_id.clone(),
                        kind: fact.kind(),
                        reason: UnusedFactReason::NotBinding,
                    });
                }
            }
            ClauseBindingOutcome::Binding => {
                let Some(fact) = fact else {
                    open.push(entry(
                        clause,
                        DischargeState::Open,
                        Some("no discharge fact".to_owned()),
                        None,
                    ));
                    continue;
                };
                if let Some(current) = current_attestation(fact.attestation(), as_of)? {
                    open.push(entry(
                        clause,
                        DischargeState::Open,
                        Some(current),
                        Some(fact),
                    ));
                    continue;
                }
                match fact {
                    DischargeFact::Direct(_) => {
                        direct.push(entry(clause, DischargeState::Direct, None, Some(fact)));
                    }
                    DischargeFact::Disposition(_) => {
                        dispositions.push(entry(
                            clause,
                            DischargeState::Disposition,
                            None,
                            Some(fact),
                        ));
                    }
                }
            }
        }
    }

    for fact in &parsed_facts {
        if !known.contains(&fact.clause_id()) {
            unused_facts.push(UnusedDischargeFact {
                clause_id: fact.clause_id().to_owned(),
                kind: fact.kind(),
                reason: UnusedFactReason::UnknownClause,
            });
        }
    }

    Ok(DischargeReport {
        schema_version: DischargeSchemaVersion::V1,
        clause_set: request.binding.clause_set.clone(),
        clause_set_digest: request.binding.clause_set_digest.clone(),
        context: request.binding.context.clone(),
        as_of: request.as_of.clone(),
        binding: DischargeBinding {
            direct,
            dispositions,
            open,
        },
        unresolved,
        not_binding,
        unused_facts,
    })
}

/// `entry(clause, state, reason?, fact?)`.
fn entry(
    clause: &ClauseBinding,
    state: DischargeState,
    reason: Option<String>,
    fact: Option<&DischargeFact>,
) -> ClauseDischarge {
    ClauseDischarge {
        clause_id: clause.clause_id.clone(),
        force: clause.force,
        state,
        expected_outputs: clause.expected_outputs.clone(),
        // `...(reason ? { reason } : {})` is a TRUTHINESS test, so a reason
        // that renders as the empty string omits the key rather than emitting
        // `"reason": ""`. `reasonFor` produces exactly that when every reason
        // the binder gave carries an empty message.
        reason: reason.filter(|reason| !reason.is_empty()),
        fact: fact.cloned(),
    }
}

/// `reasonFor(clause)`.
fn reason_for(clause: &ClauseBinding) -> String {
    if clause.reasons.is_empty() {
        "applicability is unresolved".to_owned()
    } else {
        clause
            .reasons
            .iter()
            .map(|reason| reason.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::super::fixtures::{binding, direct, request};
    use super::build_discharge_report;

    /// The parse loop runs to completion before the duplicate check, so a
    /// malformed later fact beats a duplicate earlier one.
    /// Trace: FR-046-AC-1, FR-046-AC-5
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_433_a_malformed_fact_is_refused_before_a_duplicate_is_noticed() {
        let mut malformed = direct();
        malformed["kind"] = serde_json::json!("invented");
        let error = build_discharge_report(&request(vec![direct(), direct(), malformed]))
            .expect_err("the third fact is malformed");
        assert_eq!(error.0, "kind must be one of direct, disposition");
    }

    /// `parseFact` reads the attestation BEFORE `exact`, so the attestation
    /// message wins when a fact is wrong in both ways.
    /// Trace: FR-046-AC-5
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_434_the_attestation_is_read_before_the_unknown_field_check() {
        let mut fact = direct();
        fact["inventedScore"] = serde_json::json!(100);
        fact["attestation"]["authority"] = serde_json::json!("");
        let error = build_discharge_report(&request(vec![fact])).expect_err("both predicates fail");
        assert_eq!(error.0, "authority must not be empty");
    }

    /// `asOf` is echoed verbatim; only the comparison uses the parsed number.
    /// Trace: FR-046-AC-4, FR-046-AC-6
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_435_as_of_carries_the_original_request_string() {
        let mut input = request(vec![direct()]);
        input.as_of = "2026-08-15T05:30:00.000+05:30".to_owned();
        let report = build_discharge_report(&input).expect("an offset instant is valid");
        assert_eq!(report.as_of, "2026-08-15T05:30:00.000+05:30");
        // And it was compared as the UTC instant it names, which is inside the
        // attestation window, so the clause discharged rather than reopening.
        assert_eq!(report.binding.direct.len(), 1);
    }

    /// The retained `entry()` spreads on TRUTHINESS, so an all-empty reason
    /// set omits the key rather than emitting `"reason": ""`.
    /// Trace: FR-046-AC-3, FR-046-AC-6
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_436_an_empty_joined_reason_omits_the_key_entirely() {
        let mut input = request(vec![]);
        input.binding.clauses[0].outcome = quoin_quire_types::ClauseBindingOutcome::Unresolved;
        input.binding.clauses[0].reasons = vec![quoin_quire_types::ClauseBindingReason {
            code: "unnamed".to_owned(),
            dimension: None,
            message: String::new(),
        }];
        let report = build_discharge_report(&input).expect("an empty message is not an error");
        let value = serde_json::to_value(&report).expect("it serialises");
        assert!(
            value["unresolved"][0]
                .as_object()
                .unwrap()
                .get("reason")
                .is_none(),
            "an empty reason must not appear as a key"
        );
    }

    /// `reason` and `fact` are absent keys, never nulls.
    /// Trace: FR-046-AC-2, FR-046-AC-6
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_437_a_discharged_entry_omits_reason_and_an_undischarged_one_omits_fact() {
        let report = build_discharge_report(&request(vec![direct()])).expect("valid");
        let value = serde_json::to_value(&report).expect("it serialises");
        let discharged = value["binding"]["direct"][0].as_object().unwrap();
        assert!(discharged.get("reason").is_none());
        assert!(discharged.get("fact").is_some());
        let not_binding = value["notBinding"][0].as_object().unwrap();
        assert!(not_binding.get("reason").is_none());
        assert!(not_binding.get("fact").is_none());
    }

    /// The internally tagged enum must put `kind` beside the payload, not
    /// around it.
    /// Trace: FR-046-AC-2, FR-046-AC-6
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_438_a_fact_serialises_with_its_discriminant_flat() {
        let report = build_discharge_report(&request(vec![direct()])).expect("valid");
        let value = serde_json::to_value(&report).expect("it serialises");
        assert_eq!(value["binding"]["direct"][0]["fact"], direct());
    }

    /// Clause-ordered entries first, then fact-ordered ones.
    /// Trace: FR-046-AC-3, FR-046-AC-6
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_439_unused_facts_are_clause_ordered_then_fact_ordered() {
        let mut unknown = direct();
        unknown["clauseId"] = serde_json::json!("SYN-999");
        let mut spent_on_not_binding = direct();
        spent_on_not_binding["clauseId"] = serde_json::json!("SYN-005");

        // The unknown-clause fact is supplied FIRST; it must still be listed
        // last, because the retained code walks the clauses before the facts.
        let report = build_discharge_report(&request(vec![unknown, spent_on_not_binding]))
            .expect("both facts are well formed");
        let listed: Vec<&str> = report
            .unused_facts
            .iter()
            .map(|fact| fact.clause_id.as_str())
            .collect();
        assert_eq!(listed, ["SYN-005", "SYN-999"]);
    }

    /// `context` is a copy, and an ordered one.
    /// Trace: FR-046-AC-6
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_440_context_is_copied_and_serialises_in_a_stable_order() {
        let report = build_discharge_report(&request(vec![])).expect("valid");
        assert_eq!(report.context, binding().context);
        let text = serde_json::to_string(&report.context).expect("it serialises");
        assert_eq!(text, r#"{"deployment":"test","product":"widget"}"#);
    }
}
