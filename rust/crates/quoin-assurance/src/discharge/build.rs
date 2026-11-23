// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Assembling the discharge report: `build_discharge_report` (FR-046).
//!
//! Walks the clause binding, spends each fact against the clause it names,
//! and reports the ones nothing asked for. The shape it emits is in
//! [`super::report`]; the readers it admits facts through are in
//! [`super::parse`].

use std::collections::{BTreeMap, BTreeSet};

use quoin_quire_types::{ClauseBinding, ClauseBindingOutcome};

use super::parse::{
    current_attestation, current_decision_expiry, instant, parse_fact_with_approval_requirements,
};
use super::report::{
    BuildDischargeRequest, Checked, ClauseDischarge, DischargeBinding, DischargeFact,
    DischargeReport, DischargeSchemaVersion, DischargeState, UnusedDischargeFact, UnusedFactReason,
    reject,
};

/// Build a complete, non-scored discharge partition by attestation currency.
///
/// Nothing is resolved or verified here: a clause this partition calls direct
/// is discharged only once [`super::check_report`] has resolved its evidence
/// and verified its approval (FR-211).
///
/// # Errors
///
/// [`crate::DischargeError`] when `asOf` is not an instant, when any supplied fact
/// fails `parseFact`/`parseAttestation`, or when two facts name the same
/// clause. FR-046-AC-5: a duplicate is rejected, never resolved by ordering.
pub fn build_discharge_report(request: &BuildDischargeRequest) -> Checked<DischargeReport> {
    build_discharge_report_with_approval_requirements(request, &BTreeSet::new())
}

/// Build the partition, admitting a blank `approvalRef` on the clause ids
/// whose set declares approval roles so that FR-211 can leave them open.
///
/// The caller resolves the exact clause-set identity before calling this
/// function; the binding report does not carry that metadata.
///
/// # Errors
/// As [`build_discharge_report`].
pub fn build_discharge_report_with_approval_requirements(
    request: &BuildDischargeRequest,
    required_approval_clauses: &BTreeSet<String>,
) -> Checked<DischargeReport> {
    let as_of = instant("asOf", &request.as_of)?;

    // Every fact is parsed BEFORE the duplicate check, because the retained
    // code is `request.facts.map(parseFact)` followed by the loop that fills
    // the map. A malformed second fact therefore beats a duplicate first one,
    // and reversing that would change which message a caller sees.
    let mut parsed_facts = Vec::with_capacity(request.facts.len());
    for value in &request.facts {
        parsed_facts.push(parse_fact_with_approval_requirements(
            value,
            required_approval_clauses,
        )?);
    }
    let facts = index_facts(&parsed_facts)?;
    let mut partition = classify_binding_clauses(&request.binding.clauses, &facts, as_of)?;
    append_unknown_facts(&mut partition, &parsed_facts);

    Ok(DischargeReport {
        schema_version: DischargeSchemaVersion::V1,
        clause_set: request.binding.clause_set.clone(),
        context: request.binding.context.clone(),
        as_of: request.as_of.clone(),
        binding: DischargeBinding {
            direct: partition.direct,
            dispositions: partition.dispositions,
            open: partition.open,
        },
        unresolved: partition.unresolved,
        not_binding: partition.not_binding,
        unused_facts: partition.unused_facts,
    })
}

#[derive(Default)]
struct DischargePartition {
    direct: Vec<ClauseDischarge>,
    dispositions: Vec<ClauseDischarge>,
    open: Vec<ClauseDischarge>,
    unresolved: Vec<ClauseDischarge>,
    not_binding: Vec<ClauseDischarge>,
    unused_facts: Vec<UnusedDischargeFact>,
    known: Vec<String>,
}

fn index_facts(facts: &[DischargeFact]) -> Checked<BTreeMap<&str, &DischargeFact>> {
    let mut indexed = BTreeMap::new();
    for fact in facts {
        if indexed.contains_key(fact.clause_id()) {
            return reject(format!(
                "duplicate discharge fact for clause {}",
                fact.clause_id()
            ));
        }
        indexed.insert(fact.clause_id(), fact);
    }
    Ok(indexed)
}

fn classify_binding_clauses(
    clauses: &[ClauseBinding],
    facts: &BTreeMap<&str, &DischargeFact>,
    as_of: i64,
) -> Checked<DischargePartition> {
    let mut seen = std::collections::BTreeSet::new();
    let mut partition = DischargePartition::default();
    for clause in clauses {
        if !seen.insert(clause.clause_id.as_str()) {
            return reject(format!(
                "duplicate clause {} in the binding report",
                clause.clause_id
            ));
        }
        partition.known.push(clause.clause_id.clone());
        classify_clause(
            clause,
            facts.get(clause.clause_id.as_str()).copied(),
            as_of,
            &mut partition,
        )?;
    }
    Ok(partition)
}

