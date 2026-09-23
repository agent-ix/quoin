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
//! arrives with a [`Ranked::intake`] position from outside this module, and
//! an [`OrderSource`] saying where the positions came from — the CLI passes
//! the first-parent git commit that added each file. A collection with no
//! position sorts after every positioned one, and the stated timestamp only
//! breaks ties for a deterministic listing. The candidate, or a
//! `prior-collection` rule's prior, whose place is tied or contradicted by
//! the stated timestamps is `order_unattested` (see [`order`]).
//!
//! # Every run counts
//!
//! Every collection holding an observation of the plan's metric under the
//! plan's id and `definition_version` is a run. The rule is applied to every
//! run against its own history, so a run that regressed stays visible even
//! when a later one passed; and a regressed run with the candidate's own
//! apparatus — the same source revision, configuration, tool, corpus and
//! verification stack — means the same thing was re-run until it passed,
//! which is `rerun_until_pass`. A changed protected apparatus does not make
//! it a different run: editing the answer key between a failed run and a
//! pass is the rerun this rule exists to catch.
//!
//! # Protected apparatus
//!
//! A run is a baseline only for runs that recorded the same protected
//! apparatus, and an earlier run that recorded another is a finding against
//! the candidate — see [`apparatus`] (PLAT-975).

mod apparatus;
mod constant_predictor;
mod order;
mod reason;
pub mod rows;
mod tamper;
mod types;
mod wire;

use std::collections::BTreeMap;

use engineering_assurance::measurement::{Baseline, Comparator, DecisionRule, RuleReference};
use quoin_store::JsonValue;

use crate::types::collection::{MeasurementCollection, ResolvedApparatus};
use crate::types::observation::MeasurementObservation;
use crate::types::plan::{MeasurementPlan, StatisticalDesign};

pub use reason::{Reason, Verdict};
pub use rows::{Estimate, EstimateBasis};
pub use types::{
    Counts, Finding, MeasurementVerdict, OrderSource, Ranked, SliceDecision, TamperFacts,
};
pub use wire::{VERDICT_SCHEMA, verdict_json};

/// One run: a collection, its position, and each matching observation's
/// estimate or the reason it has none.
struct Run<'a> {
    collection: &'a MeasurementCollection,
    intake: Option<u64>,
    /// The protected apparatus the collection recorded for the plan, read
    /// whatever the plan declares today; `None` when it recorded none.
    apparatus: Option<&'a ResolvedApparatus>,
    /// Whether the caller found this run's recorded protected-apparatus
    /// digests to disagree with the committed source they claim (PLAT-985).
    apparatus_forged: bool,
    slices: Vec<(&'a MeasurementObservation, Result<Estimate, Reason>)>,
}

/// What the checker holds while it decides.
struct Check<'a> {
    rule: Option<DecisionRule>,
    runs: Vec<Run<'a>>,
    findings: Vec<Finding>,
}

