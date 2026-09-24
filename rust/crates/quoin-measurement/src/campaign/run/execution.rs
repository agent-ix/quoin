// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One bounded producer invocation and exact selected inputs.

use super::CampaignCancellation;
use super::domain::complete_attempt;
use super::{
    BTreeMap, BTreeSet, BindingFailure, CampaignAttempt, CampaignAttemptStatus, CampaignDefinition,
    CampaignRawArtifact, CampaignRunError, ContentDigest, EnvironmentSource, InputBinding,
    InputSource, MeasurementPlan, MeasurementProcedure, Path, ProcessEvidenceAdapter,
    ProcessEvidenceObservation, ProducerExecutionResult, ProducerExecutionState, ProducerExecutor,
    Read, RunMemberBindings, SourceTreeBinding, VerifiedSource, read_bounded, read_digest_bytes,
    resolve_procedure, retain_bytes, retain_value,
};
use crate::campaign::input_origin::selected_declarations;

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one invocation records its request, result, artifacts, and checker handoff"
)]
pub(super) fn run_member(
    repo: &Path,
    definition: &CampaignDefinition,
    definition_digest: &str,
    run_id: &str,
    member: &engineering_assurance::campaign::CampaignMember,
    plan: &MeasurementPlan,
    procedure: &MeasurementProcedure,
    runtime: &RunMemberBindings,
    source: &VerifiedSource,
    plan_source: &VerifiedSource,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[CampaignAttempt],
    executor: &ProducerExecutor,
    cancellation: &CampaignCancellation,
    index: i64,
) -> Result<CampaignAttempt, CampaignRunError> {
    let mut attempt = CampaignAttempt {
        checker_request_digest: None,
        checker_result_digest: None,
        collection_digest: None,
        collection_id: None,
        domain_verdict_digest: None,
        index,
        member: member.name.clone(),
        raw_artifacts: None,
        reason: None,
        request_digest: None,
        result_digest: None,
        status: CampaignAttemptStatus::InvalidRequest,
        verdict_digest: None,
    };
    let staging =
        tempfile::tempdir().map_err(|error| CampaignRunError::execution(error.to_string()))?;
    source.stage_into(staging.path())?;
    let inputs = match select_inputs(
        repo,
        member,
        runtime,
        source,
        sources,
        prior,
        staging.path(),
    ) {
        Ok(inputs) => inputs,
        Err(CampaignRunError::Binding(_)) => {
            attempt.reason = Some("invalid_input_binding".to_owned());
            return Ok(attempt);
        }
        Err(error) => return Err(error),
    };
    let mut bindings = runtime.producer.clone();
    bindings.capability_root = staging.path().to_string_lossy().into_owned();
    bindings.inputs = inputs;
    bindings.input_origins = selected_declarations(&runtime.inputs, procedure);
    bindings.source_tree = Some(SourceTreeBinding {
        repository: source.repository.clone(),
        manifest: source.manifest.clone(),
    });
    for (name, selected) in &runtime.environment_sources {
        if !bindings.environment.contains_key(name) {
            attempt.reason = Some("undeclared_source_environment".to_owned());
            return Ok(attempt);
        }
        let value = match selected {
            EnvironmentSource::SourceRevision(repository) => {
                let Some(source) = definition
                    .source_graph
                    .iter()
                    .find(|entry| entry.repository == *repository)
                else {
                    attempt.reason = Some("environment_source_missing".to_owned());
                    return Ok(attempt);
                };
                source.revision.clone()
            }
            EnvironmentSource::CleanSourceState => "clean".to_owned(),
        };
        bindings.environment.insert(name.clone(), value);
    }
    let resolved = match resolve_procedure(procedure, &definition.source_graph, bindings) {
        Ok(resolved) => resolved,
        Err(_error) => {
            attempt.reason = Some("invalid_request".to_owned());
            return Ok(attempt);
        }
    };
    let adapter = ProcessEvidenceAdapter::new(
        resolved.request.response.protocol.clone(),
        resolved.request.response.adapter.clone(),
    )
    .map_err(|error| CampaignRunError::binding(error.to_string()))?;
    let request_digest = resolved.identity.digest.as_str().to_owned();
    let request_value = serde_json::to_value(&resolved.request)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let (stored_request, _) = retain_value(repo, "requests", &request_value)?;
    if stored_request != request_digest {
        return Err(CampaignRunError::identity(
            "EA request JCS identity".to_owned(),
        ));
    }
    attempt.request_digest = Some(request_digest);
    let executed = {
        let active = cancellation.register(resolved.request.cancellation.clone());
        executor.execute(&resolved.request, active.token(), &adapter)
    };
    let result = match executed {
        Ok(result) => result,
        Err(_error) => {
            attempt.reason = Some("invalid_request".to_owned());
            return Ok(attempt);
        }
    };
    let result_digest = result
        .identity()
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?
        .digest
        .as_str()
        .to_owned();
    let result_value = serde_json::to_value(&result)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let (stored_result, _) = retain_value(repo, "results", &result_value)?;
    if stored_result != result_digest {
        return Err(CampaignRunError::identity(
            "EA result JCS identity".to_owned(),
        ));
    }
    let artifacts = retain_artifacts(repo, &result)?;
    let status = status_of(&result.state);
    attempt.raw_artifacts = Some(artifacts);
    attempt.result_digest = Some(result_digest);
    attempt.status = status;
    if attempt.status == CampaignAttemptStatus::Completed
        && let Err(error) = complete_attempt(
            repo,
            definition,
            definition_digest,
            run_id,
            member,
            plan,
            runtime,
            procedure,
            source,
            plan_source,
            sources,
            prior,
            executor,
            cancellation,
            &result,
            &mut attempt,
        )
    {
        match error {
            CampaignRunError::Measurement(_)
            | CampaignRunError::Binding(BindingFailure::InvalidConfiguration(_)) => {
                attempt.reason = Some("measurement_or_checker_refused".to_owned());
            }
            other => return Err(other),
        }
    }
    Ok(attempt)
}

