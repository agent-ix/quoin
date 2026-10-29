// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The verification receipt (FR-065).
//!
//! A faithful port of `verifyReceipt` and the receipt half of
//! `verifyChangeAssurance` (`src/change-assurance/verify.ts`).
//!
//! A receipt is the only shape here that re-states its own conclusion: it
//! carries both a reason list and an outcome, and `validateOutcomeAndReasons`
//! refuses any receipt whose outcome disagrees with the precedence its reasons
//! imply, or whose reasons are unordered or repeated. That check is what makes
//! a receipt readable without re-running the verification, and it is
//! reproduced here exactly — including over each nested check and each proof.

use quoin_store::JsonValue;

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::ids::{
    AttestationId, EventId, NonEmptyText, ObligationId, ProofId, RecordId, Revision, RunId,
    StatementId,
};
use crate::model::decision::ReviewDecision;
use crate::model::json::{Fields, number, object, string_values};
use crate::model::outcome::{Check, Outcome, Reason, normalize_reasons, outcome_for_reasons};
use crate::model::record::Disposition;

/// The one `record_type` a verification receipt carries.
pub const RECORD_TYPE: &str = "verification_receipt";

/// The schema version every receipt in this family carries.
pub const SCHEMA_VERSION: u64 = 1;

const SUBJECT: Subject = Subject::Receipt;

/// The decision event a receipt cites.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptDecisionEvent {
    /// The ix-flow run.
    pub run_id: RunId,
    /// The event that carried the decision.
    pub event_id: EventId,
    /// That event's position in the run's history, from one.
    pub sequence: u64,
    /// The actor recorded against the decision. Attribution only.
    pub recorded_actor: NonEmptyText,
    /// What was decided.
    pub decision: ReviewDecision,
}

/// One audit finding carried through to the receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditFinding {
    /// The obligation the finding is about.
    pub obligation_id: ObligationId,
    /// The finding's kind, as the auditor spelled it.
    pub kind: NonEmptyText,
}

/// One proof obligation's verdict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofResult {
    /// The obligation this verdict is about.
    pub proof_id: ProofId,
    /// The evidence obligations it discharges.
    pub obligation_ids: Vec<ObligationId>,
    /// The attestation selected for it, when one was.
    pub attestation_id: Option<AttestationId>,
    /// The findings the auditor recorded against this obligation's evidence.
    pub audit_findings: Vec<AuditFinding>,
    /// The verdict.
    pub outcome: Outcome,
    /// The premises it refused.
    pub reasons: Vec<Reason>,
}

/// An unknown, carried into the receipt with its disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptUnknown {
    /// The unknown's identity.
    pub id: StatementId,
    /// What the review decided about it.
    pub disposition: Disposition,
}

/// The four named checks a receipt always carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptChecks {
    /// The record's own schema.
    pub record: Check,
    /// The parent chain.
    pub lineage: Check,
    /// The review decision.
    pub review: Check,
    /// The impact snapshot and the unknowns.
    pub impact: Check,
}

/// A verification receipt. It carries no id of its own: it is
/// re-derived from the retained inputs and compared byte for byte.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReceipt {
    /// The record verified, by identity.
    pub record_id: RecordId,
    /// The revision of that record.
    pub record_revision: Revision,
    /// The candidate revision it was verified at.
    pub candidate_revision: NonEmptyText,
    /// The decision event cited, when one was found.
    pub decision_event: Option<ReceiptDecisionEvent>,
    /// The four named checks.
    pub checks: ReceiptChecks,
    /// One verdict per proof obligation, in `proof_id` order.
    pub proofs: Vec<ProofResult>,
    /// The unknowns, in identity order.
    pub unknowns: Vec<ReceiptUnknown>,
    /// The overall verdict.
    pub outcome: Outcome,
    /// Every premise refused anywhere, deduplicated and ordered.
    pub reasons: Vec<Reason>,
    /// The `MeasurementPlan` id this receipt was checked against, exactly as
    /// [`crate::verify::input::GoverningPlan::id`] carried
    /// it in (PLAT-1015, FR-111).
    ///
    /// `None` records "no plan was linked" as a fact the receipt states,
    /// rather than something an auditor has to infer from `apparatus_touched`
    /// / `diff_missing` never firing — a caller that omits `--plan` produces
    /// that same silence.
    pub governing_plan_id: Option<NonEmptyText>,
    /// Whether the caller supplied `diff_paths` at all (PLAT-1015, FR-111),
    /// distinct from a diff that was supplied and touched nothing.
    ///
    /// `true` when [`crate::verify::input::VerificationInput::diff_paths`]
    /// was `Some(_)` (even `Some(vec![])`), `false` when it was `None`.
    /// Without this, a receipt with neither `apparatus_touched` nor
    /// `diff_missing` cannot be told apart from one whose diff was simply
    /// never given — the omission this ticket exists to make auditable.
    pub diff_paths_supplied: bool,
}

