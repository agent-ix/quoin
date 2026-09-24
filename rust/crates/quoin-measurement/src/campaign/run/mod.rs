// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Sequential generic EA campaign execution and immutable attempt intake.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use engineering_assurance::campaign::{
    CAMPAIGN_RUN_VERSION, CampaignAttempt, CampaignAttemptStatus, CampaignDefinition,
    CampaignRawArtifact, CampaignRun, CampaignVerdict, MeasurementProcedure, PlanRegistration,
    ProcedureBindings, SourceTreeBinding, canonical_digest, resolve_procedure, validate_definition,
    validate_run,
};
use engineering_assurance::producer_execution::{
    ContentDigest, InputBinding, ProducerExecutionResult, ProducerExecutionState, ProducerExecutor,
};
use thiserror::Error;

use super::adapter::{ProcessEvidenceAdapter, ProcessEvidenceObservation};
use super::checker::{
    CHECKER_DEFINITION_PATH, CHECKER_INPUT_PATH, CHECKER_RAW_BUNDLE_PATH, CHECKER_REQUEST_PATH,
    CHECKER_RESULT_PATH, DOMAIN_CHECK_INPUT_SCHEMA, DependencyResultInput, DomainCheckInput,
    DomainVerdictReceipt, RawArtifactInput, assess_receipt, raw_artifact_bundle,
    staged_dependency_path, staged_dependency_raw_path,
};
use super::source::{SourceError, VerifiedSource, verify_source_graph};
use super::store::{
    CampaignStoreError, read_bounded, read_digest_bytes, read_typed, retain_bytes,
    retain_json_bytes, retain_value, run_path,
};
use super::{Attempt, AttemptEvidence, CampaignOutcome, Member, all_required};
use crate::plans::load_selected_measurement_plans;
use crate::source::DiskMeasurement;
use crate::types::plan::MeasurementPlan;
use crate::verify::{OrderSource, Ranked, TamperFacts, Verdict, verdict_json, verify};

/// One explicit producer input, selected from a local file or prior artifact.
#[derive(Clone, Debug)]
pub struct SelectedInput {
    /// Authored procedure input role.
    pub role: String,
    /// Relative path EA will stage beneath the invocation root.
    pub path: String,
    /// Preserve execution mode for a dependent executable input.
    pub executable: bool,
    /// Where the exact input bytes come from.
    pub source: InputSource,
}

/// A file or a prior member's sealed output artifact.
#[derive(Clone, Debug)]
pub enum InputSource {
    /// One exact local regular file, selected before bounded execution.
    File(PathBuf),
    /// A tracked file from one exact verified campaign source repository.
    SourceFile {
        /// Repository key in the campaign source graph.
        repository: String,
        /// Normal path in that repository's Git tree.
        path: String,
    },
    /// A previous member attempt's retained artifact by role.
    Dependency {
        /// Declared dependency member.
        member: String,
        /// Positive attempt index.
        index: i64,
        /// Exact EA output artifact role.
        artifact_role: String,
    },
}

/// A declared runtime environment variable derived from verified sources.
#[derive(Clone, Debug)]
pub enum EnvironmentSource {
    /// Exact clean commit revision of one campaign repository.
    SourceRevision(String),
    /// The literal `clean`, granted only after source verification.
    CleanSourceState,
}

/// Machine-selected bindings for one named campaign member.
#[derive(Clone, Debug)]
pub struct RunMemberBindings {
    /// Base EA producer bindings. Source tree, root and inputs are filled here.
    pub producer: ProcedureBindings,
    /// Base EA checker bindings when the member declares a checker procedure.
    pub checker: Option<ProcedureBindings>,
    /// Explicit inputs selected for the producer procedure.
    pub inputs: Vec<SelectedInput>,
    /// Caller-provided RFC 3339 UTC timestamp for collections.
    pub timestamp: String,
    /// Actual Node/Rust/Python toolchain identities under the existing store contract.
    pub toolchains: BTreeMap<String, String>,
    /// Exact remote display names for the source graph's repositories.
    pub source_remotes: BTreeMap<String, String>,
    /// Runtime environment values selected from checked source identities.
    pub environment_sources: BTreeMap<String, EnvironmentSource>,
}

