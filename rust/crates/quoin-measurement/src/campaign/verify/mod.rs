// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Independent checks over retained campaign identities and bytes (FR-114).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod collection;
mod origin;

use engineering_assurance::campaign::{
    CampaignAttempt, CampaignAttemptStatus, CampaignDefinition, CampaignRun, CampaignVerdict,
    MeasurementProcedure, PlanRegistration, canonical_digest, validate_definition, validate_run,
};
use engineering_assurance::producer_execution::InputBinding;
use quoin_store::{digest_bytes_sha256, parse_strict_json};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use super::checker::{
    CHECKER_DEFINITION_PATH, CHECKER_INPUT_PATH, CHECKER_RAW_BUNDLE_PATH, CHECKER_REQUEST_PATH,
    CHECKER_RESULT_PATH, DOMAIN_CHECK_INPUT_SCHEMA, DependencyResultInput, DomainCheckInput,
    DomainVerdictReceipt, RawArtifactBundle, RawArtifactInput, assess_receipt, raw_artifact_bundle,
    staged_dependency_path, staged_dependency_raw_path,
};
use super::source::{SourceError, verify_source_graph};
use super::store::{
    CampaignStoreError, digest_path, read_bounded, read_digest_bytes, read_typed, run_path,
};
use super::{
    Attempt, AttemptEvidence, CampaignDecision, CampaignOutcome, CampaignReason, Member,
    all_required,
};
use crate::plans::PlanLoadOptions;
use crate::source::DiskMeasurement;
use crate::types::plan::MeasurementPlan;
use crate::verify::{OrderSource, Ranked, TamperFacts, verdict_json, verify};

