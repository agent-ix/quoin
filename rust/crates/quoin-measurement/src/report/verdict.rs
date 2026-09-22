// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What a `ratchet` or `target` plan's objective says about its newest value
//! (PLAT-958).
//!
//! [`crate::compare`] answers only whether two values may be compared and
//! stays verdict-free. The verdict lives here, in the report layer, because it
//! is a policy the plan authored — its stage and its `objective` — applied to
//! the evidence the report already holds.
//!
//! # The rule
//!
//! A value's *badness* under an objective is one number where smaller is
//! better — see `badness`. A `ratchet` plan's newest value is `held` when its
//! badness is no greater than the best (least bad) value any **earlier**
//! collection measured for the same plan, slice and `definition_version`, and
//! `regressed` otherwise. A `target` plan reports how far the newest value
//! sits from the objective's bound and whether it has reached it; that is
//! information, not a pass/fail verdict. `gate` is not decided here.
//!
//! # Never green on missing evidence
//!
//! No newest value, a value measured under another definition, and an
//! incomplete or empty population all give `inconclusive`, never `held` or
//! `reached`. So does a ratchet with nothing earlier to hold against: the first
//! collection a ratchet ever sees has no floor yet, and calling that `held`
//! would be a green verdict with no comparison behind it — the failure quoin's
//! `--ratchet` with no baseline once had (CR-029, agent-ix/quoin#169). The
//! same rule decides which earlier values may *set* the best: an earlier value
//! that is incomplete, empty or under another definition is not one.

use std::cmp::Ordering;

use engineering_assurance::measurement::{Direction, Objective};

use crate::compare::incomplete;
use crate::types::collection::MeasurementCollection;
use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::{MeasurementPlan, MeasurementStage};

/// Why a stage verdict could not be reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum InconclusiveReason {
    /// The newest collection holds no measured value for the plan's slice.
    NoCurrentValue,
    /// The newest value was measured under a different `definition_version`
    /// than the plan's.
    DefinitionMismatch,
    /// The newest value's population is incomplete.
    IncompletePopulation,
    /// The newest value's population examined nothing.
    EmptyPopulation,
    /// No earlier collection measured the plan's slice under the plan's
    /// `definition_version`, so a ratchet has no best value to hold against.
    NoPrior,
    /// The objective states no bound where the verdict needs one.
    NoBound,
}

impl InconclusiveReason {
    /// Every reason, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::NoCurrentValue,
        Self::DefinitionMismatch,
        Self::IncompletePopulation,
        Self::EmptyPopulation,
        Self::NoPrior,
        Self::NoBound,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoCurrentValue => "no_current_value",
            Self::DefinitionMismatch => "definition_mismatch",
            Self::IncompletePopulation => "incomplete_population",
            Self::EmptyPopulation => "empty_population",
            Self::NoPrior => "no_prior",
            Self::NoBound => "no_bound",
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
            Self::DefinitionMismatch => {
                "the newest value was measured under another definition version"
            }
            Self::IncompletePopulation => "the newest value's population is incomplete",
            Self::EmptyPopulation => "the newest value's population examined nothing",
            Self::NoPrior => {
                "no earlier collection measured this plan under its definition version"
            }
            Self::NoBound => "the objective states no bound",
        }
    }
}

/// The best earlier value a ratchet holds against, and where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct BestPrior {
    /// The value.
    pub value: f64,
    /// The collection that measured it — the earliest, when several tie.
    pub collection_id: String,
}

/// A `ratchet` plan's verdict on its newest value.
#[derive(Clone, Debug, PartialEq)]
pub enum RatchetOutcome {
    /// The newest value is no worse than the best earlier one.
    Held {
        /// The newest value.
        current: f64,
        /// The best earlier value.
        best_prior: BestPrior,
    },
    /// The newest value is worse than the best earlier one.
    Regressed {
        /// The newest value.
        current: f64,
        /// The best earlier value.
        best_prior: BestPrior,
    },
    /// No verdict, for this reason.
    Inconclusive(InconclusiveReason),
}

impl RatchetOutcome {
    /// The stable wire spelling of the verdict.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::Held { .. } => "held",
            Self::Regressed { .. } => "regressed",
            Self::Inconclusive(_) => "inconclusive",
        }
    }
}

