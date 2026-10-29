// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Verifying retained change-assurance evidence (FR-065).
//!
//! # A verification always produces a receipt
//!
//! Nothing here refuses to answer. A record that does not parse produces a
//! `schema_invalid` receipt rather than an error, because "this evidence
//! cannot be read" is itself the verification result and it has to be
//! retainable and citable like any other. The receipt names the record by the
//! `(record_id, revision)` the caller asked about.
//!
//! # Receipts are re-derived
//!
//! A receipt carries no digest. [`verify_receipt`] re-derives the receipt from
//! the retained inputs and compares the canonical bytes of the two, byte for
//! byte.

pub mod apparatus;
pub mod audit;
pub mod input;
pub mod lineage;
pub mod proofs;

use quoin_store::{JsonValue, canonical_bytes};

use crate::error::ChangeAssuranceError;
use crate::model::json::compare_utf16;
use crate::model::outcome::{Check, Outcome, Reason, normalize_reasons, outcome_for_reasons};
use crate::model::receipt::{ReceiptChecks, ReceiptUnknown, VerificationReceipt};
use crate::model::record::{ChangeAssuranceRecord, Completeness, Disposition};
use crate::records::{read_change_record_json, validate_decision, verify_lineage};
use crate::verify::apparatus::ApparatusContext;
use crate::verify::input::VerificationInput;
use crate::verify::lineage::lineage_reason;
use crate::verify::proofs::{ProofContext, judge, ordered_obligations, retained_under};

pub use crate::verify::audit::{AdaptedAudit, AuditState, adapt_audit, map_audit_finding};
pub use crate::verify::input::{
    AuditReport, GoverningPlan, ReportFinding, ReportUnevaluated, RetainedAttestation,
    RetainedAudit, Selection, VerificationInput as Input,
};

/// Verify retained evidence and produce a receipt.
///
/// # Errors
///
/// Only when the receipt fails its own schema, which means this crate
/// disagrees with itself rather than that the evidence is bad.
pub fn verify_change_assurance(
    input: &VerificationInput,
) -> Result<VerificationReceipt, ChangeAssuranceError> {
    let mut record_reasons: Vec<Reason> = Vec::new();
    let Ok(record) = read_change_record_json(&input.record) else {
        return schema_invalid_receipt(input);
    };
    if record.record_id != input.record_id || record.revision != input.record_revision {
        record_reasons.push(Reason::RecordRevisionMismatch);
    }
    let record_check = Check::from_reasons(&record_reasons, Outcome::Invalid);

    let mut lineage_reasons: Vec<Reason> = Vec::new();
    if let Err(error) = verify_lineage(&record, &input.parents) {
        lineage_reasons.push(lineage_reason(&error));
    }
    let lineage_check = Check::from_reasons(&lineage_reasons, Outcome::Invalid);

    let decision = validate_decision(&record, input.decision_history.as_ref());
    let review_check = Check {
        outcome: decision.outcome,
        reasons: decision.reasons.clone(),
    };

    let impact_check = impact_check(&record);

    let mut global_reasons: Vec<Reason> = Vec::new();
    if input.selections.iter().any(|selection| {
        !record
            .definition
            .proof_obligations
            .iter()
            .any(|proof| proof.proof_id.as_str() == selection.proof_id)
    }) {
        global_reasons.push(Reason::ProofIdMismatch);
    }

    let proofs = judge_proofs(input, &record);

    let plan = input.governing_plan.as_ref();
    global_reasons.extend(apparatus::judge(&ApparatusContext {
        protected_apparatus: plan.and_then(|plan| plan.protected_apparatus.as_ref()),
        negative_controls: plan.and_then(|plan| plan.negative_controls.as_ref()),
        diff_paths: input.diff_paths.as_deref(),
    }));

    let mut all: Vec<Reason> = global_reasons;
    all.extend(record_check.reasons.iter().copied());
    all.extend(lineage_check.reasons.iter().copied());
    all.extend(review_check.reasons.iter().copied());
    all.extend(impact_check.reasons.iter().copied());
    for proof in &proofs {
        all.extend(proof.reasons.iter().copied());
    }
    let reasons = normalize_reasons(&all);

    let mut unknowns: Vec<ReceiptUnknown> = record
        .definition
        .unknowns
        .iter()
        .map(|unknown| ReceiptUnknown {
            id: unknown.id.clone(),
            disposition: unknown.disposition,
        })
        .collect();
    unknowns.sort_by(|left, right| compare_utf16(left.id.as_str(), right.id.as_str()));

    checked(&VerificationReceipt {
        record_id: input.record_id.clone(),
        record_revision: input.record_revision,
        candidate_revision: crate::ids::NonEmptyText::parse(
            &input.candidate_revision,
            crate::error::Subject::Receipt,
            "candidate revision",
        )?,
        decision_event: decision.event,
        checks: ReceiptChecks {
            record: record_check,
            lineage: lineage_check,
            review: review_check,
            impact: impact_check,
        },
        proofs,
        unknowns,
        outcome: outcome_for_reasons(&reasons),
        reasons,
        governing_plan_id: governing_plan_id(input)?,
        diff_paths_supplied: input.diff_paths.is_some(),
    })
}

