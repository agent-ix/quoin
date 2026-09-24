// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reopen independent checker request, result, bundles, and receipt.

use super::attempt::{check_request_contract, check_request_inputs};
use super::{
    AttemptEvidence, BTreeMap, CHECKER_DEFINITION_PATH, CHECKER_INPUT_PATH,
    CHECKER_RAW_BUNDLE_PATH, CHECKER_REQUEST_PATH, CHECKER_RESULT_PATH, CampaignAttempt,
    CampaignAttemptStatus, CampaignDefinition, CampaignStoreError, DOMAIN_CHECK_INPUT_SCHEMA,
    DependencyResultInput, DomainCheckInput, DomainVerdictReceipt, EvidenceError, InputBinding,
    Path, RawArtifactBundle, RawArtifactInput, Value, assess_receipt, canonical_digest,
    parse_strict_json, raw_artifact_bundle, read_digest_bytes, required, retained_json,
    staged_dependency_path, staged_dependency_raw_path,
};

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "every exact checker identity and selected byte is checked in one chain"
)]
pub(super) fn check_domain_receipt(
    repo: &Path,
    definition: &CampaignDefinition,
    definition_digest: &str,
    attempt: &CampaignAttempt,
    run_attempts: &[CampaignAttempt],
    result: &Value,
    request_digest: &str,
    result_digest: &str,
    raw_artifacts: Vec<RawArtifactInput>,
    source_inputs: &BTreeMap<String, Vec<InputBinding>>,
) -> Result<AttemptEvidence, EvidenceError> {
    let member = definition
        .members
        .iter()
        .find(|member| member.name == attempt.member)
        .ok_or(EvidenceError::Contradiction)?;
    let checker_request_digest = required(attempt.checker_request_digest.as_ref())?;
    let checker_result_digest = required(attempt.checker_result_digest.as_ref())?;
    let checker_request = retained_json(repo, "requests", checker_request_digest)?;
    let checker_result = retained_json(repo, "results", checker_result_digest)?;
    let checker_procedure = member
        .checker_procedure
        .as_ref()
        .ok_or(EvidenceError::Contradiction)?;
    let checker_source = source_inputs
        .get(&checker_procedure.source_repository)
        .ok_or(EvidenceError::Contradiction)?;
    check_request_contract(
        &checker_request,
        checker_procedure,
        definition,
        checker_source,
    )?;
    check_request_inputs(repo, &checker_request, checker_source)?;
    if checker_result
        .pointer("/requestIdentity/digest")
        .and_then(Value::as_str)
        != Some(checker_request_digest)
        || checker_result
            .pointer("/state/kind")
            .and_then(Value::as_str)
            != Some("completed")
        || checker_result.get("producer") != checker_request.get("producer")
    {
        return Err(EvidenceError::Contradiction);
    }
    let receipt_digest = required(attempt.domain_verdict_digest.as_ref())?;
    let receipt_value = retained_json(repo, "domain-verdicts", receipt_digest)?;
    let checker_artifacts = checker_result
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    if checker_artifacts.len() != 1 {
        return Err(EvidenceError::Contradiction);
    }
    let verdict_artifact = checker_artifacts
        .iter()
        .find(|item| item.get("role").and_then(Value::as_str) == Some("verdict"))
        .ok_or(EvidenceError::Missing)?;
    let raw_receipt_digest = verdict_artifact
        .get("digest")
        .and_then(Value::as_str)
        .ok_or(EvidenceError::Contradiction)?;
    let raw_receipt =
        read_digest_bytes(repo, "raw", raw_receipt_digest, "bin").map_err(|error| match error {
            CampaignStoreError::Io { source, .. }
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                EvidenceError::Missing
            }
            _ => EvidenceError::Contradiction,
        })?;
    if verdict_artifact.get("byteLength").and_then(Value::as_u64)
        != u64::try_from(raw_receipt.len()).ok()
        || !checker_request
            .get("outputs")
            .and_then(Value::as_array)
            .is_some_and(|outputs| {
                outputs.iter().any(|output| {
                    output.get("role").and_then(Value::as_str) == Some("verdict")
                        && output.get("path") == verdict_artifact.get("path")
                })
            })
    {
        return Err(EvidenceError::Contradiction);
    }
    let parsed_raw = parse_strict_json(&raw_receipt).map_err(|_| EvidenceError::Contradiction)?;
    let canonical_raw =
        quoin_store::canonical_bytes(&parsed_raw).map_err(|_| EvidenceError::Contradiction)?;
    let retained_receipt = read_digest_bytes(repo, "domain-verdicts", receipt_digest, "json")
        .map_err(|_| EvidenceError::Missing)?;
    if canonical_raw != retained_receipt {
        return Err(EvidenceError::Contradiction);
    }
    let receipt: DomainVerdictReceipt =
        serde_json::from_value(receipt_value).map_err(|_| EvidenceError::Contradiction)?;
    let dependencies = dependency_inputs(
        repo,
        member.depends_on.as_deref().unwrap_or(&[]),
        run_attempts,
    )?;
    let own_raw_bundle = raw_artifact_bundle(repo, attempt.raw_artifacts.as_deref().unwrap_or(&[]))
        .map_err(|_| EvidenceError::Contradiction)?;
    let raw_bundle_digest =
        canonical_digest(&own_raw_bundle).map_err(|_| EvidenceError::Contradiction)?;
    let retained_raw_bundle = retained_json(repo, "bundles", raw_bundle_digest.as_str())?;
    if serde_json::to_value(&own_raw_bundle).map_err(|_| EvidenceError::Contradiction)?
        != retained_raw_bundle
    {
        return Err(EvidenceError::Contradiction);
    }
    let input = DomainCheckInput {
        schema: DOMAIN_CHECK_INPUT_SCHEMA.to_owned(),
        definition_path: CHECKER_DEFINITION_PATH.to_owned(),
        definition_digest: definition_digest.to_owned(),
        member: member.name.clone(),
        plan_id: member.plan_id.clone(),
        definition_version: member.definition_version.clone(),
        source_graph_digest: canonical_digest(&definition.source_graph)
            .map_err(|_| EvidenceError::Contradiction)?
            .as_str()
            .to_owned(),
        request_digest: request_digest.to_owned(),
        request_path: CHECKER_REQUEST_PATH.to_owned(),
        result_path: CHECKER_RESULT_PATH.to_owned(),
        result_digest: result_digest.to_owned(),
        raw_bundle_digest: raw_bundle_digest.as_str().to_owned(),
        raw_bundle_path: CHECKER_RAW_BUNDLE_PATH.to_owned(),
        raw_artifacts,
        dependencies,
    };
    let bundle_digest = canonical_digest(&input).map_err(|_| EvidenceError::Contradiction)?;
    let bundle = retained_json(repo, "bundles", bundle_digest.as_str())?;
    let typed_bundle: DomainCheckInput =
        serde_json::from_value(bundle).map_err(|_| EvidenceError::Contradiction)?;
    if typed_bundle != input {
        return Err(EvidenceError::Contradiction);
    }
    let selected_inputs = checker_request
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    let mut required = vec![
        (
            "bundle".to_owned(),
            CHECKER_INPUT_PATH.to_owned(),
            bundle_digest.as_str().to_owned(),
        ),
        (
            "definition".to_owned(),
            CHECKER_DEFINITION_PATH.to_owned(),
            definition_digest.to_owned(),
        ),
        (
            "result".to_owned(),
            CHECKER_RESULT_PATH.to_owned(),
            result_digest.to_owned(),
        ),
        (
            "request".to_owned(),
            CHECKER_REQUEST_PATH.to_owned(),
            request_digest.to_owned(),
        ),
        (
            "rawBundle".to_owned(),
            CHECKER_RAW_BUNDLE_PATH.to_owned(),
            raw_bundle_digest.as_str().to_owned(),
        ),
    ];
    required.extend(input.dependencies.iter().map(|dependency| {
        (
            format!("dependency/{}/{}", dependency.member, dependency.index),
            dependency.result_path.clone(),
            dependency.result_digest.clone(),
        )
    }));
    required.extend(input.dependencies.iter().flat_map(|dependency| {
        [
            (
                format!(
                    "dependency-request/{}/{}",
                    dependency.member, dependency.index
                ),
                dependency.request_path.clone(),
                dependency.request_digest.clone(),
            ),
            (
                format!("dependencyRaw/{}/{}", dependency.member, dependency.index),
                dependency.raw_bundle_path.clone(),
                dependency.raw_bundle_digest.clone(),
            ),
        ]
    }));
    let explicit: Vec<_> = selected_inputs
        .iter()
        .filter(|selected| {
            !selected
                .get("role")
                .and_then(Value::as_str)
                .is_some_and(|role| role.starts_with("source/") || role.starts_with("source-exec/"))
        })
        .collect();
    let mut roles = std::collections::BTreeSet::new();
    let mut paths = std::collections::BTreeSet::new();
    if explicit.len() != required.len()
        || explicit.iter().any(|selected| {
            !selected
                .get("role")
                .and_then(Value::as_str)
                .is_some_and(|role| roles.insert(role))
                || !selected
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| paths.insert(path))
                || selected
                    .get("executable")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        || required.iter().any(|(role, path, digest)| {
            !explicit.iter().any(|selected| {
                selected.get("role").and_then(Value::as_str) == Some(role)
                    && selected.get("path").and_then(Value::as_str) == Some(path)
                    && selected.get("digest").and_then(Value::as_str) == Some(digest)
            })
        })
    {
        return Err(EvidenceError::Contradiction);
    }
    let stdout_digest = result
        .pointer("/process/stdout/digest")
        .and_then(Value::as_str);
    let stderr_digest = result
        .pointer("/process/stderr/digest")
        .and_then(Value::as_str);
    assess_receipt(&input, &receipt, stdout_digest, stderr_digest)
        .map_err(|_| EvidenceError::Contradiction)
}