/// A campaign cannot be executed or retained faithfully.
#[derive(Debug, Error)]
pub enum CampaignRunError {
    /// A selected source checkout cannot prove the authored Git tree.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// Retained record I/O or identity failed.
    #[error(transparent)]
    Store(#[from] CampaignStoreError),
    /// EA refused the generated campaign or request structure.
    #[error(transparent)]
    Ea(#[from] engineering_assurance::campaign::CampaignError),
    /// A `MeasurementPlan` or collection failed its own intake contract.
    #[error(transparent)]
    Measurement(MeasurementFailure),
    /// A required machine binding is missing or contradictory.
    #[error(transparent)]
    Binding(BindingFailure),
    /// Bounded EA executor could not be constructed or rejected an invalid request.
    #[error(transparent)]
    Execution(ExecutionFailure),
    /// Typed record could not be represented in JSON.
    #[error(transparent)]
    Encoding(EncodingFailure),
}

/// Stable measurement-intake refusal category.
#[derive(Debug, Error)]
pub enum MeasurementFailure {
    /// A source-bound plan or generated collection failed its typed contract.
    #[error("measurement plan or collection refused: {0}")]
    Invalid(String),
}

/// Stable campaign binding refusal categories.
#[derive(Debug, Error)]
pub enum BindingFailure {
    /// Required configuration or source-bound selection was absent or invalid.
    #[error("invalid campaign binding: {0}")]
    InvalidConfiguration(String),
    /// Collection intake requires at least one supported, nonempty toolchain identity.
    #[error("campaign member {member} has invalid toolchain identities")]
    InvalidToolchains {
        /// Campaign member whose runtime toolchain inventory is invalid.
        member: String,
    },
    /// A retained request/result/artifact identity differed from the selected bytes.
    #[error("campaign identity mismatch: {0}")]
    IdentityMismatch(String),
}

/// Stable EA execution or staging refusal category.
#[derive(Debug, Error)]
pub enum ExecutionFailure {
    /// A bounded executor, staging operation, or artifact reader failed.
    #[error("bounded campaign execution failed: {0}")]
    Failed(String),
}

/// Stable typed-record encoding refusal category.
#[derive(Debug, Error)]
pub enum EncodingFailure {
    /// A typed campaign record could not be encoded canonically.
    #[error("campaign record encoding failed: {0}")]
    Failed(String),
}

impl CampaignRunError {
    fn measurement(detail: String) -> Self {
        Self::Measurement(MeasurementFailure::Invalid(detail))
    }

    fn binding(detail: String) -> Self {
        Self::Binding(BindingFailure::InvalidConfiguration(detail))
    }

    fn identity(detail: String) -> Self {
        Self::Binding(BindingFailure::IdentityMismatch(detail))
    }

    fn execution(detail: String) -> Self {
        Self::Execution(ExecutionFailure::Failed(detail))
    }

    fn encoding(detail: String) -> Self {
        Self::Encoding(EncodingFailure::Failed(detail))
    }
}

/// Run every member sequentially in dependency order using only EA's bounded
/// direct executor. Every invocation receives one retained attempt record.
///
/// # Errors
/// Refuses malformed campaign/config/source before publication, and any store
/// failure. A valid failed process is retained as an inconclusive attempt.
pub fn run_campaign(
    repo: &Path,
    definition: &CampaignDefinition,
    run_id: &str,
    source_checkouts: &BTreeMap<String, PathBuf>,
    bindings: &BTreeMap<String, RunMemberBindings>,
) -> Result<CampaignRun, CampaignRunError> {
    run_campaign_with_cancellation(
        repo,
        definition,
        run_id,
        source_checkouts,
        bindings,
        &CampaignCancellation::new(),
    )
}

/// Run a campaign with a caller-controlled event that can cancel an active
/// producer or checker through its exact EA request binding.
///
/// # Errors
/// Refuses invalid source, bindings, retained evidence or durable publication.
#[allow(
    clippy::too_many_lines,
    reason = "sequential member inventory and publication are one transaction"
)]
pub fn run_campaign_with_cancellation(
    repo: &Path,
    definition: &CampaignDefinition,
    run_id: &str,
    source_checkouts: &BTreeMap<String, PathBuf>,
    bindings: &BTreeMap<String, RunMemberBindings>,
    cancellation: &CampaignCancellation,
) -> Result<CampaignRun, CampaignRunError> {
    let publication_path = run_path(repo, run_id)?;
    match fs::symlink_metadata(&publication_path) {
        Ok(_) => {
            return Err(CampaignRunError::binding(
                "campaign run already published".to_owned(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(CampaignStoreError::Io {
                path: publication_path,
                source: error,
            }
            .into());
        }
    }
    let sources = verify_source_graph(definition, source_checkouts)?;
    let own_root = repo
        .canonicalize()
        .map_err(|error| CampaignRunError::binding(format!("campaign repository: {error}")))?;
    let own_source = sources
        .values()
        .find(|source| source.checkout.canonicalize().ok().as_ref() == Some(&own_root))
        .ok_or_else(|| {
            CampaignRunError::binding("campaign plan repository is not in sourceGraph".to_owned())
        })?;
    let plan_root =
        tempfile::tempdir().map_err(|error| CampaignRunError::execution(error.to_string()))?;
    own_source.stage_into(plan_root.path())?;
    let selected_ids = definition
        .members
        .iter()
        .map(|member| member.plan_id.as_str())
        .collect::<BTreeSet<_>>();
    let plans =
        load_selected_measurement_plans(&DiskMeasurement::new(plan_root.path()), &selected_ids)
            .map_err(|error| CampaignRunError::measurement(error.to_string()))?;
    let procedures = load_procedures(plan_root.path(), &plans)?;
    for plan in &plans {
        if !own_source.contains_path(&plan.path)
            || plan
                .execution_procedure
                .as_deref()
                .is_some_and(|path| !own_source.contains_path(path))
        {
            return Err(CampaignRunError::binding(
                "plan or procedure is not in exact source tree".to_owned(),
            ));
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
    validate_definition(definition, &registrations)?;
    if bindings.len() != definition.members.len()
        || definition
            .members
            .iter()
            .any(|member| !bindings.contains_key(&member.name))
    {
        return Err(CampaignRunError::binding(
            "member binding inventory".to_owned(),
        ));
    }
    for (member, runtime) in bindings {
        if !valid_toolchains(&runtime.toolchains) {
            return Err(CampaignRunError::Binding(
                BindingFailure::InvalidToolchains {
                    member: member.clone(),
                },
            ));
        }
    }
    let definition_value = serde_json::to_value(definition)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let definition_digest = canonical_digest(definition)?.as_str().to_owned();
    let source_graph_digest = canonical_digest(&definition.source_graph)?
        .as_str()
        .to_owned();
    let (retained_digest, _) = retain_value(repo, "definitions", &definition_value)?;
    if retained_digest != definition_digest {
        return Err(CampaignRunError::identity(
            "definition JCS identity".to_owned(),
        ));
    }
    let total_attempts = definition
        .members
        .iter()
        .try_fold(0_usize, |total, member| {
            let procedure = procedures
                .get(member.plan_id.as_str())
                .ok_or_else(|| CampaignRunError::binding("member procedure".to_owned()))?;
            let repetitions = usize::try_from(procedure.repetitions)
                .map_err(|_| CampaignRunError::binding("procedure repetitions".to_owned()))?;
            total
                .checked_add(repetitions)
                .ok_or_else(|| CampaignRunError::binding("campaign attempt population".to_owned()))
        })?;
    let checkpointed = checkpoint::load_prefix(
        repo,
        run_id,
        &definition_digest,
        &source_graph_digest,
        total_attempts,
    )?;
    checkpoint::validate_prefix(
        repo,
        definition,
        &definition_digest,
        run_id,
        &checkpointed,
        &plans,
        &procedures,
        &sources,
        own_source,
    )?;
    let executor =
        ProducerExecutor::new(1).map_err(|error| CampaignRunError::execution(error.to_string()))?;
    let mut attempts = Vec::new();
    let mut ordinal = 0_usize;
    let mut completed = BTreeSet::new();
    while completed.len() < definition.members.len() {
        let Some(member) = definition.members.iter().find(|member| {
            !completed.contains(member.name.as_str())
                && member
                    .depends_on
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .all(|dependency| completed.contains(dependency.as_str()))
        }) else {
            return Err(CampaignRunError::binding("dependency order".to_owned()));
        };
        let plan = plans
            .iter()
            .find(|plan| plan.id.as_str() == member.plan_id)
            .ok_or_else(|| CampaignRunError::binding("member plan".to_owned()))?;
        let procedure = procedures
            .get(member.plan_id.as_str())
            .ok_or_else(|| CampaignRunError::binding("member procedure".to_owned()))?;
        let runtime = bindings
            .get(&member.name)
            .ok_or_else(|| CampaignRunError::binding("member runtime".to_owned()))?;
        let source = sources
            .get(&procedure.source_repository)
            .ok_or_else(|| CampaignRunError::binding("procedure source".to_owned()))?;
        for index in 1..=procedure.repetitions {
            ordinal = ordinal
                .checked_add(1)
                .ok_or_else(|| CampaignRunError::binding("campaign attempt ordinal".to_owned()))?;
            if let Some(prior) = checkpointed.get(ordinal - 1) {
                if prior.member != member.name || prior.index != index {
                    return Err(CampaignRunError::binding(
                        "campaign checkpoint schedule".to_owned(),
                    ));
                }
                attempts.push(prior.clone());
            } else {
                let attempt = run_member(
                    repo,
                    definition,
                    &definition_digest,
                    run_id,
                    member,
                    plan,
                    procedure,
                    runtime,
                    source,
                    own_source,
                    &sources,
                    &attempts,
                    &executor,
                    cancellation,
                    index,
                )?;
                checkpoint::publish(
                    repo,
                    run_id,
                    &definition_digest,
                    &source_graph_digest,
                    ordinal,
                    &attempt,
                )?;
                attempts.push(attempt);
            }
        }
        completed.insert(member.name.as_str());
    }
    let views: Vec<Attempt<'_>> = attempts
        .iter()
        .filter_map(|attempt| {
            u64::try_from(attempt.index).ok().map(|index| Attempt {
                member: &attempt.member,
                index,
                completed: attempt.status == CampaignAttemptStatus::Completed,
                evidence: evidence_of_attempt(repo, attempt),
            })
        })
        .collect();
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
    let campaign_verdict = match all_required(&members, &views).verdict {
        CampaignOutcome::Accept => CampaignVerdict::Accepted,
        CampaignOutcome::Reject => CampaignVerdict::Rejected,
        CampaignOutcome::Inconclusive => CampaignVerdict::Inconclusive,
    };
    let run = CampaignRun {
        attempts: Some(attempts),
        definition_digest,
        id: run_id.to_owned(),
        schema_version: CAMPAIGN_RUN_VERSION.to_owned(),
        source_graph_digest,
        verdict: campaign_verdict,
    };
    validate_run(&run, definition)?;
    let run_value = serde_json::to_value(&run)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let canonical = crate::json_bridge::from_serde(&run_value)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))
        .and_then(|value| {
            quoin_store::canonical_bytes(&value)
                .map_err(|error| CampaignRunError::encoding(error.to_string()))
        })?;
    quoin_store::store::write_content_addressed(&publication_path, &canonical)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    Ok(run)
}

fn load_procedures(
    repo: &Path,
    plans: &[MeasurementPlan],
) -> Result<BTreeMap<String, MeasurementProcedure>, CampaignRunError> {
    let mut found = BTreeMap::new();
    for plan in plans {
        if let Some(path) = &plan.execution_procedure {
            let procedure: MeasurementProcedure = read_typed(&repo.join(path))?;
            found.insert(plan.id.as_str().to_owned(), procedure);
        }
    }
    Ok(found)
}

mod cancellation;
mod checkpoint;
mod collection;
mod domain;
mod evidence;
mod execution;
mod preflight;
pub use cancellation::CampaignCancellation;
use evidence::evidence_of_attempt;
use execution::run_member;
use preflight::valid_toolchains;
