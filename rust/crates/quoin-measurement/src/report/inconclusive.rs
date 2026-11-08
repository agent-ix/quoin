// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The reason a stage verdict is `inconclusive` (PLAT-958), split from
//! [`super::verdict`] to keep that module under the crate's size ceiling.
//! Re-exported there; codes are the API.

/// Why a stage verdict could not be reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum InconclusiveReason {
    /// The newest collection holds no measured value for the plan's slice.
    NoCurrentValue,
    /// The newest value names a different `planId` than the plan's.
    PlanMismatch,
    /// The newest value was measured under a different `definition_version`
    /// than the plan's.
    DefinitionMismatch,
    /// The newest value states no `population`, so nothing says what it was
    /// measured over.
    PopulationUnstated,
    /// The newest value's population is incomplete.
    IncompletePopulation,
    /// The newest value's population examined nothing.
    EmptyPopulation,
    /// No earlier collection measured the plan's slice under the plan's
    /// `definition_version`, so a ratchet has no best value to hold against.
    NoPrior,
    /// A `target` plan's objective states no bound to measure progress
    /// against. A ratchet never reports it: engineering-assurance's
    /// `Objective` requires a bound for `direction: target`, the one
    /// direction whose ratchet needs one.
    NoBound,
    /// An earlier value of the slice under the plan's `definition_version`
    /// came from a collection whose recorded protected apparatus differs from
    /// the newest one's (PLAT-975).
    ApparatusChanged,
    /// In a protected series, the newest collection, or one behind an earlier
    /// value, recorded no protected apparatus (PLAT-975).
    ApparatusUnrecorded,
    /// A `gate` plan states no `statistical_design.decision_rule` (PLAT-958
    /// part 2). `objective.bound` is informational only and is never a
    /// substitute.
    NoDecisionRule,
    /// The rule's baseline is `constant-predictor`, which needs per-item
    /// answers by answer family; no collection the report layer reads
    /// carries them, so a gate never evaluates this baseline (PLAT-958
    /// part 2). `quoin measurement verify` has the same limit.
    ConstantPredictorUnsupported,
    /// The rule's baseline is `external-reference`: a per-dimension value
    /// resolved from a source outside the plan, which the report layer, like
    /// `quoin measurement verify`, has none of (PLAT-1032).
    ExternalReferenceUnsupplied,
    /// The rule states an `interval_level` and the newest observation states
    /// no valid `interval` (EA-26, FR-107-AC-10). A malformed interval is
    /// reported the same way here; `quoin measurement verify` is what
    /// rejects it.
    IntervalUnstated,
    /// The newest observation's `interval` level is below the rule's
    /// `interval_level` (EA-26, FR-107-AC-10).
    IntervalLevelShort,
    /// Engineering-assurance could not evaluate the rule on these numbers —
    /// a non-finite estimate or reference, or an estimate outside its stated
    /// interval (PLAT-958 part 2, EA-26).
    RuleNotEvaluable,
}

impl InconclusiveReason {
    crate::all_variants::all_variants! {
        /// Every reason, in declaration order; a variant missing from the list
        /// is a compile error.
        InconclusiveReason:
        NoCurrentValue,
        PlanMismatch,
        DefinitionMismatch,
        PopulationUnstated,
        IncompletePopulation,
        EmptyPopulation,
        NoPrior,
        NoBound,
        ApparatusChanged,
        ApparatusUnrecorded,
        NoDecisionRule,
        ConstantPredictorUnsupported,
        ExternalReferenceUnsupplied,
        IntervalUnstated,
        IntervalLevelShort,
        RuleNotEvaluable,
    }

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoCurrentValue => "no_current_value",
            Self::PlanMismatch => "plan_mismatch",
            Self::DefinitionMismatch => "definition_mismatch",
            Self::PopulationUnstated => "population_unstated",
            Self::IncompletePopulation => "incomplete_population",
            Self::EmptyPopulation => "empty_population",
            Self::NoPrior => "no_prior",
            Self::NoBound => "no_bound",
            Self::ApparatusChanged => "apparatus_changed",
            Self::ApparatusUnrecorded => "apparatus_unrecorded",
            Self::NoDecisionRule => "no_decision_rule",
            Self::ConstantPredictorUnsupported => "constant_predictor_unsupported",
            Self::ExternalReferenceUnsupplied => "external_reference_unsupplied",
            Self::IntervalUnstated => "interval_unstated",
            Self::IntervalLevelShort => "interval_level_short",
            Self::RuleNotEvaluable => "rule_not_evaluable",
        }
    }

    /// Recover a reason from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }

    /// The sentence the text report prints beside the code. Not contractual;
    /// the code is.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::NoCurrentValue => "no measured value in the newest collection",
            Self::PlanMismatch => "the newest value names another plan",
            Self::PopulationUnstated => "the newest value states no population",
            Self::DefinitionMismatch => {
                "the newest value was measured under another definition version"
            }
            Self::IncompletePopulation => "the newest value's population is incomplete",
            Self::EmptyPopulation => "the newest value's population examined nothing",
            Self::NoPrior => {
                "no earlier collection measured this plan under its definition version"
            }
            Self::NoBound => "the objective states no bound",
            Self::ApparatusChanged => {
                "an earlier value was measured with a different protected apparatus"
            }
            Self::ApparatusUnrecorded => "a collection recorded no protected apparatus",
            Self::NoDecisionRule => "the plan states no decision rule",
            Self::ConstantPredictorUnsupported => {
                "the rule's constant-predictor baseline needs per-item answers no collection carries"
            }
            Self::ExternalReferenceUnsupplied => {
                "the rule's external-reference baseline has no source quoin can read it from"
            }
            Self::IntervalUnstated => {
                "the plan's rule needs an interval and the newest value states none"
            }
            Self::IntervalLevelShort => {
                "the newest value's interval is at a level below the rule's interval_level"
            }
            Self::RuleNotEvaluable => "the decision rule could not be evaluated on these numbers",
        }
    }
}
