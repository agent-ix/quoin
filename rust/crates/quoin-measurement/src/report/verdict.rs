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
//! No newest value, a value naming another plan or measured under another
//! definition, and an unstated, incomplete or empty population all give
//! `inconclusive`, never `held` or `reached`. A slice an earlier collection
//! measured that the newest one drops is reported too — see
//! [`crate::report::vanished::VanishedSlice`] — so a slice cannot regress by being left out. So does a ratchet with nothing earlier to hold against: the first
//! collection a ratchet ever sees has no floor yet, and calling that `held`
//! would be a green verdict with no comparison behind it — the failure quoin's
//! `--ratchet` with no baseline once had (CR-029, agent-ix/quoin#169). The
//! same rule decides which earlier values may *set* the best: an earlier value
//! that is incomplete, empty or under another definition is not one.
//!
//! Under a plan that protects apparatus (PLAT-975), an earlier value measured
//! with a different recorded protected apparatus is not a floor either, and
//! its presence makes the ratchet `inconclusive` (`apparatus_changed`); a
//! newest collection that recorded none is `apparatus_unrecorded`.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use engineering_assurance::measurement::{Direction, Objective};
use quoin_store::JsonValue;

use crate::compare::incomplete;
use crate::types::collection::MeasurementCollection;
use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::{MeasurementPlan, MeasurementStage};

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
    /// the newest one's, or that recorded none (PLAT-975).
    ApparatusChanged,
    /// The newest collection recorded no protected apparatus although the
    /// plan protects apparatus (PLAT-975).
    ApparatusUnrecorded,
}