/// How far a `target` plan's newest value is from its objective's bound.
#[derive(Clone, Debug, PartialEq)]
pub enum TargetOutcome {
    /// The newest value, measured against the bound.
    Measured {
        /// The newest value.
        current: f64,
        /// The objective's bound.
        bound: f64,
        /// How far `current` is from `bound`, never negative — see
        /// `target_progress`.
        distance: f64,
        /// Whether `current` has reached `bound` in the objective's direction.
        reached: bool,
    },
    /// No measurement against the bound, for this reason.
    Inconclusive(InconclusiveReason),
}

impl TargetOutcome {
    /// The stable wire spelling of the outcome.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::Measured { reached: true, .. } => "reached",
            Self::Measured { reached: false, .. } => "not_reached",
            Self::Inconclusive(_) => "inconclusive",
        }
    }
}

/// What a plan's stage and objective say about one report row.
#[derive(Clone, Debug, PartialEq)]
pub enum StageVerdict {
    /// A `ratchet` plan's verdict.
    Ratchet {
        /// The objective the verdict was reached under.
        objective: Objective,
        /// The verdict.
        outcome: RatchetOutcome,
    },
    /// A `target` plan's progress. Informational: not a pass/fail verdict.
    Target {
        /// The objective the progress was measured under.
        objective: Objective,
        /// The progress.
        outcome: TargetOutcome,
    },
}

impl StageVerdict {
    /// The stage this verdict belongs to.
    #[must_use]
    pub const fn stage(&self) -> MeasurementStage {
        match *self {
            Self::Ratchet { .. } => MeasurementStage::Ratchet,
            Self::Target { .. } => MeasurementStage::Target,
        }
    }

    /// The objective it was reached under.
    #[must_use]
    pub const fn objective(&self) -> Objective {
        match *self {
            Self::Ratchet { objective, .. } | Self::Target { objective, .. } => objective,
        }
    }

    /// The stable wire spelling of the verdict or outcome.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::Ratchet { ref outcome, .. } => outcome.as_str(),
            Self::Target { ref outcome, .. } => outcome.as_str(),
        }
    }

    /// Why there is no verdict, when there is none.
    #[must_use]
    pub const fn inconclusive_reason(&self) -> Option<InconclusiveReason> {
        match *self {
            Self::Ratchet {
                outcome: RatchetOutcome::Inconclusive(reason),
                ..
            }
            | Self::Target {
                outcome: TargetOutcome::Inconclusive(reason),
                ..
            } => Some(reason),
            Self::Ratchet { .. } | Self::Target { .. } => None,
        }
    }
}

/// The verdict for one report row, or `None` when the plan states no
/// `objective` or sits at a stage this module does not decide.
///
/// `observation` is the row's newest observation; `earlier` is every
/// collection older than the one it came from, in collection order.
#[must_use]
pub fn stage_verdict(
    plan: &MeasurementPlan,
    observation: Option<&MeasurementObservation>,
    earlier: &[MeasurementCollection],
) -> Option<StageVerdict> {
    let objective = plan.objective?;
    match plan.stage {
        MeasurementStage::Ratchet => Some(StageVerdict::Ratchet {
            objective,
            outcome: ratchet(plan, objective, observation, earlier),
        }),
        MeasurementStage::Target => Some(StageVerdict::Target {
            objective,
            outcome: target_progress(plan, objective, observation),
        }),
        MeasurementStage::Observe
        | MeasurementStage::Baseline
        | MeasurementStage::BranchComparison
        | MeasurementStage::Trend
        | MeasurementStage::Gate => None,
    }
}

/// A value's badness under `objective`: one number where smaller is better,
/// or `None` for a `target` objective with no bound to measure against.
///
/// `higher` → `-value`; `lower` → `value`; `zero` → `|value|`; `target` →
/// `|value - bound|`. The one place a direction's meaning of "better" is
/// written down.
fn badness(objective: Objective, value: f64) -> Option<f64> {
    match objective.direction() {
        Direction::Higher => Some(-value),
        Direction::Lower => Some(value),
        Direction::Zero => Some(value.abs()),
        Direction::Target => objective.bound().map(|bound| (value - bound).abs()),
    }
}

