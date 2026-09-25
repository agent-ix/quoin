// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One independently bounded checker invocation and receipt.

use super::CampaignCancellation;
use super::collection::publish_collection;
use super::{
    BTreeMap, BindingFailure, CHECKER_DEFINITION_PATH, CHECKER_INPUT_PATH, CHECKER_RAW_BUNDLE_PATH,
    CHECKER_REQUEST_PATH, CHECKER_RESULT_PATH, CampaignAttempt, CampaignDefinition,
    CampaignRunError, CampaignStoreError, ContentDigest, DOMAIN_CHECK_INPUT_SCHEMA,
    DependencyResultInput, DomainCheckInput, DomainVerdictReceipt, InputBinding, MeasurementPlan,
    Path, ProcessEvidenceAdapter, ProcessEvidenceObservation, ProducerExecutionResult,
    ProducerExecutionState, ProducerExecutor, RawArtifactInput, Read, RunMemberBindings,
    SourceError, SourceTreeBinding, VerifiedSource, assess_receipt, canonical_digest,
    raw_artifact_bundle, read_digest_bytes, resolve_procedure, retain_bytes, retain_json_bytes,
    retain_value, staged_dependency_path, staged_dependency_raw_path,
};

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the checker completion binds one immutable chain of selected identities"
)]
pub(super) fn complete_attempt(
    repo: &Path,
    definition: &CampaignDefinition,
    definition_digest: &str,
    run_id: &str,
    member: &engineering_assurance::campaign::CampaignMember,
    plan: &MeasurementPlan,
    runtime: &RunMemberBindings,
    procedure: &engineering_assurance::campaign::MeasurementProcedure,
    producer_source: &VerifiedSource,
    plan_source: &VerifiedSource,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[CampaignAttempt],
    executor: &ProducerExecutor,
    cancellation: &CampaignCancellation,
    result: &ProducerExecutionResult<ProcessEvidenceObservation>,
    attempt: &mut CampaignAttempt,
) -> Result<(), CampaignRunError> {
    let verdict = match check_domain_attempt(
        repo,
        definition,
        definition_digest,
        member,
        runtime,
        sources,
        prior,
        executor,
        cancellation,
        result,
        attempt,
    ) {
        Ok(verdict) => verdict,
        Err(error) => {
            let (reason, verdict) = classify_checker_failure(&error);
            attempt.reason = Some(format!("{reason}:{error}"));
            verdict
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
        producer_source,
        plan_source,
        result,
        attempt,
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

fn classify_checker_failure(error: &CampaignRunError) -> (&'static str, super::AttemptEvidence) {
    match error {
        CampaignRunError::Binding(BindingFailure::IdentityMismatch(_))
        | CampaignRunError::Store(
            CampaignStoreError::Digest(_)
            | CampaignStoreError::Json { .. }
            | CampaignStoreError::File(_),
        )
        | CampaignRunError::Source(
            SourceError::Dirty(_)
            | SourceError::Revision(_)
            | SourceError::Tree(_)
            | SourceError::Path(_),
        ) => (
            "checker_stage_identity_unverified",
            super::AttemptEvidence::Inconclusive,
        ),
        _ => (
            "checker_stage_unavailable",
            super::AttemptEvidence::Inconclusive,
        ),
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the checker completion binds one immutable chain of selected identities"
)]
fn check_domain_attempt(
    repo: &Path,
    definition: &CampaignDefinition,
    definition_digest: &str,
    member: &engineering_assurance::campaign::CampaignMember,
    runtime: &RunMemberBindings,
    sources: &BTreeMap<String, VerifiedSource>,
    prior: &[CampaignAttempt],
    executor: &ProducerExecutor,
    cancellation: &CampaignCancellation,
    result: &ProducerExecutionResult<ProcessEvidenceObservation>,
    attempt: &mut CampaignAttempt,
) -> Result<super::AttemptEvidence, CampaignRunError> {
    let Some(checker_procedure) = &member.checker_procedure else {
        attempt.reason = Some("independent_checker_unavailable".to_owned());
        return Ok(super::AttemptEvidence::Inconclusive);
    };
    let Some(checker_runtime) = runtime.checker.clone() else {
        attempt.reason = Some("independent_checker_unavailable".to_owned());
        return Ok(super::AttemptEvidence::Inconclusive);
    };
    let checker_source = sources
        .get(&checker_procedure.source_repository)
        .ok_or_else(|| CampaignRunError::binding("checker source missing".to_owned()))?;
    let definition_bytes = read_digest_bytes(repo, "definitions", definition_digest, "json")?;
    let producer_digest = attempt
        .result_digest
        .as_deref()
        .ok_or_else(|| CampaignRunError::binding("producer result missing".to_owned()))?;
    let producer_bytes = read_digest_bytes(repo, "results", producer_digest, "json")?;
    let producer_request_digest = attempt
        .request_digest
        .as_deref()
        .ok_or_else(|| CampaignRunError::binding("producer request missing".to_owned()))?;
    let producer_request_bytes =
        read_digest_bytes(repo, "requests", producer_request_digest, "json")?;
    let mut selected_inputs = Vec::new();
    selected_inputs.push(stage_checker_input(
        repo,
        checker_source,
        "definition",
        CHECKER_DEFINITION_PATH,
        &definition_bytes,
    )?);
    selected_inputs.push(stage_checker_input(
        repo,
        checker_source,
        "result",
        CHECKER_RESULT_PATH,
        &producer_bytes,
    )?);
    selected_inputs.push(stage_checker_input(
        repo,
        checker_source,
        "request",
        CHECKER_REQUEST_PATH,
        &producer_request_bytes,
    )?);
    let own_raw_bundle =
        raw_artifact_bundle(repo, attempt.raw_artifacts.as_deref().unwrap_or(&[]))?;
    let own_raw_value = serde_json::to_value(&own_raw_bundle)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let (raw_bundle_digest, _) = retain_value(repo, "bundles", &own_raw_value)?;
    let raw_bundle_bytes = read_digest_bytes(repo, "bundles", &raw_bundle_digest, "json")?;
    selected_inputs.push(stage_checker_input(
        repo,
        checker_source,
        "rawBundle",
        CHECKER_RAW_BUNDLE_PATH,
        &raw_bundle_bytes,
    )?);
    let mut raw_artifacts: Vec<_> = own_raw_bundle
        .artifacts
        .iter()
        .map(|artifact| RawArtifactInput {
            role: artifact.role.clone(),
            digest: artifact.digest.clone(),
        })
        .collect();
    raw_artifacts.sort_by(|a, b| a.role.cmp(&b.role));
    let mut dependencies = Vec::new();
    for name in member.depends_on.as_deref().unwrap_or(&[]) {
        let selected: Vec<_> = prior
            .iter()
            .filter(|prior_attempt| prior_attempt.member == *name)
            .collect();
        if selected.is_empty() {
            attempt.reason = Some("dependency_result_missing".to_owned());
            return Ok(super::AttemptEvidence::Inconclusive);
        }
        for dependency in selected {
            let (Some(request_digest), Some(result_digest)) = (
                dependency.request_digest.as_deref(),
                dependency.result_digest.as_deref(),
            ) else {
                attempt.reason = Some("dependency_result_missing".to_owned());
                return Ok(super::AttemptEvidence::Inconclusive);
            };
            let bytes = read_digest_bytes(repo, "results", result_digest, "json")?;
            let path = staged_dependency_path(&dependency.member, dependency.index, result_digest);
            selected_inputs.push(stage_checker_input(
                repo,
                checker_source,
                &format!("dependency/{}/{}", dependency.member, dependency.index),
                &path,
                &bytes,
            )?);
            let request_bytes = read_digest_bytes(repo, "requests", request_digest, "json")?;
            let request_path =
                staged_dependency_path(&dependency.member, dependency.index, request_digest);
            selected_inputs.push(stage_checker_input(
                repo,
                checker_source,
                &format!(
                    "dependency-request/{}/{}",
                    dependency.member, dependency.index
                ),
                &request_path,
                &request_bytes,
            )?);
            let raw_bundle =
                raw_artifact_bundle(repo, dependency.raw_artifacts.as_deref().unwrap_or(&[]))?;
            let raw_value = serde_json::to_value(&raw_bundle)
                .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
            let (raw_bundle_digest, _) = retain_value(repo, "bundles", &raw_value)?;
            let raw_bundle_bytes = read_digest_bytes(repo, "bundles", &raw_bundle_digest, "json")?;
            let raw_bundle_path = staged_dependency_raw_path(
                &dependency.member,
                dependency.index,
                &raw_bundle_digest,
            );
            selected_inputs.push(stage_checker_input(
                repo,
                checker_source,
                &format!("dependencyRaw/{}/{}", dependency.member, dependency.index),
                &raw_bundle_path,
                &raw_bundle_bytes,
            )?);
            dependencies.push(DependencyResultInput {
                member: dependency.member.clone(),
                index: dependency.index,
                request_digest: request_digest.to_owned(),
                request_path,
                result_digest: result_digest.to_owned(),
                result_path: path,
                raw_bundle_digest,
                raw_bundle_path,
            });
        }
    }
    dependencies.sort_by(|a, b| (a.member.as_str(), a.index).cmp(&(b.member.as_str(), b.index)));
    let bundle = DomainCheckInput {
        schema: DOMAIN_CHECK_INPUT_SCHEMA.to_owned(),
        definition_path: CHECKER_DEFINITION_PATH.to_owned(),
        definition_digest: definition_digest.to_owned(),
        member: member.name.clone(),
        plan_id: member.plan_id.clone(),
        definition_version: member.definition_version.clone(),
        source_graph_digest: canonical_digest(&definition.source_graph)?
            .as_str()
            .to_owned(),
        request_digest: producer_request_digest.to_owned(),
        request_path: CHECKER_REQUEST_PATH.to_owned(),
        result_path: CHECKER_RESULT_PATH.to_owned(),
        result_digest: producer_digest.to_owned(),
        raw_bundle_digest,
        raw_bundle_path: CHECKER_RAW_BUNDLE_PATH.to_owned(),
        raw_artifacts,
        dependencies,
    };
    let bundle_value = serde_json::to_value(&bundle)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let (bundle_digest, _) = retain_value(repo, "bundles", &bundle_value)?;
    let bundle_bytes = read_digest_bytes(repo, "bundles", &bundle_digest, "json")?;
    selected_inputs.push(stage_checker_input(
        repo,
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
            attempt.reason = Some(format!("checker_invalid_request:{error}"));
            return Ok(super::AttemptEvidence::Inconclusive);
        }
    };
    let checker_adapter = match ProcessEvidenceAdapter::new(
        checker_request.request.response.protocol.clone(),
        checker_request.request.response.adapter.clone(),
    ) {
        Ok(adapter) => adapter,
        Err(error) => {
            attempt.reason = Some(format!("checker_adapter_unsupported:{error}"));
            return Ok(super::AttemptEvidence::Inconclusive);
        }
    };
    let request_digest = checker_request.identity.digest.as_str().to_owned();
    let request_value = serde_json::to_value(&checker_request.request)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let (stored_request, _) = retain_value(repo, "requests", &request_value)?;
    if stored_request != request_digest {
        return Err(CampaignRunError::identity(
            "checker request JCS identity".to_owned(),
        ));
    }
    attempt.checker_request_digest = Some(request_digest);
    let executed = {
        let active = cancellation.register(checker_request.request.cancellation.clone());
        executor.execute(&checker_request.request, active.token(), &checker_adapter)
    };
    let checker_result = match executed {
        Ok(result) => result,
        Err(error) => {
            attempt.reason = Some(format!("checker_invalid_request:{error}"));
            return Ok(super::AttemptEvidence::Inconclusive);
        }
    };
    let result_digest = checker_result
        .identity()
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?
        .digest
        .as_str()
        .to_owned();
    let result_value = serde_json::to_value(&checker_result)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let (stored_result, _) = retain_value(repo, "results", &result_value)?;
    if stored_result != result_digest {
        return Err(CampaignRunError::identity(
            "checker result JCS identity".to_owned(),
        ));
    }
    attempt.checker_result_digest = Some(result_digest);
    if !matches!(
        checker_result.state,
        ProducerExecutionState::Completed { .. }
    ) {
        attempt.reason = Some("independent_checker_incomplete".to_owned());
        return Ok(super::AttemptEvidence::Inconclusive);
    }
    let Some(verdict_artifact) = checker_result
        .artifacts
        .iter()
        .find(|artifact| artifact.role == "verdict")
    else {
        attempt.reason = Some("checker_verdict_missing".to_owned());
        return Ok(super::AttemptEvidence::Inconclusive);
    };
    if checker_result.artifacts.len() != 1 {
        attempt.reason = Some("checker_artifact_inventory".to_owned());
        return Ok(super::AttemptEvidence::Inconclusive);
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
    let (raw_receipt_digest, _) = retain_bytes(repo, "raw", "bin", &receipt_bytes)?;
    if raw_receipt_digest != verdict_artifact.digest.as_str() {
        return Err(CampaignRunError::identity(
            "checker verdict raw digest".to_owned(),
        ));
    }
    if quoin_store::parse_strict_json(&receipt_bytes).is_err() {
        attempt.reason = Some("checker_verdict_malformed".to_owned());
        return Ok(super::AttemptEvidence::Inconclusive);
    }
    let Some(receipt) = serde_json::from_slice::<DomainVerdictReceipt>(&receipt_bytes).ok() else {
        attempt.reason = Some("checker_verdict_malformed".to_owned());
        return Ok(super::AttemptEvidence::Inconclusive);
    };
    let (receipt_digest, _) = retain_json_bytes(repo, "domain-verdicts", &receipt_bytes)?;
    attempt.domain_verdict_digest = Some(receipt_digest);
    let Ok(verdict) = assess_receipt(
        &bundle,
        &receipt,
        result
            .process
            .as_ref()
            .map(|process| process.stdout.digest.as_str()),
        result
            .process
            .as_ref()
            .map(|process| process.stderr.digest.as_str()),
    ) else {
        attempt.reason = Some("checker_verdict_identity".to_owned());
        return Ok(super::AttemptEvidence::Reject);
    };
    Ok(verdict)
}

fn stage_checker_input(
    repo: &Path,
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
    retain_bytes(repo, "inputs", "bin", bytes)?;
    Ok(InputBinding {
        role: role.to_owned(),
        path: relative.to_owned(),
        digest: ContentDigest::of_bytes(bytes),
        executable: false,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        CampaignRunError, CampaignStoreError, VerifiedSource, classify_checker_failure,
        stage_checker_input,
    };
    use crate::campaign::AttemptEvidence;

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
            root.path(),
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
            root.path(),
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

    /// Trace: FR-114-AC-2, FR-114-AC-4
    /// Provenance: PLAT-1043
    #[test]
    fn tc_1942_checker_stage_read_fault_is_inconclusive() {
        let error = CampaignRunError::Store(CampaignStoreError::Io {
            path: PathBuf::from("checker-input"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        });
        assert_eq!(
            classify_checker_failure(&error),
            ("checker_stage_unavailable", AttemptEvidence::Inconclusive)
        );
    }

    /// Trace: FR-114-AC-2, FR-114-AC-4
    /// Provenance: PLAT-1043
    #[test]
    fn tc_1942_checker_stage_identity_fault_has_no_unverified_reject() {
        let error = CampaignRunError::identity("checker verdict raw digest".to_owned());
        assert_eq!(
            classify_checker_failure(&error),
            (
                "checker_stage_identity_unverified",
                AttemptEvidence::Inconclusive
            )
        );
    }
}
