// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Independent replay of EA terminal and preflight attempts.

use std::collections::BTreeMap;
use std::path::Path;

use engineering_assurance::campaign::{
    CampaignAttempt, CampaignAttemptStatus, CampaignDefinition, MeasurementProcedure,
};
use engineering_assurance::producer_execution::InputBinding;

use super::attempt::{check_request_contract, check_request_inputs, checked_raw_artifacts};
use super::{EvidenceError, Value, retained_json};

pub(super) fn check_terminal_attempt(
    repo: &Path,
    definition: &CampaignDefinition,
    attempt: &CampaignAttempt,
    procedures: &BTreeMap<String, MeasurementProcedure>,
    source_inputs: &BTreeMap<String, Vec<InputBinding>>,
) -> Result<(), EvidenceError> {
    if attempt.collection_id.is_some()
        || attempt.collection_digest.is_some()
        || attempt.verdict_digest.is_some()
        || attempt.domain_verdict_digest.is_some()
        || attempt.checker_request_digest.is_some()
        || attempt.checker_result_digest.is_some()
    {
        return Err(EvidenceError::Contradiction);
    }
    let member = definition
        .members
        .iter()
        .find(|member| member.name == attempt.member)
        .ok_or(EvidenceError::Contradiction)?;
    let procedure = procedures
        .get(&member.plan_id)
        .ok_or(EvidenceError::Contradiction)?;
    let source = source_inputs
        .get(&procedure.source_repository)
        .ok_or(EvidenceError::Contradiction)?;
    let Some(request_digest) = attempt.request_digest.as_deref() else {
        return if attempt.status == CampaignAttemptStatus::InvalidRequest
            && attempt.result_digest.is_none()
            && attempt.raw_artifacts.is_none()
        {
            Ok(())
        } else {
            Err(EvidenceError::Contradiction)
        };
    };
    let request = retained_json(repo, "requests", request_digest)?;
    check_request_contract(&request, procedure, definition, source)?;
    check_request_inputs(repo, &request, source)?;
    let Some(result_digest) = attempt.result_digest.as_deref() else {
        return if attempt.status == CampaignAttemptStatus::InvalidRequest
            && attempt.raw_artifacts.is_none()
        {
            Ok(())
        } else {
            Err(EvidenceError::Contradiction)
        };
    };
    if attempt.status == CampaignAttemptStatus::InvalidRequest {
        return Err(EvidenceError::Contradiction);
    }
    let result = retained_json(repo, "results", result_digest)?;
    if result
        .pointer("/requestIdentity/digest")
        .and_then(Value::as_str)
        != Some(request_digest)
        || result.get("producer") != request.get("producer")
        || result.pointer("/state/kind").and_then(Value::as_str)
            != Some(state_kind(&attempt.status).ok_or(EvidenceError::Contradiction)?)
    {
        return Err(EvidenceError::Contradiction);
    }
    checked_raw_artifacts(repo, attempt, &result)?;
    Ok(())
}

fn state_kind(status: &CampaignAttemptStatus) -> Option<&'static str> {
    match status {
        CampaignAttemptStatus::Unavailable => Some("unavailable"),
        CampaignAttemptStatus::Refused => Some("refused"),
        CampaignAttemptStatus::Failed => Some("failed"),
        CampaignAttemptStatus::TimedOut => Some("timed_out"),
        CampaignAttemptStatus::MalformedResponse => Some("malformed_response"),
        CampaignAttemptStatus::ContainmentFailure => Some("containment_failure"),
        CampaignAttemptStatus::Cancelled => Some("cancelled"),
        CampaignAttemptStatus::Completed | CampaignAttemptStatus::InvalidRequest => None,
    }
}
