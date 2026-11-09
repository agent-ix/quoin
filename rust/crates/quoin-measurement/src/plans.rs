// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Loading `MeasurementPlan`s out of assurance-document frontmatter.
//!
//! Ports `src/measurement/plans.ts`. The walk itself is
//! [`crate::discovery::documents`] — see that module for why it is shared with
//! [`crate::profiles`] rather than written twice.
//!
//! # `ground_truth_kind` and `statistical_design` (PLAT-960)
//!
//! Both are optional. A document that states neither loads exactly as it did
//! before PLAT-960. A document that states one with an unacceptable value —
//! a `ground_truth_kind` outside its three spellings, a `statistical_design`
//! that is not an object, or a `minimum_population` or `repetitions` that is
//! not a whole number from 1 to 4,294,967,295 — refuses the plan load with
//! [`MeasurementErrorCode::PlanInvalid`], the same as any other shape error here.
//! Enforcing `minimum_population` and `repetitions` against a collection is
//! [`crate::validate::measurement_collection`]'s job, not this module's.
//!
//! # `objective` (PLAT-958)
//!
//! Optional, and parsed into engineering-assurance's own
//! [`engineering_assurance::measurement::Objective`]. A block EA refuses — an
//! unknown `direction`, an unknown member, a non-numeric or non-finite
//! `bound`, or a `target` with no `bound` — refuses the plan load with
//! [`MeasurementErrorCode::PlanInvalid`].
//!
//! # `statistical_design.estimator` and `.decision_rule` (PLAT-961)
//!
//! Optional, and parsed into engineering-assurance's own `Estimator` and
//! `DecisionRule` (its FR-021). A value EA refuses, a `constant-predictor`
//! baseline under an estimator other than `proportion`, or a comparator that
//! disagrees with the plan's `objective` refuses the plan load with
//! [`MeasurementErrorCode::PlanInvalid`] naming the member.
//!
//! # `protected_apparatus` and `negative_controls` (PLAT-975)
//!
//! Optional, and parsed into engineering-assurance's own types (its FR-024).
//! A list EA refuses, a `gate` plan missing either list, or an
//! `apparatus-edit` control with no `protected_apparatus`, refuses the plan
//! load with
//! [`MeasurementErrorCode::PlanInvalid`] naming the member.

mod apparatus;
mod design;
mod history;

pub use history::{definition_changed_without_version_bump, plan_from_text};

use crate::discovery;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::source::MeasurementSource;
use crate::types::ids::NonEmptyText;
use crate::types::plan::{LifecycleStatus, MeasurementPlan, MeasurementStage};
#[cfg(feature = "campaign")]
use std::collections::BTreeSet;

/// The code every refusal in this module carries.
const CODE: MeasurementErrorCode = MeasurementErrorCode::PlanInvalid;

/// What a plan load should include beyond the members every caller needs.
///
/// `plans.ts:20` takes `{ includeGovernance?: boolean }` and spreads `owner`
/// and `action` in only when it is set, so a caller that did not ask cannot
/// read them by accident.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlanLoadOptions {
    /// Carry `owner` and `action` through when the document states them.
    pub include_governance: bool,
}

/// Load every `MeasurementPlan` the repository's assurance roots declare.
///
/// The result is sorted by metric, then id (`plans.ts:25`).
///
/// # Errors
///
/// [`MeasurementErrorCode::PlanInvalid`] when a `MeasurementPlan` document
/// lacks a required member, or names an unknown stage or status;
/// [`MeasurementErrorCode::Yaml`] for unreadable frontmatter; and
/// [`MeasurementErrorCode::Io`] when a document cannot be read.
pub fn load_measurement_plans<S: MeasurementSource + ?Sized>(
    source: &S,
    options: PlanLoadOptions,
) -> Result<Vec<MeasurementPlan>, MeasurementError> {
    let mut plans = discovery::documents(source, "MeasurementPlan", |path, value, _text| {
        plan_from(path, value, options)
    })?;
    // Stable, so documents that agree on metric and id keep the walk's own
    // path order, as `Array.prototype.sort` does in V8.
    plans.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Ok(plans)
}

/// Load only plans named by an exact campaign definition. Other tracked plans
/// do not participate in this campaign's semantic contract.
///
/// The document walk still parses frontmatter, while full plan validation is
/// applied to every selected plan. Missing and duplicate registrations are
/// rejected by the campaign definition validator.
#[cfg(feature = "campaign")]
pub(crate) fn load_selected_measurement_plans<S: MeasurementSource + ?Sized>(
    source: &S,
    ids: &BTreeSet<&str>,
) -> Result<Vec<MeasurementPlan>, MeasurementError> {
    let mut plans = discovery::documents(source, "MeasurementPlan", |path, value, _text| {
        if value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|id| ids.contains(id))
        {
            plan_from(path, value, PlanLoadOptions::default()).map(Some)
        } else {
            Ok(None)
        }
    })?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    plans.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Ok(plans)
}