/// A campaign record cannot be read or does not satisfy its EA shape.
#[derive(Debug, Error)]
pub enum CampaignVerificationError {
    /// Exact source graph could not be recovered from clean checkouts.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// A required retained file failed safe reading.
    #[error(transparent)]
    Store(#[from] CampaignStoreError),
    /// The EA campaign contract refused a run.
    #[error("invalid campaign run: {0}")]
    Run(#[from] engineering_assurance::campaign::CampaignError),
    /// The requested run id differs from the stored run.
    #[error("campaign run id does not match retained filename")]
    RunId,
}

/// Identity-bound Quoin receipt for an independently checked campaign.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignVerdictReceipt {
    /// Versioned receipt discriminator.
    pub schema: &'static str,
    /// Campaign identity from the definition.
    pub campaign_id: String,
    /// Exact run identity.
    pub run_id: String,
    /// SHA-256-JCS of the validated campaign definition.
    pub definition_digest: String,
    /// SHA-256-JCS of the source graph.
    pub source_graph_digest: String,
    /// SHA-256-JCS of the complete retained run.
    pub run_digest: String,
    /// SHA-256-JCS of its complete attempt inventory.
    pub attempt_inventory_digest: String,
    /// Quoin's independent all-required decision.
    pub decision: CampaignDecision,
}

/// Reopen all retained evidence named by a run and decide `all-required`.
///
/// Missing attempt evidence stays inconclusive. A present but contradictory
/// reference rejects its attempt. Structural EA errors refuse the whole run.
///
/// # Errors
/// Refuses unreadable definition/run records and invalid EA run bindings.
#[allow(
    clippy::too_many_lines,
    reason = "source, attempt, and aggregate identity checks form one replay chain"
)]
pub fn verify_retained_campaign(
    repo: &Path,
    definition_digest: &str,
    run_id: &str,
    source_checkouts: &BTreeMap<String, PathBuf>,
) -> Result<CampaignVerdictReceipt, CampaignVerificationError> {
    let definition_path = digest_path(repo, "definitions", definition_digest, "json")?;
    let definition: CampaignDefinition = read_typed(&definition_path)?;
    let actual_definition = canonical_digest(&definition)?;
    if actual_definition.as_str() != definition_digest {
        return Err(CampaignStoreError::Digest(definition_path).into());
    }
    let sources = verify_source_graph(&definition, source_checkouts)?;
    let source_inputs: BTreeMap<String, Vec<InputBinding>> = sources
        .iter()
        .map(|(name, source)| Ok((name.clone(), source.input_inventory()?)))
        .collect::<Result<_, SourceError>>()?;
    let own_root = repo
        .canonicalize()
        .map_err(|error| CampaignStoreError::Io {
            path: repo.to_path_buf(),
            source: error,
        })?;
    let own_source = sources
        .values()
        .find(|source| source.checkout.canonicalize().ok().as_ref() == Some(&own_root))
        .ok_or_else(|| SourceError::Missing("campaign plan repository".to_owned()))?;
    let plan_source_inputs = source_inputs
        .get(&own_source.repository)
        .ok_or_else(|| SourceError::Missing(own_source.repository.clone()))?;
    let run_file = run_path(repo, run_id)?;
    let run: CampaignRun = read_typed(&run_file)?;
    if run.id != run_id {
        return Err(CampaignVerificationError::RunId);
    }
    validate_run(&run, &definition)?;
    let plan_root = tempfile::tempdir().map_err(|source| CampaignStoreError::Io {
        path: repo.to_path_buf(),
        source,
    })?;
    own_source.stage_into(plan_root.path())?;
    let plans = crate::load_measurement_plans(
        &DiskMeasurement::new(plan_root.path()),
        PlanLoadOptions::default(),
    )
    .map_err(|error| CampaignStoreError::Json {
        path: repo.to_path_buf(),
        message: error.to_string(),
    })?;
    if plans.iter().any(|plan| {
        !own_source.contains_path(&plan.path)
            || plan
                .execution_procedure
                .as_deref()
                .is_some_and(|path| !own_source.contains_path(path))
    }) {
        return Err(
            SourceError::Path("plan or procedure is not in exact source tree".to_owned()).into(),
        );
    }
    let mut procedures = BTreeMap::new();
    for plan in &plans {
        if let Some(path) = &plan.execution_procedure {
            let procedure: MeasurementProcedure = read_typed(&plan_root.path().join(path))?;
            procedures.insert(plan.id.as_str().to_owned(), procedure);
        }
    }
    let registrations: Vec<PlanRegistration<'_>> = plans
        .iter()
        .map(|plan| PlanRegistration {
            id: plan.id.as_str(),
            definition_version: plan.definition_version.as_str(),
            procedure: procedures.get(plan.id.as_str()),
        })
        .collect();
    validate_definition(&definition, &registrations)?;
    let (missing_repetition, extra_repetition) =
        repetition_inventory(&definition, &run, &procedures)?;
    let source_graph_digest = canonical_digest(&definition.source_graph)?
        .as_str()
        .to_owned();
    let run_digest = canonical_digest(&run)?.as_str().to_owned();
    let inventory_digest = canonical_digest(&run.attempts)?.as_str().to_owned();
    let members: Vec<Member<'_>> = definition
        .members
        .iter()
        .map(|member| Member {
            name: &member.name,
            group: member.group.as_deref(),
            required: member.required,
            depends_on: member.depends_on.as_deref().unwrap_or(&[]),
        })
        .collect();
    let run_attempts = run.attempts.as_deref().unwrap_or(&[]);
    let assessed: Vec<(bool, AttemptEvidence)> = run_attempts
        .iter()
        .map(|attempt| {
            let completed = attempt.status == CampaignAttemptStatus::Completed;
            let evidence = if completed {
                assess_attempt(
                    repo,
                    &definition,
                    definition_digest,
                    run_id,
                    attempt,
                    run_attempts,
                    &plans,
                    &procedures,
                    &source_inputs,
                    plan_source_inputs,
                )
            } else {
                AttemptEvidence::Inconclusive
            };
            (completed, evidence)
        })
        .collect();
    let attempts: Vec<Attempt<'_>> = run
        .attempts
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .zip(&assessed)
        .filter_map(|(attempt, (completed, evidence))| {
            u64::try_from(attempt.index).ok().map(|index| Attempt {
                member: &attempt.member,
                index,
                completed: *completed,
                evidence: *evidence,
            })
        })
        .collect();
    let mut decision = all_required(&members, &attempts);
    let claimed = match run.verdict {
        CampaignVerdict::Accepted => CampaignOutcome::Accept,
        CampaignVerdict::Rejected => CampaignOutcome::Reject,
        CampaignVerdict::Inconclusive => CampaignOutcome::Inconclusive,
    };
    if extra_repetition {
        decision.verdict = CampaignOutcome::Reject;
        decision.reasons.push(CampaignReason::EvidenceContradiction);
    } else if missing_repetition && decision.verdict == CampaignOutcome::Accept {
        decision.verdict = CampaignOutcome::Inconclusive;
        decision.reasons.push(CampaignReason::MissingAttempt);
    }
    if claimed != decision.verdict {
        decision.verdict = CampaignOutcome::Reject;
        decision.reasons.push(CampaignReason::EvidenceContradiction);
    }
    decision.reasons.sort_unstable();
    decision.reasons.dedup();
    Ok(CampaignVerdictReceipt {
        schema: super::CAMPAIGN_VERDICT_SCHEMA,
        campaign_id: definition.id,
        run_id: run.id,
        definition_digest: definition_digest.to_owned(),
        source_graph_digest,
        run_digest,
        attempt_inventory_digest: inventory_digest,
        decision,
    })
}