/// Decide `plan`'s verdict from every stored collection, ordered as `order`
/// says the positions in `collections` were obtained.
#[must_use]
pub fn verify(
    plan: &MeasurementPlan,
    collections: &[Ranked<'_>],
    tamper: TamperFacts<'_>,
    order: OrderSource,
    claimed: Option<Verdict>,
) -> MeasurementVerdict {
    let design = plan.statistical_design.unwrap_or(StatisticalDesign {
        minimum_population: None,
        repetitions: None,
        estimator: None,
        decision_rule: None,
    });
    // Positions are ignored when the order's source attests none.
    let positioned = matches!(
        order,
        OrderSource::GitFirstParentAdd | OrderSource::CallerSupplied
    );
    let collections: Vec<Ranked<'_>> = collections
        .iter()
        .map(|ranked| Ranked {
            intake: ranked.intake.filter(|_| positioned),
            ..*ranked
        })
        .collect();
    let collections = collections.as_slice();
    let mut check = Check {
        rule: design.decision_rule,
        runs: runs(plan, &design, collections),
        findings: Vec::new(),
    };
    let counts = count(&check.runs);
    let mut decisions = Vec::new();
    let mut regressed = Vec::new();
    if order == OrderSource::GitShallow {
        check.findings.push(plan_level(Reason::OrderUnattested));
    }
    check.findings.extend(tamper::findings(&check.runs, tamper));
    match (check.rule, design.estimator) {
        (None, _) => check.findings.push(plan_level(Reason::NoDecisionRule)),
        (_, None) => check.findings.push(plan_level(Reason::NoEstimator)),
        (Some(_), Some(_)) if check.runs.is_empty() => {
            check.findings.push(plan_level(Reason::NoCollections));
        }
        (Some(rule), Some(_)) => {
            let last = check.runs.len() - 1;
            for index in 0..=last {
                let outcome = check.decide(rule, index, index == last);
                if outcome.regressed {
                    regressed.push(index);
                }
                if index == last {
                    decisions = outcome.decisions;
                    check.candidate_checks(last, &regressed, &outcome.priors, collections);
                    let found = apparatus::findings(plan, &check.runs);
                    check.findings.extend(found);
                }
            }
        }
    }
    let regressed_runs: Vec<String> = regressed
        .iter()
        .filter_map(|&index| check.runs.get(index).map(|run| id_of(run.collection)))
        .collect();
    let mut result = MeasurementVerdict {
        plan_id: plan.id.as_str().to_owned(),
        definition_version: plan.definition_version.as_str().to_owned(),
        verdict: Verdict::Inconclusive,
        reasons: Vec::new(),
        claimed,
        candidate: check.runs.last().map(|run| id_of(run.collection)),
        decisions,
        findings: check.findings,
        counts: Counts {
            regressed_runs: regressed_runs.len(),
            ..counts
        },
        regressed_runs,
        order_source: order,
    };
    settle(&mut result);
    result
}

/// Every run of `plan`, in intake order.
fn runs<'a>(
    plan: &MeasurementPlan,
    design: &StatisticalDesign,
    collections: &[Ranked<'a>],
) -> Vec<Run<'a>> {
    let mut runs: Vec<Run<'a>> = collections
        .iter()
        .map(|ranked| Run {
            collection: ranked.collection,
            intake: ranked.intake,
            apparatus: ranked.collection.protected_apparatus_of(plan.id.as_str()),
            apparatus_forged: ranked.apparatus_forged,
            slices: observations_of(plan, ranked.collection)
                .into_iter()
                .map(|observation| {
                    let assessed = design.estimator.map_or(Err(Reason::NoEstimator), |e| {
                        rows::assess(design, e, observation)
                    });
                    (observation, assessed)
                })
                .collect(),
        })
        .filter(|run| !run.slices.is_empty())
        .collect();
    order::sort(&mut runs);
    runs
}

/// The observations of `plan`'s metric under its id and definition.
fn observations_of<'a>(
    plan: &MeasurementPlan,
    collection: &'a MeasurementCollection,
) -> Vec<&'a MeasurementObservation> {
    collection
        .observations
        .iter()
        .filter(|observation| {
            observation.plan_id.as_str() == plan.id.as_str()
                && observation.metric.as_str() == plan.metric.as_str()
                && observation.definition_version.as_str() == plan.definition_version.as_str()
        })
        .collect()
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
    /// The runs a `prior-collection` rule compared each slice against.
    priors: Vec<usize>,
}

