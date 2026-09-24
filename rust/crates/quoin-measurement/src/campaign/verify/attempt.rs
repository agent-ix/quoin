// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Recompute one retained producer attempt from exact source and inputs.

use super::domain::check_domain_receipt;
use super::{
    AttemptEvidence, BTreeMap, CampaignAttempt, CampaignDefinition, CampaignStoreError,
    EvidenceError, InputBinding, MeasurementPlan, MeasurementProcedure, OrderSource, Path, Ranked,
    RawArtifactInput, TamperFacts, Value, canonical_digest, digest_bytes_sha256, parse_strict_json,
    read_bounded, read_digest_bytes, required, retained_json, verdict_json, verify,
};

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "one attempt's retained identity chain is verified together"
)]
pub(super) fn check_attempt(
    repo: &Path,
    definition: &CampaignDefinition,
    definition_digest: &str,
    attempt: &CampaignAttempt,
    run_attempts: &[CampaignAttempt],
    plans: &[MeasurementPlan],
    procedures: &BTreeMap<String, MeasurementProcedure>,
    source_inputs: &BTreeMap<String, Vec<InputBinding>>,
) -> Result<AttemptEvidence, EvidenceError> {
    let member = definition
        .members
        .iter()
        .find(|member| member.name == attempt.member)
        .ok_or(EvidenceError::Contradiction)?;
    let request_digest = required(attempt.request_digest.as_ref())?;
    let result_digest = required(attempt.result_digest.as_ref())?;
    let request = retained_json(repo, "requests", request_digest)?;
    let result = retained_json(repo, "results", result_digest)?;
    let procedure = procedures
        .get(&member.plan_id)
        .ok_or(EvidenceError::Contradiction)?;
    let source = source_inputs
        .get(&procedure.source_repository)
        .ok_or(EvidenceError::Contradiction)?;
    check_request_contract(&request, procedure, definition)?;
    check_request_inputs(repo, &request, source)?;
    if result
        .pointer("/requestIdentity/digest")
        .and_then(Value::as_str)
        != Some(request_digest)
        || result.pointer("/state/kind").and_then(Value::as_str) != Some("completed")
        || result.get("producer") != request.get("producer")
    {
        return Err(EvidenceError::Contradiction);
    }
    let collection_id = required(attempt.collection_id.as_ref())?;
    let collection_digest = required(attempt.collection_digest.as_ref())?;
    let collection_path = crate::store::measurement_path(
        repo,
        &crate::types::ids::CollectionId::parse(collection_id)
            .map_err(|_| EvidenceError::Contradiction)?,
    );
    let collection_bytes = read_bounded(&collection_path).map_err(|error| match error {
        CampaignStoreError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
            EvidenceError::Missing
        }
        _ => EvidenceError::Contradiction,
    })?;
    if digest_bytes_sha256(&collection_bytes).as_hex() != collection_digest {
        return Err(EvidenceError::Contradiction);
    }
    parse_strict_json(&collection_bytes).map_err(|_| EvidenceError::Contradiction)?;
    let collection: Value =
        serde_json::from_slice(&collection_bytes).map_err(|_| EvidenceError::Contradiction)?;
    if collection.get("collectionId").and_then(Value::as_str) != Some(collection_id)
        || !collection
            .get("observations")
            .and_then(Value::as_array)
            .is_some_and(|observations| {
                observations.iter().any(|observation| {
                    observation.get("planId").and_then(Value::as_str) == Some(&member.plan_id)
                        && observation.get("definitionVersion").and_then(Value::as_str)
                            == Some(&member.definition_version)
                })
            })
    {
        return Err(EvidenceError::Contradiction);
    }
    let expected_raw_evidence = serde_json::json!({
        "schema":"quoin.campaign-attempt-evidence/v1",
        "definitionDigest":definition_digest,
        "sourceGraphDigest":canonical_digest(&definition.source_graph)
            .map_err(|_| EvidenceError::Contradiction)?.as_str(),
        "requestDigest":request_digest,
        "resultDigest":result_digest,
        "checkerRequestDigest":attempt.checker_request_digest,
        "checkerResultDigest":attempt.checker_result_digest,
        "domainVerdictDigest":attempt.domain_verdict_digest,
        "rawArtifacts":attempt.raw_artifacts,
    });
    if collection.get("rawEvidence") != Some(&expected_raw_evidence) {
        return Err(EvidenceError::Contradiction);
    }
    let plan = plans
        .iter()
        .find(|plan| {
            plan.id.as_str() == member.plan_id
                && plan.definition_version.as_str() == member.definition_version
        })
        .ok_or(EvidenceError::Contradiction)?;
    let parsed_collection = crate::validate::stored_measurement_collection(
        &parse_strict_json(&collection_bytes).map_err(|_| EvidenceError::Contradiction)?,
    )
    .map_err(|_| EvidenceError::Contradiction)?;
    let recomputed = verify(
        plan,
        &[Ranked::new(&parsed_collection, Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    );
    let verdict_wire = verdict_json(&recomputed).map_err(|_| EvidenceError::Contradiction)?;
    let recomputed_digest =
        canonical_digest(&verdict_wire).map_err(|_| EvidenceError::Contradiction)?;
    let verdict_digest = required(attempt.verdict_digest.as_ref())?;
    let verdict = retained_json(repo, "verdicts", verdict_digest)?;
    let retained_digest = canonical_digest(&verdict).map_err(|_| EvidenceError::Contradiction)?;
    if recomputed_digest.as_str() != verdict_digest || retained_digest.as_str() != verdict_digest {
        return Err(EvidenceError::Contradiction);
    }
    let plan_accept = recomputed.verdict == crate::verify::Verdict::Accept;
    let raw_artifacts = checked_raw_artifacts(repo, attempt, &result)?;
    if member.checker_procedure.is_some() {
        check_domain_receipt(
            repo,
            definition,
            definition_digest,
            attempt,
            run_attempts,
            &result,
            request_digest,
            result_digest,
            raw_artifacts,
            source_inputs,
        )
        .map(|domain| {
            if plan_accept {
                domain
            } else {
                AttemptEvidence::Reject
            }
        })
    } else if plan_accept {
        Ok(AttemptEvidence::Accept)
    } else {
        Ok(AttemptEvidence::Reject)
    }
}

pub(super) fn check_request_inputs(
    repo: &Path,
    request: &Value,
    expected: &[InputBinding],
) -> Result<(), EvidenceError> {
    let observed = request
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    let mut roles = std::collections::BTreeSet::new();
    let mut paths = std::collections::BTreeSet::new();
    let mut source_count = 0;
    for input in observed {
        let role = input
            .get("role")
            .and_then(Value::as_str)
            .ok_or(EvidenceError::Contradiction)?;
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or(EvidenceError::Contradiction)?;
        let digest = input
            .get("digest")
            .and_then(Value::as_str)
            .ok_or(EvidenceError::Contradiction)?;
        if !roles.insert(role) || !paths.insert(path) {
            return Err(EvidenceError::Contradiction);
        }
        if role.starts_with("source/") || role.starts_with("source-exec/") {
            source_count += 1;
            if !expected
                .iter()
                .any(|binding| serde_json::to_value(binding).ok().as_ref() == Some(input))
            {
                return Err(EvidenceError::Contradiction);
            }
        } else {
            let bytes = read_digest_bytes(repo, "inputs", digest, "bin")
                .map_err(|_| EvidenceError::Missing)?;
            if quoin_store::digest_bytes_sha256(&bytes).as_hex() != digest
                || path.is_empty()
                || Path::new(path).is_absolute()
                || Path::new(path)
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(EvidenceError::Contradiction);
            }
        }
    }
    if source_count != expected.len() {
        return Err(EvidenceError::Contradiction);
    }
    Ok(())
}

