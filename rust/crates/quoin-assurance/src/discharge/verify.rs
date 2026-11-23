// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Evidence resolution and approval verification for a discharge report
//! (FR-211).
//!
//! The partition in [`super::build`] decides by currency alone. This module
//! decides whether a clause that partition discharged still holds when its
//! evidence references are resolved against the retained store and its
//! approval is verified by engineering-assurance. A clause that fails is moved
//! to the open population with a reason that names the failing reference.
//!
//! Quoin verifies no signature here. The only trust it places is in the
//! [`ApprovalVerdict`] an [`ApprovalVerifier`] returns, and it never decides
//! whether a person's acceptance of risk was reasonable.

use std::collections::HashMap;

use quoin_evidence::source::EvidenceSource;
use quoin_quire_types::{ClauseBinding, ClausePolicy, ClauseSetKey, SetPolicy};

use super::report::{
    ClauseDischarge, DirectDischargeFact, DischargeFact, DischargeReport, DischargeState,
    DispositionDecision, DispositionFact,
};
use crate::argument::js_trim_is_empty;

/// The longest `record_id` engineering-assurance accepts, in bytes.
pub const MAX_RECORD_ID_BYTES: usize = 256;

/// The approval member of the `approval-verify` result Quoin reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalVerdict {
    /// The result's `status`.
    pub status: String,
    /// The result's `meaning` array; empty for a status that carries none.
    pub meaning: Vec<String>,
    /// The result's `approver`; absent for a status other than `approved`.
    pub approver: Option<String>,
}

/// What one `approval-verify` call is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalRequest<'a> {
    /// The signed annotated tag, the fact's `approvalRef`.
    pub tag: &'a str,
    /// `<authority>/<set id>/<version>/<clauseId>`.
    pub record_id: &'a str,
    /// `sha256:` and the lowercase hex of the RFC 8785 digest of the fact.
    pub record_digest: &'a str,
}

/// Why an approval verification produced no verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalRefusal {
    /// engineering-assurance refused the allowed-signers file. A usage error.
    AllowedSignersInvalid(String),
    /// engineering-assurance or git is unavailable, or the call was refused
    /// for another reason.
    Unavailable(String),
}

/// Verifies a signed approval. Implemented over engineering-assurance's
/// `approval-verify`; Quoin supplies no implementation that checks a
/// signature itself.
pub trait ApprovalVerifier {
    /// Verify the approval named by `request`.
    ///
    /// # Errors
    /// [`ApprovalRefusal`] when no verdict was produced.
    fn verify(&self, request: &ApprovalRequest<'_>) -> Result<ApprovalVerdict, ApprovalRefusal>;
}

/// What a check run is given besides the report and the store.
pub struct DischargeChecks<'a> {
    /// The policy of each clause of the report's clause set.
    pub policy: &'a SetPolicy,
    /// The approval verifier; `None` leaves an approval-class clause open with
    /// `approval-verifier-unavailable`.
    pub verifier: Option<&'a dyn ApprovalVerifier>,
    /// Never call the verifier: an approval-class clause is open with
    /// `not-verified-offline`.
    pub offline: bool,
}

/// A clause whose check failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFailure {
    /// The clause.
    pub clause_id: String,
    /// The reason, as the report states it.
    pub reason: String,
}

/// A check run that produced no report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// engineering-assurance refused the allowed-signers file.
    AllowedSignersInvalid(String),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllowedSignersInvalid(detail) => {
                write!(f, "the allowed-signers file is refused: {detail}")
            }
        }
    }
}

impl std::error::Error for CheckError {}

/// A report with every failing clause moved to the open population.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedReport {
    /// The report after the checks.
    pub report: DischargeReport,
    /// Each clause that failed, in the order the report listed it.
    pub failures: Vec<CheckFailure>,
}

