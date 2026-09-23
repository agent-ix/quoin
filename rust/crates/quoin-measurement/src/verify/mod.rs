// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The independent measurement-verdict checker behind `quoin measurement
//! verify` (quoin FR-108, PLAT-961).
//!
//! # What it is, and what it is not
//!
//! A small trusted checker that decides a plan's verdict from the stored data,
//! independent of the producer that measured it. It is pure: a plan, every
//! stored collection with its intake position, and an optional claimed
//! verdict go in; a [`MeasurementVerdict`] comes out. It reads no file, clock,
//! network or environment — the caller in `quoin-core` does the I/O — so it
//! can be specified and proved later.
//!
//! It shares no estimator code with any producer: the only estimate it
//! computes is [`rows::recompute`], over the population the collection
//! states. The decision rule's meaning — comparators, margins, which way a
//! margin moves — is engineering-assurance's (FR-021) and is applied through
//! EA's own [`DecisionRule::holds`], never restated here.
//!
//! # Order
//!
//! The store records no intake order, and a collection's `timestamp` and
//! `collectionId` are the producer's to choose (PLAT-961's review of
//! quoin#587: a backdated collection sorts as a prior). So every collection
//! arrives with an [`Ranked::intake`] position from outside this module — the
//! CLI passes the git commit that first added the file — and the checker
//! orders by that. A collection with no position sorts after every positioned
//! one. The stated timestamp only breaks ties, for a deterministic listing,
//! and whenever a tie decides which collection is the candidate or which is
//! the prior, the verdict is `inconclusive` with `order_unattested`.
//!
//! # Every run counts
//!
//! Every collection holding an observation of the plan's metric under the
//! plan's id and `definition_version` is a run. The rule is applied to every
//! run against its own history, so a run that regressed stays visible even
//! when a later one passed; and a regressed run with the candidate's own
//! apparatus — the same source revision, configuration, tool, corpus and
//! verification stack — means the same thing was re-run until it passed,
//! which is `rerun_until_pass`.

mod reason;
pub mod rows;
mod wire;

use std::collections::BTreeMap;

use engineering_assurance::measurement::{Baseline, Comparator, DecisionRule, RuleReference};
use quoin_store::JsonValue;

use crate::store::read::collection_order;
use crate::types::collection::MeasurementCollection;
use crate::types::observation::MeasurementObservation;
use crate::types::plan::{MeasurementPlan, StatisticalDesign};

pub use reason::{Reason, Verdict};
pub use rows::{Estimate, EstimateBasis};
pub use wire::{VERDICT_SCHEMA, verdict_json};

/// One stored collection and its intake position.
#[derive(Clone, Copy, Debug)]
pub struct Ranked<'a> {
    /// The collection as stored.
    pub collection: &'a MeasurementCollection,
    /// Its position in an order the producer cannot choose — lower is
    /// earlier, equal is a tie — or `None` when nothing attests one.
    pub intake: Option<u64>,
}

/// One reason, and where it was found.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// Why.
    pub reason: Reason,
    /// The collection it was found in; `None` for a plan-level reason.
    pub collection_id: Option<String>,
    /// The observation's dimensions; `None` for a collection- or plan-level
    /// reason.
    pub dimensions: Option<BTreeMap<String, JsonValue>>,
}

/// The rule applied to one slice of the candidate collection.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceDecision {
    /// The slice.
    pub dimensions: BTreeMap<String, JsonValue>,
    /// The estimate, and whether it was recomputed or asserted.
    pub estimate: Estimate,
    /// The baseline value, for a baseline rule that found one.
    pub baseline: Option<f64>,
    /// Whether the rule holds; `None` when it could not be evaluated.
    pub holds: Option<bool>,
}

/// What the checker counted.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    /// Runs: collections holding the plan's metric under its definition.
    pub collections_considered: usize,
    /// Runs the rule does not hold for, against their own history.
    pub regressed_runs: usize,
    /// Observations whose estimate was recomputed from `matched` and
    /// `examined` — including those found to disagree with their `value`.
    pub observations_recomputed: usize,
    /// Observations whose stored `value` was used as stated.
    pub observations_asserted: usize,
    /// Runs with an intake position.
    pub order_attested: usize,
    /// Runs with none.
    pub order_unattested: usize,
}

