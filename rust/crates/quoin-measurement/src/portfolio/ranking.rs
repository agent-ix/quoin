// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! An advisory, cross-repository ranking of what to improve next (PLAT-968,
//! FR-113).
//!
//! The rest of [`crate::portfolio`] deliberately makes no priority call — its
//! module doc says so. This is the one place that does, and it does it as a
//! ranking, never a decision: it orders plans, it does not pass or fail
//! anything, and [`RANKING_ADVISORY_NOTE`] says so in both rendered forms.
//!
//! # What makes a plan rankable
//!
//! A plan enters the ranked list only when three things are all true of it:
//!
//! 1. It states an `objective` (engineering-assurance's PLAT-967 type).
//! 2. That objective states a `bound` — `higher`/`lower`/`zero` objectives
//!    may state one even though only `target` requires it, and a ranking
//!    needs *some* goal to measure distance from.
//! 3. Its newest value is usable evidence exactly as
//!    [`crate::report::verdict::usable`] decides it for a stage verdict — one
//!    "usable evidence" rule, not a second one restated here.
//!
//! A plan missing any of the three is *listed*, in [`PortfolioRanking::unranked`],
//! with the reason, and is never scored — matching PLAT-968's acceptance
//! criterion that a plan with no objective is listed, not ranked.
//!
//! `weight`, `value_half_life` and `budget` (PLAT-967's steering fields) are
//! themselves optional even on a rankable objective. Excluding a plan from
//! the ranked list because one of *those* is absent would defeat the reason
//! the ranking exists — EA's own type documents them as advisory refinements,
//! not preconditions — so each missing one falls back to a stated neutral
//! default instead (below). This is a judgment call, not something the
//! ticket's acceptance criteria mandate either way; it is what lets "ranking
//! order with and without an explicit weight" be a meaningful comparison
//! within the ranked list, rather than a comparison against the unranked one.
//!
//! # The formula
//!
//! For a rankable plan with newest value `current`, objective `bound` and
//! direction, and optional `weight`, `value_half_life` and `budget`:
//!
//! ```text
//! gap           = shortfall(direction, current, bound)          // 0 once the bound is met
//! weight'      = weight.unwrap_or(1.0)                       // neutral: no declared priority tilts nothing
//! decay         = 0.5 ^ (age_days / value_half_life)            // 1.0 when value_half_life or age is unstated
//! budget_floor  = max(budget.unwrap_or(1.0), MIN_BUDGET_FLOOR)  // 1.0 when budget is unstated
//! score         = (weight' * gap * decay) / budget_floor
//! ```
//!
//! Ranked descending by `score`: the plan furthest from its bound, most
//! heavily weighted, least stale and cheapest to fund sorts first.
//!
//! ## Weighted gap-to-bound
//!
//! `gap` is how far the value still falls short of the bound, read in the
//! objective's direction: for `higher` it is `max(bound - current, 0)`, for
//! `lower` `max(current - bound, 0)`, for `zero` `max(|current| - |bound|, 0)`,
//! and for `target` the same `|current - bound|` [`crate::report::verdict`]
//! computes as a `target` plan's progress distance (overshooting a target
//! misses it too). A plan that has met or passed its bound scores `0` and
//! sorts last; it is still listed as ranked, because it does have an
//! objective, a bound and a value. A direction-blind `|current - bound|`
//! would instead rank a `higher` plan that cleared its bound by a mile as the
//! most urgent one. `weight` scales that gap directly — EA's doc
//! states it as "relative value against the project's other objectives", and
//! a linear scale is the simplest reading that preserves that: doubling the
//! weight doubles the plan's claim on attention for the same gap.
//!
//! ## Value half-life discount
//!
//! The concern PLAT-968 raises is that an old, unaddressed gap must not
//! dominate the ranking forever just because nobody has re-measured it. This
//! applies a standard half-life decay to the gap itself, keyed on how stale
//! the evidence behind it is: `age_days` is the newest value's collection
//! timestamp's distance from the portfolio's own newest collection timestamp
//! — the same reference point [`crate::portfolio::types::Staleness`] already
//! uses, so "how old" means one thing across the whole portfolio view rather
//! than two. `decay = 0.5 ^ (age_days / value_half_life)` halves the gap's
//! contribution every `value_half_life` days: at `age_days = 0` a plan scores
//! at full strength, at `age_days = value_half_life` it scores at half, and it
//! keeps shrinking (never reaching exactly zero) rather than being dropped
//! outright — an old gap should count for less, not be silently erased,
//! because "less" still lets a fresh, small gap in a *different* plan outrank
//! it, which is the whole point.
//!
//! No `value_half_life` states no belief about how fast this objective's
//! value decays, so no decay is applied (`decay = 1.0`) rather than guessing
//! one; the same holds when a timestamp is missing or unparsable, so a data
//! gap never manufactures an artificial discount.
//!
//! ## Budget normalization
//!
//! `budget` is "the time, token, or compute budget per attempt" (EA's doc,
//! not a total remaining-room figure), so dividing the discounted, weighted
//! gap by it reads as *gap per unit of what an attempt costs*: a plan that
//! needs a large budget per attempt to move the needle scores lower for the
//! same gap than one that is cheap to attempt, which is the "bang for the
//! buck" framing PLAT-968 asks for. Dividing by a `budget` near zero would
//! send the score toward infinity for no real reason — a plan someone
//! declared cheap to attempt should not out-rank everything else by an
//! unbounded margin — so [`MIN_BUDGET_FLOOR`] floors the divisor. No stated
//! `budget` divides by exactly that same floor, so a plan with no budget
//! declared and a plan whose tiny declared budget is floored to it are
//! ranked identically: absence and "as cheap as the floor admits" are not
//! distinguishable from a budget alone, and neither should be, since a small
//! budget is exactly the "absurdly high" case this floor exists to prevent.
//!
//! # What this never does
//!
//! No score is compared against a threshold, no plan is passed or failed, and
//! nothing here feeds [`crate::report::verdict::stage_verdict`] or any
//! `DecisionRule`. The ranked list is an ordering over plans whose evidence
//! already exists elsewhere in the report; removing this module removes a
//! sort order, not a fact.