/// Re-run evidence resolution and approval verification over every direct and
/// disposition entry of `report`.
///
/// `store` is `None` when no repository was supplied. `order` is the binding
/// report's clauses when the caller has it, which keeps the open population in
/// binding order; without it, moved clauses follow the ones already open.
/// A `verifiedApprover` the report arrived with is discarded: the approver is
/// whoever the verifier names now.
///
/// # Errors
/// [`CheckError::AllowedSignersInvalid`] when the verifier refuses the trust
/// file, in which case no report is produced.
pub fn check_report(
    mut report: DischargeReport,
    store: Option<&dyn EvidenceSource>,
    checks: &DischargeChecks<'_>,
    order: Option<&[ClauseBinding]>,
) -> Result<CheckedReport, CheckError> {
    for entry in report
        .binding
        .open
        .iter_mut()
        .chain(&mut report.unresolved)
        .chain(&mut report.not_binding)
    {
        entry.verified_approver = None;
    }
    let mut failures = Vec::new();
    let mut moved = Vec::new();
    let direct = std::mem::take(&mut report.binding.direct);
    report.binding.direct = settle(
        direct,
        &report.clause_set.clone(),
        store,
        checks,
        &mut failures,
        &mut moved,
    )?;
    let dispositions = std::mem::take(&mut report.binding.dispositions);
    report.binding.dispositions = settle(
        dispositions,
        &report.clause_set.clone(),
        store,
        checks,
        &mut failures,
        &mut moved,
    )?;
    report.binding.open.append(&mut moved);
    if let Some(order) = order {
        let position: HashMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(index, clause)| (clause.clause_id.as_str(), index))
            .collect();
        report.binding.open.sort_by_key(|entry| {
            position
                .get(entry.clause_id.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });
    }
    Ok(CheckedReport { report, failures })
}

fn settle(
    entries: Vec<ClauseDischarge>,
    set: &ClauseSetKey,
    store: Option<&dyn EvidenceSource>,
    checks: &DischargeChecks<'_>,
    failures: &mut Vec<CheckFailure>,
    moved: &mut Vec<ClauseDischarge>,
) -> Result<Vec<ClauseDischarge>, CheckError> {
    let mut kept = Vec::with_capacity(entries.len());
    for mut entry in entries {
        entry.verified_approver = None;
        let Some(fact) = entry.fact.clone() else {
            kept.push(entry);
            continue;
        };
        match check_one(set, store, checks, &fact)? {
            Ok(approver) => {
                entry.verified_approver = approver;
                kept.push(entry);
            }
            Err(reason) => {
                failures.push(CheckFailure {
                    clause_id: entry.clause_id.clone(),
                    reason: reason.clone(),
                });
                entry.state = DischargeState::Open;
                entry.reason = Some(reason);
                moved.push(entry);
            }
        }
    }
    Ok(kept)
}

/// Check each fact on its own, without the currency partition, and name every
/// one that fails. Used where no report is built, such as recording facts.
///
/// # Errors
/// [`CheckError::AllowedSignersInvalid`] as for [`check_report`].
pub fn check_facts(
    set: &ClauseSetKey,
    facts: &[DischargeFact],
    store: Option<&dyn EvidenceSource>,
    checks: &DischargeChecks<'_>,
) -> Result<Vec<CheckFailure>, CheckError> {
    let mut failures = Vec::new();
    for fact in facts {
        if let Err(reason) = check_one(set, store, checks, fact)? {
            failures.push(CheckFailure {
                clause_id: fact.clause_id().to_owned(),
                reason,
            });
        }
    }
    Ok(failures)
}

fn check_one(
    set: &ClauseSetKey,
    store: Option<&dyn EvidenceSource>,
    checks: &DischargeChecks<'_>,
    fact: &DischargeFact,
) -> Verdict {
    let fallback = ClausePolicy::default();
    let policy = checks.policy.get(fact.clause_id()).unwrap_or(&fallback);
    check_fact(
        &Context {
            set,
            store,
            checks,
            policy,
        },
        fact,
    )
}

struct Context<'a> {
    set: &'a ClauseSetKey,
    store: Option<&'a dyn EvidenceSource>,
    checks: &'a DischargeChecks<'a>,
    policy: &'a ClausePolicy,
}

/// `Ok(Ok(approver))` when the fact stands, `Ok(Err(reason))` when it does not.
type Verdict = Result<Result<Option<String>, String>, CheckError>;

fn check_fact(context: &Context<'_>, fact: &DischargeFact) -> Verdict {
    match fact {
        DischargeFact::Direct(direct) => check_direct(context, direct, fact),
        DischargeFact::Disposition(disposition) => check_disposition(context, disposition, fact),
    }
}

fn check_direct(context: &Context<'_>, direct: &DirectDischargeFact, fact: &DischargeFact) -> Verdict {
    if context.policy.customer_owned() {
        return Ok(Err(
            "customer-owned: the clause's outputs are procedures the customer performs".to_owned(),
        ));
    }
    let Some(store) = context.store else {
        return Ok(Err("no-store: no evidence store was supplied".to_owned()));
    };
    if context.policy.approval_class() && context.checks.offline {
        return Ok(Err(not_verified_offline()));
    }
    for reference in &direct.evidence_refs {
        if let Err(reason) = check_reference(store, context.policy, reference) {
            return Ok(Err(reason));
        }
    }
    if !context.policy.approval_class() {
        return Ok(Ok(None));
    }
    let approver = match verify_approval(context, fact, direct.approval_ref.as_deref())? {
        Ok(approver) => approver,
        Err(reason) => return Ok(Err(reason)),
    };
    if direct.attestation.attested_by != approver {
        return Ok(Err(format!(
            "attester-not-approver: the attestation names {:?} and the verified approver is {approver:?}",
            direct.attestation.attested_by
        )));
    }
    Ok(Ok(Some(approver)))
}