impl VerificationReceipt {
    /// The JSON form.
    ///
    /// # Errors
    ///
    /// Never in practice: only [`JsonValue::number`]'s non-finite refusal.
    pub fn to_json(&self) -> Result<JsonValue, ChangeAssuranceError> {
        write_receipt(self)
    }

    /// Read and validate a receipt's shape.
    ///
    /// # Errors
    ///
    /// Every refusal of the receipt schema, including an undeclared member
    /// such as a stored digest, a reason list that is unordered or repeated, and an
    /// outcome that disagrees with the precedence its own reasons imply.
    pub fn from_json(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        let root = Fields::read(value, SUBJECT, "verification receipt")?;
        root.exact(&[
            "schema_version",
            "record_type",
            "record_id",
            "record_revision",
            "candidate_revision",
            "decision_event",
            "checks",
            "proofs",
            "unknowns",
            "outcome",
            "reasons",
            "governing_plan_id",
            "diff_paths_supplied",
        ])?;
        root.equals("schema_version", &number(SCHEMA_VERSION)?)?;
        root.equals("record_type", &JsonValue::string(RECORD_TYPE))?;
        let record_id = RecordId::parse(root.text("record_id")?, SUBJECT, "record_id")?;
        let record_revision = Revision::new(
            root.whole_number("record_revision")?,
            SUBJECT,
            "record_revision",
        )?;
        let candidate_revision = NonEmptyText::parse(
            root.text("candidate_revision")?,
            SUBJECT,
            "candidate revision",
        )?;
        let decision_event = read_decision_event(root)?;
        let checks = read_checks(root)?;
        let proofs = read_proofs(root)?;
        let unknowns = read_unknowns(root)?;
        let (outcome, reasons) = read_outcome_and_reasons(root)?;
        let governing_plan_id = match root.value("governing_plan_id")? {
            JsonValue::Null => None,
            _ => Some(NonEmptyText::parse(
                root.text("governing_plan_id")?,
                SUBJECT,
                "governing plan id",
            )?),
        };
        let diff_paths_supplied = root.boolean("diff_paths_supplied")?;

        Ok(Self {
            record_id,
            record_revision,
            candidate_revision,
            decision_event,
            checks,
            proofs,
            unknowns,
            outcome,
            reasons,
            governing_plan_id,
            diff_paths_supplied,
        })
    }
}

fn read_decision_event(
    root: Fields<'_>,
) -> Result<Option<ReceiptDecisionEvent>, ChangeAssuranceError> {
    if matches!(root.value("decision_event")?, JsonValue::Null) {
        return Ok(None);
    }
    let event = root.nested("decision_event")?;
    event.exact(&[
        "run_id",
        "event_id",
        "sequence",
        "recorded_actor",
        "decision",
    ])?;
    Ok(Some(ReceiptDecisionEvent {
        run_id: RunId::parse(event.text("run_id")?, SUBJECT, "decision run_id")?,
        event_id: EventId::parse(event.text("event_id")?, SUBJECT, "decision event_id")?,
        sequence: match event.whole_number("sequence")? {
            0 => {
                return Err(ChangeAssuranceError::shape(
                    SUBJECT,
                    FieldFailure::malformed("decision sequence"),
                ));
            }
            sequence => sequence,
        },
        recorded_actor: NonEmptyText::parse(
            event.text("recorded_actor")?,
            SUBJECT,
            "decision recorded_actor",
        )?,
        decision: ReviewDecision::parse(event.text("decision")?).ok_or_else(|| {
            ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("decision"))
        })?,
    }))
}

