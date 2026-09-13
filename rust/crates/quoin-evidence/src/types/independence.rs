// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Profile-selected independence requirements and their assessments (FR-094).

use serde::{Deserialize, Serialize};

use crate::ids::{ObligationId, ProfileId, SuiteId};

/// One separation axis a profile can ask two evidence lines to differ on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum IndependenceDimension {
    /// Who produced the evidence.
    Actor,
    /// The toolchain the implementation was built with.
    ImplementationToolchain,
    /// The verification technique.
    Technique,
    /// Where the inputs came from.
    DataSource,
    /// The review path the result travelled.
    ReviewPath,
}

impl IndependenceDimension {
    /// Every dimension, in the order the retained union declares them.
    pub const ALL: [Self; 5] = [
        Self::Actor,
        Self::ImplementationToolchain,
        Self::Technique,
        Self::DataSource,
        Self::ReviewPath,
    ];

    /// The spelling written to disk and printed in a summary.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::ImplementationToolchain => "implementation-toolchain",
            Self::Technique => "technique",
            Self::DataSource => "data-source",
            Self::ReviewPath => "review-path",
        }
    }
}

impl std::fmt::Display for IndependenceDimension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One exact obligation for which a profile requests two separated lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IndependenceRequirement {
    /// The requirement's own id, unique within the policy.
    pub id: String,
    /// The obligation it applies to, named at most once in the policy.
    pub obligation: ObligationId,
    /// The dimensions two lines must both differ on.
    pub dimensions: Vec<IndependenceDimension>,
    /// Why the profile asks for it.
    pub rationale: String,
}

/// Normalized projection of profile-selected independence requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IndependencePolicy {
    /// Always `1`.
    pub schema_version: u32,
    /// `AP-<digits>`.
    pub profile: ProfileId,
    /// The requirements, as the profile declared them.
    pub requirements: Vec<IndependenceRequirement>,
}

/// What one dimension looked like across the bound suites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IndependenceDimensionAssessment {
    /// The dimension.
    pub dimension: IndependenceDimension,
    /// The distinct recorded values, sorted and deduplicated.
    pub values: Vec<String>,
    /// The suites whose lineage records nothing for it, sorted.
    pub missing_suites: Vec<SuiteId>,
}

/// Whether two separated lines were found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum IndependenceStatus {
    /// Two bound suites differ on every requested dimension.
    Satisfied,
    /// No such pair exists.
    Insufficient,
}

impl IndependenceStatus {
    /// The spelling written to disk and rendered by the assurance view.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Insufficient => "insufficient",
        }
    }
}

/// Explainable result for one requested obligation, successful or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IndependenceAssessment {
    /// The profile that asked.
    pub profile: ProfileId,
    /// The requirement's id.
    pub requirement: String,
    /// The obligation it applies to.
    pub obligation: ObligationId,
    /// The verdict.
    pub status: IndependenceStatus,
    /// One entry per requested dimension, in the requested order.
    pub dimensions: Vec<IndependenceDimensionAssessment>,
    /// The two suites that satisfied it, when one pair did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub satisfied_by: Option<(SuiteId, SuiteId)>,
    /// A sentence naming what was found or what was missing.
    pub summary: String,
}
