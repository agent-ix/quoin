// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One independently bounded checker invocation and receipt.

use super::collection::publish_collection;
use super::{
    AttemptEvidence, BTreeMap, CHECKER_DEFINITION_PATH, CHECKER_INPUT_PATH,
    CHECKER_RAW_BUNDLE_PATH, CHECKER_REQUEST_PATH, CHECKER_RESULT_PATH, CampaignCancellation,
    CampaignDefinition, CampaignRunError, CampaignStoreError, DOMAIN_CHECK_INPUT_SCHEMA,
    DependencyResultInput, DomainCheckInput, DomainVerdictReceipt, InputBinding, MeasurementPlan,
    Path, ProcessEvidenceAdapter, ProcessEvidenceObservation, ProducerExecutionResult,
    ProducerExecutionState, ProducerExecutor, RawArtifactInput, Read, Retained, RunMemberBindings,
    SourceTreeBinding, VerifiedSource, assess_receipt, encode_value, raw_artifact_bundle,
    resolve_procedure, staged_dependency_path, staged_dependency_raw_path,
    staged_dependency_request_path,
};

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the checker completion binds one chain of selected inputs"
)]
pub(super) fn complete_attempt(
    repo: &Path,
    definition: &CampaignDefinition,
    run_id: &str,
    member: &engineering_assurance::campaign::CampaignMember,
    plan: &MeasurementPlan,
    runtime: &RunMemberBindings,
    procedure: &engineering_assurance::campaign::MeasurementProcedure,
    plan_source: &VerifiedSource,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[Retained],
    executor: &ProducerExecutor,
    cancellation: &CampaignCancellation,
    result: &ProducerExecutionResult<ProcessEvidenceObservation>,
    retained: &mut Retained,
) -> Result<(), CampaignRunError> {
    let verdict = match check_domain_attempt(
        repo,
        definition,
        member,
        runtime,
        sources,
        prior,
        executor,
        cancellation,
        retained,
    ) {
        Ok(verdict) => verdict,
        Err(error) => {
            retained.attempt.reason = Some(format!("checker_stage_unavailable:{error}"));
            AttemptEvidence::Inconclusive
        }
    };
    publish_collection(
        repo,
        definition,
        run_id,
        member,
        plan,
        runtime,
        procedure,
        plan_source,
        result,
        retained,
        verdict,
    )
    .map_err(|error| {
        // `run_member` has a legacy handler for measurement and configuration
        // refusals. A failed collection publication must still escape that
        // handler: FR-114-AC-2 requires a retained collection per completion.
        CampaignRunError::Store(CampaignStoreError::Store {
            path: crate::store::measurements_root(repo),
            message: error.to_string(),
        })
    })
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the checker completion binds one chain of selected inputs"
)]
fn check_domain_attempt(
    repo: &Path,
    definition: &CampaignDefinition,
    member: &engineering_assurance::campaign::CampaignMember,
    runtime: &RunMemberBindings,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[Retained],
    executor: &ProducerExecutor,
    cancellation: &CampaignCancellation,
    retained: &mut Retained,
) -> Result<AttemptEvidence, CampaignRunError> {
    let Some(checker_procedure) = &member.checker_procedure else {
        retained.attempt.reason = Some("independent_checker_unavailable".to_owned());
        return Ok(AttemptEvidence::Inconclusive);
    };
    let Some(checker_runtime) = runtime.checker.clone() else {
        retained.attempt.reason = Some("independent_checker_unavailable".to_owned());
        return Ok(AttemptEvidence::Inconclusive);
    };
    let checker_source = sources
        .get(&checker_procedure.source_repository)
        .ok_or_else(|| CampaignRunError::binding("checker source missing".to_owned()))?;
    let definition_value = serde_json::to_value(definition)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let definition_bytes = encode_value(repo, &definition_value)?;
    let mut selected_inputs = vec![
        stage_checker_input(
            checker_source,
            "definition",
            CHECKER_DEFINITION_PATH,
            &definition_bytes,
        )?,
        stage_checker_input(
            checker_source,
            "result",
            CHECKER_RESULT_PATH,
            &retained.result,
        )?,
        stage_checker_input(
            checker_source,
            "request",
            CHECKER_REQUEST_PATH,
            &retained.request,
        )?,
    ];
    let own_raw_bundle = raw_artifact_bundle(&retained.artifacts);
    let own_raw_value = serde_json::to_value(&own_raw_bundle)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let own_raw_bytes = encode_value(repo, &own_raw_value)?;
    selected_inputs.push(stage_checker_input(
        checker_source,
        "rawBundle",
        CHECKER_RAW_BUNDLE_PATH,
        &own_raw_bytes,
    )?);
    let raw_artifacts: Vec<_> = own_raw_bundle
        .artifacts
        .iter()
        .map(|artifact| RawArtifactInput {
            role: artifact.role.clone(),
        })
        .collect();
    let mut dependencies = Vec::new();
    let mut chosen: Vec<&Retained> = Vec::new();
    for name in member.depends_on.as_deref().unwrap_or(&[]) {
        let selected: Vec<_> = prior
            .iter()
            .filter(|dependency| dependency.attempt.member == *name)
            .collect();
        if selected.is_empty()
            || selected
                .iter()
                .any(|dependency| dependency.result.is_empty())
        {
            retained.attempt.reason = Some("dependency_result_missing".to_owned());
            return Ok(AttemptEvidence::Inconclusive);
        }
        chosen.extend(selected);
    }
    chosen.sort_by(|a, b| {
        (a.attempt.member.as_str(), a.attempt.index)
            .cmp(&(b.attempt.member.as_str(), b.attempt.index))
    });
    for (position, dependency) in chosen.into_iter().enumerate() {
        let name = &dependency.attempt.member;
        let index = dependency.attempt.index;
        let result_path = staged_dependency_path(position);
        selected_inputs.push(stage_checker_input(
            checker_source,
            &format!("dependency/{name}/{index}"),
            &result_path,
            &dependency.result,
        )?);
        let request_path = staged_dependency_request_path(position);
        selected_inputs.push(stage_checker_input(
            checker_source,
            &format!("dependency-request/{name}/{index}"),
            &request_path,
            &dependency.request,
        )?);
        let raw_value = serde_json::to_value(raw_artifact_bundle(&dependency.artifacts))
            .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
        let raw_bytes = encode_value(repo, &raw_value)?;
        let raw_bundle_path = staged_dependency_raw_path(position);
        selected_inputs.push(stage_checker_input(
            checker_source,
            &format!("dependencyRaw/{name}/{index}"),
            &raw_bundle_path,
            &raw_bytes,
        )?);
        dependencies.push(DependencyResultInput {
            member: name.clone(),
            index,
            request_path,
            result_path,
            raw_bundle_path,
        });
    }
    let bundle = DomainCheckInput {
        schema: DOMAIN_CHECK_INPUT_SCHEMA.to_owned(),
        definition_path: CHECKER_DEFINITION_PATH.to_owned(),
        member: member.name.clone(),
        plan_id: member.plan_id.clone(),
        definition_version: member.definition_version.clone(),
        request_path: CHECKER_REQUEST_PATH.to_owned(),
        result_path: CHECKER_RESULT_PATH.to_owned(),
        raw_bundle_path: CHECKER_RAW_BUNDLE_PATH.to_owned(),
        raw_artifacts,
        dependencies,
    };
    let bundle_value = serde_json::to_value(&bundle)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let bundle_bytes = encode_value(repo, &bundle_value)?;
    selected_inputs.push(stage_checker_input(
        checker_source,
        "bundle",
        CHECKER_INPUT_PATH,
        &bundle_bytes,
    )?);
    let mut checker_bindings = checker_runtime;
    checker_bindings.capability_root = checker_source.checkout.to_string_lossy().into_owned();
    checker_bindings.inputs = selected_inputs;
    checker_bindings.source_tree = Some(SourceTreeBinding {
        repository: checker_source.repository.clone(),
        manifest: checker_source.manifest.clone(),
    });
    let checker_request = match resolve_procedure(
        checker_procedure,
        &definition.source_graph,
        checker_bindings,
    ) {
        Ok(request) => request,
        Err(error) => {
            retained.attempt.reason = Some(format!("checker_invalid_request:{error}"));
            return Ok(AttemptEvidence::Inconclusive);
        }
    };
    let checker_adapter = match ProcessEvidenceAdapter::new(
        checker_request.request.response.protocol.clone(),
        checker_request.request.response.adapter.clone(),
    ) {
        Ok(adapter) => adapter,
        Err(error) => {
            retained.attempt.reason = Some(format!("checker_adapter_unsupported:{error}"));
            return Ok(AttemptEvidence::Inconclusive);
        }
    };
    let executed = {
        let active = cancellation.register(checker_request.request.cancellation.clone());
        executor.execute(&checker_request.request, active.token(), &checker_adapter)
    };
    let checker_result = match executed {
        Ok(result) => result,
        Err(error) => {
            retained.attempt.reason = Some(format!("checker_invalid_request:{error}"));
            return Ok(AttemptEvidence::Inconclusive);
        }
    };
    if !matches!(
        checker_result.state,
        ProducerExecutionState::Completed { .. }
    ) {
        retained.attempt.reason = Some("independent_checker_incomplete".to_owned());
        return Ok(AttemptEvidence::Inconclusive);
    }
    let Some(verdict_artifact) = checker_result
        .artifacts
        .iter()
        .find(|artifact| artifact.role == "verdict")
    else {
        retained.attempt.reason = Some("checker_verdict_missing".to_owned());
        return Ok(AttemptEvidence::Inconclusive);
    };
    if checker_result.artifacts.len() != 1 {
        retained.attempt.reason = Some("checker_artifact_inventory".to_owned());
        return Ok(AttemptEvidence::Inconclusive);
    }
    let mut reader = verdict_artifact
        .try_reader()
        .map_err(|error| CampaignRunError::execution(error.to_string()))?;
    let mut receipt_bytes = Vec::new();
    reader
        .by_ref()
        .take(crate::campaign::store::MAX_CAMPAIGN_EVIDENCE_BYTES.saturating_add(1))
        .read_to_end(&mut receipt_bytes)
        .map_err(|error| CampaignRunError::execution(error.to_string()))?;
    if quoin_store::parse_strict_json(&receipt_bytes).is_err() {
        retained.attempt.reason = Some("checker_verdict_malformed".to_owned());
        return Ok(AttemptEvidence::Inconclusive);
    }
    let Some(receipt) = serde_json::from_slice::<DomainVerdictReceipt>(&receipt_bytes).ok() else {
        retained.attempt.reason = Some("checker_verdict_malformed".to_owned());
        return Ok(AttemptEvidence::Inconclusive);
    };
    let Ok(verdict) = assess_receipt(&bundle, &receipt) else {
        retained.attempt.reason = Some("checker_verdict_identity".to_owned());
        return Ok(AttemptEvidence::Reject);
    };
    Ok(verdict)
}