fn read_checks(root: Fields<'_>) -> Result<ReceiptChecks, ChangeAssuranceError> {
    let checks = root.nested("checks")?;
    checks.exact(&["record", "lineage", "review", "impact"])?;
    let one = |name: &'static str| -> Result<Check, ChangeAssuranceError> {
        let fields = checks.nested(name)?;
        fields.exact(&["outcome", "reasons"])?;
        let (outcome, reasons) = read_outcome_and_reasons(fields)?;
        Ok(Check { outcome, reasons })
    };
    Ok(ReceiptChecks {
        record: one("record")?,
        lineage: one("lineage")?,
        review: one("review")?,
        impact: one("impact")?,
    })
}

fn read_proofs(root: Fields<'_>) -> Result<Vec<ProofResult>, ChangeAssuranceError> {
    let mut proofs = Vec::new();
    for entry in root.array("proofs", false)? {
        let fields = Fields::read(entry, SUBJECT, "proofs")?;
        fields.exact(&[
            "proof_id",
            "obligation_ids",
            "attestation_id",
            "audit_findings",
            "outcome",
            "reasons",
        ])?;
        let proof_id = ProofId::parse(fields.text("proof_id")?, SUBJECT, "proof_id")?;
        let mut obligation_ids = Vec::new();
        for obligation in fields.array("obligation_ids", true)? {
            let text = obligation.as_str().ok_or_else(|| {
                ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("obligation_id"))
            })?;
            obligation_ids.push(ObligationId::parse(text, SUBJECT, "obligation_id")?);
        }
        let attestation_id = match fields.value("attestation_id")? {
            JsonValue::Null => None,
            _ => Some(AttestationId::parse(
                fields.text("attestation_id")?,
                SUBJECT,
                "attestation_id",
            )?),
        };
        let mut audit_findings = Vec::new();
        for raw in fields.array("audit_findings", false)? {
            let finding = Fields::read(raw, SUBJECT, "audit finding")?;
            finding.exact(&["obligation_id", "kind"])?;
            audit_findings.push(AuditFinding {
                obligation_id: ObligationId::parse(
                    finding.text("obligation_id")?,
                    SUBJECT,
                    "audit obligation_id",
                )?,
                kind: NonEmptyText::parse(finding.text("kind")?, SUBJECT, "audit finding kind")?,
            });
        }
        let (outcome, reasons) = read_outcome_and_reasons(fields)?;
        proofs.push(ProofResult {
            proof_id,
            obligation_ids,
            attestation_id,
            audit_findings,
            outcome,
            reasons,
        });
    }
    Ok(proofs)
}

fn read_unknowns(root: Fields<'_>) -> Result<Vec<ReceiptUnknown>, ChangeAssuranceError> {
    let mut unknowns = Vec::new();
    for raw in root.array("unknowns", false)? {
        let fields = Fields::read(raw, SUBJECT, "receipt unknown")?;
        fields.exact(&["id", "disposition"])?;
        unknowns.push(ReceiptUnknown {
            id: StatementId::parse(fields.text("id")?, SUBJECT, "unknown id")?,
            disposition: Disposition::parse(fields.text("disposition")?).ok_or_else(|| {
                ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("unknown disposition"))
            })?,
        });
    }
    Ok(unknowns)
}

/// Read an `outcome`/`reasons` pair and refuse one that disagrees with itself.
///
/// The port of `validateOutcomeAndReasons`: every reason must be in the closed
/// vocabulary, the list must already be deduplicated and ordered, and the
/// outcome must be exactly the one the reasons imply.
fn read_outcome_and_reasons(
    fields: Fields<'_>,
) -> Result<(Outcome, Vec<Reason>), ChangeAssuranceError> {
    let outcome = Outcome::parse(fields.text("outcome")?)
        .ok_or_else(|| ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("outcome")))?;
    let mut reasons = Vec::new();
    for raw in fields.array("reasons", false)? {
        let text = raw.as_str().ok_or_else(|| {
            ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("reason"))
        })?;
        reasons.push(Reason::parse(text).ok_or_else(|| {
            ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("reason"))
        })?);
    }
    if normalize_reasons(&reasons) != reasons {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("reasons must be unique and lexicographically ordered"),
        ));
    }
    if outcome != outcome_for_reasons(&reasons) {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("outcome disagrees with reason precedence"),
        ));
    }
    Ok((outcome, reasons))
}

