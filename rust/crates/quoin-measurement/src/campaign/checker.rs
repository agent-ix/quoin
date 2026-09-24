// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Identity-bound input and receipt for a member's independent domain checker.

use engineering_assurance::campaign::canonical_digest;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::campaign::AttemptEvidence;
use crate::campaign::store::{CampaignStoreError, read_digest_bytes};
use engineering_assurance::campaign::CampaignRawArtifact;
use std::path::Path;

/// Quoin's checker input bundle discriminator.
pub const DOMAIN_CHECK_INPUT_SCHEMA: &str = "quoin.domain-check-input/v1";
/// Reserved names under EA's isolated staged cwd, never host paths.
pub const CHECKER_INPUT_PATH: &str = ".quoin-campaign/check-input.json";
/// Staged definition path bound by the checker request.
pub const CHECKER_DEFINITION_PATH: &str = ".quoin-campaign/definition.json";
/// Staged producer result path bound by the checker request.
pub const CHECKER_RESULT_PATH: &str = ".quoin-campaign/result.json";
/// Staged producer request path bound by the checker request.
pub const CHECKER_REQUEST_PATH: &str = ".quoin-campaign/request.json";
/// Staged complete producer raw artifact bundle.
pub const CHECKER_RAW_BUNDLE_PATH: &str = ".quoin-campaign/raw-bundle.json";

/// Portable staged path for one artifact, keyed by its digest.
#[must_use]
pub fn staged_raw_path(digest: &str) -> String {
    format!(".quoin-campaign/raw/{digest}.bin")
}

/// Portable staged path for a dependency's retained EA result.
#[must_use]
pub fn staged_dependency_path(member: &str, index: i64, digest: &str) -> String {
    let key = quoin_store::digest_bytes_sha256(format!("{member}\0{index}").as_bytes());
    format!(
        ".quoin-campaign/dependencies/{}/{digest}.json",
        key.as_hex()
    )
}

/// Portable staged path for one complete dependency output bundle.
#[must_use]
pub fn staged_dependency_raw_path(member: &str, index: i64, digest: &str) -> String {
    let key = quoin_store::digest_bytes_sha256(format!("{member}\0{index}").as_bytes());
    format!(
        ".quoin-campaign/dependency-raw/{}/{digest}.json",
        key.as_hex()
    )
}

/// One exact sealed artifact byte sequence inside a dependency transport.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactBytes {
    /// EA artifact role, including output-tree relative leaf path.
    pub role: String,
    /// SHA-256 of exact bytes.
    pub digest: String,
    /// Exact bounded bytes from EA's sealed descriptor.
    pub bytes: Vec<u8>,
}

/// Generic transport for a dependency's complete EA output artifacts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawArtifactBundle {
    /// Exact Quoin schema discriminator.
    pub schema: String,
    /// Sorted complete artifact population.
    pub artifacts: Vec<ArtifactBytes>,
}

/// Schema of a dependency raw-artifact transport.
pub const RAW_ARTIFACT_BUNDLE_SCHEMA: &str = "quoin.raw-artifact-bundle/v1";

/// Reopen every retained artifact byte and form a portable dependency bundle.
///
/// # Errors
/// Refuses a missing or digest-mismatched retained artifact.
pub fn raw_artifact_bundle(
    repo: &Path,
    artifacts: &[CampaignRawArtifact],
) -> Result<RawArtifactBundle, CampaignStoreError> {
    let mut retained = artifacts
        .iter()
        .map(|artifact| {
            let bytes = read_digest_bytes(repo, "raw", &artifact.digest, "bin")?;
            Ok(ArtifactBytes {
                role: artifact.role.clone(),
                digest: artifact.digest.clone(),
                bytes,
            })
        })
        .collect::<Result<Vec<_>, CampaignStoreError>>()?;
    retained.sort_by(|a, b| a.role.cmp(&b.role));
    Ok(RawArtifactBundle {
        schema: RAW_ARTIFACT_BUNDLE_SCHEMA.to_owned(),
        artifacts: retained,
    })
}

/// One completed dependency attempt selected for independent domain checking.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyResultInput {
    /// Declared dependency member.
    pub member: String,
    /// Positive attempt index.
    pub index: i64,
    /// EA request identity of the dependency attempt.
    pub request_digest: String,
    /// Deterministic relative path to EA-staged request bytes.
    pub request_path: String,
    /// EA result identity of the dependency attempt.
    pub result_digest: String,
    /// Deterministic relative path to EA-staged result bytes.
    pub result_path: String,
    /// SHA-256-JCS of complete dependency raw artifact transport.
    pub raw_bundle_digest: String,
    /// Deterministic relative path to staged raw transport.
    pub raw_bundle_path: String,
}

/// One exact raw output-artifact input copied from EA's sealed descriptor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawArtifactInput {
    /// EA output role.
    pub role: String,
    /// SHA-256 of those exact bytes, bare lowercase hex.
    pub digest: String,
}

/// Complete domain-checker input, bound before launching its EA request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DomainCheckInput {
    /// Exact versioned Quoin bundle protocol.
    pub schema: String,
    /// Deterministic relative path to the EA-staged `CampaignDefinition`.
    pub definition_path: String,
    /// SHA-256-JCS of that definition.
    pub definition_digest: String,
    /// Campaign member name.
    pub member: String,
    /// Governing `MeasurementPlan` id.
    pub plan_id: String,
    /// Exact plan definition version.
    pub definition_version: String,
    /// SHA-256-JCS of the definition's source graph.
    pub source_graph_digest: String,
    /// EA request identity digest.
    pub request_digest: String,
    /// Deterministic relative path to the EA-staged producer request.
    pub request_path: String,
    /// Deterministic relative path to the EA-staged producer result.
    pub result_path: String,
    /// EA result identity digest.
    pub result_digest: String,
    /// SHA-256-JCS of the complete producer raw artifact transport.
    pub raw_bundle_digest: String,
    /// Deterministic relative path to that staged transport.
    pub raw_bundle_path: String,
    /// Exact retained output-artifact bytes, sorted by role.
    pub raw_artifacts: Vec<RawArtifactInput>,
    /// Complete retained results of declared dependencies, sorted by member/index.
    pub dependencies: Vec<DependencyResultInput>,
}

