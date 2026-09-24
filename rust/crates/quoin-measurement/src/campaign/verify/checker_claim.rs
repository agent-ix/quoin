// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Verify claimed checker records before a domain verdict exists.

use super::attempt::{check_request_contract, check_request_inputs};
use super::{
    BTreeMap, CampaignAttempt, CampaignDefinition, CampaignStoreError, EvidenceError, InputBinding,
    Path, Value, parse_strict_json, read_digest_bytes, retained_json,
};

/// A missing domain verdict never credits the checker. Any checker request or
/// result that the run *does* claim must still be reopened and source-bound;
/// otherwise corrupting a malformed checker's retained bytes is invisible.
pub(super) fn check_partial_checker_claim(
    repo: &Path,
    definition: &CampaignDefinition,
    attempt: &CampaignAttempt,
    source_inputs: &BTreeMap<String, Vec<InputBinding>>,
) -> Result<(), EvidenceError> {
    let Some(request_digest) = attempt.checker_request_digest.as_deref() else {
        return if attempt.checker_result_digest.is_none() {
            Ok(())
        } else {
            Err(EvidenceError::Contradiction)
        };
    };
    let member = definition
        .members
        .iter()
        .find(|member| member.name == attempt.member)
        .ok_or(EvidenceError::Contradiction)?;
    let procedure = member
        .checker_procedure
        .as_ref()
        .ok_or(EvidenceError::Contradiction)?;
    let source = source_inputs
        .get(&procedure.source_repository)
        .ok_or(EvidenceError::Contradiction)?;
    let request = retained_json(repo, "requests", request_digest)?;
    check_request_contract(&request, procedure, definition, source)?;
    check_request_inputs(repo, &request, source)?;
    if let Some(result_digest) = attempt.checker_result_digest.as_deref() {
        let result = retained_json(repo, "results", result_digest)?;
        if result
            .pointer("/requestIdentity/digest")
            .and_then(Value::as_str)
            != Some(request_digest)
            || result.get("producer") != request.get("producer")
        {
            return Err(EvidenceError::Contradiction);
        }
        check_checker_result_reason(repo, attempt, &request, &result)?;
    }
    Ok(())
}

/// Reconcile a claimed pre-receipt checker outcome with the EA result that
/// actually supports it. In particular, a malformed verdict is possible only
/// after a completed checker yielded one retained verdict artifact.
fn check_checker_result_reason(
    repo: &Path,
    attempt: &CampaignAttempt,
    request: &Value,
    result: &Value,
) -> Result<(), EvidenceError> {
    let state = result
        .pointer("/state/kind")
        .and_then(Value::as_str)
        .ok_or(EvidenceError::Contradiction)?;
    let artifacts = result
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    match attempt.reason.as_deref() {
        Some("independent_checker_incomplete") if state != "completed" => Ok(()),
        Some("checker_verdict_missing")
            if state == "completed"
                && !artifacts.iter().any(|artifact| {
                    artifact.get("role").and_then(Value::as_str) == Some("verdict")
                }) =>
        {
            Ok(())
        }
        Some("checker_artifact_inventory")
            if state == "completed"
                && artifacts.len() != 1
                && artifacts.iter().any(|artifact| {
                    artifact.get("role").and_then(Value::as_str) == Some("verdict")
                }) =>
        {
            Ok(())
        }
        Some("checker_verdict_malformed") if state == "completed" && artifacts.len() == 1 => {
            let artifact = artifacts.first().ok_or(EvidenceError::Contradiction)?;
            if artifact.get("role").and_then(Value::as_str) != Some("verdict") {
                return Err(EvidenceError::Contradiction);
            }
            if !request
                .get("outputs")
                .and_then(Value::as_array)
                .is_some_and(|outputs| {
                    outputs.iter().any(|output| {
                        output.get("role").and_then(Value::as_str) == Some("verdict")
                            && output.get("path") == artifact.get("path")
                    })
                })
            {
                return Err(EvidenceError::Contradiction);
            }
            let digest = artifact
                .get("digest")
                .and_then(Value::as_str)
                .ok_or(EvidenceError::Contradiction)?;
            let bytes =
                read_digest_bytes(repo, "raw", digest, "bin").map_err(|error| match error {
                    CampaignStoreError::Io { source, .. }
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        EvidenceError::Missing
                    }
                    _ => EvidenceError::Contradiction,
                })?;
            if artifact.get("byteLength").and_then(Value::as_u64) != u64::try_from(bytes.len()).ok()
                || (parse_strict_json(&bytes).is_ok()
                    && serde_json::from_slice::<crate::campaign::checker::DomainVerdictReceipt>(
                        &bytes,
                    )
                    .is_ok())
            {
                return Err(EvidenceError::Contradiction);
            }
            Ok(())
        }
        Some(reason) if reason.starts_with("checker_stage_") => Ok(()),
        _ => Err(EvidenceError::Contradiction),
    }
}
