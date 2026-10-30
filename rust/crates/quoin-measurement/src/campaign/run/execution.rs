// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One bounded producer invocation and exact selected inputs.

use super::CampaignCancellation;
use super::Retained;
use super::domain::complete_attempt;
use super::{
    AttemptEvidence, BTreeMap, BTreeSet, BindingFailure, CampaignAttempt, CampaignAttemptStatus,
    CampaignDefinition, CampaignRawArtifact, CampaignRunError, ContentDigest, EnvironmentSource,
    InputBinding, InputSource, MeasurementPlan, MeasurementProcedure, Path, ProcessEvidenceAdapter,
    ProcessEvidenceObservation, ProducerExecutionResult, ProducerExecutionState, ProducerExecutor,
    Read, RunMemberBindings, SourceTreeBinding, VerifiedSource, encode_value, read_bounded,
    resolve_procedure,
};
use crate::campaign::input_origin::selected_declarations;

fn unretained(attempt: CampaignAttempt) -> Retained {
    Retained {
        attempt,
        request: Vec::new(),
        result: Vec::new(),
        artifacts: Vec::new(),
        evidence: AttemptEvidence::Inconclusive,
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one invocation records its request, result, artifacts, and checker handoff"
)]
pub(super) fn run_member(
    repo: &Path,
    definition: &CampaignDefinition,
    run_id: &str,
    member: &engineering_assurance::campaign::CampaignMember,
    plan: &MeasurementPlan,
    procedure: &MeasurementProcedure,
    runtime: &RunMemberBindings,
    source: &VerifiedSource,
    plan_source: &VerifiedSource,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[Retained],
    executor: &ProducerExecutor,
    cancellation: &CampaignCancellation,
    index: i64,
) -> Result<Retained, CampaignRunError> {
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
    let inputs = match select_inputs(member, runtime, source, sources, prior) {
        Ok(inputs) => inputs,
        Err(CampaignRunError::Binding(_)) => {
            attempt.reason = Some("invalid_input_binding".to_owned());
            return Ok(unretained(attempt));
        }
        Err(error) => return Err(error),
    };
    let mut bindings = runtime.producer.clone();
    bindings.capability_root = source.checkout.to_string_lossy().into_owned();
    bindings.inputs = inputs;
    bindings.input_origins = selected_declarations(&runtime.inputs, procedure);
    bindings.source_tree = Some(SourceTreeBinding {
        repository: source.repository.clone(),
        manifest: source.manifest.clone(),
    });
    for (name, selected) in &runtime.environment_sources {
        if !bindings.environment.contains_key(name) {
            attempt.reason = Some("undeclared_source_environment".to_owned());
            return Ok(unretained(attempt));
        }
        let value = match selected {
            EnvironmentSource::SourceRevision(repository) => {
                let Some(source) = definition
                    .source_graph
                    .iter()
                    .find(|entry| entry.repository == *repository)
                else {
                    attempt.reason = Some("environment_source_missing".to_owned());
                    return Ok(unretained(attempt));
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
            return Ok(unretained(attempt));
        }
    };
    let adapter = ProcessEvidenceAdapter::new(
        resolved.request.response.protocol.clone(),
        resolved.request.response.adapter.clone(),
    )
    .map_err(|error| CampaignRunError::binding(error.to_string()))?;
    let request_value = serde_json::to_value(&resolved.request)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let request_bytes = encode_value(repo, &request_value)?;
    let executed = {
        let active = cancellation.register(resolved.request.cancellation.clone());
        executor.execute(&resolved.request, active.token(), &adapter)
    };
    let result = match executed {
        Ok(result) => result,
        Err(_error) => {
            attempt.reason = Some("invalid_request".to_owned());
            return Ok(unretained(attempt));
        }
    };
    let result_value = serde_json::to_value(&result)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let result_bytes = encode_value(repo, &result_value)?;
    let (artifacts, artifact_bytes) = retain_artifacts(&result)?;
    attempt.raw_artifacts = Some(artifacts);
    attempt.status = status_of(&result.state);
    let mut retained = Retained {
        attempt,
        request: request_bytes,
        result: result_bytes,
        artifacts: artifact_bytes,
        evidence: AttemptEvidence::Inconclusive,
    };
    if retained.attempt.status == CampaignAttemptStatus::Completed
        && let Err(error) = complete_attempt(
            repo,
            definition,
            run_id,
            member,
            plan,
            runtime,
            procedure,
            plan_source,
            sources,
            prior,
            executor,
            cancellation,
            &result,
            &mut retained,
        )
    {
        match error {
            CampaignRunError::Measurement(_)
            | CampaignRunError::Binding(BindingFailure::InvalidConfiguration(_)) => {
                retained.attempt.reason = Some("measurement_or_checker_refused".to_owned());
            }
            other => return Err(other),
        }
    }
    Ok(retained)
}

fn select_inputs(
    member: &engineering_assurance::campaign::CampaignMember,
    runtime: &RunMemberBindings,
    source: &VerifiedSource,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[Retained],
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
                    prior
                        .iter()
                        .find(|retained| {
                            retained.attempt.member == *dependency
                                && retained.attempt.index == *index
                        })
                        .and_then(|retained| {
                            retained
                                .artifacts
                                .iter()
                                .find(|(role, _)| role == artifact_role)
                        })
                        .map(|(_, bytes)| bytes.clone())
                        .ok_or_else(|| {
                            CampaignRunError::binding("dependency artifact unavailable".to_owned())
                        })?
                }
            };
            let path = source.checkout.join(&selected.path);
            if source.contains_path(&selected.path) {
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
            Ok(InputBinding {
                role: selected.role.clone(),
                path: selected.path.clone(),
                // BLOCKED-BY QSL-1275 (owner: planner titan:QSL:plan): EA InputBinding.digest is required by the producer request at rev 08bbf1cc.
                digest: ContentDigest::of_bytes(&bytes),
                executable: selected.executable,
            })
        })
        .collect()
}