fn check_disposition(
    context: &Context<'_>,
    disposition: &DispositionFact,
    fact: &DischargeFact,
) -> Verdict {
    if context.policy.customer_owned()
        && (disposition.decision != DispositionDecision::Delegated
            || js_trim_is_empty(&disposition.rationale)
            || js_trim_is_empty(&disposition.approval_ref))
    {
        return Ok(Err(
            "customer-owned: only a delegated decision with a rationale and an approval reference discharges it"
                .to_owned(),
        ));
    }
    if !context.policy.approval_class() {
        return Ok(Ok(None));
    }
    if context.store.is_none() {
        return Ok(Err("no-store: no evidence store was supplied".to_owned()));
    }
    if context.checks.offline {
        return Ok(Err(not_verified_offline()));
    }
    Ok(verify_approval(context, fact, Some(&disposition.approval_ref))?.map(Some))
}

fn not_verified_offline() -> String {
    "not-verified-offline: an approval-class clause is verified only by a run that calls engineering-assurance"
        .to_owned()
}

fn verify_approval(
    context: &Context<'_>,
    fact: &DischargeFact,
    approval_ref: Option<&str>,
) -> Result<Result<String, String>, CheckError> {
    let Some(tag) = approval_ref.filter(|tag| !js_trim_is_empty(tag)) else {
        return Ok(Err("approval-missing: the fact carries no approvalRef".to_owned()));
    };
    let key = context.set;
    let record_id = match record_id(
        &key.authority,
        &key.id,
        &key.version,
        fact.clause_id(),
    ) {
        Ok(record_id) => record_id,
        Err(reason) => return Ok(Err(reason)),
    };
    let Some(verifier) = context.checks.verifier else {
        return Ok(Err(
            "approval-verifier-unavailable: no engineering-assurance verifier was supplied"
                .to_owned(),
        ));
    };
    let Ok(record_digest) = record_digest(fact) else {
        return Ok(Err(
            "approval-unverified: the fact has no canonical form to digest".to_owned()
        ));
    };
    let verdict = match verifier.verify(&ApprovalRequest {
        tag,
        record_id: &record_id,
        record_digest: &record_digest,
    }) {
        Ok(verdict) => verdict,
        Err(ApprovalRefusal::AllowedSignersInvalid(detail)) => {
            return Err(CheckError::AllowedSignersInvalid(detail));
        }
        Err(ApprovalRefusal::Unavailable(detail)) => {
            return Ok(Err(format!("approval-verifier-unavailable: {detail}")));
        }
    };
    if verdict.status != "approved" {
        return Ok(Err(format!(
            "approval-unverified: {tag}: status {}",
            verdict.status
        )));
    }
    let roles = &context.policy.approval_roles;
    let role_matches = matches!(verdict.meaning.as_slice(), [meaning] if roles.contains(meaning));
    let Some(approver) = verdict.approver.filter(|_| role_matches) else {
        return Ok(Err(format!(
            "approval-unverified: {tag}: meaning {:?} is not exactly one of the roles {roles:?}",
            verdict.meaning
        )));
    };
    Ok(Ok(approver))
}

/// The record identity of a fact: `<authority>/<set id>/<version>/<clauseId>`,
/// checked as engineering-assurance checks it.
///
/// # Errors
/// The reason, `invalid-record-identity` or `record-id-too-long`.
pub fn record_id(
    authority: &str,
    set_id: &str,
    version: &str,
    clause_id: &str,
) -> Result<String, String> {
    let record_id = format!("{authority}/{set_id}/{version}/{clause_id}");
    if record_id.chars().any(char::is_control) || record_id.starts_with('-') {
        return Err(format!(
            "invalid-record-identity: {record_id:?} is empty, holds a control character or starts with '-'"
        ));
    }
    if record_id.len() > MAX_RECORD_ID_BYTES {
        return Err(format!(
            "record-id-too-long: the record id is {} bytes, over {MAX_RECORD_ID_BYTES}",
            record_id.len()
        ));
    }
    Ok(record_id)
}

