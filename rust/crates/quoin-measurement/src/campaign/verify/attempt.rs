// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Recompute one retained producer attempt from exact source and inputs.

use super::checker_claim::check_partial_checker_claim;
use super::collection::{CollectionObservationState, check_collection_context};
use super::domain::check_domain_receipt;
use super::origin::check_input_origins;
use super::{
    AttemptEvidence, BTreeMap, CampaignAttempt, CampaignDefinition, CampaignStoreError,
    EvidenceError, InputBinding, MeasurementPlan, MeasurementProcedure, OrderSource, Path, Ranked,
    RawArtifactInput, TamperFacts, Value, canonical_digest, digest_bytes_sha256, parse_strict_json,
    read_bounded, read_digest_bytes, required, retained_json, verdict_json, verify,
};
use engineering_assurance::campaign::{ProcedureBindings, resolve_procedure};
use engineering_assurance::producer_execution::ProducerExecutionRequest;

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "one attempt's retained identity chain is verified together"
)]
pub(super) fn check_attempt(
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
    check_request_contract(&request, procedure, definition, source)?;
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
    let input_origins: crate::campaign::input_origin::InputOriginInventory =
        serde_json::from_value(
            collection
                .pointer("/rawEvidence/inputOrigins")
                .cloned()
                .ok_or(EvidenceError::Contradiction)?,
        )
        .map_err(|_| EvidenceError::Contradiction)?;
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
        "inputOrigins":input_origins,
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
    check_input_origins(
        repo,
        &input_origins,
        &request,
        member,
        attempt,
        run_attempts,
        source_inputs,
        procedure,
    )?;
    let collection_state = check_collection_context(
        &collection,
        definition,
        member,
        run_id,
        attempt,
        plan,
        &result,
        result_digest,
        plan_source_inputs,
    )?;
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
    let raw_artifacts = checked_raw_artifacts(repo, attempt, &result)?;
    if collection_state == CollectionObservationState::NotComputed {
        if member.checker_procedure.is_some() {
            if attempt.domain_verdict_digest.is_some() {
                let domain = check_domain_receipt(
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
                )?;
                if domain != AttemptEvidence::Inconclusive {
                    return Err(EvidenceError::Contradiction);
                }
            } else {
                check_partial_checker_claim(repo, definition, attempt, source_inputs)?;
            }
        } else if attempt.checker_request_digest.is_some()
            || attempt.checker_result_digest.is_some()
            || attempt.domain_verdict_digest.is_some()
        {
            return Err(EvidenceError::Contradiction);
        }
        return Ok(AttemptEvidence::Inconclusive);
    }
    let plan_accept = recomputed.verdict == crate::verify::Verdict::Accept;
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
    } else {
        // The process-evidence adapter has no independent numeric observation.
        // With no declared checker the producer may complete, but a computed
        // score would claim evidence the adapter never supplied.
        Err(EvidenceError::Contradiction)
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
    let implicit = expected
        .iter()
        .map(|binding| serde_json::to_value(binding).map_err(|_| EvidenceError::Contradiction))
        .collect::<Result<Vec<_>, _>>()?;
    if implicit
        .iter()
        .any(|binding| observed.iter().filter(|input| *input == binding).count() != 1)
    {
        return Err(EvidenceError::Contradiction);
    }
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
        if !implicit.contains(input) {
            if expected
                .iter()
                .any(|binding| binding.role == role || binding.path == path)
            {
                return Err(EvidenceError::Contradiction);
            }
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
    Ok(())
}

pub(super) fn check_request_contract(
    request: &Value,
    procedure: &MeasurementProcedure,
    definition: &CampaignDefinition,
    source_inputs: &[InputBinding],
) -> Result<(), EvidenceError> {
    let retained = ProducerExecutionRequest::from_retained_value(request)
        .map_err(|_| EvidenceError::Contradiction)?;
    let explicit: Vec<InputBinding> = retained
        .inputs
        .iter()
        .filter(|input| !source_inputs.contains(input))
        .cloned()
        .collect();
    let input_origins = procedure.input_origins.as_ref().map(|declared| {
        declared
            .iter()
            .filter(|origin| explicit.iter().any(|input| input.role == origin.role))
            .cloned()
            .collect()
    });
    // The retained request is the only authority for runtime-selected paths,
    // budgets and containment. The exact Git procedure remains the authority
    // for arguments, environment declarations, roles, response and timeout.
    // Resolving again checks the complete authored contract before any receipt
    // derived from the request can count as evidence.
    let bindings = ProcedureBindings {
        producer: retained.producer.clone(),
        caller: retained.caller.clone(),
        capability_root: retained.capability_root.clone(),
        environment: retained.environment.clone(),
        inputs: explicit,
        input_origins,
        source_tree: None,
        outputs: retained.outputs.clone(),
        output_trees: retained.output_trees.clone(),
        stdin: retained.stdin.clone(),
        containment: retained.containment.clone(),
        cancellation: retained.cancellation.clone(),
        budget: retained.budget,
        response_protocol: retained.response.protocol.clone(),
        response_adapter: retained.response.adapter.clone(),
        exit_codes: retained.response.exit_codes.clone(),
    };
    let mut resolved = resolve_procedure(procedure, &definition.source_graph, bindings)
        .map_err(|_| EvidenceError::Contradiction)?
        .request;
    // EA appends verified Git source inputs after explicit selections. They
    // are supplied by the independent Git inventory, never by the request.
    resolved.inputs.extend_from_slice(source_inputs);
    if resolved != retained {
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