use engineering_assurance::measurement::Direction;

use crate::date_time::Rfc3339DateTime;
use crate::error::MeasurementError;
use crate::portfolio::types::{DAY_MS, PortfolioReport};
use crate::report::render::row_label;
use crate::report::verdict::usable;

/// The sentence both rendered forms carry, verbatim, so a reader never sees
/// the ranked list without also seeing that it decides nothing.
pub const RANKING_ADVISORY_NOTE: &str = "Advisory only: this ranking orders plans \
    by weighted, time-discounted, budget-normalized gap to their objective's \
    bound. It makes no decision and is not a gate.";

/// Below this, a stated `budget` is floored before it normalizes a score, so
/// a near-zero budget cannot send a score toward infinity. See the module
/// doc's "Budget normalization" section.
const MIN_BUDGET_FLOOR: f64 = 1.0;

/// Why a plan is listed but not scored.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum UnrankedReason {
    /// The plan states no `objective`.
    NoObjective,
    /// The plan's `objective` states no `bound`.
    NoBound,
    /// The row's newest value is not usable evidence — see
    /// [`crate::report::verdict::usable`] for the exact reasons this covers
    /// (no current value, a mismatched plan or definition, or an unstated,
    /// incomplete or empty population).
    NoCurrentEstimate,
}

impl UnrankedReason {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoObjective => "no_objective",
            Self::NoBound => "no_bound",
            Self::NoCurrentEstimate => "no_current_estimate",
        }
    }
}