fn classify_clause(
    clause: &ClauseBinding,
    fact: Option<&DischargeFact>,
    as_of: i64,
    partition: &mut DischargePartition,
) -> Checked<()> {
    match clause.outcome {
        ClauseBindingOutcome::Unresolved => {
            partition.unresolved.push(entry(
                clause,
                DischargeState::Unresolved,
                Some(reason_for(clause)),
                None,
            ));
            if let Some(fact) = fact {
                partition.unused_facts.push(UnusedDischargeFact {
                    clause_id: clause.clause_id.clone(),
                    kind: fact.kind(),
                    reason: UnusedFactReason::Unresolved,
                });
            }
        }
        ClauseBindingOutcome::NotBinding => {
            partition
                .not_binding
                .push(entry(clause, DischargeState::NotBinding, None, None));
            if let Some(fact) = fact {
                partition.unused_facts.push(UnusedDischargeFact {
                    clause_id: clause.clause_id.clone(),
                    kind: fact.kind(),
                    reason: UnusedFactReason::NotBinding,
                });
            }
        }
        ClauseBindingOutcome::Binding => {
            classify_binding_clause(clause, fact, as_of, partition)?;
        }
    }
    Ok(())
}

fn classify_binding_clause(
    clause: &ClauseBinding,
    fact: Option<&DischargeFact>,
    as_of: i64,
    partition: &mut DischargePartition,
) -> Checked<()> {
    let Some(fact) = fact else {
        partition.open.push(entry(
            clause,
            DischargeState::Open,
            Some("no discharge fact".to_owned()),
            None,
        ));
        return Ok(());
    };
    // A temporary exception's decision expiry is the more specific state
    // when it coincides with (or precedes) attestation expiry.
    if let Some(expired) = current_decision_expiry(fact, as_of)? {
        partition.open.push(entry(
            clause,
            DischargeState::Open,
            Some(expired),
            Some(fact),
        ));
        return Ok(());
    }
    if let Some(current) = current_attestation(fact.attestation(), as_of)? {
        partition.open.push(entry(
            clause,
            DischargeState::Open,
            Some(current),
            Some(fact),
        ));
        return Ok(());
    }
    match fact {
        DischargeFact::Direct(_) => {
            partition
                .direct
                .push(entry(clause, DischargeState::Direct, None, Some(fact)));
        }
        DischargeFact::Disposition(_) => partition.dispositions.push(entry(
            clause,
            DischargeState::Disposition,
            None,
            Some(fact),
        )),
    }
    Ok(())
}

fn append_unknown_facts(partition: &mut DischargePartition, facts: &[DischargeFact]) {
    for fact in facts {
        if !partition
            .known
            .iter()
            .any(|clause_id| clause_id == fact.clause_id())
        {
            partition.unused_facts.push(UnusedDischargeFact {
                clause_id: fact.clause_id().to_owned(),
                kind: fact.kind(),
                reason: UnusedFactReason::UnknownClause,
            });
        }
    }
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
        verified_approver: None,
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
    use ix_trace_rs::trace;

    /// The parse loop runs to completion before the duplicate check, so a
    /// malformed later fact beats a duplicate earlier one.
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-1", "FR-046-AC-5")]
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
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-5")]
    #[test]
    fn tc_447_434_the_attestation_is_read_before_the_unknown_field_check() {
        let mut fact = direct();
        fact["inventedScore"] = serde_json::json!(100);
        fact["attestation"]["authority"] = serde_json::json!("");
        let error = build_discharge_report(&request(vec![fact])).expect_err("both predicates fail");
        assert_eq!(error.0, "authority must not be empty");
    }

    /// `asOf` is echoed verbatim; only the comparison uses the parsed number.
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-4", "FR-046-AC-6")]
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
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-3", "FR-046-AC-6")]
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
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-2", "FR-046-AC-6")]
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
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-2", "FR-046-AC-6")]
    #[test]
    fn tc_447_438_a_fact_serialises_with_its_discriminant_flat() {
        let report = build_discharge_report(&request(vec![direct()])).expect("valid");
        let value = serde_json::to_value(&report).expect("it serialises");
        assert_eq!(value["binding"]["direct"][0]["fact"], direct());
    }

    /// Clause-ordered entries first, then fact-ordered ones.
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-3", "FR-046-AC-6")]
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
    /// Provenance: agent-ix/quoin#447
    #[trace("FR-046-AC-6")]
    #[test]
    fn tc_447_440_context_is_copied_and_serialises_in_a_stable_order() {
        let report = build_discharge_report(&request(vec![])).expect("valid");
        assert_eq!(report.context, binding().context);
        let text = serde_json::to_string(&report.context).expect("it serialises");
        assert_eq!(text, r#"{"deployment":"test","product":"widget"}"#);
    }
}