#[allow(
    clippy::type_complexity,
    reason = "the EA artifact record and its exact bytes travel together"
)]
fn retain_artifacts(
    result: &ProducerExecutionResult<ProcessEvidenceObservation>,
) -> Result<(Vec<CampaignRawArtifact>, Vec<(String, Vec<u8>)>), CampaignRunError> {
    let mut records = Vec::new();
    let mut retained = Vec::new();
    for artifact in &result.artifacts {
        let mut reader = artifact
            .try_reader()
            .map_err(|error| CampaignRunError::execution(error.to_string()))?;
        let mut bytes = Vec::new();
        reader
            .by_ref()
            .take(crate::campaign::store::MAX_CAMPAIGN_EVIDENCE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| CampaignRunError::execution(error.to_string()))?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX)
            > crate::campaign::store::MAX_CAMPAIGN_EVIDENCE_BYTES
        {
            return Err(CampaignRunError::execution(
                "output artifact exceeds the evidence size ceiling".to_owned(),
            ));
        }
        records.push(CampaignRawArtifact {
            role: artifact.role.clone(),
            // BLOCKED-BY QSL-1275 (owner: planner titan:QSL:plan): EA CampaignRawArtifact.digest is a required field at rev 08bbf1cc; the value is EA's own.
            digest: artifact.digest.as_str().to_owned(),
        });
        retained.push((artifact.role.clone(), bytes));
    }
    Ok((records, retained))
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

#[cfg(test)]
mod tests {
    use super::*;
    use engineering_assurance::producer_execution::{ExecutionFailure, ExecutionRefusal};

    /// Trace: FR-114-AC-2, TC-1941. Every EA terminal state maps to its
    /// distinct retained campaign attempt status, including adapter refusal.
    #[test]
    fn tc_1941_typed_ea_terminal_states_map_to_attempt_statuses() {
        let cases = [
            (
                ProducerExecutionState::Completed {
                    observation: ProcessEvidenceObservation {
                        schema_version: crate::campaign::adapter::PROCESS_EVIDENCE_PROTOCOL,
                    },
                },
                CampaignAttemptStatus::Completed,
            ),
            (
                ProducerExecutionState::Unavailable,
                CampaignAttemptStatus::Unavailable,
            ),
            (
                ProducerExecutionState::Refused {
                    reason: ExecutionRefusal::ExecutableIdentity,
                },
                CampaignAttemptStatus::Refused,
            ),
            (
                ProducerExecutionState::Failed {
                    reason: ExecutionFailure::StdoutTooLarge,
                },
                CampaignAttemptStatus::Failed,
            ),
            (
                ProducerExecutionState::TimedOut,
                CampaignAttemptStatus::TimedOut,
            ),
            (
                ProducerExecutionState::MalformedResponse,
                CampaignAttemptStatus::MalformedResponse,
            ),
            (
                ProducerExecutionState::ContainmentFailure,
                CampaignAttemptStatus::ContainmentFailure,
            ),
            (
                ProducerExecutionState::Cancelled,
                CampaignAttemptStatus::Cancelled,
            ),
        ];
        for (state, expected) in cases {
            assert_eq!(status_of(&state), expected);
        }
    }
}