fn dependency_inputs(
    repo: &Path,
    names: &[String],
    run_attempts: &[CampaignAttempt],
) -> Result<Vec<DependencyResultInput>, EvidenceError> {
    let mut dependencies = Vec::new();
    for name in names {
        let selected: Vec<_> = run_attempts
            .iter()
            .filter(|candidate| candidate.member == *name)
            .collect();
        if selected.is_empty() {
            return Err(EvidenceError::Missing);
        }
        for dependency in selected {
            if dependency.status != CampaignAttemptStatus::Completed {
                return Err(EvidenceError::Missing);
            }
            let dependency_request = required(dependency.request_digest.as_ref())?;
            let dependency_result = required(dependency.result_digest.as_ref())?;
            let request = retained_json(repo, "requests", dependency_request)?;
            let result = retained_json(repo, "results", dependency_result)?;
            if result
                .pointer("/requestIdentity/digest")
                .and_then(Value::as_str)
                != Some(dependency_request)
                || result.pointer("/state/kind").and_then(Value::as_str) != Some("completed")
                || result.get("producer") != request.get("producer")
            {
                return Err(EvidenceError::Contradiction);
            }
            let artifacts = dependency.raw_artifacts.as_deref().unwrap_or(&[]);
            let observed = result
                .get("artifacts")
                .and_then(Value::as_array)
                .ok_or(EvidenceError::Contradiction)?;
            let declared_roles: std::collections::BTreeSet<_> = artifacts
                .iter()
                .map(|artifact| artifact.role.as_str())
                .collect();
            let observed_roles: std::collections::BTreeSet<_> = observed
                .iter()
                .filter_map(|item| item.get("role").and_then(Value::as_str))
                .collect();
            if observed.len() != artifacts.len()
                || declared_roles.len() != artifacts.len()
                || observed_roles.len() != observed.len()
                || declared_roles != observed_roles
                || artifacts.iter().any(|artifact| {
                    !observed.iter().any(|item| {
                        item.get("role").and_then(Value::as_str) == Some(&artifact.role)
                            && item.get("digest").and_then(Value::as_str) == Some(&artifact.digest)
                    })
                })
            {
                return Err(EvidenceError::Contradiction);
            }
            let raw_bundle =
                raw_artifact_bundle(repo, artifacts).map_err(|_| EvidenceError::Contradiction)?;
            let raw_bundle_digest =
                canonical_digest(&raw_bundle).map_err(|_| EvidenceError::Contradiction)?;
            let retained = retained_json(repo, "bundles", raw_bundle_digest.as_str())?;
            let retained_bundle: RawArtifactBundle =
                serde_json::from_value(retained).map_err(|_| EvidenceError::Contradiction)?;
            if retained_bundle != raw_bundle {
                return Err(EvidenceError::Contradiction);
            }
            dependencies.push(DependencyResultInput {
                member: name.clone(),
                index: dependency.index,
                request_digest: dependency_request.to_owned(),
                request_path: staged_dependency_path(name, dependency.index, dependency_request),
                result_digest: dependency_result.to_owned(),
                result_path: staged_dependency_path(name, dependency.index, dependency_result),
                raw_bundle_digest: raw_bundle_digest.as_str().to_owned(),
                raw_bundle_path: staged_dependency_raw_path(
                    name,
                    dependency.index,
                    raw_bundle_digest.as_str(),
                ),
            });
        }
    }
    dependencies.sort_by(|a, b| (a.member.as_str(), a.index).cmp(&(b.member.as_str(), b.index)));
    Ok(dependencies)
}