pub(crate) fn plan_from(
    path: &str,
    value: &serde_json::Value,
    options: PlanLoadOptions,
) -> Result<MeasurementPlan, MeasurementError> {
    let required = |name: &str| -> Result<NonEmptyText, MeasurementError> {
        discovery::required(value, name)
            .ok_or_else(|| {
                MeasurementError::new(
                    CODE,
                    format!("{path}: MeasurementPlan requires non-empty `{name}`"),
                )
            })
            .and_then(|text| NonEmptyText::parse(text, CODE, name))
    };
    // `plans.ts:44-56` checks the six required members before it looks at the
    // stage or the status, so a document missing `stage` entirely is refused
    // for the missing member rather than for an unknown stage.
    let id = required("id")?;
    let title = required("title")?;
    let status = required("status")?;
    let stage = required("stage")?;
    let metric = required("metric")?;
    let definition_version = required("definition_version")?;
    let execution_procedure = execution_procedure_from(path, value)?;
    // The suffix is reserved for constant-predictor item observations
    // (PLAT-1016): a plan governing `x.constant-predictor-item` would read
    // plan `x`'s item rows as its own run, and intake and `compare` could no
    // longer tell an item row from a governed observation.
    if crate::types::observation::constant_predictor_governed_metric(metric.as_str()).is_some() {
        return Err(MeasurementError::new(
            CODE,
            format!(
                "{path}: MeasurementPlan metric `{metric}` ends with the reserved \
                 constant-predictor item suffix `{}`",
                crate::types::observation::CONSTANT_PREDICTOR_ITEM_METRIC_SUFFIX
            ),
        ));
    }

    let stage = MeasurementStage::from_wire(stage.as_str()).ok_or_else(|| {
        MeasurementError::new(CODE, format!("{path}: unknown measurement stage `{stage}`"))
    })?;
    let status = LifecycleStatus::from_wire(status.as_str()).ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!("{path}: unknown MeasurementPlan status `{status}`"),
        )
    })?;

    let governance = |name: &str| -> Option<String> {
        if options.include_governance {
            discovery::required(value, name).map(str::to_owned)
        } else {
            None
        }
    };
    let ground_truth_kind = design::ground_truth_kind_from(path, value)?;
    let statistical_design = design::statistical_design_from(path, value)?;
    let objective = design::objective_from(path, value)?;
    design::rule_agrees_with_objective(path, statistical_design.as_ref(), objective.as_ref())?;
    let (protected_apparatus, negative_controls) = apparatus::apparatus_from(path, value, stage)?;
    if let Some(procedure) = execution_procedure.as_deref() {
        let covered = protected_apparatus.as_ref().is_some_and(|apparatus| {
            apparatus.iter().any(|entry| {
                entry.as_str() == procedure
                    || entry.directory().is_some_and(|directory| {
                        procedure.starts_with(directory)
                            && procedure.as_bytes().get(directory.len()) == Some(&b'/')
                    })
            })
        });
        if !covered {
            return Err(MeasurementError::new(
                CODE,
                format!("{path}: execution_procedure must be protected by protected_apparatus"),
            ));
        }
    }
    Ok(MeasurementPlan {
        id,
        title,
        status,
        stage,
        metric,
        definition_version,
        execution_procedure,
        path: path.to_owned(),
        owner: governance("owner"),
        action: governance("action"),
        ground_truth_kind,
        statistical_design,
        objective,
        protected_apparatus,
        negative_controls,
    })
}

fn execution_procedure_from(
    plan_path: &str,
    value: &serde_json::Value,
) -> Result<Option<String>, MeasurementError> {
    let Some(raw) = value.get("execution_procedure") else {
        return Ok(None);
    };
    let procedure_path = raw.as_str().ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!("{plan_path}: execution_procedure must be a path"),
        )
    })?;
    let parsed = engineering_assurance::measurement::ApparatusPath::new(procedure_path).map_err(
        |error| {
            MeasurementError::new(
                CODE,
                format!("{plan_path}: invalid execution_procedure: {error}"),
            )
        },
    )?;
    let json_file = std::path::Path::new(procedure_path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if parsed.directory().is_some() || !json_file {
        return Err(MeasurementError::new(
            CODE,
            format!("{plan_path}: execution_procedure must name one JSON file"),
        ));
    }
    Ok(Some(procedure_path.to_owned()))
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests;