fn repetition_inventory(
    definition: &CampaignDefinition,
    run: &CampaignRun,
    procedures: &BTreeMap<String, MeasurementProcedure>,
) -> Result<(bool, bool), CampaignVerificationError> {
    let mut missing = false;
    let mut extra = false;
    for member in &definition.members {
        let procedure = procedures.get(&member.plan_id).ok_or_else(|| {
            engineering_assurance::campaign::CampaignError::MissingProcedure {
                plan: member.plan_id.clone(),
            }
        })?;
        let recorded: std::collections::BTreeSet<_> = run
            .attempts
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter(|attempt| attempt.member == member.name)
            .map(|attempt| attempt.index)
            .collect();
        missing |= (1..=procedure.repetitions).any(|index| !recorded.contains(&index));
        extra |= recorded.iter().any(|index| *index > procedure.repetitions);
    }
    Ok((missing, extra))
}

#[allow(
    clippy::too_many_arguments,
    reason = "the replay inputs are independent identities"
)]
fn assess_attempt(
    repo: &Path,
    definition: &CampaignDefinition,
    definition_digest: &str,
    run_id: &str,
    attempt: &CampaignAttempt,
    run_attempts: &[CampaignAttempt],
    plans: &[MeasurementPlan],
    procedures: &BTreeMap<String, MeasurementProcedure>,
    source_inputs: &BTreeMap<String, Vec<InputBinding>>,
    plan_source_inputs: &[InputBinding],
) -> AttemptEvidence {
    match check_attempt(
        repo,
        definition,
        definition_digest,
        run_id,
        attempt,
        run_attempts,
        plans,
        procedures,
        source_inputs,
        plan_source_inputs,
    ) {
        Ok(evidence) => evidence,
        Err(EvidenceError::Missing) => AttemptEvidence::Inconclusive,
        Err(EvidenceError::Contradiction) => AttemptEvidence::Reject,
    }
}

#[derive(Clone, Copy, Debug)]
enum EvidenceError {
    Missing,
    Contradiction,
}

fn required(value: Option<&String>) -> Result<&str, EvidenceError> {
    value.map(String::as_str).ok_or(EvidenceError::Missing)
}

fn retained_json(repo: &Path, kind: &'static str, digest: &str) -> Result<Value, EvidenceError> {
    let bytes = read_digest_bytes(repo, kind, digest, "json").map_err(|error| match error {
        CampaignStoreError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
            EvidenceError::Missing
        }
        _ => EvidenceError::Contradiction,
    })?;
    parse_strict_json(&bytes).map_err(|_| EvidenceError::Contradiction)?;
    serde_json::from_slice(&bytes).map_err(|_| EvidenceError::Contradiction)
}

mod attempt;
mod domain;
use attempt::check_attempt;
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, reason = "fixture setup must fail the test")]
    use std::collections::BTreeMap;

    use engineering_assurance::campaign::{CampaignDefinition, CampaignRun, MeasurementProcedure};
    use serde_json::json;

    use super::repetition_inventory;

    /// Trace: FR-114-AC-4
    /// Provenance: PLAT-1043
    #[test]
    fn missing_second_attempt_is_detected_before_aggregate_acceptance() {
        let definition: CampaignDefinition = serde_json::from_value(json!({
            "schemaVersion":"engineering-assurance.campaign-definition/v1",
            "id":"fixture", "subjectName":"fixture", "subjectVersion":"1",
            "sourceGraph":[{"repository":"fixture","revision":"r","digest":"a".repeat(64)}],
            "members":[{"name":"member","planId":"MP-FIXTURE","definitionVersion":"v1","required":true}],
            "completionRule":"all_required"
        })).expect("typed definition");
        let procedure: MeasurementProcedure = serde_json::from_value(json!({
            "schemaVersion":"engineering-assurance.measurement-procedure/v1",
            "producerName":"fixture", "producerVersion":"1", "sourceRepository":"fixture",
            "responseProtocol":"quoin.process-evidence/v1",
            "responseAdapter":"quoin.process-evidence-adapter", "responseAdapterVersion":"1",
            "repetitions":2, "timeoutMillis":1000
        }))
        .expect("typed procedure");
        let run: CampaignRun = serde_json::from_value(json!({
            "schemaVersion":"engineering-assurance.campaign-run/v1", "id":"run",
            "definitionDigest":"b".repeat(64), "sourceGraphDigest":"c".repeat(64),
            "attempts":[{"member":"member","index":1,"status":"invalid_request"}],
            "verdict":"inconclusive"
        }))
        .expect("typed run");
        let procedures = BTreeMap::from([("MP-FIXTURE".to_owned(), procedure)]);
        assert_eq!(
            repetition_inventory(&definition, &run, &procedures).expect("inventory"),
            (true, false)
        );
    }
}
