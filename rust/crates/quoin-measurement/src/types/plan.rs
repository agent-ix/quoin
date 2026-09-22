// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `MeasurementPlan` that governs whether an observation may be recorded.
//!
//! Ports `src/measurement/types.ts:60-77`, and the `STAGES` / status sets that
//! `src/measurement/plans.ts:7-15,57-68` checks against.

use quoin_store::RawFileSha256Digest;

use crate::types::ids::NonEmptyText;

/// Where a plan is in its lifecycle.
///
/// Shared with [`crate::types::profile::AssuranceProfileSummary`]: `plans.ts:62`
/// and `profiles.ts:50` check the same three spellings, so there is one enum
/// rather than two.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LifecycleStatus {
    /// Drafted; observations against it are refused.
    Proposed,
    /// The one status that admits an observation.
    Active,
    /// Withdrawn; observations against it are refused.
    Retired,
}

impl LifecycleStatus {
    /// Every status, in declaration order.
    pub const ALL: [Self; 3] = [Self::Proposed, Self::Active, Self::Retired];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Retired => "retired",
        }
    }

    /// Recover a status from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// How far along the measurement ladder a plan sits.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MeasurementStage {
    /// Record the value; assert nothing.
    Observe,
    /// Establish the value to compare against.
    Baseline,
    /// Compare two branches.
    BranchComparison,
    /// Watch the value over time.
    Trend,
    /// Refuse a regression against the best value seen.
    Ratchet,
    /// Drive towards a stated value.
    Target,
    /// Refuse a value outside the stated bound.
    Gate,
}

impl MeasurementStage {
    /// Every stage, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Observe,
        Self::Baseline,
        Self::BranchComparison,
        Self::Trend,
        Self::Ratchet,
        Self::Target,
        Self::Gate,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Baseline => "baseline",
            Self::BranchComparison => "branch-comparison",
            Self::Trend => "trend",
            Self::Ratchet => "ratchet",
            Self::Target => "target",
            Self::Gate => "gate",
        }
    }

    /// Recover a stage from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// One measurement plan, as read from an assurance document's frontmatter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasurementPlan {
    /// The plan's identity.
    pub id: NonEmptyText,
    /// Its human title.
    pub title: NonEmptyText,
    /// Where it is in its lifecycle.
    pub status: LifecycleStatus,
    /// How far along the measurement ladder it sits.
    pub stage: MeasurementStage,
    /// The metric it governs.
    pub metric: NonEmptyText,
    /// The definition version an observation must match.
    pub definition_version: NonEmptyText,
    /// The repository-relative, `/`-separated document it was read from.
    pub path: String,
    /// Who owns it, when governance fields were requested.
    pub owner: Option<String>,
    /// What to do about it, when governance fields were requested.
    pub action: Option<String>,
    /// The plan's own tamper-evidence attestation over its "Comparison and
    /// Enforcement" section (PLAT-936), when the document declares one.
    pub preregistration: Option<PlanPreregistration>,
}

impl MeasurementPlan {
    /// The order `plans.ts:25` sorts loaded plans into: metric, then id.
    #[must_use]
    pub fn sort_key(&self) -> (&str, &str) {
        (self.metric.as_str(), self.id.as_str())
    }
}

/// A `MeasurementPlan`'s `preregistration` block: tamper evidence over its own
/// "Comparison and Enforcement" section, not an ordering proof.
///
/// This closes exactly one gap: a bar text silently edited after it was
/// recorded, without the frontmatter digest being recomputed to match. It
/// proves nothing about *when* the bar was written relative to any result —
/// PLAT-935's design research found that check unbuildable, because both a
/// commit's author-date and a collection's `timestamp` are self-reported
/// strings nobody attests independently. Residual gaps this does not close:
/// reporting only a favourable pre-registered variant, repeated re-runs until
/// one passes, and editing an unprotected input (such as the baseline) instead
/// of the bar text. See PLAT-936.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanPreregistration {
    /// The digest the document's frontmatter declares.
    pub declared_digest: RawFileSha256Digest,
    /// The digest actually computed from the document's own "Comparison and
    /// Enforcement" section at load time.
    pub computed_digest: RawFileSha256Digest,
}

impl PlanPreregistration {
    /// Whether the declared digest still matches the section as it reads now.
    ///
    /// `false` is exactly the condition PLAT-936 exists to catch: the section
    /// changed since `declared_digest` was recorded, and nobody updated it to
    /// match.
    #[must_use]
    pub fn matches(&self) -> bool {
        self.declared_digest == self.computed_digest
    }
}