#[allow(
    clippy::too_many_arguments,
    reason = "selected inputs bind exact member, source, dependency, and staging identities"
)]
fn select_inputs(
    repo: &Path,
    member: &engineering_assurance::campaign::CampaignMember,
    runtime: &RunMemberBindings,
    source: &VerifiedSource,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[CampaignAttempt],
    root: &Path,
) -> Result<Vec<InputBinding>, CampaignRunError> {
    let mut paths = BTreeSet::new();
    let mut roles = BTreeSet::new();
    runtime
        .inputs
        .iter()
        .map(|selected| {
            let relative = Path::new(&selected.path);
            if selected.path.is_empty()
                || relative.is_absolute()
                || relative
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
                || !paths.insert(selected.path.as_str())
                || !roles.insert(selected.role.as_str())
                || source.has_link_ancestor(&selected.path)
            {
                return Err(CampaignRunError::binding(
                    "unsafe or duplicate producer input".to_owned(),
                ));
            }
            let bytes = match &selected.source {
                InputSource::File(path) => read_bounded(path)?,
                InputSource::SourceFile { repository, path } => {
                    let source = sources.get(repository).ok_or_else(|| {
                        CampaignRunError::binding("selected source repository".to_owned())
                    })?;
                    if !source.contains_path(path) {
                        return Err(CampaignRunError::binding(
                            "selected source file is not tracked".to_owned(),
                        ));
                    }
                    source.read_tracked_file(path)?
                }
                InputSource::Dependency {
                    member: dependency,
                    index,
                    artifact_role,
                } => {
                    if !member
                        .depends_on
                        .as_deref()
                        .unwrap_or(&[])
                        .contains(dependency)
                    {
                        return Err(CampaignRunError::binding(
                            "undeclared input dependency".to_owned(),
                        ));
                    }
                    let artifact = prior
                        .iter()
                        .find(|attempt| attempt.member == *dependency && attempt.index == *index)
                        .and_then(|attempt| attempt.raw_artifacts.as_deref())
                        .and_then(|items| {
                            items
                                .iter()
                                .find(|artifact| artifact.role == *artifact_role)
                        })
                        .ok_or_else(|| {
                            CampaignRunError::binding("dependency artifact unavailable".to_owned())
                        })?;
                    read_digest_bytes(repo, "raw", &artifact.digest, "bin")?
                }
            };
            let path = root.join(&selected.path);
            if path.exists() {
                return Err(CampaignRunError::binding(
                    "producer input collides with tracked source".to_owned(),
                ));
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| CampaignRunError::execution(error.to_string()))?;
            }
            std::fs::write(&path, &bytes)
                .map_err(|error| CampaignRunError::execution(error.to_string()))?;
            retain_bytes(repo, "inputs", "bin", &bytes)?;
            Ok(InputBinding {
                role: selected.role.clone(),
                path: selected.path.clone(),
                digest: ContentDigest::of_bytes(&bytes),
                executable: selected.executable,
            })
        })
        .collect()
}

fn retain_artifacts(
    repo: &Path,
    result: &ProducerExecutionResult<ProcessEvidenceObservation>,
) -> Result<Vec<CampaignRawArtifact>, CampaignRunError> {
    result
        .artifacts
        .iter()
        .map(|artifact| {
            let mut reader = artifact
                .try_reader()
                .map_err(|error| CampaignRunError::execution(error.to_string()))?;
            let mut bytes = Vec::new();
            reader
                .by_ref()
                .take(crate::campaign::store::MAX_CAMPAIGN_EVIDENCE_BYTES.saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|error| CampaignRunError::execution(error.to_string()))?;
            let (digest, _) = retain_bytes(repo, "raw", "bin", &bytes)?;
            if digest != artifact.digest.as_str() {
                return Err(CampaignRunError::identity(
                    "EA output artifact digest".to_owned(),
                ));
            }
            Ok(CampaignRawArtifact {
                role: artifact.role.clone(),
                digest,
            })
        })
        .collect()
}

fn status_of(state: &ProducerExecutionState<ProcessEvidenceObservation>) -> CampaignAttemptStatus {
    match state {
        ProducerExecutionState::Completed { .. } => CampaignAttemptStatus::Completed,
        ProducerExecutionState::Unavailable => CampaignAttemptStatus::Unavailable,
        ProducerExecutionState::Refused { .. } => CampaignAttemptStatus::Refused,
        ProducerExecutionState::Failed { .. } => CampaignAttemptStatus::Failed,
        ProducerExecutionState::TimedOut => CampaignAttemptStatus::TimedOut,
        ProducerExecutionState::MalformedResponse => CampaignAttemptStatus::MalformedResponse,
        ProducerExecutionState::ContainmentFailure => CampaignAttemptStatus::ContainmentFailure,
        ProducerExecutionState::Cancelled => CampaignAttemptStatus::Cancelled,
    }
}
