// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Input and receipt for a member's independent domain checker.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::campaign::AttemptEvidence;

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

/// Portable staged path for a dependency's retained EA result, by its
/// position in the sorted dependency list.
#[must_use]
pub fn staged_dependency_path(position: usize) -> String {
    format!(".quoin-campaign/dependencies/{position}/result.json")
}

/// Portable staged path for a dependency's retained EA request.
#[must_use]
pub fn staged_dependency_request_path(position: usize) -> String {
    format!(".quoin-campaign/dependencies/{position}/request.json")
}

/// Portable staged path for one complete dependency output bundle.
#[must_use]
pub fn staged_dependency_raw_path(position: usize) -> String {
    format!(".quoin-campaign/dependencies/{position}/raw-bundle.json")
}

/// One artifact byte sequence inside a raw-artifact transport.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactBytes {
    /// EA artifact role, including output-tree relative leaf path.
    pub role: String,
    /// Exact bounded bytes of the artifact.
    pub bytes: Vec<u8>,
}

/// Generic transport for a complete set of EA output artifacts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawArtifactBundle {
    /// Exact Quoin schema discriminator.
    pub schema: String,
    /// Artifact population sorted by role.
    pub artifacts: Vec<ArtifactBytes>,
}

/// Schema of a raw-artifact transport.
pub const RAW_ARTIFACT_BUNDLE_SCHEMA: &str = "quoin.raw-artifact-bundle/v1";

/// Form a portable bundle from retained artifact bytes.
#[must_use]
pub fn raw_artifact_bundle(artifacts: &[(String, Vec<u8>)]) -> RawArtifactBundle {
    let mut retained: Vec<ArtifactBytes> = artifacts
        .iter()
        .map(|(role, bytes)| ArtifactBytes {
            role: role.clone(),
            bytes: bytes.clone(),
        })
        .collect();
    retained.sort_by(|a, b| a.role.cmp(&b.role));
    RawArtifactBundle {
        schema: RAW_ARTIFACT_BUNDLE_SCHEMA.to_owned(),
        artifacts: retained,
    }
}

/// One completed dependency attempt selected for independent domain checking.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyResultInput {
    /// Declared dependency member.
    pub member: String,
    /// Positive attempt index.
    pub index: i64,
    /// Deterministic relative path to EA-staged request bytes.
    pub request_path: String,
    /// Deterministic relative path to EA-staged result bytes.
    pub result_path: String,
    /// Deterministic relative path to staged raw transport.
    pub raw_bundle_path: String,
}

/// One raw output-artifact role the checker may read from the staged transport.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawArtifactInput {
    /// EA output role.
    pub role: String,
}

/// Complete domain-checker input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DomainCheckInput {
    /// Exact versioned Quoin bundle protocol.
    pub schema: String,
    /// Deterministic relative path to the EA-staged `CampaignDefinition`.
    pub definition_path: String,
    /// Campaign member name.
    pub member: String,
    /// Governing `MeasurementPlan` id.
    pub plan_id: String,
    /// Exact plan definition version.
    pub definition_version: String,
    /// Deterministic relative path to the EA-staged producer request.
    pub request_path: String,
    /// Deterministic relative path to the EA-staged producer result.
    pub result_path: String,
    /// Deterministic relative path to the staged raw artifact transport.
    pub raw_bundle_path: String,
    /// Output-artifact roles, sorted.
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

/// A checker receipt that cannot be bound to the selected member.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ReceiptError {
    /// Unsupported or empty protocol discriminator.
    #[error("checker receipt schema is invalid")]
    Schema,
    /// The receipt names another member or plan.
    #[error("checker receipt identity mismatch")]
    Identity,
    /// Artifact inventory is duplicated or not ordered by role.
    #[error("checker raw-artifact inventory is not canonical")]
    ArtifactInventory,
}

/// Common identity envelope every domain checker must emit.
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
    /// The domain checker's independent outcome.
    pub verdict: DomainOutcome,
    /// Typed domain reason codes, interpreted only by the domain checker.
    pub reasons: Vec<String>,
    /// Optional domain-owned evidence detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Compare the receipt's identity envelope to the checker input.
///
/// # Errors
/// Returns an identity mismatch if the receipt names another member or plan.
pub fn assess_receipt(
    input: &DomainCheckInput,
    receipt: &DomainVerdictReceipt,
) -> Result<AttemptEvidence, ReceiptError> {
    if input.schema != DOMAIN_CHECK_INPUT_SCHEMA || receipt.schema.trim().is_empty() {
        return Err(ReceiptError::Schema);
    }
    if receipt.member != input.member
        || receipt.plan_id != input.plan_id
        || receipt.definition_version != input.definition_version
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
    Ok(match receipt.verdict {
        DomainOutcome::Accept => AttemptEvidence::Accept,
        DomainOutcome::Reject => AttemptEvidence::Reject,
        DomainOutcome::Inconclusive => AttemptEvidence::Inconclusive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> DomainCheckInput {
        DomainCheckInput {
            schema: DOMAIN_CHECK_INPUT_SCHEMA.to_owned(),
            definition_path: CHECKER_DEFINITION_PATH.to_owned(),
            member: "one".to_owned(),
            plan_id: "MP-ONE".to_owned(),
            definition_version: "v1".to_owned(),
            request_path: CHECKER_REQUEST_PATH.to_owned(),
            result_path: CHECKER_RESULT_PATH.to_owned(),
            raw_bundle_path: CHECKER_RAW_BUNDLE_PATH.to_owned(),
            raw_artifacts: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    fn receipt(verdict: DomainOutcome) -> DomainVerdictReceipt {
        DomainVerdictReceipt {
            schema: "fictional.domain-verdict/v1".to_owned(),
            member: "one".to_owned(),
            plan_id: "MP-ONE".to_owned(),
            definition_version: "v1".to_owned(),
            verdict,
            reasons: Vec::new(),
            details: None,
        }
    }

    /// Trace: FR-114-AC-3, TC-1983.
    #[test]
    fn tc_1983_receipt_must_name_the_checked_member_plan_and_version() {
        assert_eq!(
            assess_receipt(&input(), &receipt(DomainOutcome::Accept)),
            Ok(AttemptEvidence::Accept)
        );
        assert_eq!(
            assess_receipt(&input(), &receipt(DomainOutcome::Reject)),
            Ok(AttemptEvidence::Reject)
        );
        for alter in [
            |r: &mut DomainVerdictReceipt| r.member = "two".to_owned(),
            |r: &mut DomainVerdictReceipt| r.plan_id = "MP-TWO".to_owned(),
            |r: &mut DomainVerdictReceipt| r.definition_version = "v2".to_owned(),
        ] {
            let mut other = receipt(DomainOutcome::Accept);
            alter(&mut other);
            assert_eq!(
                assess_receipt(&input(), &other),
                Err(ReceiptError::Identity)
            );
        }
        let mut blank = receipt(DomainOutcome::Accept);
        blank.schema = " ".to_owned();
        assert_eq!(assess_receipt(&input(), &blank), Err(ReceiptError::Schema));
    }
}