impl Check<'_> {
    /// Apply `rule` to every slice of the run at `index` against the runs
    /// before it. The candidate records every reason; an earlier run records
    /// only evidence of tampering in its own data
    /// ([`Reason::carries_from_history`]) — its shortfalls and its failed
    /// rule are history, counted but not held against the candidate.
    fn decide(&mut self, rule: DecisionRule, index: usize, is_candidate: bool) -> RunOutcome {
        let mut outcome = RunOutcome {
            regressed: false,
            decisions: Vec::new(),
            priors: Vec::new(),
        };
        let Some(run) = self.runs.get(index) else {
            return outcome;
        };
        let history = self.runs.get(..index).unwrap_or_default();
        for (observation, assessed) in &run.slices {
            let dimensions = observation.dimensions.entries();
            let mut record = |reason: Reason| {
                if is_candidate || reason.carries_from_history() {
                    self.findings.push(Finding {
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
            let (baseline, holds) = match baseline(
                rule,
                dimensions,
                observation,
                run.collection,
                history,
                run.apparatus,
            ) {
                Ok((value, prior)) => {
                    outcome.priors.extend(prior);
                    let holds = rule.holds(estimate.value, value).ok();
                    if holds.is_none() && is_candidate {
                        record(Reason::RuleNotEvaluable);
                    }
                    (value, holds)
                }
                Err(reason) => {
                    if is_candidate {
                        record(reason);
                    }
                    (None, None)
                }
            };
            if holds == Some(false) {
                outcome.regressed = true;
                if is_candidate {
                    record(Reason::RuleNotMet);
                }
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

    /// The checks that look at the candidate against everything else: its
    /// place in the order and its prior's, slices and observations it
    /// dropped, and a regressed rerun of its own apparatus.
    fn candidate_checks(
        &mut self,
        last: usize,
        regressed: &[usize],
        priors: &[usize],
        collections: &[Ranked<'_>],
    ) {
        let Some(candidate) = self.runs.get(last) else {
            return;
        };
        let at = |reason: Reason, collection: &MeasurementCollection| Finding {
            reason,
            collection_id: Some(id_of(collection)),
            dimensions: None,
        };
        let mut found = Vec::new();
        if order::unattested(&self.runs, last)
            || priors
                .iter()
                .any(|&prior| order::unattested(&self.runs, prior))
        {
            found.push(at(Reason::OrderUnattested, candidate.collection));
        }
        let mut dropped: Vec<&BTreeMap<String, JsonValue>> = Vec::new();
        for run in self.runs.get(..last).unwrap_or_default() {
            for (observation, _) in &run.slices {
                let slice = observation.dimensions.entries();
                let measured = candidate
                    .slices
                    .iter()
                    .any(|(own, _)| own.dimensions.entries() == slice);
                if !measured && !dropped.contains(&slice) {
                    dropped.push(slice);
                }
            }
        }
        found.extend(dropped.into_iter().map(|slice| Finding {
            reason: Reason::SliceMissing,
            collection_id: Some(id_of(candidate.collection)),
            dimensions: Some(slice.clone()),
        }));
        for ranked in collections {
            let is_run = self
                .runs
                .iter()
                .any(|run| std::ptr::eq(run.collection, ranked.collection));
            if !is_run
                && ranked.collection.subject == candidate.collection.subject
                && ranked.collection.scope == candidate.collection.scope
                && order::not_before(ranked.intake, candidate.intake)
            {
                found.push(at(Reason::ObservationMissing, ranked.collection));
            }
        }
        if !regressed.contains(&last) {
            for &index in regressed {
                if let Some(run) = self.runs.get(index)
                    && same_apparatus(run.collection, candidate.collection)
                {
                    found.push(at(Reason::RerunUntilPass, run.collection));
                }
            }
        }
        self.findings.extend(found);
    }
}

/// The baseline value for `slice` over `history`, and the index of the run
/// it came from when that one run alone decides it (`prior-collection`).
///
/// Only runs that recorded `apparatus` — the deciding run's own protected
/// apparatus — are a baseline (PLAT-975); in a series where no run recorded
/// one, every run's is `None` and all of them are.
///
/// `constant-predictor` is different in kind from the other two: it is not a
/// comparison against an earlier run at all, but a property of the
/// population `observation`'s own run measured (MP-222/PLAT-932), so it reads
/// `collection` — the run being decided, not `history` — through
/// [`constant_predictor::baseline`].
fn baseline(
    rule: DecisionRule,
    slice: &BTreeMap<String, JsonValue>,
    observation: &MeasurementObservation,
    collection: &MeasurementCollection,
    history: &[Run<'_>],
    apparatus: Option<&ResolvedApparatus>,
) -> Result<(Option<f64>, Option<usize>), Reason> {
    let RuleReference::Baseline { baseline, .. } = rule.reference() else {
        return Ok((None, None));
    };
    if baseline == Baseline::ConstantPredictor {
        let value = constant_predictor::baseline(
            collection,
            observation.plan_id.as_str(),
            observation.definition_version.as_str(),
            observation.metric.as_str(),
        )
        .ok_or(Reason::ConstantPredictorRowsAbsent)?;
        return Ok((Some(value), None));
    }
    // A baseline is computed from the checker's own estimates: a run that is
    // not usable evidence — tampered, short, incomplete — contributes nothing.
    let mut earlier = history.iter().enumerate().filter_map(|(index, run)| {
        if run.apparatus != apparatus {
            return None;
        }
        run.slices.iter().find_map(|(observation, assessed)| {
            (observation.dimensions.entries() == slice)
                .then_some(assessed.as_ref().ok())
                .flatten()
                .map(|estimate| (index, estimate.value))
        })
    });
    let found = match baseline {
        Baseline::ConstantPredictor => unreachable!("handled above"),
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
    result.reasons = result
        .findings
        .iter()
        .map(|finding| finding.reason)
        .collect();
    result.reasons.sort_unstable();
    result.reasons.dedup();
}