pub(super) fn check_request_contract(
    request: &Value,
    procedure: &MeasurementProcedure,
    definition: &CampaignDefinition,
) -> Result<(), EvidenceError> {
    let source = definition
        .source_graph
        .iter()
        .find(|source| source.repository == procedure.source_repository)
        .ok_or(EvidenceError::Contradiction)?;
    if request.pointer("/producer/name").and_then(Value::as_str) != Some(&procedure.producer_name)
        || request.pointer("/producer/version").and_then(Value::as_str)
            != Some(&procedure.producer_version)
        || request
            .pointer("/producer/sourceRevision")
            .and_then(Value::as_str)
            != Some(&source.revision)
        || request
            .pointer("/response/protocol/kind")
            .and_then(Value::as_str)
            != Some(&procedure.response_protocol)
        || request
            .pointer("/response/adapter/kind")
            .and_then(Value::as_str)
            != Some(&procedure.response_adapter)
        || request
            .pointer("/response/adapter/version")
            .and_then(Value::as_str)
            != Some(&procedure.response_adapter_version)
        || request
            .pointer("/budget/timeoutMillis")
            .and_then(Value::as_i64)
            != Some(procedure.timeout_millis)
        || request.get("procedure").and_then(Value::as_str) != Some("direct")
    {
        return Err(EvidenceError::Contradiction);
    }
    let inputs = request
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    let explicit: Vec<_> = inputs
        .iter()
        .filter_map(|input| {
            let role = input.get("role")?.as_str()?;
            (!role.starts_with("source/") && !role.starts_with("source-exec/")).then_some(role)
        })
        .collect();
    for role in &explicit {
        let exact = procedure
            .inputs
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .any(|declared| declared.role == *role);
        let prefix = procedure
            .input_role_prefixes
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .any(|declared| role.starts_with(&declared.prefix));
        if !exact && !prefix {
            return Err(EvidenceError::Contradiction);
        }
    }
    if procedure
        .inputs
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .any(|declared| declared.required && !explicit.contains(&declared.role.as_str()))
        || procedure
            .input_role_prefixes
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .any(|declared| {
                declared.required
                    && !explicit
                        .iter()
                        .any(|role| role.starts_with(&declared.prefix))
            })
    {
        return Err(EvidenceError::Contradiction);
    }
    Ok(())
}

