// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The checker's verdict and reason-code vocabulary (quoin FR-108-AC-5).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling. Codes are the API: add members, never rename or reuse one.

/// The checker's answer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Verdict {
    /// The rule holds on data the checker could check, with nothing wrong.
    Accept,
    /// The data contradicts itself, the plan, the rule or the claim.
    Reject,
    /// Nothing wrong was found, but the data cannot support a verdict.
    Inconclusive,
}

impl Verdict {
    /// Every verdict, in declaration order.
    pub const ALL: [Self; 3] = [Self::Accept, Self::Reject, Self::Inconclusive];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Reject => "reject",
            Self::Inconclusive => "inconclusive",
        }
    }

    /// Recover a verdict from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// Why the checker did not accept. Each reason implies one verdict,
/// [`Reason::verdict`]; codes are the API and are never renamed or reused.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Reason {
    /// The plan states no `statistical_design.decision_rule`.
    NoDecisionRule,
    /// The plan states no `statistical_design.estimator`.
    NoEstimator,
    /// No stored collection measured the plan under its definition.
    NoCollections,
    /// An observation carries no measured value.
    NoValue,
    /// No `population`, `examined` or `complete`; no `matched` under a
    /// `proportion` or `count` estimator; or no `repetitions` when the plan
    /// requires more than one.
    PopulationUnstated,
    /// The population states `complete: false`.
    PopulationIncomplete,
    /// The population examined nothing, under a plan with no
    /// `minimum_population` (under one, it is below the minimum).
    PopulationEmpty,
    /// A baseline rule has no earlier usable run to compare against.
    NoPrior,
    /// The candidate, or the prior a `prior-collection` rule compares
    /// against, is tied in intake order with another run.
    OrderUnattested,
    /// A `constant-predictor` baseline needs per-item answers by answer
    /// family, which no collection carries.
    ConstantPredictorRowsAbsent,
    /// Engineering-assurance could not evaluate the rule on these numbers.
    RuleNotEvaluable,
    /// A `proportion` observation's unit is not a fraction (`percent of …`,
    /// or anything else not spelled `fraction` or `fraction of …`).
    UnitUnsupported,
    /// A slice an earlier run measured under this definition is absent from
    /// the candidate.
    SliceMissing,
    /// A collection of the candidate's subject and scope that is not before
    /// it in intake order carries no observation of the plan.
    ObservationMissing,
    /// In a protected series, a run — the candidate or an earlier one —
    /// recorded no protected apparatus (PLAT-975).
    ApparatusUnrecorded,
    /// The rule does not hold for the candidate's estimate.
    RuleNotMet,
    /// A stored `value` disagrees with the estimate recomputed from its
    /// `matched` and `examined`.
    ValueDisagreesWithRows,
    /// `examined` is below the plan's `minimum_population`.
    PopulationBelowMinimum,
    /// `repetitions` is below the plan's.
    RepetitionsShort,
    /// `examined`, `matched` or `repetitions` is not a whole number, or
    /// `matched` exceeds `examined`.
    PopulationMalformed,
    /// A regressed run with the candidate's own apparatus preceded it.
    RerunUntilPass,
    /// An earlier run under this definition recorded a different protected
    /// apparatus than the candidate: the apparatus changed without the
    /// `definition_version` bump engineering-assurance FR-024 requires
    /// (PLAT-975). That run is not a usable baseline.
    ApparatusEdit,
    /// The claimed verdict is not the checker's.
    ClaimedVerdictDisagrees,
    /// A collection's recorded protected-apparatus digest for this plan does
    /// not match `git show <sourceRevision>:<path>` — the file's actual
    /// bytes at the commit the collection claims to be from. A collection
    /// written by hand, rather than through intake's own resolver, can state
    /// any digest it likes; this is the check that catches it (PLAT-985).
    ApparatusForged,
    /// A collection that measured this plan was added to the store and later
    /// removed, so it no longer appears among the runs counted — detected
    /// from the store's git history, not from anything a producer states
    /// (PLAT-985).
    CollectionDeleted,
    /// A collection's stored file was edited by a commit after the one that
    /// first added it — detected from the store's git history (PLAT-985).
    CollectionEdited,
    /// The plan's objective, estimator, decision rule or protected apparatus
    /// changed between two committed revisions that share a
    /// `definition_version` (engineering-assurance FR-021's
    /// `definition_change_without_version_bump`).
    DefinitionChangedWithoutVersionBump,
}

