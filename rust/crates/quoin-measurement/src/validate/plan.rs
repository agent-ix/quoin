// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Which plan governs one observation at intake, and the untyped findings
//! tying the observation to it.
//!
//! Split from [`super`] on responsibility (quoin#464's module-size ceiling)
//! when constant-predictor item rows (PLAT-1016) gave the plan lookup a
//! second case.

use std::collections::BTreeMap;

use crate::types::observation::{MeasurementObservation, constant_predictor_governed_metric};
use crate::types::plan::{LifecycleStatus, MeasurementPlan};

/// The plan `observation` answers to, and the plan whose population rules
/// bind it.
///
/// An observation of a planned metric answers to that plan on both counts. A
/// constant-predictor item row (`{metric}.constant-predictor-item`,
/// PLAT-1016) answers to `metric`'s plan — the same active-plan, plan-id and
/// definition-version checks — but is one graded item, not a population, so
/// no `minimum_population` or `repetitions` binds it; those bind the
/// aggregate observation.
pub(super) fn governing<'a>(
    by_metric: &BTreeMap<&str, &'a MeasurementPlan>,
    observation: &MeasurementObservation,
) -> (Option<&'a MeasurementPlan>, Option<&'a MeasurementPlan>) {
    let metric = observation.metric.as_str();
    match by_metric.get(metric).copied() {
        Some(plan) => (Some(plan), Some(plan)),
        None => (
            constant_predictor_governed_metric(metric)
                .and_then(|governed| by_metric.get(governed).copied()),
            None,
        ),
    }
}

/// The untyped findings tying one observation to its plan: none governs it,
/// or the one that does is inactive, differently named, or at another
/// definition version.
pub(super) fn findings(
    observation: &MeasurementObservation,
    plan: Option<&MeasurementPlan>,
) -> Vec<String> {
    let metric = observation.metric.as_str();
    let Some(plan) = plan else {
        return vec![format!(
            "metric `{metric}` has no MeasurementPlan under spec/assurance or assurance; \
             record refused"
        )];
    };
    let mut out = Vec::new();
    if plan.status != LifecycleStatus::Active {
        out.push(format!(
            "metric `{metric}` plan {} is {}, not active",
            plan.id,
            plan.status.as_str()
        ));
    }
    if observation.plan_id != plan.id {
        out.push(format!(
            "metric `{metric}` names plan {}; active plan is {}",
            observation.plan_id, plan.id
        ));
    }
    if observation.definition_version != plan.definition_version {
        out.push(format!(
            "metric `{metric}` definition {} does not match {}",
            observation.definition_version, plan.definition_version
        ));
    }
    out
}
