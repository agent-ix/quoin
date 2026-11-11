// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `MeasurementPlan` that governs whether an observation may be recorded.
//!
//! Ports `src/measurement/types.ts:60-77`, and the `STAGES` / status sets that
//! `src/measurement/plans.ts:7-15,57-68` checks against.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use engineering_assurance::measurement::{
    AnswerFamilies, DecisionRule, Estimator, NegativeControls, Objective, ProtectedApparatus,
};

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
/// (PLAT-960, PLAT-961).
///
/// The engineering-assurance schema requires seven members of this block;
/// the prose ones (`population`, `sampling`, `error_model`, `uncertainty`) are
/// that schema's to validate and are not read here. `estimator` and
/// `decision_rule` are closed data since engineering-assurance FR-021 and are
/// read into EA's own types. Each member below is optional so a plan without
/// it loads exactly as it did before.
///
/// `PartialEq` only, not `Eq`: [`DecisionRule`]'s threshold and margin are
/// `f64`.
#[derive(Clone, Debug, PartialEq)]
pub struct StatisticalDesign {
    /// The smallest `population.examined` a measured observation may carry
    /// before intake refuses it.
    pub minimum_population: Option<NonZeroU32>,
    /// How many repeat runs the plan requires before a result counts,
    /// checked at intake against each measured observation's
    /// `population.repetitions`.
    pub repetitions: Option<NonZeroU32>,
    /// How the metric is computed from the collected population
    /// (engineering-assurance FR-021).
    pub estimator: Option<Estimator>,
    /// The rule `quoin measurement verify` applies to the estimate
    /// (engineering-assurance FR-021). EA owns its evaluation.
    pub decision_rule: Option<DecisionRule>,
    /// The families the constant-predictor baseline is computed over
    /// (engineering-assurance, PLAT-1020). A set; part of the versioned
    /// measurement definition.
    pub answer_families: Option<AnswerFamilies>,
    /// The enumerated set of ids a percentage metric is taken over (PLAT-110).
    pub population_set: Option<PopulationSet>,
    /// Why this plan's percentage has no enumerable population (PLAT-1401):
    /// a non-empty reason. A `proportion` plan declares this or a
    /// `population_set`, or its metric cannot read as a pass.
    pub population_exempt: Option<String>,
}

impl StatisticalDesign {
    /// Whether this plan is a `proportion` that neither declares a
    /// `population_set` nor states why it has none (PLAT-1401).
    #[must_use]
    pub fn proportion_without_population(&self) -> bool {
        self.estimator == Some(Estimator::Proportion)
            && self.population_set.is_none()
            && self.population_exempt.is_none()
    }
}

/// The plan-declared enumeration of a percentage metric's population: a
/// non-empty set of distinct, non-empty ids. The denominator of the metric is
/// this set, so a producer cannot choose it (PLAT-110).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationSet(BTreeSet<String>);

impl PopulationSet {
    /// Build a set from declared ids.
    ///
    /// # Errors
    ///
    /// A message naming the problem when `ids` is empty, holds an empty id,
    /// or repeats an id.
    pub fn new(ids: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut set = BTreeSet::new();
        for id in ids {
            if id.is_empty() {
                return Err("an id must not be empty".to_owned());
            }
            if let Some(repeated) = set.replace(id) {
                return Err(format!("id `{repeated}` is declared twice"));
            }
        }
        if set.is_empty() {
            return Err("the set must hold at least one id".to_owned());
        }
        Ok(Self(set))
    }

    /// Whether `id` belongs to the set.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.0.contains(id)
    }

    /// The declared ids, in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// How many ids the set declares.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`: a set is never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
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
    /// Repository-relative path to an EA `MeasurementProcedure` record when
    /// this plan participates in a runnable campaign. Historical plans need
    /// no procedure and remain readable.
    pub execution_procedure: Option<String>,
    /// The repository-relative, `/`-separated document it was read from.
    pub path: String,
    /// Who owns it, when governance fields were requested.
    pub owner: Option<String>,
    /// What to do about it, when governance fields were requested.
    pub action: Option<String>,
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
    /// The files that produce the plan's number, when the document declares
    /// `protected_apparatus` (PLAT-975). The type is engineering-assurance's
    /// (its FR-024); intake resolves every entry when it writes a
    /// collection this plan governs.
    pub protected_apparatus: Option<ProtectedApparatus>,
    /// The gaming scenarios the plan declares it guards against, when the
    /// document declares `negative_controls` (engineering-assurance FR-024).
    /// The checker reads only whether `apparatus-edit` is among them.
    pub negative_controls: Option<NegativeControls>,
    /// Whether results from collections built under different profiles may be
    /// compared. Absent in the document means `false`: a profile change blocks
    /// the comparison unless the plan says otherwise.
    pub cross_profile_comparison: bool,
    /// The relation two runs must satisfy for this plan's metric, when the
    /// document states `paired_run` (PLAT-114).
    pub paired_run: Option<PairedRun>,
}

/// How a paired-run metric relates the same slice in two runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PairedRelation {
    /// The two values are equal.
    Equal,
    /// The two values differ by at most this finite, non-negative amount.
    WithinDelta(f64),
    /// The later value is no worse than the earlier under the plan's
    /// `objective`.
    NotWorse,
}

/// A plan's `paired_run` block: the relation and how many slice comparisons
/// a run pair must contain before the relation says anything.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairedRun {
    /// What must hold between the two runs.
    pub relation: PairedRelation,
    /// Fewer comparable slices than this is inconclusive, never zero.
    pub minimum_pairs: NonZeroU32,
}

impl MeasurementPlan {
    /// The order `plans.ts:25` sorts loaded plans into: metric, then id.
    #[must_use]
    pub fn sort_key(&self) -> (&str, &str) {
        (self.metric.as_str(), self.id.as_str())
    }
}
