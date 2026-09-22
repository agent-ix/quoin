// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `MeasurementPlan` that governs whether an observation may be recorded.
//!
//! Ports `src/measurement/types.ts:60-77`, and the `STAGES` / status sets that
//! `src/measurement/plans.ts:7-15,57-68` checks against.

use std::fmt;
use std::num::NonZeroU32;

use engineering_assurance::measurement::Objective;
use quoin_store::{RawFileSha256Digest, digest_bytes_sha256};

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

/// How the correct answers a plan grades against were produced (PLAT-960).
///
/// The engineering-assurance `MeasurementPlan` schema's `ground_truth_kind`,
/// with its three spellings. Carried into `quoin report` beside the plan so a
/// reader can tell a gate graded against people's labels from one graded
/// against another agent's.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GroundTruthKind {
    /// Labelled by a person.
    HumanLabelled,
    /// Labelled by another AI agent.
    AgentLabelled,
    /// Derived mechanically, e.g. by a deterministic oracle or a replay.
    Mechanical,
}

impl GroundTruthKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 3] = [Self::HumanLabelled, Self::AgentLabelled, Self::Mechanical];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HumanLabelled => "human-labelled",
            Self::AgentLabelled => "agent-labelled",
            Self::Mechanical => "mechanical",
        }
    }

    /// Recover a kind from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// The members of a plan's `statistical_design` block this crate reads
/// (PLAT-960).
///
/// The engineering-assurance schema requires seven members of this block;
/// the prose ones (`population`, `sampling`, `estimator`, `error_model`,
/// `uncertainty`, `decision_rule`) are that schema's to validate and are not
/// read here. Each member below is optional so a plan without it loads exactly
/// as it did before PLAT-960.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct StatisticalDesign {
    /// The smallest `population.examined` a measured observation may carry
    /// before intake refuses it.
    pub minimum_population: Option<NonZeroU32>,
    /// How many repeat runs the plan requires before a result counts,
    /// checked at intake against each measured observation's
    /// `population.repetitions`.
    pub repetitions: Option<NonZeroU32>,
}

/// One measurement plan, as read from an assurance document's frontmatter.
///
/// `PartialEq` only, not `Eq`: [`Objective`]'s bound is an `f64`.
#[derive(Clone, Debug, PartialEq)]
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
    /// How the plan's ground truth was produced, when the document says
    /// (PLAT-960).
    pub ground_truth_kind: Option<GroundTruthKind>,
    /// The `statistical_design` members this crate reads, when the document
    /// declares the block (PLAT-960).
    pub statistical_design: Option<StatisticalDesign>,
    /// Which way the metric should move, and the informational goal, when the
    /// document states an `objective` (PLAT-958). The type is
    /// engineering-assurance's: EA owns the block's schema and validation.
    pub objective: Option<Objective>,
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
    pub declared_digest: BarDigest,
    /// The digest actually computed from the document's own "Comparison and
    /// Enforcement" section at load time.
    pub computed_digest: BarDigest,
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

/// A sha256 digest over a plan's normalized "Comparison and Enforcement"
/// section text (PLAT-936).
///
/// Deliberately **not** [`RawFileSha256Digest`]. That type's domain, per
/// `quoin-store`'s own `DigestDomain` doc, is "the complete bytes of a file on
/// disk... as measurement records and contract pins reference them" —
/// `FR-201-canonical-identity-domain` forbids substituting one digest domain
/// for another, even when the algorithm and stored spelling agree. This
/// digest is over neither a whole file nor bytes exactly as supplied: it is a
/// normalized **excerpt** of a document's body (CRLF collapsed, each line's
/// trailing whitespace trimmed, leading/trailing blank lines dropped before
/// hashing — see [`crate::plans`]'s `bar_section`). That is a fourth question
/// none of `quoin-store`'s existing domains answer, so it gets its own type
/// rather than borrowing one whose doc comment would then be wrong about what
/// it holds.
///
/// What *is* shared with [`RawFileSha256Digest`], deliberately: the algorithm
/// and the `sha256:<64 hex>` stored spelling, and the hashing itself still
/// routes through [`quoin_store::digest_bytes_sha256`] — the one sha256 call
/// site `FR-100-CON-4` permits this crate — via [`BarDigest::of`].
/// [`BarDigest::parse_stored`] reuses [`RawFileSha256Digest::parse_stored`]
/// purely as a format validator (same grammar, so no second parser), and
/// immediately re-wraps the result rather than keeping it: nothing here ever
/// holds a live `RawFileSha256Digest`, so a `BarDigest` cannot compile where a
/// raw-file digest belongs, or vice versa.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct BarDigest(String);

impl BarDigest {
    /// Parse a stored `sha256:<64 hex>` value, or `None` when it is not one.
    #[must_use]
    pub fn parse_stored(value: &str) -> Option<Self> {
        RawFileSha256Digest::parse_stored(value)
            .ok()
            .map(|digest| Self(digest.to_stored()))
    }

    /// The digest of `bytes` exactly as supplied — the caller normalizes
    /// first; this does not.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(digest_bytes_sha256(bytes).to_stored())
    }

    /// The stored spelling, `sha256:` prefix included.
    #[must_use]
    pub fn as_stored(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BarDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