fn checked_raw_artifacts(
    repo: &Path,
    attempt: &CampaignAttempt,
    result: &Value,
) -> Result<Vec<RawArtifactInput>, EvidenceError> {
    let observed = result
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    let declared = attempt.raw_artifacts.as_deref().unwrap_or(&[]);
    if declared.len() != observed.len() {
        return Err(EvidenceError::Contradiction);
    }
    let mut declared_roles = std::collections::BTreeSet::new();
    let mut observed_roles = std::collections::BTreeSet::new();
    if declared
        .iter()
        .any(|artifact| !declared_roles.insert(artifact.role.as_str()))
        || observed.iter().any(|artifact| {
            !artifact
                .get("role")
                .and_then(Value::as_str)
                .is_some_and(|role| observed_roles.insert(role))
        })
        || declared_roles != observed_roles
    {
        return Err(EvidenceError::Contradiction);
    }
    let mut inputs = Vec::with_capacity(declared.len());
    for artifact in declared {
        let bytes =
            read_digest_bytes(repo, "raw", &artifact.digest, "bin").map_err(
                |error| match error {
                    CampaignStoreError::Io { source, .. }
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        EvidenceError::Missing
                    }
                    _ => EvidenceError::Contradiction,
                },
            )?;
        if !observed.iter().any(|item| {
            item.get("role").and_then(Value::as_str) == Some(&artifact.role)
                && item.get("digest").and_then(Value::as_str) == Some(&artifact.digest)
                && item.get("byteLength").and_then(Value::as_u64) == u64::try_from(bytes.len()).ok()
        }) {
            return Err(EvidenceError::Contradiction);
        }
        inputs.push(RawArtifactInput {
            role: artifact.role.clone(),
            digest: artifact.digest.clone(),
        });
    }
    inputs.sort_by(|a, b| a.role.cmp(&b.role));
    Ok(inputs)
}