/// `sha256:` and the lowercase hex SHA-256 of the RFC 8785 canonical JSON of
/// the fact without its `approvalRef` member.
///
/// # Errors
/// A description when the fact cannot be put in canonical form.
pub fn record_digest(fact: &DischargeFact) -> Result<String, String> {
    let mut value = serde_json::to_value(fact).map_err(|error| error.to_string())?;
    if let Some(object) = value.as_object_mut() {
        object.remove("approvalRef");
    }
    let text = serde_json::to_string(&value).map_err(|error| error.to_string())?;
    let parsed = quoin_store::parse_strict_json_str(&text).map_err(|error| error.to_string())?;
    let bytes = quoin_store::canonical_bytes(&parsed).map_err(|error| error.to_string())?;
    Ok(format!(
        "sha256:{}",
        quoin_store::attachment::sha256_hex(&bytes)
    ))
}

fn check_reference(
    store: &dyn EvidenceSource,
    policy: &ClausePolicy,
    reference: &str,
) -> Result<(), String> {
    let kind = resolve_reference(store, reference)
        .map_err(|detail| format!("unresolved: {reference}: {detail}"))?;
    if policy.outputs.is_empty() {
        return Ok(());
    }
    let mut producers = policy
        .outputs
        .iter()
        .flat_map(|output| output.producers.iter())
        .peekable();
    if producers.peek().is_none() {
        return Err(format!(
            "no-producer: {reference}: no expected output of the clause names the kinds that satisfy it"
        ));
    }
    if producers.any(|producer| *producer == kind) {
        Ok(())
    } else {
        Err(format!(
            "kind-not-expected: {reference} is a {kind} record, which no expected output of the clause accepts"
        ))
    }
}

/// The retained kind of the record an `evidence://` reference names.
fn resolve_reference(store: &dyn EvidenceSource, reference: &str) -> Result<String, &'static str> {
    let path = reference
        .strip_prefix("evidence://")
        .ok_or("the reference is not an evidence:// reference")?;
    let segments: Vec<&str> = path.split('/').collect();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || matches!(*segment, "." | ".."))
    {
        return Err("the path has an empty, '.' or '..' segment");
    }
    let (family, singular) = match segments.as_slice() {
        [family @ ("runs" | "scans" | "mock-inspections" | "analysis-reports"), _, _] => {
            (*family, Some(singular_kind(family)))
        }
        [family @ ("experiments" | "operational"), _] => (*family, None),
        ["scenarios", integration, scenario]
            if integration.to_ascii_lowercase() == *integration
                && scenario.to_ascii_lowercase() == *scenario =>
        {
            ("scenarios", Some("scenario"))
        }
        ["scenarios", _, _] => return Err("a scenarios segment is not in the store's folded form"),
        _ => return Err("the path names no citable record family"),
    };
    let text = store
        .read(&format!("{path}.json"))
        .ok()
        .flatten()
        .ok_or("the store holds no readable record at that path")?;
    let record: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "the record is not JSON")?;
    if let Some(kind) = singular {
        return Ok(kind.to_owned());
    }
    record
        .pointer("/subject/kind")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or(if family == "experiments" {
            "the experiment record carries no subject kind"
        } else {
            "the operational record carries no subject kind"
        })
}

fn singular_kind(family: &str) -> &'static str {
    match family {
        "runs" => "run",
        "scans" => "scan",
        "mock-inspections" => "mock-inspection",
        _ => "analysis-report",
    }
}

/// The record identity and digest a person signs an approval over.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalSubject {
    /// `<authority>/<set id>/<version>/<clauseId>`.
    pub record_id: String,
    /// `sha256:` and the lowercase hex of the RFC 8785 digest of the fact
    /// without its `approvalRef`.
    pub record_digest: String,
}

/// The approval subject of one fact against the binding report's clause set.
///
/// A blank `approvalRef` is admitted: the person is signing before the tag
/// exists, and the digest ignores the member either way.
///
/// # Errors
/// A description when the fact is malformed, its clause is not in the binding
/// report, or the record id is refused.
pub fn approval_subject(
    binding: &quoin_quire_types::ClauseBindingReport,
    fact: &serde_json::Value,
) -> Result<ApprovalSubject, String> {
    let clause_id = fact
        .get("clauseId")
        .and_then(serde_json::Value::as_str)
        .ok_or("the fact carries no clauseId")?;
    let parsed = super::parse::parse_fact_with_approval_requirements(
        fact,
        &std::collections::BTreeSet::from([clause_id.to_owned()]),
    )
    .map_err(|error| error.to_string())?;
    if !binding
        .clauses
        .iter()
        .any(|clause| clause.clause_id == clause_id)
    {
        return Err(format!("clause {clause_id} is not in the binding report"));
    }
    let set = &binding.clause_set;
    Ok(ApprovalSubject {
        record_id: record_id(&set.authority, &set.id, &set.version, clause_id)?,
        record_digest: record_digest(&parsed)?,
    })
}