impl Reason {
    /// Every reason, in declaration order.
    pub const ALL: [Self; 27] = [
        Self::NoDecisionRule,
        Self::NoEstimator,
        Self::NoCollections,
        Self::NoValue,
        Self::PopulationUnstated,
        Self::PopulationIncomplete,
        Self::PopulationEmpty,
        Self::NoPrior,
        Self::OrderUnattested,
        Self::ConstantPredictorRowsAbsent,
        Self::RuleNotEvaluable,
        Self::UnitUnsupported,
        Self::SliceMissing,
        Self::ObservationMissing,
        Self::ApparatusUnrecorded,
        Self::RuleNotMet,
        Self::ValueDisagreesWithRows,
        Self::PopulationBelowMinimum,
        Self::RepetitionsShort,
        Self::PopulationMalformed,
        Self::RerunUntilPass,
        Self::ApparatusEdit,
        Self::ClaimedVerdictDisagrees,
        Self::ApparatusForged,
        Self::CollectionDeleted,
        Self::CollectionEdited,
        Self::DefinitionChangedWithoutVersionBump,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoDecisionRule => "no_decision_rule",
            Self::NoEstimator => "no_estimator",
            Self::NoCollections => "no_collections",
            Self::NoValue => "no_value",
            Self::PopulationUnstated => "population_unstated",
            Self::PopulationIncomplete => "population_incomplete",
            Self::PopulationEmpty => "population_empty",
            Self::NoPrior => "no_prior",
            Self::OrderUnattested => "order_unattested",
            Self::ConstantPredictorRowsAbsent => "constant_predictor_rows_absent",
            Self::RuleNotEvaluable => "rule_not_evaluable",
            Self::UnitUnsupported => "unit_unsupported",
            Self::SliceMissing => "slice_missing",
            Self::ObservationMissing => "observation_missing",
            Self::ApparatusUnrecorded => "apparatus_unrecorded",
            Self::RuleNotMet => "rule_not_met",
            Self::ValueDisagreesWithRows => "value_disagrees_with_rows",
            Self::PopulationBelowMinimum => "population_below_minimum",
            Self::RepetitionsShort => "repetitions_short",
            Self::PopulationMalformed => "population_malformed",
            Self::RerunUntilPass => "rerun_until_pass",
            Self::ApparatusEdit => "apparatus_edit",
            Self::ClaimedVerdictDisagrees => "claimed_verdict_disagrees",
            Self::ApparatusForged => "apparatus_forged",
            Self::CollectionDeleted => "collection_deleted",
            Self::CollectionEdited => "collection_edited",
            Self::DefinitionChangedWithoutVersionBump => "definition_changed_without_version_bump",
        }
    }

    /// Recover a reason from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }

    /// Whether a run before the candidate carrying this reason still counts
    /// against the candidate: only evidence of tampering does. An earlier
    /// run that fell short of the plan is excluded from baselines and
    /// otherwise left in the past.
    #[must_use]
    pub const fn carries_from_history(self) -> bool {
        matches!(
            self,
            Self::ValueDisagreesWithRows | Self::PopulationMalformed
        )
    }

    /// The verdict this reason forces: a contradiction rejects, missing
    /// evidence leaves the question open.
    #[must_use]
    pub const fn verdict(self) -> Verdict {
        match self {
            Self::NoDecisionRule
            | Self::NoEstimator
            | Self::NoCollections
            | Self::NoValue
            | Self::PopulationUnstated
            | Self::PopulationIncomplete
            | Self::PopulationEmpty
            | Self::NoPrior
            | Self::OrderUnattested
            | Self::ConstantPredictorRowsAbsent
            | Self::RuleNotEvaluable
            | Self::UnitUnsupported
            | Self::SliceMissing
            | Self::ObservationMissing
            | Self::ApparatusUnrecorded => Verdict::Inconclusive,
            Self::RuleNotMet
            | Self::ValueDisagreesWithRows
            | Self::PopulationBelowMinimum
            | Self::RepetitionsShort
            | Self::PopulationMalformed
            | Self::RerunUntilPass
            | Self::ApparatusEdit
            | Self::ClaimedVerdictDisagrees
            | Self::ApparatusForged
            | Self::CollectionDeleted
            | Self::CollectionEdited
            | Self::DefinitionChangedWithoutVersionBump => Verdict::Reject,
        }
    }
}