/// One rankable plan's row, scored.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioRankingEntry {
    /// The repository name, as the portfolio names it.
    pub repository: String,
    /// The plan's identity.
    pub plan_id: String,
    /// Where the plan is authored.
    pub plan_path: String,
    /// The metric the plan governs.
    pub metric: String,
    /// `metric` with the row's dimension slice, when it has one — `metric` or
    /// `metric [k=v, …]`, exactly as [`crate::report::render::row_label`]
    /// composes it for the per-repository report. A plan with multiple
    /// dimension-sliced rows (PLAT-968's "tracking two related quantities"
    /// pattern, e.g. `dimensions.quantity: cost` vs `dimensions.quantity:
    /// latency`) otherwise produces two ranked rows a reader cannot tell
    /// apart (PLAT-1019).
    pub label: String,
    /// The newest usable value.
    pub current: f64,
    /// The objective's stated bound.
    pub bound: f64,
    /// The direction the metric is supposed to move.
    pub direction: Direction,
    /// How far `current` falls short of `bound` in the objective's direction
    /// (`0` once met), before weight, decay or budget are applied.
    pub gap: f64,
    /// The weight actually used: `objective.weight()` when stated, `1.0`
    /// otherwise.
    pub weight: f64,
    /// The objective's stated half-life, when it stated one. `None` means no
    /// decay was applied, not that decay was applied at `0`.
    pub value_half_life: Option<f64>,
    /// How many whole days behind the portfolio's newest collection the
    /// value that produced [`Self::gap`] is, when both timestamps parsed.
    pub age_days: Option<i64>,
    /// The half-life discount actually applied: `1.0` when
    /// [`Self::value_half_life`] or [`Self::age_days`] is `None`.
    pub decay_factor: f64,
    /// The objective's stated budget, when it stated one.
    pub budget: Option<f64>,
    /// `(weight * gap * decay_factor) / max(budget.unwrap_or(1.0), MIN_BUDGET_FLOOR)`.
    /// Higher sorts first.
    pub score: f64,
}

/// One plan the ranking could not score, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct UnrankedPlan {
    /// The repository name, as the portfolio names it.
    pub repository: String,
    /// The plan's identity.
    pub plan_id: String,
    /// Where the plan is authored.
    pub plan_path: String,
    /// The metric the plan governs.
    pub metric: String,
    /// Why it is not scored.
    pub reason: UnrankedReason,
}

/// The whole advisory ranking: every rankable plan, scored and ordered, and
/// every other active plan, listed with why it was not.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PortfolioRanking {
    /// Descending by [`PortfolioRankingEntry::score`]; ties broken by
    /// repository name then plan id, for a deterministic order.
    pub ranked: Vec<PortfolioRankingEntry>,
    /// Every plan row this ranking could not score, in repository-then-plan
    /// order.
    pub unranked: Vec<UnrankedPlan>,
}

