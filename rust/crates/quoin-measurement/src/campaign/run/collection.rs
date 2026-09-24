// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Typed campaign `MeasurementCollection` emission.

use super::{
    BTreeMap, CampaignAttempt, CampaignDefinition, CampaignRawArtifact, CampaignRunError,
    MeasurementPlan, OrderSource, Path, ProcessEvidenceObservation, ProducerExecutionResult,
    Ranked, RunMemberBindings, TamperFacts, Verdict, VerifiedSource, canonical_digest,
    read_bounded, retain_value, verdict_json, verify,
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionPayload<'a> {
    schema_version: u32,
    collection_id: &'a str,
    subject: &'a str,
    scope: CollectionScope<'a>,
    tool_identity: &'a str,
    tool_version: &'a str,
    config_digest: String,
    timestamp: &'a str,
    source_revision: &'a str,
    environment: CollectionEnvironment<'a>,
    verification_stack: CollectionStack<'a>,
    observations: Vec<CollectionObservation<'a>>,
    raw_evidence: CollectionRawEvidence<'a>,
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
    lock_digest: String,
    executable_digest: String,
    build_profile: &'static str,
    toolchains: &'a BTreeMap<String, String>,
    sources: BTreeMap<String, CollectionSource<'a>>,
    capabilities: [&'static str; 1],
    artifacts: BTreeMap<String, String>,
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
    value: u8,
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
struct CollectionRawEvidence<'a> {
    schema: &'static str,
    definition_digest: String,
    source_graph_digest: &'a str,
    request_digest: &'a str,
    result_digest: &'a str,
    checker_request_digest: Option<&'a str>,
    checker_result_digest: &'a str,
    domain_verdict_digest: &'a str,
    raw_artifacts: Option<&'a [CampaignRawArtifact]>,
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "collection intake records one complete and independently checked invocation"
)]
pub(super) fn publish_collection(
    repo: &Path,
    definition: &CampaignDefinition,
    run_id: &str,
    member: &engineering_assurance::campaign::CampaignMember,
    plan: &MeasurementPlan,
    runtime: &RunMemberBindings,
    plan_source: &VerifiedSource,
    result: &ProducerExecutionResult<ProcessEvidenceObservation>,
    attempt: &mut CampaignAttempt,
    verdict: super::AttemptEvidence,
) -> Result<(), CampaignRunError> {
    if verdict == super::AttemptEvidence::Inconclusive {
        attempt.reason = Some("independent_checker_inconclusive".to_owned());
        return Ok(());
    }
    let score = u8::from(verdict == super::AttemptEvidence::Accept);
    let identity_text = format!("{run_id}/{}#{}", member.name, attempt.index);
    let collection_id = format!(
        "campaign-{}",
        quoin_store::digest_bytes_sha256(identity_text.as_bytes()).as_hex()
    );
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
    let source_graph_digest = canonical_digest(&definition.source_graph)?
        .as_str()
        .to_owned();
    let result_digest = attempt
        .result_digest
        .as_deref()
        .ok_or_else(|| CampaignRunError::binding("producer result missing".to_owned()))?;
    let result_relative = format!("spec/evidence/campaigns/results/{result_digest}.json");
    let request_digest = attempt
        .request_digest
        .as_deref()
        .ok_or_else(|| CampaignRunError::binding("producer request missing".to_owned()))?;
    let checker_result_digest = attempt
        .checker_result_digest
        .as_deref()
        .ok_or_else(|| CampaignRunError::binding("checker result missing".to_owned()))?;
    let domain_verdict_digest = attempt
        .domain_verdict_digest
        .as_deref()
        .ok_or_else(|| CampaignRunError::binding("domain verdict missing".to_owned()))?;
    let mut artifacts = BTreeMap::new();
    artifacts.insert(result_relative, format!("sha256:{result_digest}"));
    artifacts.extend(protected_artifacts(plan, plan_source)?);
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
        config_digest: format!("sha256:{source_graph_digest}"),
        timestamp: &runtime.timestamp,
        source_revision: &result.producer.source_revision,
        environment: CollectionEnvironment {
            campaign: &definition.id,
            member: &member.name,
        },
        verification_stack: CollectionStack {
            schema_version: "verification-stack-attestation-v1",
            lock_digest: format!("sha256:{source_graph_digest}"),
            executable_digest: format!("sha256:{}", result.producer.executable_digest.as_str()),
            build_profile: "release",
            toolchains: &runtime.toolchains,
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
            state: "measured",
            value: score,
            unit: "accepted checks",
            population: CollectionPopulation {
                examined: 1,
                matched: score,
                complete: true,
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
            definition_digest: canonical_digest(definition)?.as_str().to_owned(),
            source_graph_digest: &source_graph_digest,
            request_digest,
            result_digest,
            checker_request_digest: attempt.checker_request_digest.as_deref(),
            checker_result_digest,
            domain_verdict_digest,
            raw_artifacts: attempt.raw_artifacts.as_deref(),
        },
    })
    .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let candidate = crate::json_bridge::from_serde(&collection_value)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let path = crate::write_measurement_collection(repo, &candidate)
        .map_err(|error| CampaignRunError::measurement(error.to_string()))?;
    let bytes = read_bounded(&path)?;
    let collection_digest = quoin_store::digest_bytes_sha256(&bytes).as_hex().to_owned();
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
    let verdict_value =
        verdict_json(&result).map_err(|error| CampaignRunError::measurement(error.to_string()))?;
    let (verdict_digest, _) = retain_value(repo, "verdicts", &verdict_value)?;
    attempt.collection_id = Some(collection_id);
    attempt.collection_digest = Some(collection_digest);
    attempt.verdict_digest = Some(verdict_digest);
    if result.verdict != Verdict::Accept && verdict == super::AttemptEvidence::Accept {
        attempt.reason = Some("measurement_plan_inconclusive_or_rejected".to_owned());
    }
    Ok(())
}

fn protected_artifacts(
    plan: &MeasurementPlan,
    source: &VerifiedSource,
) -> Result<BTreeMap<String, String>, CampaignRunError> {
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
    selected
        .into_iter()
        .map(|path| {
            let bytes = source.read_tracked_file(&path)?;
            let digest = quoin_store::digest_bytes_sha256(&bytes);
            Ok((path, format!("sha256:{}", digest.as_hex())))
        })
        .collect()
}
