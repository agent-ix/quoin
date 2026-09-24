// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reconstruct collection context from the definition and exact source inventory.

use std::collections::{BTreeMap, BTreeSet};

use engineering_assurance::campaign::{CampaignAttempt, CampaignDefinition, CampaignMember};
use engineering_assurance::producer_execution::InputBinding;
use serde_json::{Value, json};

use super::{EvidenceError, MeasurementPlan, canonical_digest};

#[allow(
    clippy::too_many_arguments,
    reason = "collection context binds separate definition, attempt, result, and source identities"
)]
pub(super) fn check_collection_context(
    collection: &Value,
    definition: &CampaignDefinition,
    member: &CampaignMember,
    run_id: &str,
    attempt: &CampaignAttempt,
    plan: &MeasurementPlan,
    result: &Value,
    result_digest: &str,
    source_inputs: &[InputBinding],
) -> Result<(), EvidenceError> {
    let source_graph_digest =
        canonical_digest(&definition.source_graph).map_err(|_| EvidenceError::Contradiction)?;
    let scope = json!({
        "campaign": definition.id,
        "member": member.name,
        "run": run_id,
        "attempt": attempt.index,
    });
    let producer = result.get("producer").ok_or(EvidenceError::Contradiction)?;
    let producer_name = text(producer, "name")?;
    let producer_version = text(producer, "version")?;
    let source_revision = text(producer, "sourceRevision")?;
    let executable_digest = text(producer, "executableDigest")?;
    let stack = collection
        .get("verificationStack")
        .ok_or(EvidenceError::Contradiction)?;
    let expected_graph_digest = format!("sha256:{}", source_graph_digest.as_str());
    if collection.get("schemaVersion") != Some(&json!(crate::MEASUREMENT_SCHEMA_VERSION))
        || collection.get("subject") != Some(&json!(definition.subject_name))
        || collection.get("scope") != Some(&scope)
        || collection.get("environment")
            != Some(&json!({"campaign": definition.id, "member": member.name}))
        || collection.get("toolIdentity") != Some(&json!(producer_name))
        || collection.get("toolVersion") != Some(&json!(producer_version))
        || collection.get("sourceRevision") != Some(&json!(source_revision))
        || collection.get("configDigest") != Some(&json!(expected_graph_digest))
        || stack.get("schemaVersion") != Some(&json!("verification-stack-attestation-v1"))
        || stack.get("lockDigest") != Some(&json!(expected_graph_digest))
        || stack.get("executableDigest") != Some(&json!(format!("sha256:{executable_digest}")))
        || stack.get("buildProfile") != Some(&json!("release"))
        || stack.get("capabilities") != Some(&json!(["campaign.direct-process"]))
    {
        return Err(EvidenceError::Contradiction);
    }
    check_sources(stack, definition)?;
    let expected_artifacts = expected_artifacts(plan, source_inputs, result_digest)?;
    if stack.get("artifacts") != Some(&json!(expected_artifacts)) {
        return Err(EvidenceError::Contradiction);
    }
    let observations = collection
        .get("observations")
        .and_then(Value::as_array)
        .ok_or(EvidenceError::Contradiction)?;
    if observations.len() != 1 {
        return Err(EvidenceError::Contradiction);
    }
    let observation = observations.first().ok_or(EvidenceError::Contradiction)?;
    let value = observation
        .get("value")
        .and_then(Value::as_u64)
        .filter(|value| *value <= 1)
        .ok_or(EvidenceError::Contradiction)?;
    if observation.get("planId") != Some(&json!(member.plan_id))
        || observation.get("definitionVersion") != Some(&json!(member.definition_version))
        || observation.get("metric") != Some(&json!(plan.metric.as_str()))
        || observation.get("dimensions") != Some(&json!({"member":member.name}))
        || observation.get("shape") != Some(&json!("count"))
        || observation.get("state") != Some(&json!("measured"))
        || observation.get("unit") != Some(&json!("accepted checks"))
        || observation.get("population")
            != Some(&json!({
                "examined":1,
                "matched":value,
                "complete":true,
                "repetitions":1,
                "identity":scope,
            }))
    {
        return Err(EvidenceError::Contradiction);
    }
    Ok(())
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, EvidenceError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(EvidenceError::Contradiction)
}

fn check_sources(stack: &Value, definition: &CampaignDefinition) -> Result<(), EvidenceError> {
    let sources = stack
        .get("sources")
        .and_then(Value::as_object)
        .ok_or(EvidenceError::Contradiction)?;
    if sources.len() != definition.source_graph.len() {
        return Err(EvidenceError::Contradiction);
    }
    for source in &definition.source_graph {
        let retained = sources
            .get(&source.repository)
            .ok_or(EvidenceError::Contradiction)?;
        if retained.get("revision") != Some(&json!(source.revision))
            || retained.get("sourceState") != Some(&json!("clean"))
            || retained
                .get("remote")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            return Err(EvidenceError::Contradiction);
        }
    }
    Ok(())
}

fn expected_artifacts(
    plan: &MeasurementPlan,
    source_inputs: &[InputBinding],
    result_digest: &str,
) -> Result<BTreeMap<String, String>, EvidenceError> {
    let mut artifacts = BTreeMap::from([(
        format!("spec/evidence/campaigns/results/{result_digest}.json"),
        format!("sha256:{result_digest}"),
    )]);
    let mut protected = BTreeSet::new();
    if let Some(apparatus) = &plan.protected_apparatus {
        for entry in apparatus.iter() {
            let name = entry.as_str();
            let prefix = format!("{name}/");
            let mut found = false;
            for input in source_inputs {
                if input.path == name || input.path.starts_with(&prefix) {
                    protected.insert(input.path.as_str());
                    found = true;
                }
            }
            if !found {
                return Err(EvidenceError::Contradiction);
            }
        }
    }
    for path in protected {
        let input = source_inputs
            .iter()
            .find(|input| input.path == path)
            .ok_or(EvidenceError::Contradiction)?;
        artifacts.insert(path.to_owned(), format!("sha256:{}", input.digest.as_str()));
    }
    Ok(artifacts)
}