/// Rank every repository's current plan rows in `report`.
///
/// Pure: `report` is already built, so this reads no store and runs no
/// producer. See the module doc for the formula and for what "rankable"
/// means.
///
/// # Errors
///
/// [`crate::MeasurementErrorCode::Store`] when a ranked row's dimensions
/// cannot cross [`crate::json_bridge`] while composing [`row_label`] — see
/// that function.
pub fn rank_portfolio(report: &PortfolioReport) -> Result<PortfolioRanking, MeasurementError> {
    let newest = report
        .newest_collection_timestamp
        .as_deref()
        .and_then(epoch_millis);

    let mut ranked = Vec::new();
    let mut unranked = Vec::new();
    for repository in &report.repositories {
        let Some(measurements) = repository.measurements.as_ref() else {
            continue;
        };
        for row in &measurements.current {
            let unrankable = |reason: UnrankedReason| UnrankedPlan {
                repository: repository.name.clone(),
                plan_id: row.plan_id.clone(),
                plan_path: row.plan_path.clone(),
                metric: row.metric.clone(),
                reason,
            };
            let Some(plan) = repository
                .plans
                .iter()
                .find(|candidate| candidate.id.as_str() == row.plan_id)
            else {
                unranked.push(unrankable(UnrankedReason::NoObjective));
                continue;
            };
            let Some(objective) = plan.objective else {
                unranked.push(unrankable(UnrankedReason::NoObjective));
                continue;
            };
            let Some(bound) = objective.bound() else {
                unranked.push(unrankable(UnrankedReason::NoBound));
                continue;
            };
            let Ok((_, current)) = usable(plan, row.observation.as_ref()) else {
                unranked.push(unrankable(UnrankedReason::NoCurrentEstimate));
                continue;
            };

            let gap = shortfall(objective.direction(), current, bound);
            let weight = objective.weight().unwrap_or(1.0);
            let age_days = row
                .collection
                .as_ref()
                .and_then(|collection| epoch_millis(&collection.timestamp))
                .zip(newest)
                .map(|(timestamp, newest)| (newest - timestamp).max(0) / DAY_MS);
            let decay_factor = decay(objective.value_half_life(), age_days);
            let budget = objective.budget();
            let normalizer = budget.unwrap_or(MIN_BUDGET_FLOOR).max(MIN_BUDGET_FLOOR);
            let score = weight * gap * decay_factor / normalizer;

            ranked.push(PortfolioRankingEntry {
                repository: repository.name.clone(),
                plan_id: row.plan_id.clone(),
                plan_path: row.plan_path.clone(),
                metric: row.metric.clone(),
                label: row_label(row)?,
                current,
                bound,
                direction: objective.direction(),
                gap,
                weight,
                value_half_life: objective.value_half_life(),
                age_days,
                decay_factor,
                budget,
                score,
            });
        }
    }

    ranked.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.repository.cmp(&right.repository))
            .then_with(|| left.plan_id.cmp(&right.plan_id))
    });
    unranked.sort_by(|left, right| {
        left.repository
            .cmp(&right.repository)
            .then_with(|| left.plan_id.cmp(&right.plan_id))
    });

    Ok(PortfolioRanking { ranked, unranked })
}

/// How far `current` still falls short of `bound`, in the direction the
/// objective says is better — `0.0` once the bound is met or passed.
///
/// `higher` → `max(bound - current, 0)`; `lower` → `max(current - bound, 0)`;
/// `zero` → `max(|current| - |bound|, 0)` (the bound as the furthest from zero
/// the goal tolerates, which is `|current|` for the usual bound of `0`);
/// `target` → `|current - bound|`, since overshooting a target misses it too.
/// A plain `|current - bound|` for every direction would score a `higher`
/// plan that has already cleared its bound as a gap, and rank it higher the
/// further it cleared it.
fn shortfall(direction: Direction, current: f64, bound: f64) -> f64 {
    match direction {
        Direction::Higher => (bound - current).max(0.0),
        Direction::Lower => (current - bound).max(0.0),
        Direction::Zero => (current.abs() - bound.abs()).max(0.0),
        Direction::Target => (current - bound).abs(),
    }
}

/// The half-life discount for a value `age_days` old: `0.5 ^ (age_days /
/// half_life)`, or `1.0` — no discount — when either input is absent, or
/// `half_life` is not a positive number (engineering-assurance's own
/// [`engineering_assurance::measurement::Objective::with_steering`] already
/// refuses a non-positive `value_half_life` at parse time, so this is a
/// second refusal to trust, not the first).
#[expect(
    clippy::cast_precision_loss,
    reason = "age_days is a whole day count over a realistic collection \
              history, far inside f64's 53-bit exact integer range; this is \
              the same trade tc_958's badness computations already make"
)]
fn decay(half_life: Option<f64>, age_days: Option<i64>) -> f64 {
    match (half_life, age_days) {
        (Some(half_life), Some(age_days)) if half_life > 0.0 => {
            0.5_f64.powf(age_days as f64 / half_life)
        }
        _ => 1.0,
    }
}

/// `Date.parse(timestamp)`, read by this crate's one instant reader — the
/// same helper [`crate::portfolio::build`] uses for
/// [`crate::portfolio::types::Staleness`], so "how old" means the same thing
/// in both places.
fn epoch_millis(timestamp: &str) -> Option<i64> {
    Rfc3339DateTime::parse(timestamp)
        .ok()
        .map(|instant| instant.epoch_millis())
}