/// The checker's typed result.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementVerdict {
    /// The plan checked.
    pub plan_id: String,
    /// Its definition version.
    pub definition_version: String,
    /// The verdict.
    pub verdict: Verdict,
    /// Every distinct reason, sorted; empty exactly when accepted.
    pub reasons: Vec<Reason>,
    /// The verdict the caller claimed, when one was.
    pub claimed: Option<Verdict>,
    /// The candidate: the last run in intake order.
    pub candidate: Option<String>,
    /// The rule applied to each slice of the candidate.
    pub decisions: Vec<SliceDecision>,
    /// Every reason, and where.
    pub findings: Vec<Finding>,
    /// The runs that regressed, in intake order.
    pub regressed_runs: Vec<String>,
    /// What was counted.
    pub counts: Counts,
}

/// One run: a collection, its position, and each matching observation's
/// estimate or the reason it has none.
struct Run<'a> {
    collection: &'a MeasurementCollection,
    intake: Option<u64>,
    slices: Vec<(&'a MeasurementObservation, Result<Estimate, Reason>)>,
}

/// Decide `plan`'s verdict from every stored collection.
#[must_use]
pub fn verify(
    plan: &MeasurementPlan,
    collections: &[Ranked<'_>],
    claimed: Option<Verdict>,
) -> MeasurementVerdict {
    let mut findings = Vec::new();
    let mut decisions = Vec::new();
    let mut regressed = Vec::new();
    let design = plan.statistical_design.unwrap_or(StatisticalDesign {
        minimum_population: None,
        repetitions: None,
        estimator: None,
        decision_rule: None,
    });
    let runs = design
        .estimator
        .map(|estimator| runs(plan, &design, estimator, collections))
        .unwrap_or_default();
    let mut counts = count(&runs);
    match (design.decision_rule, design.estimator) {
        (None, _) => findings.push(plan_level(Reason::NoDecisionRule)),
        (_, None) => findings.push(plan_level(Reason::NoEstimator)),
        (Some(_), Some(_)) if runs.is_empty() => findings.push(plan_level(Reason::NoCollections)),
        (Some(rule), Some(_)) => {
            for (index, run) in runs.iter().enumerate() {
                let history = runs.get(..index).unwrap_or_default();
                let is_candidate = index + 1 == runs.len();
                let outcome = decide(rule, run, history, is_candidate, &mut findings);
                if outcome.regressed {
                    regressed.push(index);
                }
                if is_candidate {
                    decisions = outcome.decisions;
                }
            }
            candidate_checks(&runs, &regressed, &mut findings);
        }
    }
    counts.regressed_runs = regressed.len();
    let mut result = MeasurementVerdict {
        plan_id: plan.id.as_str().to_owned(),
        definition_version: plan.definition_version.as_str().to_owned(),
        verdict: Verdict::Inconclusive,
        reasons: Vec::new(),
        claimed,
        candidate: runs.last().map(|run| id_of(run.collection)),
        decisions,
        findings,
        regressed_runs: regressed
            .iter()
            .filter_map(|&index| runs.get(index).map(|run| id_of(run.collection)))
            .collect(),
        counts,
    };
    settle(&mut result);
    result
}

/// Every run of `plan`, in intake order.
fn runs<'a>(
    plan: &MeasurementPlan,
    design: &StatisticalDesign,
    estimator: engineering_assurance::measurement::Estimator,
    collections: &[Ranked<'a>],
) -> Vec<Run<'a>> {
    let mut runs: Vec<Run<'a>> = collections
        .iter()
        .map(|ranked| Run {
            collection: ranked.collection,
            intake: ranked.intake,
            slices: ranked
                .collection
                .observations
                .iter()
                .filter(|observation| {
                    observation.plan_id.as_str() == plan.id.as_str()
                        && observation.metric.as_str() == plan.metric.as_str()
                        && observation.definition_version.as_str()
                            == plan.definition_version.as_str()
                })
                .map(|observation| (observation, rows::assess(design, estimator, observation)))
                .collect(),
        })
        .filter(|run| !run.slices.is_empty())
        .collect();
    // Positioned runs first, by position; unpositioned after. The stated
    // timestamp only makes a tie's listing deterministic.
    runs.sort_by(|a, b| {
        (a.intake.is_none(), a.intake)
            .cmp(&(b.intake.is_none(), b.intake))
            .then_with(|| collection_order(a.collection, b.collection))
    });
    runs
}