fn reasons_json(reasons: &[Reason]) -> JsonValue {
    JsonValue::Array(
        reasons
            .iter()
            .map(|reason| JsonValue::string(reason.as_str()))
            .collect(),
    )
}

fn check_json(check: &Check) -> JsonValue {
    object(vec![
        ("outcome", JsonValue::string(check.outcome.as_str())),
        ("reasons", reasons_json(&check.reasons)),
    ])
}

fn write_receipt(receipt: &VerificationReceipt) -> Result<JsonValue, ChangeAssuranceError> {
    let members = vec![
        ("schema_version", number(SCHEMA_VERSION)?),
        ("record_type", JsonValue::string(RECORD_TYPE)),
        ("record_id", JsonValue::string(receipt.record_id.as_str())),
        ("record_revision", number(receipt.record_revision.get())?),
        (
            "candidate_revision",
            JsonValue::string(receipt.candidate_revision.as_str()),
        ),
        ("decision_event", decision_event_json(receipt)?),
        (
            "checks",
            object(vec![
                ("record", check_json(&receipt.checks.record)),
                ("lineage", check_json(&receipt.checks.lineage)),
                ("review", check_json(&receipt.checks.review)),
                ("impact", check_json(&receipt.checks.impact)),
            ]),
        ),
        ("proofs", write_proofs(&receipt.proofs)),
        (
            "unknowns",
            JsonValue::Array(
                receipt
                    .unknowns
                    .iter()
                    .map(|unknown| {
                        object(vec![
                            ("id", JsonValue::string(unknown.id.as_str())),
                            (
                                "disposition",
                                JsonValue::string(unknown.disposition.as_str()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("outcome", JsonValue::string(receipt.outcome.as_str())),
        ("reasons", reasons_json(&receipt.reasons)),
        (
            "governing_plan_id",
            receipt
                .governing_plan_id
                .as_ref()
                .map_or(JsonValue::Null, |id| JsonValue::string(id.as_str())),
        ),
        (
            "diff_paths_supplied",
            JsonValue::Bool(receipt.diff_paths_supplied),
        ),
    ];
    Ok(object(members))
}

fn write_proofs(proofs: &[ProofResult]) -> JsonValue {
    JsonValue::Array(
        proofs
            .iter()
            .map(|proof| {
                let obligation_ids: Vec<String> = proof
                    .obligation_ids
                    .iter()
                    .map(|id| id.as_str().to_owned())
                    .collect();
                object(vec![
                    ("proof_id", JsonValue::string(proof.proof_id.as_str())),
                    ("obligation_ids", string_values(&obligation_ids)),
                    (
                        "attestation_id",
                        proof
                            .attestation_id
                            .as_ref()
                            .map_or(JsonValue::Null, |id| JsonValue::string(id.as_str())),
                    ),
                    (
                        "audit_findings",
                        JsonValue::Array(
                            proof
                                .audit_findings
                                .iter()
                                .map(|finding| {
                                    object(vec![
                                        (
                                            "obligation_id",
                                            JsonValue::string(finding.obligation_id.as_str()),
                                        ),
                                        ("kind", JsonValue::string(finding.kind.as_str())),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    ("outcome", JsonValue::string(proof.outcome.as_str())),
                    ("reasons", reasons_json(&proof.reasons)),
                ])
            })
            .collect(),
    )
}

fn decision_event_json(receipt: &VerificationReceipt) -> Result<JsonValue, ChangeAssuranceError> {
    let Some(event) = receipt.decision_event.as_ref() else {
        return Ok(JsonValue::Null);
    };
    Ok(object(vec![
        ("run_id", JsonValue::string(event.run_id.as_str())),
        ("event_id", JsonValue::string(event.event_id.as_str())),
        ("sequence", number(event.sequence)?),
        (
            "recorded_actor",
            JsonValue::string(event.recorded_actor.as_str()),
        ),
        ("decision", JsonValue::string(event.decision.as_str())),
    ]))
}
