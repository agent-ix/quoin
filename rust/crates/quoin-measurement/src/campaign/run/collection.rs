// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Typed campaign `MeasurementCollection` emission.

use super::{
    AttemptEvidence, BTreeMap, CampaignDefinition, CampaignRunError, MeasurementPlan, OrderSource,
    Path, ProcessEvidenceObservation, ProducerExecutionResult, Ranked, Retained, RunMemberBindings,
    TamperFacts, Verdict, VerifiedSource, read_bounded, verify,
};
use serde::Serialize;
use std::collections::BTreeSet;

use crate::campaign::input_origin::{InputOriginInventory, from_runtime};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionPayload<'a> {
    schema_version: u32,
    collection_id: &'a str,
    subject: &'a str,
    scope: CollectionScope<'a>,
    tool_identity: &'a str,
    tool_version: &'a str,
    timestamp: &'a str,
    source_revision: &'a str,
    environment: CollectionEnvironment<'a>,
    verification_stack: CollectionStack<'a>,
    observations: Vec<CollectionObservation<'a>>,
    raw_evidence: CollectionRawEvidence,
}

#[derive(Serialize)]
struct CollectionScope<'a> {
    campaign: &'a str,
    member: &'a str,
    run: &'a str,
    attempt: i64,
}

#[derive(Serialize)]
struct CollectionEnvironment<'a> {
    campaign: &'a str,
    member: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionStack<'a> {
    schema_version: &'static str,
    build_profile: &'static str,
    sources: BTreeMap<String, CollectionSource<'a>>,
    capabilities: [&'static str; 1],
    artifacts: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionSource<'a> {
    remote: &'a str,
    revision: &'a str,
    source_state: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionObservation<'a> {
    plan_id: &'a str,
    definition_version: &'a str,
    metric: &'a str,
    dimensions: CollectionDimension<'a>,
    shape: &'static str,
    state: &'static str,
    value: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    unit: &'static str,
    population: CollectionPopulation<'a>,
}

#[derive(Serialize)]
struct CollectionDimension<'a> {
    member: &'a str,
}