fn count(runs: &[Run<'_>]) -> Counts {
    let mut counts = Counts {
        collections_considered: runs.len(),
        ..Counts::default()
    };
    for run in runs {
        if run.intake.is_some() {
            counts.order_attested += 1;
        } else {
            counts.order_unattested += 1;
        }
        for (_, outcome) in &run.slices {
            match outcome {
                Ok(Estimate {
                    basis: EstimateBasis::Recomputed,
                    ..
                })
                | Err(Reason::ValueDisagreesWithRows) => counts.observations_recomputed += 1,
                Ok(Estimate {
                    basis: EstimateBasis::Asserted,
                    ..
                }) => counts.observations_asserted += 1,
                Err(_) => {}
            }
        }
    }
    counts
}

/// What applying the rule to one run found.
struct RunOutcome {
    regressed: bool,
    decisions: Vec<SliceDecision>,
}

/// Apply `rule` to every slice of `run` against `history`, recording
/// findings. The candidate records every reason; an earlier run records only
/// the contradictions in its own data — its missing evidence and its failed
/// rule are history, counted but not held against the candidate.
fn decide(
    rule: DecisionRule,
    run: &Run<'_>,
    history: &[Run<'_>],
    is_candidate: bool,
    findings: &mut Vec<Finding>,
) -> RunOutcome {
    let mut outcome = RunOutcome {
        regressed: false,
        decisions: Vec::new(),
    };
    for (observation, assessed) in &run.slices {
        let dimensions = observation.dimensions.entries();
        let mut record = |reason: Reason| {
            if is_candidate || reason.verdict() == Verdict::Reject {
                findings.push(Finding {
                    reason,
                    collection_id: Some(id_of(run.collection)),
                    dimensions: Some(dimensions.clone()),
                });
            }
        };
        let estimate = match assessed {
            Ok(estimate) => *estimate,
            Err(reason) => {
                record(*reason);
                continue;
            }
        };
        let (baseline, holds) = match baseline(rule, dimensions, history) {
            Ok((value, prior)) => {
                if is_candidate && prior.is_some_and(|prior| tied(history, prior)) {
                    record(Reason::OrderUnattested);
                }
                let holds = rule.holds(estimate.value, value).ok();
                if holds.is_none() {
                    record_open(&mut record, is_candidate, Reason::RuleNotEvaluable);
                }
                (value, holds)
            }
            Err(reason) => {
                record_open(&mut record, is_candidate, reason);
                (None, None)
            }
        };
        if holds == Some(false) {
            outcome.regressed = true;
            record_open(&mut record, is_candidate, Reason::RuleNotMet);
        }
        if is_candidate {
            outcome.decisions.push(SliceDecision {
                dimensions: dimensions.clone(),
                estimate,
                baseline,
                holds,
            });
        }
    }
    outcome
}

/// Record `reason` for the candidate only: an earlier run's missing
/// baseline, unevaluable rule or failed rule is its history, not a finding.
fn record_open(record: &mut impl FnMut(Reason), is_candidate: bool, reason: Reason) {
    if is_candidate {
        record(reason);
    }
}

/// The baseline value for `slice` over `history`, and the index of the run
/// it came from when that one run alone decides it (`prior-collection`).
fn baseline(
    rule: DecisionRule,
    slice: &BTreeMap<String, JsonValue>,
    history: &[Run<'_>],
) -> Result<(Option<f64>, Option<usize>), Reason> {
    let RuleReference::Baseline { baseline, .. } = rule.reference() else {
        return Ok((None, None));
    };
    // A baseline is computed from the checker's own estimates: a run whose
    // stored value disagrees with its rows contributes nothing.
    let mut earlier = history.iter().enumerate().filter_map(|(index, run)| {
        run.slices.iter().find_map(|(observation, assessed)| {
            (observation.dimensions.entries() == slice)
                .then_some(assessed.as_ref().ok())
                .flatten()
                .map(|estimate| (index, estimate.value))
        })
    });
    let found = match baseline {
        Baseline::ConstantPredictor => return Err(Reason::ConstantPredictorRowsAbsent),
        Baseline::PriorCollection => earlier.next_back(),
        Baseline::BestSeen => match rule.comparator() {
            Comparator::Gt | Comparator::Ge => {
                earlier.max_by(|left, right| left.1.total_cmp(&right.1))
            }
            Comparator::Lt | Comparator::Le | Comparator::Eq => {
                earlier.min_by(|left, right| left.1.total_cmp(&right.1))
            }
        },
    };
    let (index, value) = found.ok_or(Reason::NoPrior)?;
    let prior = (baseline == Baseline::PriorCollection).then_some(index);
    Ok((Some(value), prior))
}

/// Whether the run at `index` shares its intake position with another run in
/// `runs`, so nothing attests which of them came later.
fn tied(runs: &[Run<'_>], index: usize) -> bool {
    runs.get(index).is_some_and(|run| {
        runs.iter()
            .enumerate()
            .any(|(other, candidate)| other != index && candidate.intake == run.intake)
    })
}

/// The checks that look at the candidate against every other run.
fn candidate_checks(runs: &[Run<'_>], regressed: &[usize], findings: &mut Vec<Finding>) {
    let Some(last) = runs.len().checked_sub(1) else {
        return;
    };
    let Some(candidate) = runs.get(last) else {
        return;
    };
    if tied(runs, last) {
        findings.push(Finding {
            reason: Reason::OrderUnattested,
            collection_id: Some(id_of(candidate.collection)),
            dimensions: None,
        });
    }
    if regressed.contains(&last) {
        return;
    }
    for &index in regressed {
        if let Some(run) = runs.get(index)
            && same_apparatus(run.collection, candidate.collection)
        {
            findings.push(Finding {
                reason: Reason::RerunUntilPass,
                collection_id: Some(id_of(run.collection)),
                dimensions: None,
            });
        }
    }
}

/// Whether two collections were produced by the same apparatus: source
/// revision, configuration, tool, corpus and verification stack.
fn same_apparatus(left: &MeasurementCollection, right: &MeasurementCollection) -> bool {
    let stack = |collection: &MeasurementCollection| {
        collection
            .verification_stack
            .as_ref()
            .map(|stack| (stack.lock_digest.clone(), stack.executable_digest.clone()))
    };
    left.source_revision == right.source_revision
        && left.config_digest == right.config_digest
        && left.tool_identity == right.tool_identity
        && left.tool_version == right.tool_version
        && left.corpus_revision == right.corpus_revision
        && stack(left) == stack(right)
}

fn plan_level(reason: Reason) -> Finding {
    Finding {
        reason,
        collection_id: None,
        dimensions: None,
    }
}

fn id_of(collection: &MeasurementCollection) -> String {
    collection.collection_id.as_str().to_owned()
}

/// Settle the verdict from the findings, then hold the claim against it.
fn settle(result: &mut MeasurementVerdict) {
    let forced = |findings: &[Finding]| {
        let any = |verdict: Verdict| findings.iter().any(|f| f.reason.verdict() == verdict);
        if any(Verdict::Reject) {
            Verdict::Reject
        } else if any(Verdict::Inconclusive) {
            Verdict::Inconclusive
        } else {
            Verdict::Accept
        }
    };
    if result
        .claimed
        .is_some_and(|claimed| claimed != forced(&result.findings))
    {
        result
            .findings
            .push(plan_level(Reason::ClaimedVerdictDisagrees));
    }
    result.verdict = forced(&result.findings);
    result.reasons = result.findings.iter().map(|finding| finding.reason).collect();
    result.reasons.sort_unstable();
    result.reasons.dedup();
}
