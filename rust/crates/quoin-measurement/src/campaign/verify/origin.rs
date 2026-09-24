// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Recompute each claimed explicit input origin from independent retained evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use engineering_assurance::campaign::{
    CampaignAttempt, CampaignAttemptStatus, CampaignMember, MeasurementProcedure,
    validate_procedure,
};
use engineering_assurance::producer_execution::InputBinding;
use serde_json::Value;

use crate::campaign::input_origin::{
    INPUT_ORIGINS_SCHEMA, InputOriginClaim, InputOriginInventory, OriginSource, declaration_for,
};

use super::{EvidenceError, read_digest_bytes, retained_json};

#[allow(
    clippy::too_many_arguments,
    reason = "origin replay joins exact request, source graph, and prior EA result identities"
)]
pub(super) fn check_input_origins(
    repo: &Path,
    inventory: &InputOriginInventory,
    request: &Value,
    member: &CampaignMember,
    attempt: &CampaignAttempt,
    run_attempts: &[CampaignAttempt],
    source_inputs: &BTreeMap<String, Vec<InputBinding>>,
    procedure: &MeasurementProcedure,
) -> Result<(), EvidenceError> {
    if validate_procedure(procedure).is_err()
        || inventory.schema != INPUT_ORIGINS_SCHEMA
        || inventory.declaration
            != declaration_for(&inventory.inputs, procedure)
                .map_err(|_| EvidenceError::Contradiction)?
    {
        return Err(EvidenceError::Contradiction);
    }
    let request_inputs = request
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    let source_inventory = source_inputs
        .get(&procedure.source_repository)
        .ok_or(EvidenceError::Contradiction)?;
    let implicit = source_inventory
        .iter()
        .map(|binding| serde_json::to_value(binding).map_err(|_| EvidenceError::Contradiction))
        .collect::<Result<Vec<_>, _>>()?;
    if implicit.iter().any(|binding| {
        request_inputs
            .iter()
            .filter(|input| *input == binding)
            .count()
            != 1
    }) {
        return Err(EvidenceError::Contradiction);
    }
    let explicit: Vec<_> = request_inputs
        .iter()
        .filter(|input| !implicit.contains(input))
        .collect();
    if explicit.len() != inventory.inputs.len()
        || request_inputs.len() != implicit.len() + explicit.len()
    {
        return Err(EvidenceError::Contradiction);
    }
    let mut roles = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut previous_role: Option<&str> = None;
    for origin in &inventory.inputs {
        if !roles.insert(origin.binding.role.as_str())
            || !paths.insert(origin.binding.path.as_str())
            || previous_role.is_some_and(|previous| previous >= origin.binding.role.as_str())
        {
            return Err(EvidenceError::Contradiction);
        }
        previous_role = Some(origin.binding.role.as_str());
        let selected = explicit
            .iter()
            .find(|input| {
                input.get("role").and_then(Value::as_str) == Some(&origin.binding.role)
                    && input.get("path").and_then(Value::as_str) == Some(&origin.binding.path)
            })
            .ok_or(EvidenceError::Contradiction)?;
        if selected.get("digest").and_then(Value::as_str) != Some(&origin.binding.bytes.digest)
            || selected
                .get("executable")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                != origin.binding.bytes.executable
        {
            return Err(EvidenceError::Contradiction);
        }
        check_origin(repo, origin, member, attempt, run_attempts, source_inputs)?;
    }
    Ok(())
}

fn check_origin(
    repo: &Path,
    origin: &InputOriginClaim,
    member: &CampaignMember,
    attempt: &CampaignAttempt,
    run_attempts: &[CampaignAttempt],
    source_inputs: &BTreeMap<String, Vec<InputBinding>>,
) -> Result<(), EvidenceError> {
    match &origin.source {
        OriginSource::SelectedBytes => Ok(()),
        OriginSource::SourceFile { repository, path } => {
            let source = source_inputs
                .get(repository)
                .ok_or(EvidenceError::Contradiction)?;
            if source.iter().any(|input| {
                input.path == *path && input.digest.as_str() == origin.binding.bytes.digest
            }) {
                Ok(())
            } else {
                Err(EvidenceError::Contradiction)
            }
        }
        OriginSource::Dependency {
            member: dependency_member,
            index,
            artifact_role,
        } => check_dependency(
            repo,
            origin,
            member,
            attempt,
            run_attempts,
            dependency_member,
            *index,
            artifact_role,
        ),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "one dependency claim binds its member, attempt, role, and exact EA result"
)]
fn check_dependency(
    repo: &Path,
    origin: &InputOriginClaim,
    member: &CampaignMember,
    attempt: &CampaignAttempt,
    run_attempts: &[CampaignAttempt],
    dependency_member: &str,
    index: i64,
    artifact_role: &str,
) -> Result<(), EvidenceError> {
    if index < 1
        || !member
            .depends_on
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .any(|name| name == dependency_member)
    {
        return Err(EvidenceError::Contradiction);
    }
    let position = run_attempts
        .iter()
        .position(|item| item.member == attempt.member && item.index == attempt.index)
        .ok_or(EvidenceError::Contradiction)?;
    let dependency = run_attempts
        .iter()
        .take(position)
        .find(|item| item.member == dependency_member && item.index == index)
        .ok_or(EvidenceError::Contradiction)?;
    if dependency.status != CampaignAttemptStatus::Completed
        || !dependency
            .raw_artifacts
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .any(|artifact| {
                artifact.role == artifact_role && artifact.digest == origin.binding.bytes.digest
            })
    {
        return Err(EvidenceError::Contradiction);
    }
    let result_digest = dependency
        .result_digest
        .as_deref()
        .ok_or(EvidenceError::Contradiction)?;
    let request_digest = dependency
        .request_digest
        .as_deref()
        .ok_or(EvidenceError::Contradiction)?;
    let result = retained_json(repo, "results", result_digest)?;
    let bytes = read_digest_bytes(repo, "raw", &origin.binding.bytes.digest, "bin")
        .map_err(|_| EvidenceError::Contradiction)?;
    if result
        .pointer("/requestIdentity/digest")
        .and_then(Value::as_str)
        != Some(request_digest)
        || result.pointer("/state/kind").and_then(Value::as_str) != Some("completed")
        || !result
            .get("artifacts")
            .and_then(Value::as_array)
            .is_some_and(|artifacts| {
                artifacts.iter().any(|artifact| {
                    artifact.get("role").and_then(Value::as_str) == Some(artifact_role)
                        && artifact.get("digest").and_then(Value::as_str)
                            == Some(&origin.binding.bytes.digest)
                        && artifact.get("byteLength").and_then(Value::as_u64)
                            == u64::try_from(bytes.len()).ok()
                })
            })
    {
        return Err(EvidenceError::Contradiction);
    }
    Ok(())
}

#[cfg(test)]
#[path = "origin/tests.rs"]
mod tests;