fn stage_checker_input(
    source: &VerifiedSource,
    role: &str,
    relative: &str,
    bytes: &[u8],
) -> Result<InputBinding, CampaignRunError> {
    if !relative.starts_with(".quoin-campaign/") {
        return Err(CampaignRunError::binding(
            "checker input path is not reserved".to_owned(),
        ));
    }
    if source.has_link_ancestor(relative) {
        return Err(CampaignRunError::binding(
            "unsafe checker input path".to_owned(),
        ));
    }
    if source.contains_path(relative) {
        return Err(CampaignRunError::binding(
            "checker input collides with tracked source".to_owned(),
        ));
    }
    let path = source.checkout.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| CampaignRunError::execution(error.to_string()))?;
    }
    std::fs::write(&path, bytes).map_err(|error| CampaignRunError::execution(error.to_string()))?;
    Ok(InputBinding {
        role: role.to_owned(),
        path: relative.to_owned(),
        executable: false,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{CampaignRunError, VerifiedSource, stage_checker_input};

    /// Trace: FR-114-AC-2
    /// Provenance: PLAT-1071
    /// A tracked `.quoin-campaign` link cannot carry checker input bytes out of
    /// the checkout, and a tracked file at an input path is not overwritten.
    #[cfg(unix)]
    #[test]
    fn tc_1942_checker_input_refuses_tracked_link_and_tracked_path() {
        let root = tempfile::tempdir().expect("root");
        let checkout = root.path().join("checkout");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&checkout).expect("checkout");
        std::fs::create_dir_all(&outside).expect("outside");
        std::fs::write(checkout.join("tracked.txt"), b"tracked").expect("tracked file");
        std::os::unix::fs::symlink(&outside, checkout.join(".quoin-campaign")).expect("link");
        let source = VerifiedSource {
            repository: "fictional/source".to_owned(),
            checkout,
            manifest: b"120000 blob 0000000000000000000000000000000000000000\t.quoin-campaign\0"
                .to_vec(),
        };
        let refused = stage_checker_input(
            &source,
            "definition",
            ".quoin-campaign/definition.json",
            b"bytes",
        );
        assert!(
            matches!(refused, Err(CampaignRunError::Binding(_))),
            "{refused:?}"
        );
        assert_eq!(std::fs::read_dir(&outside).expect("outside").count(), 0);

        let tracked = VerifiedSource {
            manifest: b"100644 blob 0000000000000000000000000000000000000000\t.quoin-campaign/definition.json\0"
                .to_vec(),
            ..source
        };
        std::fs::remove_file(tracked.checkout.join(".quoin-campaign")).expect("unlink");
        std::fs::create_dir_all(tracked.checkout.join(".quoin-campaign")).expect("directory");
        std::fs::write(
            tracked.checkout.join(".quoin-campaign/definition.json"),
            b"tracked bytes",
        )
        .expect("tracked definition");
        let refused = stage_checker_input(
            &tracked,
            "definition",
            ".quoin-campaign/definition.json",
            b"bytes",
        );
        assert!(
            matches!(refused, Err(CampaignRunError::Binding(_))),
            "{refused:?}"
        );
        assert_eq!(
            std::fs::read(tracked.checkout.join(".quoin-campaign/definition.json"))
                .expect("tracked bytes"),
            b"tracked bytes"
        );
    }
}