impl InconclusiveReason {
    /// Every reason, in declaration order.
    pub const ALL: [Self; 10] = [
        Self::NoCurrentValue,
        Self::PlanMismatch,
        Self::DefinitionMismatch,
        Self::PopulationUnstated,
        Self::IncompletePopulation,
        Self::EmptyPopulation,
        Self::NoPrior,
        Self::NoBound,
        Self::ApparatusChanged,
        Self::ApparatusUnrecorded,
    ];

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
            Self::ApparatusUnrecorded => {
                "the newest collection recorded no protected apparatus the plan protects"
            }
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
        /// The distance still to go: `0` once `reached`, positive otherwise.
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
/// `observation` is the row's newest observation, from `collection`;
/// `earlier` is every collection older than that one, in collection order.
#[must_use]
pub fn stage_verdict(
    plan: &MeasurementPlan,
    observation: Option<&MeasurementObservation>,
    collection: Option<&MeasurementCollection>,
    earlier: &[MeasurementCollection],
) -> Option<StageVerdict> {
    let objective = plan.objective?;
    match plan.stage {
        MeasurementStage::Ratchet => Some(StageVerdict::Ratchet {
            objective,
            outcome: ratchet(plan, objective, observation, collection, earlier),
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

/// The observation and value `observation` may contribute to a verdict
/// under `plan`, or why it may not.
///
/// The one place "usable evidence" is decided, for the newest value and for
/// every earlier value alike: it names the plan, is measured under the plan's
/// definition, carries a value, and states a population that is neither
/// incomplete nor empty.
pub(super) fn usable<'a>(
    plan: &MeasurementPlan,
    observation: Option<&'a MeasurementObservation>,
) -> Result<(&'a MeasurementObservation, f64), InconclusiveReason> {
    let observation = observation.ok_or(InconclusiveReason::NoCurrentValue)?;
    if observation.plan_id.as_str() != plan.id.as_str() {
        return Err(InconclusiveReason::PlanMismatch);
    }
    if observation.definition_version.as_str() != plan.definition_version.as_str() {
        return Err(InconclusiveReason::DefinitionMismatch);
    }
    let value = match (observation.state, observation.value) {
        (MeasurementState::Measured, Some(value)) => value,
        (MeasurementState::Measured | MeasurementState::NotComputed, _) => {
            return Err(InconclusiveReason::NoCurrentValue);
        }
    };
    let population = observation
        .population
        .as_ref()
        .ok_or(InconclusiveReason::PopulationUnstated)?;
    if incomplete(observation) {
        return Err(InconclusiveReason::IncompletePopulation);
    }
    if population
        .examined
        .is_some_and(|examined| examined.partial_cmp(&0.0) == Some(Ordering::Equal))
    {
        return Err(InconclusiveReason::EmptyPopulation);
    }
    Ok((observation, value))
}

/// Every usable earlier value of `plan`'s slice `slice`, oldest first, with
/// the collection that measured it.
fn earlier_values<'a>(
    plan: &'a MeasurementPlan,
    slice: &'a BTreeMap<String, JsonValue>,
    earlier: &'a [MeasurementCollection],
) -> impl Iterator<Item = (f64, &'a MeasurementCollection)> + 'a {
    earlier.iter().flat_map(move |collection| {
        collection
            .observations
            .iter()
            .filter(move |candidate| {
                candidate.metric.as_str() == plan.metric.as_str()
                    && candidate.dimensions.entries() == slice
            })
            .filter_map(move |candidate| usable(plan, Some(candidate)).ok())
            .map(move |(_, value)| (value, collection))
    })
}

/// A `ratchet` plan's verdict on `observation`.
fn ratchet(
    plan: &MeasurementPlan,
    objective: Objective,
    observation: Option<&MeasurementObservation>,
    collection: Option<&MeasurementCollection>,
    earlier: &[MeasurementCollection],
) -> RatchetOutcome {
    let (observation, current) = match usable(plan, observation) {
        Ok(found) => found,
        Err(reason) => return RatchetOutcome::Inconclusive(reason),
    };
    let Some(current_badness) = badness(objective, current) else {
        return RatchetOutcome::Inconclusive(InconclusiveReason::NoBound);
    };
    if let Err(reason) = same_apparatus(plan, observation, collection, earlier) {
        return RatchetOutcome::Inconclusive(reason);
    }
    let best = earlier_values(plan, observation.dimensions.entries(), earlier)
        .filter_map(|(value, collection)| Some((badness(objective, value)?, value, collection)))
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

/// Under a plan that protects apparatus, refuse a floor unless the newest
/// collection recorded its protected apparatus and every earlier usable value
/// of the slice came from a collection that recorded the same set (PLAT-975).
///
/// A changed set needs a new `definition_version` (engineering-assurance
/// FR-024), so an earlier value under the plan's own definition with another
/// set is an apparatus edited inside one series: a floor set by it, or a
/// newest value measured against it, compares two different measurements.
/// That is `apparatus_changed` for as long as the series lasts, rather than a
/// floor quietly taken from whichever runs happen to agree.
fn same_apparatus(
    plan: &MeasurementPlan,
    observation: &MeasurementObservation,
    collection: Option<&MeasurementCollection>,
    earlier: &[MeasurementCollection],
) -> Result<(), InconclusiveReason> {
    if plan.protected_apparatus.is_none() {
        return Ok(());
    }
    let plan_id = plan.id.as_str();
    let own = collection
        .and_then(|found| found.protected_apparatus_of(plan_id))
        .ok_or(InconclusiveReason::ApparatusUnrecorded)?;
    if earlier_values(plan, observation.dimensions.entries(), earlier)
        .any(|(_, found)| found.protected_apparatus_of(plan_id) != Some(own))
    {
        return Err(InconclusiveReason::ApparatusChanged);
    }
    Ok(())
}

/// A `target` plan's progress towards its objective's bound.
///
/// `reached` is `current >= bound` for `higher`, `current <= bound` for
/// `lower`, `|current| <= bound` for `zero` (whose bound plan intake has
/// already required to be non-negative), and exact IEEE equality
/// `current == bound` for `target`, with no tolerance. `distance` is the
/// distance still to go: `0` once reached, otherwise `bound - current`,
/// `current - bound`, `|current| - bound` or `|current - bound|` respectively.
fn target_progress(
    plan: &MeasurementPlan,
    objective: Objective,
    observation: Option<&MeasurementObservation>,
) -> TargetOutcome {
    let Some(bound) = objective.bound() else {
        return TargetOutcome::Inconclusive(InconclusiveReason::NoBound);
    };
    let current = match usable(plan, observation) {
        Ok((_, value)) => value,
        Err(reason) => return TargetOutcome::Inconclusive(reason),
    };
    let shortfall = match objective.direction() {
        Direction::Higher => bound - current,
        Direction::Lower => current - bound,
        Direction::Zero => current.abs() - bound,
        Direction::Target => (current - bound).abs(),
    };
    let reached = shortfall.partial_cmp(&0.0) != Some(Ordering::Greater);
    TargetOutcome::Measured {
        current,
        bound,
        distance: if reached { 0.0 } else { shortfall },
        reached,
    }
}