/// The only accepted checker verdict values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainOutcome {
    /// Independent checker accepted the measured member.
    Accept,
    /// Independent checker found a contradiction or failed obligation.
    Reject,
    /// Independent checker could not decide from complete evidence.
    Inconclusive,
}

/// A checker receipt that cannot be bound to the selected producer evidence.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ReceiptError {
    /// Unsupported or empty protocol discriminator.
    #[error("checker receipt schema is invalid")]
    Schema,
    /// The receipt names another member, plan, source, request, or result.
    #[error("checker receipt identity mismatch")]
    Identity,
    /// Artifact inventory is duplicated or not ordered by role.
    #[error("checker raw-artifact inventory is not canonical")]
    ArtifactInventory,
    /// Artifact inventory cannot be canonically encoded.
    #[error("checker raw-artifact inventory encoding failed")]
    ArtifactEncoding,
    /// Receipt does not bind the selected artifact inventory.
    #[error("checker raw-artifact digest mismatch")]
    ArtifactDigest,
    /// Receipt does not bind the selected process streams.
    #[error("checker process-stream digest mismatch")]
    StreamDigest,
}

/// Common identity envelope every domain checker must emit.
///
/// `schema` is domain-owned; the exact checker executable and response-adapter
/// identities are carried by the separate EA checker request and result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DomainVerdictReceipt {
    /// Domain-owned versioned receipt schema.
    pub schema: String,
    /// Campaign member checked.
    pub member: String,
    /// Governing plan id.
    pub plan_id: String,
    /// Governing plan version.
    pub definition_version: String,
    /// SHA-256-JCS of the campaign definition.
    pub definition_digest: String,
    /// SHA-256-JCS of its source graph.
    pub source_graph_digest: String,
    /// EA producer request digest.
    pub request_digest: String,
    /// EA producer result digest.
    pub result_digest: String,
    /// SHA-256-JCS of the ordered raw-artifact inventory in the input bundle.
    pub raw_artifacts_digest: String,
    /// SHA-256-JCS of the complete producer raw artifact transport.
    pub raw_bundle_digest: String,
    /// SHA-256-JCS of dependency result inventory.
    pub dependencies_digest: String,
    /// Captured stdout's exact-byte digest, when a process was observed.
    pub stdout_digest: Option<String>,
    /// Captured stderr's exact-byte digest, when a process was observed.
    pub stderr_digest: Option<String>,
    /// The domain checker's independent outcome.
    pub verdict: DomainOutcome,
    /// Typed domain reason codes, interpreted only by the domain checker.
    pub reasons: Vec<String>,
}

/// Compare the receipt's complete identity envelope to the sealed checker
/// input. The caller separately verifies every named file's actual bytes and
/// the checker EA request/result identities before granting credit.
///
/// # Errors
/// Returns an identity mismatch if the receipt names another source, member,
/// plan, producer request/result, or raw-artifact inventory.
pub fn assess_receipt(
    input: &DomainCheckInput,
    receipt: &DomainVerdictReceipt,
    stdout_digest: Option<&str>,
    stderr_digest: Option<&str>,
) -> Result<AttemptEvidence, ReceiptError> {
    if input.schema != DOMAIN_CHECK_INPUT_SCHEMA || receipt.schema.trim().is_empty() {
        return Err(ReceiptError::Schema);
    }
    if receipt.member != input.member
        || receipt.plan_id != input.plan_id
        || receipt.definition_version != input.definition_version
        || receipt.definition_digest != input.definition_digest
        || receipt.source_graph_digest != input.source_graph_digest
        || receipt.request_digest != input.request_digest
        || receipt.result_digest != input.result_digest
        || receipt.raw_bundle_digest != input.raw_bundle_digest
    {
        return Err(ReceiptError::Identity);
    }
    if input
        .raw_artifacts
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left.role >= right.role))
    {
        return Err(ReceiptError::ArtifactInventory);
    }
    if input.dependencies.windows(2).any(|pair| {
        matches!(pair, [left, right] if (left.member.as_str(), left.index) >= (right.member.as_str(), right.index))
    }) {
        return Err(ReceiptError::ArtifactInventory);
    }
    let inventory_digest =
        canonical_digest(&input.raw_artifacts).map_err(|_| ReceiptError::ArtifactEncoding)?;
    if receipt.raw_artifacts_digest != inventory_digest.as_str() {
        return Err(ReceiptError::ArtifactDigest);
    }
    let dependencies_digest =
        canonical_digest(&input.dependencies).map_err(|_| ReceiptError::ArtifactEncoding)?;
    if receipt.dependencies_digest != dependencies_digest.as_str() {
        return Err(ReceiptError::ArtifactDigest);
    }
    if receipt.stdout_digest.as_deref() != stdout_digest
        || receipt.stderr_digest.as_deref() != stderr_digest
    {
        return Err(ReceiptError::StreamDigest);
    }
    Ok(match receipt.verdict {
        DomainOutcome::Accept => AttemptEvidence::Accept,
        DomainOutcome::Reject => AttemptEvidence::Reject,
        DomainOutcome::Inconclusive => AttemptEvidence::Inconclusive,
    })
}