#[derive(Serialize)]
struct CollectionPopulation<'a> {
    examined: u8,
    matched: u8,
    complete: bool,
    repetitions: u8,
    identity: CollectionScope<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionRawEvidence {
    schema: &'static str,
    input_origins: InputOriginInventory,
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "collection intake records one completed producer and its available checker evidence"
)]
pub(super) fn publish_collection(
    repo: &Path,
    definition: &CampaignDefinition,
    run_id: &str,
    member: &engineering_assurance::campaign::CampaignMember,
    plan: &MeasurementPlan,
    runtime: &RunMemberBindings,
    procedure: &engineering_assurance::campaign::MeasurementProcedure,
    plan_source: &VerifiedSource,
    result: &ProducerExecutionResult<ProcessEvidenceObservation>,
    retained: &mut Retained,
    verdict: AttemptEvidence,
) -> Result<(), CampaignRunError> {
    if verdict == AttemptEvidence::Inconclusive && retained.attempt.reason.is_none() {
        retained.attempt.reason = Some("independent_checker_inconclusive".to_owned());
    }
    let attempt = &retained.attempt;
    let score = u8::from(verdict == AttemptEvidence::Accept);
    let position = definition
        .members
        .iter()
        .position(|candidate| candidate.name == member.name)
        .unwrap_or(0);
    let collection_id = format!("campaign-{run_id}-m{position}-{}", attempt.index);
    let mut sources = BTreeMap::new();
    for source in &definition.source_graph {
        let remote = runtime
            .source_remotes
            .get(&source.repository)
            .ok_or_else(|| CampaignRunError::binding("source remote missing".to_owned()))?;
        sources.insert(
            source.repository.clone(),
            CollectionSource {
                remote,
                revision: &source.revision,
                source_state: "clean",
            },
        );
    }
    let input_origins = from_runtime(runtime, procedure)
        .map_err(|error| CampaignRunError::binding(error.to_string()))?;
    let artifacts: Vec<String> = protected_artifacts(plan, plan_source)?
        .into_iter()
        .collect();
    let collection_value = serde_json::to_value(CollectionPayload {
        schema_version: crate::MEASUREMENT_SCHEMA_VERSION,
        collection_id: &collection_id,
        subject: &definition.subject_name,
        scope: CollectionScope {
            campaign: &definition.id,
            member: &member.name,
            run: run_id,
            attempt: attempt.index,
        },
        tool_identity: &result.producer.name,
        tool_version: &result.producer.version,
        timestamp: &runtime.timestamp,
        source_revision: &result.producer.source_revision,
        environment: CollectionEnvironment {
            campaign: &definition.id,
            member: &member.name,
        },
        verification_stack: CollectionStack {
            schema_version: "verification-stack-attestation-v1",
            build_profile: "release",
            sources,
            capabilities: ["campaign.direct-process"],
            artifacts,
        },
        observations: vec![CollectionObservation {
            plan_id: &member.plan_id,
            definition_version: &member.definition_version,
            metric: plan.metric.as_str(),
            dimensions: CollectionDimension {
                member: &member.name,
            },
            shape: "count",
            state: if verdict == AttemptEvidence::Inconclusive {
                "not_computed"
            } else {
                "measured"
            },
            value: (verdict != AttemptEvidence::Inconclusive).then_some(score),
            reason: (verdict == AttemptEvidence::Inconclusive)
                .then_some(attempt.reason.as_deref())
                .flatten(),
            unit: "accepted checks",
            population: CollectionPopulation {
                examined: 1,
                matched: score,
                complete: verdict != AttemptEvidence::Inconclusive,
                repetitions: 1,
                identity: CollectionScope {
                    campaign: &definition.id,
                    member: &member.name,
                    run: run_id,
                    attempt: attempt.index,
                },
            },
        }],
        raw_evidence: CollectionRawEvidence {
            schema: "quoin.campaign-attempt-evidence/v1",
            input_origins,
        },
    })
    .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let candidate = crate::json_bridge::from_serde(&collection_value)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let path = crate::store::publish::write_measurement_collection_with_plans(
        repo,
        &candidate,
        std::slice::from_ref(plan),
    )
    .map_err(|error| CampaignRunError::measurement(error.to_string()))?;
    let bytes = read_bounded(&path)?;
    let parsed = crate::validate::stored_measurement_collection(
        &quoin_store::parse_strict_json(&bytes)
            .map_err(|error| CampaignRunError::measurement(error.to_string()))?,
    )
    .map_err(|error| CampaignRunError::measurement(error.to_string()))?;
    let result = verify(
        plan,
        &[Ranked::new(&parsed, Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    );
    retained.evidence = match (verdict, result.verdict) {
        (AttemptEvidence::Accept, Verdict::Accept) => AttemptEvidence::Accept,
        (AttemptEvidence::Reject, _) | (_, Verdict::Reject) => AttemptEvidence::Reject,
        _ => AttemptEvidence::Inconclusive,
    };
    if result.verdict != Verdict::Accept && verdict == AttemptEvidence::Accept {
        retained.attempt.reason = Some("measurement_plan_inconclusive_or_rejected".to_owned());
    }
    Ok(())
}

fn protected_artifacts(
    plan: &MeasurementPlan,
    source: &VerifiedSource,
) -> Result<BTreeSet<String>, CampaignRunError> {
    let mut selected = BTreeSet::new();
    if let Some(protected) = &plan.protected_apparatus {
        let regular = source.tracked_regular_paths()?;
        for entry in protected.iter() {
            let name = entry.as_str();
            let prefix = format!("{name}/");
            let mut matched = false;
            for path in &regular {
                if path == name || path.starts_with(&prefix) {
                    selected.insert(path.clone());
                    matched = true;
                }
            }
            if !matched {
                return Err(CampaignRunError::binding(format!(
                    "protected apparatus `{name}` is absent from the exact Git source tree"
                )));
            }
        }
    }
    Ok(selected)
}
