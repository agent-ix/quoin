// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded reliance decisions about one evidence producer use (FR-093).

use serde::{Deserialize, Serialize};

use crate::ids::TrustDecisionId;

/// The adapter half of a producer context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustAdapter {
    /// The adapter's name.
    pub name: String,
    /// The adapter's version.
    pub version: String,
}

/// Exact producer context accepted or observed for one bounded use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerContext {
    /// The producing tool.
    pub name: String,
    /// Its exact version.
    pub version: String,
    /// A digest over the configuration it ran under.
    pub configuration_digest: String,
    /// A digest over the corpus its validation was measured on.
    pub validation_corpus_digest: String,
    /// The input contract it was validated against.
    pub input_contract: String,
    /// The environment it was validated in.
    pub environment: String,
    /// The adapter that transcribed its output, when one is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<TrustAdapter>,
}

/// A fact about a producer context whose change can invalidate a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustTrigger {
    /// The producer's version changed.
    ProducerVersion,
    /// The configuration digest changed.
    Configuration,
    /// The adapter changed.
    Adapter,
    /// The validation corpus digest changed.
    ValidationCorpus,
    /// The input contract changed.
    InputContract,
    /// The environment changed.
    Environment,
}

impl TrustTrigger {
    /// The spelling written to disk.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProducerVersion => "producer-version",
            Self::Configuration => "configuration",
            Self::Adapter => "adapter",
            Self::ValidationCorpus => "validation-corpus",
            Self::InputContract => "input-contract",
            Self::Environment => "environment",
        }
    }
}

impl std::fmt::Display for TrustTrigger {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A validation result or review that supports a reliance decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustEvidenceReference {
    /// The evidence's own id.
    pub id: String,
    /// A digest over it, `<algorithm>:<hex>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Where it can be found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// The bounded use a decision is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustUse {
    /// The use's own id.
    pub id: String,
    /// What the producer is relied upon to do.
    pub intended_function: String,
    /// The decisions this use is permitted to support.
    pub permitted_decisions: Vec<String>,
}

/// An accountable decision about one producer use, never a global tool badge.
///
/// [`TrustDecision::accepted_context`] is the context the validation justified;
/// [`TrustDecision::observed_context`] is what the current consumer says it is
/// using. Quoin compares only the project-selected triggers and never silently
/// treats absence as acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustDecision {
    /// Always [`STORE_SCHEMA_VERSION`](super::STORE_SCHEMA_VERSION).
    pub schema_version: u32,
    /// `ETD-<digits>`, and the file name under `trust/`.
    pub id: TrustDecisionId,
    /// The bounded use.
    pub r#use: TrustUse,
    /// Whether the producer is relied upon for this use.
    pub decision: TrustDecisionKind,
    /// The context the validation justified.
    pub accepted_context: ProducerContext,
    /// What the consumer reports using now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_context: Option<ProducerContext>,
    /// The triggers this project selected.
    pub revalidate_on: Vec<TrustTrigger>,
    /// The validation results supporting the decision.
    pub validation_evidence: Vec<TrustEvidenceReference>,
    /// Stated limits on the reliance.
    pub limitations: Vec<String>,
    /// Who is accountable.
    pub owner: String,
    /// ISO-8601.
    pub decided_at: String,
}

/// Whether a producer is relied upon for one use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustDecisionKind {
    /// The producer is relied upon.
    ReliedUpon,
    /// The producer is explicitly not relied upon.
    NotReliedUpon,
}

/// What a decision says about the context currently observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustStatus {
    /// The observed context matches on every selected trigger.
    Accepted,
    /// It matches, and the decision states limits on the reliance.
    AcceptedWithLimitations,
    /// A selected trigger differs from the accepted context.
    Invalidated,
    /// The decision records no reliance.
    NotAccepted,
    /// No observed context was supplied, so nothing can be compared.
    Unobserved,
}

impl TrustStatus {
    /// The spelling written to disk and rendered by the assurance view.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::AcceptedWithLimitations => "accepted-with-limitations",
            Self::Invalidated => "invalidated",
            Self::NotAccepted => "not-accepted",
            Self::Unobserved => "unobserved",
        }
    }
}

/// The explainable result of comparing one decision against what is observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustAssessment {
    /// The decision's id.
    pub id: TrustDecisionId,
    /// The bounded use's id.
    pub use_id: String,
    /// The producer's name.
    pub producer: String,
    /// The verdict.
    pub status: TrustStatus,
    /// The decisions this use is permitted to support.
    pub permitted_decisions: Vec<String>,
    /// Stated limits on the reliance.
    pub limitations: Vec<String>,
    /// The triggers that differed, empty unless the status is `invalidated`.
    pub triggered_by: Vec<TrustTrigger>,
    /// Who is accountable.
    pub owner: String,
}
