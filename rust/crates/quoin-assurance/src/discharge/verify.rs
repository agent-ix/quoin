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
    /// The policy of each clause of the report's clause set, from the
    /// installed module that declares it; `None` when no installed module
    /// does. A clause without a policy is never discharged: nothing says
    /// which outputs or approval it needs.
    pub policy: Option<&'a SetPolicy>,
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
        let verdict = match entry.fact.clone() {
            Some(fact) => check_one(set, store, checks, &fact)?,
            None => Err("no-fact: the entry carries no fact to check".to_owned()),
        };
        match verdict {
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
    let Some(set_policy) = checks.policy else {
        return Ok(Err(format!(
            "no-clause-policy: no installed module declares clause set {}/{}/{}",
            set.authority, set.id, set.version
        )));
    };
    let Some(policy) = set_policy.get(fact.clause_id()) else {
        return Ok(Err(format!(
            "no-clause-policy: clause set {}/{}/{} declares no clause {}",
            set.authority,
            set.id,
            set.version,
            fact.clause_id()
        )));
    };
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

fn check_direct(
    context: &Context<'_>,
    direct: &DirectDischargeFact,
    fact: &DischargeFact,
) -> Verdict {
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
        return Ok(Err(
            "approval-missing: the fact carries no approvalRef".to_owned()
        ));
    };
    let key = context.set;
    let record_id = match record_id(&key.authority, &key.id, &key.version, fact.clause_id()) {
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
            "approval-unverified: the fact has no canonical form to digest".to_owned(),
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
        [
            family @ ("runs" | "scans" | "mock-inspections" | "analysis-reports"),
            _,
            _,
        ] => (*family, Some(singular_kind(family))),
        [family @ ("experiments" | "operational"), _] => (*family, None),
        ["scenarios", _, scenario] if scenario.to_ascii_lowercase() == *scenario => {
            ("scenarios", Some("scenario"))
        }
        ["scenarios", _, _] => {
            return Err("the scenario segment is not in the store's folded form");
        }
        ["customer-artifacts", id] if id.to_ascii_lowercase() == *id => {
            ("customer-artifacts", Some("customer-artifact"))
        }
        ["customer-artifacts", _] => {
            return Err("the artifact-id segment is not in the store's folded form");
        }
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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use ix_trace_rs::trace;
    use quoin_evidence::customer_artifact::write_customer_artifact;
    use quoin_evidence::source::{EvidenceSource, MemoryEvidence};
    use quoin_evidence::store::gc;
    use quoin_quire_types::{ClausePolicy, OutputPolicy, SetPolicy};
    use serde_json::json;

    use super::super::build::build_discharge_report;
    use super::super::fixtures::{StubVerifier, binding, direct, request, store};
    use super::super::report::DischargeFact;
    use super::{
        ApprovalRefusal, ApprovalVerdict, ApprovalVerifier, CheckError, CheckedReport,
        DischargeChecks, approval_subject, check_report, record_digest, record_id,
    };

    fn policy(approval_roles: &[&str], outputs: Vec<OutputPolicy>) -> SetPolicy {
        SetPolicy::from([(
            "SYN-001".to_owned(),
            ClausePolicy {
                approval_roles: approval_roles.iter().map(|r| (*r).to_owned()).collect(),
                outputs,
            },
        )])
    }

    fn producing(kinds: &[&str]) -> OutputPolicy {
        OutputPolicy {
            procedure: false,
            producers: kinds.iter().map(|k| (*k).to_owned()).collect(),
        }
    }

    fn run(
        facts: Vec<serde_json::Value>,
        policy: &SetPolicy,
        store: Option<&MemoryEvidence>,
        verifier: Option<&dyn ApprovalVerifier>,
        offline: bool,
    ) -> Result<CheckedReport, CheckError> {
        let partition = build_discharge_report(&request(facts)).expect("the facts are well formed");
        check_report(
            partition,
            store.map(|s| s as &dyn EvidenceSource),
            &DischargeChecks {
                policy: Some(policy),
                verifier,
                offline,
            },
            Some(&binding().clauses),
        )
    }

    fn one(
        fact: serde_json::Value,
        policy: &SetPolicy,
        store: Option<&MemoryEvidence>,
    ) -> CheckedReport {
        run(vec![fact], policy, store, None, false).expect("no verifier is consulted")
    }

    fn reason(checked: &CheckedReport) -> String {
        assert!(checked.report.binding.direct.is_empty(), "{checked:?}");
        checked.report.binding.open[0]
            .reason
            .clone()
            .expect("an open clause states why")
    }

    fn approved_fact(approver: &str) -> serde_json::Value {
        let mut fact = direct();
        fact["approvalRef"] = "approval/tag-1".into();
        fact["attestation"]["attestedBy"] = approver.into();
        fact
    }

    #[trace("FR-211-AC-1")]
    #[test]
    fn a_ref_to_a_record_of_a_producing_kind_discharges() {
        let checked = one(
            direct(),
            &policy(&[], vec![producing(&["run"])]),
            Some(&store()),
        );
        assert_eq!(checked.report.binding.direct.len(), 1);
        assert!(checked.failures.is_empty());
        // Empty expected outputs accept a record of any kind.
        let checked = one(direct(), &policy(&[], vec![]), Some(&store()));
        assert_eq!(checked.report.binding.direct.len(), 1);
    }

    #[trace("FR-211-AC-2")]
    #[test]
    fn a_ref_that_names_no_citable_record_is_unresolved_and_named() {
        for bad in [
            "evidence://runs/SUITE-1/missing",
            "evidence://discharge/a/b/c",
            "evidence://trust/x",
            "evidence://bindings.json",
            "evidence://unknown/SUITE-1/one",
            "https://runs/SUITE-1/one",
            "evidence://runs//one",
            "evidence://runs/./one",
            "evidence://runs/SUITE-1/../SUITE-1/one",
            "evidence://runs/SUITE-1/one/extra",
            "runs/SUITE-1/one",
        ] {
            let mut fact = direct();
            fact["evidenceRefs"] = json!([bad]);
            let checked = one(fact, &policy(&[], vec![]), Some(&store()));
            let reason = reason(&checked);
            assert!(reason.starts_with("unresolved: "), "{bad}: {reason}");
            assert!(reason.contains(bad), "{bad}: {reason}");
        }
    }

    #[trace("FR-211-AC-3", "FR-211-AC-23")]
    #[test]
    fn a_kind_no_producer_accepts_is_kind_not_expected_unless_none_declares_producers() {
        let kinds = policy(&[], vec![producing(&["scan"])]);
        assert!(reason(&one(direct(), &kinds, Some(&store()))).starts_with("kind-not-expected: "));
        let none = policy(&[], vec![producing(&[])]);
        assert!(reason(&one(direct(), &none, Some(&store()))).starts_with("no-producer: "));
        // One entry with producers makes a mismatch kind-not-expected.
        let mixed = policy(&[], vec![producing(&[]), producing(&["scan"])]);
        assert!(reason(&one(direct(), &mixed, Some(&store()))).starts_with("kind-not-expected: "));
    }

    #[trace("FR-211-AC-5")]
    #[test]
    fn a_catalog_without_producers_leaves_a_direct_fact_undischarged() {
        let undeclared = policy(&[], vec![OutputPolicy::default()]);
        let checked = one(direct(), &undeclared, Some(&store()));
        assert!(reason(&checked).starts_with("no-producer: "));
    }

    #[trace("FR-211-AC-4")]
    #[test]
    fn a_scenario_ref_folds_the_scenario_segment_only() {
        // The store keeps the integration segment as written and folds only
        // the scenario file name.
        let mut held = MemoryEvidence::new();
        held.write("scenarios/IT-002/it-002-sc-07a.json", b"{}")
            .unwrap();
        let producers = policy(&[], vec![producing(&["scenario"])]);
        let mut fact = direct();
        fact["evidenceRefs"] = json!(["evidence://scenarios/IT-002/it-002-sc-07a"]);
        assert_eq!(
            one(fact, &producers, Some(&held))
                .report
                .binding
                .direct
                .len(),
            1
        );
        for bad in [
            "evidence://scenarios/IT-002/It-002-sc-07a",
            "evidence://scenarios/it-002/it-002-sc-07a",
        ] {
            let mut fact = direct();
            fact["evidenceRefs"] = json!([bad]);
            let checked = one(fact, &producers, Some(&held));
            assert!(reason(&checked).starts_with("unresolved: "), "{bad}");
        }
    }

    fn customer_store() -> MemoryEvidence {
        let mut held = MemoryEvidence::new();
        write_customer_artifact(
            &mut held,
            &json!({
                "artifactId": "Complaint-001",
                "title": "Complaint record",
                "location": "/nonexistent/unreadable/path or https://unreachable.invalid/x",
                "suppliedBy": "Quality manager",
                "suppliedAt": "2026-11-20T09:30:00Z"
            }),
        )
        .unwrap();
        held
    }

    fn cites(reference: &str) -> serde_json::Value {
        let mut fact = direct();
        fact["evidenceRefs"] = json!([reference]);
        fact
    }

    #[trace("FR-211-AC-25")]
    #[test]
    fn a_customer_artifact_ref_discharges_a_clause_whose_producers_name_the_kind() {
        let producers = policy(&[], vec![producing(&["customer-artifact"])]);
        let checked = one(
            cites("evidence://customer-artifacts/complaint-001"),
            &producers,
            Some(&customer_store()),
        );
        assert_eq!(checked.report.binding.direct.len(), 1, "{checked:?}");
        assert!(checked.failures.is_empty());
    }

    #[trace("FR-211-AC-26")]
    #[test]
    fn without_a_retained_customer_artifact_of_the_kind_the_clause_stays_open() {
        let producers = policy(&[], vec![producing(&["customer-artifact"])]);
        let held = customer_store();
        let missing = one(
            cites("evidence://customer-artifacts/not-retained"),
            &producers,
            Some(&held),
        );
        assert!(reason(&missing).starts_with("unresolved: "));
        // A retained record of another kind is not a customer artifact.
        let wrong_kind = one(direct(), &producers, Some(&store()));
        assert!(reason(&wrong_kind).starts_with("kind-not-expected: "));
        // And a customer artifact is not accepted where only runs are produced.
        let runs_only = policy(&[], vec![producing(&["run"])]);
        let checked = one(
            cites("evidence://customer-artifacts/complaint-001"),
            &runs_only,
            Some(&held),
        );
        assert!(reason(&checked).starts_with("kind-not-expected: "));
    }

    #[trace("FR-211-AC-28")]
    #[test]
    fn a_customer_artifact_ref_is_citable_only_as_a_lowercase_single_segment() {
        let producers = policy(&[], vec![producing(&["customer-artifact"])]);
        let held = customer_store();
        for bad in [
            "evidence://customer-artifacts/Complaint-001",
            "evidence://customer-artifacts/",
            "evidence://customer-artifacts",
            "evidence://customer-artifacts/.",
            "evidence://customer-artifacts/..",
            "evidence://customer-artifacts/../customer-artifacts/complaint-001",
            "evidence://customer-artifacts/complaint-001/extra",
        ] {
            let checked = one(cites(bad), &producers, Some(&held));
            assert!(reason(&checked).starts_with("unresolved: "), "{bad}");
        }
    }

    #[trace("FR-211-AC-29")]
    #[test]
    fn resolving_a_customer_artifact_reads_only_the_record_and_reports_no_digest() {
        let producers = policy(&[], vec![producing(&["customer-artifact"])]);
        let checked = one(
            cites("evidence://customer-artifacts/complaint-001"),
            &producers,
            Some(&customer_store()),
        );
        assert_eq!(checked.report.binding.direct.len(), 1);
        let report = serde_json::to_string(&checked.report).unwrap();
        for forbidden in ["sha256", "digest", "hash", "size", "adequate", "unreadable"] {
            assert!(!report.contains(forbidden), "{forbidden} in {report}");
        }
    }

    #[trace("FR-211-AC-30")]
    #[test]
    fn a_customer_artifact_survives_gc_and_keeps_its_clause_discharged() {
        let producers = policy(&[], vec![producing(&["customer-artifact"])]);
        let mut held = customer_store();
        let fact_path = "discharge/quire/spec/1/SYN-001.json";
        held.write(fact_path, b"{\"stored\":\"fact\"}").unwrap();
        let fact = cites("evidence://customer-artifacts/complaint-001");
        assert_eq!(
            one(fact.clone(), &producers, Some(&held))
                .report
                .binding
                .direct
                .len(),
            1
        );
        let stored_fact = held.read(fact_path).unwrap();
        gc(&mut held, false, &mut Vec::new()).unwrap();
        assert_eq!(held.read(fact_path).unwrap(), stored_fact);
        assert_eq!(
            one(fact, &producers, Some(&held))
                .report
                .binding
                .direct
                .len(),
            1
        );
    }

    #[trace("FR-211-AC-1")]
    #[test]
    fn an_experiment_ref_takes_its_kind_from_the_subject() {
        let mut held = MemoryEvidence::new();
        held.write(
            "experiments/exp-1.json",
            br#"{"subject":{"kind":"experiment"}}"#,
        )
        .unwrap();
        held.write("operational/op-1.json", br#"{"subject":{}}"#)
            .unwrap();
        let producers = policy(&[], vec![producing(&["experiment"])]);
        let mut fact = direct();
        fact["evidenceRefs"] = json!(["evidence://experiments/exp-1"]);
        assert_eq!(
            one(fact, &producers, Some(&held))
                .report
                .binding
                .direct
                .len(),
            1
        );
        let mut fact = direct();
        fact["evidenceRefs"] = json!(["evidence://operational/op-1"]);
        assert!(reason(&one(fact, &producers, Some(&held))).starts_with("unresolved: "));
    }

    #[trace("FR-211-AC-21")]
    #[test]
    fn without_a_store_a_direct_fact_and_an_approval_class_clause_are_no_store() {
        assert!(reason(&one(direct(), &policy(&[], vec![]), None)).starts_with("no-store: "));
        let approval = policy(&["reviewer"], vec![]);
        let checked = one(approved_fact("alice"), &approval, None);
        assert!(reason(&checked).starts_with("no-store: "));
    }

    #[trace("FR-211-AC-6", "FR-211-AC-16")]
    #[test]
    fn a_verified_approval_discharges_and_the_report_names_the_approver() {
        let approval = policy(&["reviewer"], vec![]);
        let verifier = StubVerifier::approving("alice", &["reviewer"]);
        let checked = run(
            vec![approved_fact("alice")],
            &approval,
            Some(&store()),
            Some(&verifier),
            false,
        )
        .unwrap();
        assert_eq!(checked.report.binding.direct.len(), 1);
        assert_eq!(
            checked.report.binding.direct[0]
                .verified_approver
                .as_deref(),
            Some("alice")
        );
        let asked = verifier.asked.borrow();
        let (tag, id, digest) = &asked[0];
        assert_eq!(tag, "approval/tag-1");
        assert_eq!(id, "example.invalid/synthetic-widget-rules/1.0.0/SYN-001");
        assert!(
            digest.starts_with("sha256:") && digest.len() == 71,
            "{digest}"
        );
    }

    #[trace("FR-211-AC-16")]
    #[test]
    fn a_verified_approver_in_a_supplied_report_is_never_trusted() {
        let mut partition = build_discharge_report(&request(vec![direct()])).expect("well formed");
        partition.binding.direct[0].verified_approver = Some("mallory".to_owned());
        let checked = check_report(
            partition,
            Some(&store()),
            &DischargeChecks {
                policy: Some(&policy(&[], vec![])),
                verifier: None,
                offline: false,
            },
            None,
        )
        .unwrap();
        assert_eq!(checked.report.binding.direct[0].verified_approver, None);
    }

    #[trace("FR-211-AC-7")]
    #[test]
    fn every_other_status_and_meaning_leaves_the_clause_open() {
        let approval = policy(&["reviewer"], vec![]);
        for status in [
            "unsigned",
            "signed_bad",
            "untrusted_signer",
            "wrong_record",
            "meaning_missing",
            "unverifiable",
        ] {
            let verifier = StubVerifier::answering(Ok(ApprovalVerdict {
                status: status.to_owned(),
                meaning: vec![],
                approver: None,
            }));
            let checked = run(
                vec![approved_fact("alice")],
                &approval,
                Some(&store()),
                Some(&verifier),
                false,
            )
            .unwrap();
            let reason = reason(&checked);
            assert!(
                reason.starts_with("approval-unverified: approval/tag-1"),
                "{reason}"
            );
            assert!(reason.contains(status), "{reason}");
        }
        for meaning in [
            vec!["reviewer-lead"],
            vec!["reviewer", "other"],
            vec![],
            vec!["Reviewer"],
        ] {
            let verifier = StubVerifier::approving("alice", &meaning);
            let checked = run(
                vec![approved_fact("alice")],
                &approval,
                Some(&store()),
                Some(&verifier),
                false,
            )
            .unwrap();
            assert!(
                reason(&checked).starts_with("approval-unverified: approval/tag-1"),
                "{meaning:?}"
            );
        }
    }

    #[trace("FR-211-AC-8")]
    #[test]
    fn an_attester_other_than_the_verified_approver_is_refused() {
        let approval = policy(&["reviewer"], vec![]);
        let verifier = StubVerifier::approving("alice", &["reviewer"]);
        let checked = run(
            vec![approved_fact("bob")],
            &approval,
            Some(&store()),
            Some(&verifier),
            false,
        )
        .unwrap();
        assert!(reason(&checked).starts_with("attester-not-approver: "));
    }

    #[trace("FR-211-AC-9")]
    #[test]
    fn a_missing_approval_ref_is_open_and_never_calls_the_verifier() {
        let approval = policy(&["reviewer"], vec![]);
        let verifier = StubVerifier::approving("alice", &["reviewer"]);
        let mut blank = approved_fact("alice");
        blank["approvalRef"] = "  ".into();
        for fact in [direct(), blank] {
            let partition = {
                let mut partition =
                    build_discharge_report(&request(vec![direct()])).expect("well formed");
                partition.binding.direct[0].fact = Some(
                    crate::discharge::parse::parse_fact_with_approval_requirements(
                        &fact,
                        &std::collections::BTreeSet::from(["SYN-001".to_owned()]),
                    )
                    .unwrap(),
                );
                partition
            };
            let checked = check_report(
                partition,
                Some(&store()),
                &DischargeChecks {
                    policy: Some(&approval),
                    verifier: Some(&verifier),
                    offline: false,
                },
                None,
            )
            .unwrap();
            assert!(reason(&checked).starts_with("approval-missing: "));
        }
        assert!(verifier.asked.borrow().is_empty());
    }

    #[trace("FR-211-AC-10", "FR-211-AC-20")]
    #[test]
    fn the_record_identity_is_checked_as_engineering_assurance_checks_it() {
        assert!(
            record_id("a", "b", "c", "-d").is_ok(),
            "an inner '-' is accepted"
        );
        assert!(
            record_id("-a", "b", "c", "d")
                .unwrap_err()
                .starts_with("invalid-record-identity")
        );
        assert!(
            record_id("a", "b", "c", "d\u{7}")
                .unwrap_err()
                .starts_with("invalid-record-identity")
        );
        let long = "x".repeat(300);
        assert!(
            record_id("a", &long, "c", "d")
                .unwrap_err()
                .starts_with("record-id-too-long")
        );

        // The identity is refused before any engineering-assurance call.
        let approval = policy(&["reviewer"], vec![]);
        let verifier = StubVerifier::approving("alice", &["reviewer"]);
        let mut partition =
            build_discharge_report(&request(vec![approved_fact("alice")])).expect("well formed");
        partition.clause_set.authority = "-leading".to_owned();
        let checked = check_report(
            partition,
            Some(&store()),
            &DischargeChecks {
                policy: Some(&approval),
                verifier: Some(&verifier),
                offline: false,
            },
            None,
        )
        .unwrap();
        assert!(reason(&checked).starts_with("invalid-record-identity"));
        assert!(verifier.asked.borrow().is_empty());
        // A clause without approval roles is not checked and makes no call.
        let checked = run(
            vec![approved_fact("alice")],
            &policy(&[], vec![]),
            Some(&store()),
            Some(&verifier),
            false,
        )
        .unwrap();
        assert_eq!(checked.report.binding.direct.len(), 1);
        assert!(verifier.asked.borrow().is_empty());
    }

    #[trace("FR-211-AC-11")]
    #[test]
    fn a_refused_trust_file_stops_the_run_and_an_unavailable_verifier_leaves_the_clause_open() {
        let approval = policy(&["reviewer"], vec![]);
        let refusing = StubVerifier::answering(Err(ApprovalRefusal::AllowedSignersInvalid(
            "bad file".to_owned(),
        )));
        let error = run(
            vec![approved_fact("alice")],
            &approval,
            Some(&store()),
            Some(&refusing),
            false,
        )
        .unwrap_err();
        assert!(matches!(error, CheckError::AllowedSignersInvalid(_)));
        let unavailable =
            StubVerifier::answering(Err(ApprovalRefusal::Unavailable("no git".to_owned())));
        let checked = run(
            vec![approved_fact("alice")],
            &approval,
            Some(&store()),
            Some(&unavailable),
            false,
        )
        .unwrap();
        assert!(reason(&checked).starts_with("approval-verifier-unavailable: no git"));
        // No verifier at all reads the same, never as verified.
        let checked = run(
            vec![approved_fact("alice")],
            &approval,
            Some(&store()),
            None,
            false,
        )
        .unwrap();
        assert!(reason(&checked).starts_with("approval-verifier-unavailable: "));
    }

    #[trace("FR-211-AC-12")]
    #[test]
    fn a_customer_owned_clause_takes_only_a_delegated_disposition() {
        let customer = policy(
            &[],
            vec![OutputPolicy {
                procedure: true,
                producers: vec![],
            }],
        );
        assert!(reason(&one(direct(), &customer, Some(&store()))).starts_with("customer-owned: "));
        let disposition = |decision: &str| {
            json!({
                "kind": "disposition", "clauseId": "SYN-001", "decision": decision,
                "owner": "o", "rationale": "because", "approvalRef": "ref-1",
                "attestation": direct()["attestation"].clone(),
            })
        };
        let delegated = one(disposition("delegated"), &customer, Some(&store()));
        assert_eq!(delegated.report.binding.dispositions.len(), 1);
        let accepted = one(disposition("accepted_risk"), &customer, Some(&store()));
        assert!(accepted.report.binding.dispositions.is_empty());
        assert!(
            accepted.report.binding.open[0]
                .reason
                .as_deref()
                .unwrap()
                .starts_with("customer-owned: ")
        );
    }

    #[trace("FR-211-AC-13", "FR-211-AC-22")]
    #[test]
    fn a_disposition_on_an_approval_class_clause_is_verified_too() {
        let approval = policy(&["reviewer"], vec![]);
        let fact = json!({
            "kind": "disposition", "clauseId": "SYN-001", "decision": "delegated",
            "owner": "o", "rationale": "because", "approvalRef": "ref-1",
            "attestation": direct()["attestation"].clone(),
        });
        let unverified = StubVerifier::answering(Ok(ApprovalVerdict {
            status: "unsigned".to_owned(),
            meaning: vec![],
            approver: None,
        }));
        let checked = run(
            vec![fact.clone()],
            &approval,
            Some(&store()),
            Some(&unverified),
            false,
        )
        .unwrap();
        assert!(checked.report.binding.dispositions.is_empty());
        let verified = StubVerifier::approving("alice", &["reviewer"]);
        let checked = run(
            vec![fact],
            &approval,
            Some(&store()),
            Some(&verified),
            false,
        )
        .unwrap();
        assert_eq!(checked.report.binding.dispositions.len(), 1);
    }

    #[trace("FR-211-AC-13")]
    #[test]
    fn an_offline_run_never_calls_the_verifier_for_an_approval_class_clause() {
        let approval = policy(&["reviewer"], vec![]);
        let verifier = StubVerifier::approving("alice", &["reviewer"]);
        let checked = run(
            vec![approved_fact("alice")],
            &approval,
            Some(&store()),
            Some(&verifier),
            true,
        )
        .unwrap();
        assert!(reason(&checked).starts_with("not-verified-offline: "));
        assert!(verifier.asked.borrow().is_empty());
        // A clause without roles resolves as online.
        let plain = run(
            vec![direct()],
            &policy(&[], vec![]),
            Some(&store()),
            None,
            true,
        )
        .unwrap();
        assert_eq!(plain.report.binding.direct.len(), 1);
    }

    #[trace("FR-211-AC-14")]
    #[test]
    fn a_record_the_store_no_longer_holds_reopens_the_clause() {
        let partition = build_discharge_report(&request(vec![direct()])).unwrap();
        assert_eq!(partition.binding.direct.len(), 1);
        let emptied = MemoryEvidence::new();
        let checked = check_report(
            partition,
            Some(&emptied),
            &DischargeChecks {
                policy: Some(&policy(&[], vec![])),
                verifier: None,
                offline: false,
            },
            Some(&binding().clauses),
        )
        .unwrap();
        assert!(reason(&checked).contains("evidence://runs/SUITE-1/one"));
        assert_eq!(checked.failures[0].clause_id, "SYN-001");
    }

    #[trace("FR-211-AC-17", "FR-211-AC-18")]
    #[test]
    fn the_digest_ignores_the_approval_ref_and_follows_every_other_member() {
        let parse = |value: serde_json::Value| -> DischargeFact {
            crate::discharge::parse::parse_fact_with_approval_requirements(
                &value,
                &std::collections::BTreeSet::new(),
            )
            .unwrap()
        };
        let plain = record_digest(&parse(direct())).unwrap();
        let mut with_ref = direct();
        with_ref["approvalRef"] = "tag-a".into();
        let mut other_ref = direct();
        other_ref["approvalRef"] = "tag-b".into();
        assert_eq!(record_digest(&parse(with_ref.clone())).unwrap(), plain);
        assert_eq!(record_digest(&parse(other_ref)).unwrap(), plain);
        let mut changed = direct();
        changed["attestation"]["authority"] = "someone-else".into();
        assert_ne!(record_digest(&parse(changed)).unwrap(), plain);
        // The digest is the SHA-256 of the RFC 8785 text, written out.
        let expected = "{\"attestation\":{\"attestedAt\":\"2026-08-01T00:00:00.000Z\",\"attestedBy\":\"reviewer-1\",\"authority\":\"quality-lead\",\"expiresAt\":\"2026-09-01T00:00:00.000Z\"},\"clauseId\":\"SYN-001\",\"evidenceRefs\":[\"evidence://runs/SUITE-1/one\"],\"kind\":\"direct\"}";
        assert_eq!(
            plain,
            format!(
                "sha256:{}",
                quoin_store::attachment::sha256_hex(expected.as_bytes())
            )
        );

        let subject = approval_subject(&binding(), &with_ref).unwrap();
        assert_eq!(
            subject.record_id,
            "example.invalid/synthetic-widget-rules/1.0.0/SYN-001"
        );
        assert_eq!(subject.record_digest, plain);
        let mut absent = direct();
        absent["clauseId"] = "SYN-404".into();
        assert!(approval_subject(&binding(), &absent).is_err());
        assert!(approval_subject(&binding(), &json!({"kind": "direct"})).is_err());
    }

    #[trace("FR-211-AC-13")]
    #[test]
    fn open_clauses_stay_in_binding_order_after_a_clause_moves() {
        let mut value = serde_json::to_value(binding()).unwrap();
        value["clauses"].as_array_mut().unwrap().insert(
            0,
            json!({"clauseId": "SYN-000", "force": "mandatory", "outcome": "binding",
                   "reasons": [], "expectedOutputs": []}),
        );
        let wide: quoin_quire_types::ClauseBindingReport = serde_json::from_value(value).unwrap();
        let mut input = request(vec![direct()]);
        input.binding = wide;
        let partition = build_discharge_report(&input).unwrap();
        let checked = check_report(
            partition,
            None,
            &DischargeChecks {
                policy: Some(&policy(&[], vec![])),
                verifier: None,
                offline: false,
            },
            Some(&input.binding.clauses),
        )
        .unwrap();
        let open: Vec<&str> = checked
            .report
            .binding
            .open
            .iter()
            .map(|entry| entry.clause_id.as_str())
            .collect();
        assert_eq!(open, ["SYN-000", "SYN-001"]);
    }

    #[trace("FR-211-AC-24")]
    #[test]
    fn a_clause_without_a_declared_policy_is_never_discharged() {
        let partition = build_discharge_report(&request(vec![direct()])).unwrap();
        // No installed module declares the clause set.
        let checked = check_report(
            partition.clone(),
            Some(&store()),
            &DischargeChecks {
                policy: None,
                verifier: None,
                offline: false,
            },
            Some(&binding().clauses),
        )
        .unwrap();
        assert!(
            reason(&checked).starts_with("no-clause-policy: no installed module declares"),
            "{checked:?}"
        );
        // The clause set is declared but lists no such clause.
        let other = SetPolicy::from([("SYN-009".to_owned(), ClausePolicy::default())]);
        let checked = one(direct(), &other, Some(&store()));
        assert!(
            reason(&checked).starts_with("no-clause-policy: clause set"),
            "{checked:?}"
        );
        // Facts are checked the same way where no report is built.
        let facts = [
            crate::discharge::parse::parse_fact_with_approval_requirements(
                &direct(),
                &std::collections::BTreeSet::new(),
            )
            .unwrap(),
        ];
        let failures = super::check_facts(
            &binding().clause_set,
            &facts,
            Some(&store()),
            &DischargeChecks {
                policy: None,
                verifier: None,
                offline: false,
            },
        )
        .unwrap();
        assert_eq!(failures.len(), 1);
    }

    #[trace("FR-211-AC-24")]
    #[test]
    fn an_entry_without_a_fact_is_not_discharged() {
        let mut partition = build_discharge_report(&request(vec![direct()])).unwrap();
        partition.binding.direct[0].fact = None;
        let checked = check_report(
            partition,
            Some(&store()),
            &DischargeChecks {
                policy: Some(&policy(&[], vec![])),
                verifier: None,
                offline: false,
            },
            Some(&binding().clauses),
        )
        .unwrap();
        assert!(reason(&checked).starts_with("no-fact: "), "{checked:?}");
        assert_eq!(checked.failures[0].clause_id, "SYN-001");
    }

    #[trace("FR-211-AC-16")]
    #[test]
    fn a_supplied_approver_is_cleared_from_an_entry_that_then_fails() {
        let mut partition = build_discharge_report(&request(vec![direct()])).unwrap();
        partition.binding.direct[0].verified_approver = Some("mallory".to_owned());
        // No store: the direct fact fails with no-store and moves to open.
        let checked = check_report(
            partition,
            None,
            &DischargeChecks {
                policy: Some(&policy(&[], vec![])),
                verifier: None,
                offline: false,
            },
            Some(&binding().clauses),
        )
        .unwrap();
        assert!(reason(&checked).starts_with("no-store: "), "{checked:?}");
        assert_eq!(checked.report.binding.open[0].verified_approver, None);
    }

    #[trace("FR-211-AC-18")]
    #[test]
    fn a_record_id_over_the_bound_has_no_subject() {
        let mut long = serde_json::to_value(binding()).unwrap();
        long["clauseSet"]["authority"] = "a".repeat(300).into();
        let long: quoin_quire_types::ClauseBindingReport = serde_json::from_value(long).unwrap();
        assert!(approval_subject(&long, &direct()).is_err());
    }
}