/// The plan id this receipt was checked against, parsed from
/// [`GoverningPlan::id`] (PLAT-1015, FR-111).
///
/// # Errors
///
/// [`ChangeAssuranceError::Shape`] when the caller's plan id is empty — it
/// already passed [`crate::ids::Identity::parse`] at the wire boundary that
/// resolved [`VerificationInput::governing_plan`], so this is not expected to
/// fail in practice, the same way `candidate_revision`'s re-parse above is
/// not.
fn governing_plan_id(
    input: &VerificationInput,
) -> Result<Option<crate::ids::NonEmptyText>, ChangeAssuranceError> {
    input
        .governing_plan
        .as_ref()
        .map(|plan| plan.id.as_str())
        .map(|id| {
            crate::ids::NonEmptyText::parse(id, crate::error::Subject::Receipt, "governing plan id")
        })
        .transpose()
}

/// What the impact snapshot and the unknowns say about completeness.
///
/// All three are *incomplete* rather than invalid: an analysis that admits it
/// did not see everything, and a review that admits something is unsettled,
/// are honest evidence of an unfinished job rather than bad evidence.
fn impact_check(record: &ChangeAssuranceRecord) -> Check {
    let mut reasons: Vec<Reason> = Vec::new();
    if record.impact_snapshot.completeness == Completeness::Incomplete {
        reasons.push(Reason::ImpactIncomplete);
    }
    if record.impact_snapshot.truncated {
        reasons.push(Reason::ImpactTruncated);
    }
    if record
        .definition
        .unknowns
        .iter()
        .any(|unknown| unknown.disposition != Disposition::Resolved)
    {
        reasons.push(Reason::UnresolvedUnknown);
    }
    Check::from_reasons(&reasons, Outcome::Incomplete)
}

/// One verdict per proof obligation, in `proof_id` order.
fn judge_proofs(
    input: &VerificationInput,
    record: &ChangeAssuranceRecord,
) -> Vec<crate::model::receipt::ProofResult> {
    ordered_obligations(&record.definition.proof_obligations)
        .into_iter()
        .map(|obligation| {
            let selected: Vec<_> = input
                .selections
                .iter()
                .filter(|selection| selection.proof_id == obligation.proof_id.as_str())
                .collect();
            // The retained attestations are only looked up when exactly one
            // selection names this obligation: with two selections there is no
            // single id to look up, and the ambiguity is already refused.
            let retained: Vec<_> = match selected.as_slice() {
                [only] => retained_under(&input.attestations, &only.attestation_id),
                _ => Vec::new(),
            };
            let audits: Vec<_> = input
                .audits
                .iter()
                .filter(|audit| audit.proof_id == obligation.proof_id.as_str())
                .collect();
            judge(&ProofContext {
                obligation,
                record_id: &input.record_id,
                record_revision: input.record_revision,
                candidate_revision: &input.candidate_revision,
                selected,
                retained,
                audits,
            })
        })
        .collect()
}

/// The receipt a record that cannot be read produces.
///
/// It still names the evidence: the `(record_id, revision)` the caller asked
/// about.
///
/// # Errors
///
/// As [`verify_change_assurance`].
pub fn schema_invalid_receipt(
    input: &VerificationInput,
) -> Result<VerificationReceipt, ChangeAssuranceError> {
    let candidate_revision = crate::ids::NonEmptyText::parse(
        if input.candidate_revision.is_empty() {
            "invalid"
        } else {
            &input.candidate_revision
        },
        crate::error::Subject::Receipt,
        "candidate revision",
    )?;
    checked(&VerificationReceipt {
        record_id: input.record_id.clone(),
        record_revision: input.record_revision,
        candidate_revision,
        decision_event: None,
        checks: ReceiptChecks {
            record: Check {
                outcome: Outcome::Invalid,
                reasons: vec![Reason::SchemaInvalid],
            },
            lineage: Check::valid(),
            review: Check::valid(),
            impact: Check::valid(),
        },
        proofs: Vec::new(),
        unknowns: Vec::new(),
        outcome: Outcome::Invalid,
        reasons: vec![Reason::SchemaInvalid],
        governing_plan_id: governing_plan_id(input)?,
        diff_paths_supplied: input.diff_paths.is_some(),
    })
}

/// Re-read a receipt through its own schema, so nothing leaves here
/// unvalidated.
fn checked(receipt: &VerificationReceipt) -> Result<VerificationReceipt, ChangeAssuranceError> {
    VerificationReceipt::from_json(&receipt.to_json()?)
}

/// Check a stored receipt against the receipt re-derived from the retained
/// inputs, byte for byte.
///
/// # Errors
///
/// Every refusal of the receipt schema, and
/// [`ChangeAssuranceError::ReceiptMismatch`] when the canonical bytes of the
/// stored receipt are not those of the re-derived one.
pub fn verify_receipt(
    stored: &JsonValue,
    input: &VerificationInput,
) -> Result<VerificationReceipt, ChangeAssuranceError> {
    let receipt = VerificationReceipt::from_json(stored)?;
    let rederived = verify_change_assurance(input)?;
    if canonical_bytes(stored)? == canonical_bytes(&rederived.to_json()?)? {
        Ok(receipt)
    } else {
        Err(ChangeAssuranceError::ReceiptMismatch)
    }
}