/// The value `observation` may contribute to a verdict under `plan`, or why
/// it may not.
fn usable(
    plan: &MeasurementPlan,
    observation: Option<&MeasurementObservation>,
) -> Result<f64, InconclusiveReason> {
    let observation = observation.ok_or(InconclusiveReason::NoCurrentValue)?;
    if observation.definition_version.as_str() != plan.definition_version.as_str() {
        return Err(InconclusiveReason::DefinitionMismatch);
    }
    let value = match (observation.state, observation.value) {
        (MeasurementState::Measured, Some(value)) => value,
        (MeasurementState::Measured | MeasurementState::NotComputed, _) => {
            return Err(InconclusiveReason::NoCurrentValue);
        }
    };
    if incomplete(observation) {
        return Err(InconclusiveReason::IncompletePopulation);
    }
    let examined_nothing = observation
        .population
        .as_ref()
        .and_then(|population| population.examined)
        .is_some_and(|examined| examined.partial_cmp(&0.0) == Some(Ordering::Equal));
    if examined_nothing {
        return Err(InconclusiveReason::EmptyPopulation);
    }
    Ok(value)
}

/// A `ratchet` plan's verdict on `observation`.
fn ratchet(
    plan: &MeasurementPlan,
    objective: Objective,
    observation: Option<&MeasurementObservation>,
    earlier: &[MeasurementCollection],
) -> RatchetOutcome {
    let current = match usable(plan, observation) {
        Ok(value) => value,
        Err(reason) => return RatchetOutcome::Inconclusive(reason),
    };
    let Some(current_badness) = badness(objective, current) else {
        return RatchetOutcome::Inconclusive(InconclusiveReason::NoBound);
    };
    let slice = observation.map(|found| found.dimensions.entries());
    let best = earlier
        .iter()
        .flat_map(|collection| {
            collection
                .observations
                .iter()
                .filter(|candidate| {
                    candidate.metric.as_str() == plan.metric.as_str()
                        && candidate.plan_id.as_str() == plan.id.as_str()
                        && Some(candidate.dimensions.entries()) == slice
                })
                .filter_map(|candidate| usable(plan, Some(candidate)).ok())
                .filter_map(move |value| Some((badness(objective, value)?, value, collection)))
        })
        // `min_by` keeps the first of equal minima, so a tie names the
        // earliest collection that reached the best value.
        .min_by(|left, right| left.0.total_cmp(&right.0));
    let Some((best_badness, value, collection)) = best else {
        return RatchetOutcome::Inconclusive(InconclusiveReason::NoPrior);
    };
    let best_prior = BestPrior {
        value,
        collection_id: collection.collection_id.as_str().to_owned(),
    };
    if current_badness > best_badness {
        RatchetOutcome::Regressed {
            current,
            best_prior,
        }
    } else {
        RatchetOutcome::Held {
            current,
            best_prior,
        }
    }
}

/// A `target` plan's progress towards its objective's bound.
///
/// `distance` is `|current - bound|` for `higher`, `lower` and `target`, and
/// `||current| - |bound||` for `zero`, whose bound is a tolerance around zero.
/// `reached` is `current >= bound` for `higher`, `current <= bound` for
/// `lower`, `current == bound` for `target`, and `|current| <= |bound|` for
/// `zero`.
fn target_progress(
    plan: &MeasurementPlan,
    objective: Objective,
    observation: Option<&MeasurementObservation>,
) -> TargetOutcome {
    let Some(bound) = objective.bound() else {
        return TargetOutcome::Inconclusive(InconclusiveReason::NoBound);
    };
    let current = match usable(plan, observation) {
        Ok(value) => value,
        Err(reason) => return TargetOutcome::Inconclusive(reason),
    };
    let (distance, reached) = match objective.direction() {
        Direction::Higher => ((current - bound).abs(), current >= bound),
        Direction::Lower => ((current - bound).abs(), current <= bound),
        Direction::Target => {
            let distance = (current - bound).abs();
            (
                distance,
                distance.partial_cmp(&0.0) == Some(Ordering::Equal),
            )
        }
        Direction::Zero => (
            (current.abs() - bound.abs()).abs(),
            current.abs() <= bound.abs(),
        ),
    };
    TargetOutcome::Measured {
        current,
        bound,
        distance,
        reached,
    }
}
