// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A plan's `ground_truth_kind` and `statistical_design` members (PLAT-960,
//! PLAT-961), and its `objective` block (PLAT-958).
//!
//! Split from [`super`] so the plan walk stays under this crate's module-size
//! ceiling. See [`super`]'s module header for what a malformed member does.

use std::cmp::Ordering;
use std::num::NonZeroU32;

use engineering_assurance::measurement::{DecisionRule, Direction, Estimator, Objective};
use serde::Deserialize;

use crate::error::MeasurementError;
use crate::types::plan::{GroundTruthKind, StatisticalDesign};

use super::CODE;

/// Read the optional `ground_truth_kind`, or `None` when the document does
/// not state one.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] when the member is present but is not
/// one of [`GroundTruthKind`]'s three spellings.
pub(super) fn ground_truth_kind_from(
    path: &str,
    value: &serde_json::Value,
) -> Result<Option<GroundTruthKind>, MeasurementError> {
    let Some(stated) = value.get("ground_truth_kind") else {
        return Ok(None);
    };
    stated
        .as_str()
        .and_then(GroundTruthKind::from_wire)
        .map(Some)
        .ok_or_else(|| {
            MeasurementError::new(
                CODE,
                format!(
                    "{path}: ground_truth_kind must be one of human-labelled, agent-labelled, \
                     mechanical; found {stated}"
                ),
            )
        })
}

/// Read the optional `statistical_design` block's `minimum_population`,
/// `repetitions`, `estimator` and `decision_rule`, or `None` when the document
/// declares no such block.
///
/// `estimator` and `decision_rule` are engineering-assurance's (FR-021) and
/// are deserialized into EA's [`Estimator`] and [`DecisionRule`], so their
/// vocabulary and the rule's shape (exactly one of `threshold` or `baseline`,
/// a `margin` only with a `baseline`, finite numbers) are EA's to state. The
/// rule's agreement with the estimator is EA's
/// [`DecisionRule::check_estimator`], asked here; its agreement with the
/// objective is asked by [`rule_agrees_with_objective`] once both are read.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] when the block is present but is not
/// an object, when either count is present but is not a whole number from
/// 1 to [`u32::MAX`], when `estimator` or `decision_rule` is present and EA
/// refuses it, or when the rule's baseline is not allowed with the estimator.
pub(super) fn statistical_design_from(
    path: &str,
    value: &serde_json::Value,
) -> Result<Option<StatisticalDesign>, MeasurementError> {
    let Some(block) = value.get("statistical_design") else {
        return Ok(None);
    };
    let block = block.as_object().ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!("{path}: statistical_design must be an object"),
        )
    })?;
    let count = |name: &str| -> Result<Option<NonZeroU32>, MeasurementError> {
        let Some(stated) = block.get(name) else {
            return Ok(None);
        };
        // `as_u64` is `None` for a negative, fractional or non-numeric value,
        // so `2.5`, `-1` and `"3"` all land in the refusal below rather than
        // being rounded or coerced.
        stated
            .as_u64()
            .and_then(|whole| u32::try_from(whole).ok())
            .and_then(NonZeroU32::new)
            .map(Some)
            .ok_or_else(|| {
                MeasurementError::new(
                    CODE,
                    format!(
                        "{path}: statistical_design.{name} must be a whole number from 1 to {}; \
                         found {stated}",
                        u32::MAX
                    ),
                )
            })
    };
    let estimator = block
        .get("estimator")
        .map(|stated| {
            Estimator::deserialize(stated).map_err(|error| {
                MeasurementError::new(
                    CODE,
                    format!("{path}: statistical_design.estimator is invalid: {error}; found {stated}"),
                )
            })
        })
        .transpose()?;
    let decision_rule = block
        .get("decision_rule")
        .map(|stated| {
            DecisionRule::deserialize(stated).map_err(|error| {
                MeasurementError::new(
                    CODE,
                    format!(
                        "{path}: statistical_design.decision_rule is invalid: {error}; found {stated}"
                    ),
                )
            })
        })
        .transpose()?;
    if let (Some(rule), Some(estimator)) = (decision_rule, estimator) {
        rule.check_estimator(estimator).map_err(|error| {
            MeasurementError::new(
                CODE,
                format!("{path}: statistical_design.decision_rule is invalid: {error}"),
            )
        })?;
    }
    Ok(Some(StatisticalDesign {
        minimum_population: count("minimum_population")?,
        repetitions: count("repetitions")?,
        estimator,
        decision_rule,
    }))
}

/// Refuse a decision rule whose comparator disagrees with the plan's
/// objective, asking engineering-assurance's [`DecisionRule::check_against`].
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] naming
/// `statistical_design.decision_rule` when EA reports a disagreement.
pub(super) fn rule_agrees_with_objective(
    path: &str,
    design: Option<&StatisticalDesign>,
    objective: Option<&Objective>,
) -> Result<(), MeasurementError> {
    let (Some(rule), Some(objective)) = (design.and_then(|d| d.decision_rule), objective) else {
        return Ok(());
    };
    rule.check_against(objective).map_err(|error| {
        MeasurementError::new(
            CODE,
            format!("{path}: statistical_design.decision_rule is invalid: {error}"),
        )
    })
}

/// Read the optional `objective` block, or `None` when the document states
/// none.
///
/// The block is engineering-assurance's (FR-020): its shape and its rules —
/// a closed `{direction, bound?}` object, a finite `bound`, and a `bound`
/// required by `direction: target` — are enforced by deserializing into
/// [`Objective`] itself, so this crate states none of them a second time.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] when the block is
/// present and [`Objective`] refuses it, carrying EA's own reason.
pub(super) fn objective_from(
    path: &str,
    value: &serde_json::Value,
) -> Result<Option<Objective>, MeasurementError> {
    let Some(stated) = value.get("objective") else {
        return Ok(None);
    };
    let objective = Objective::deserialize(stated).map_err(|error| {
        MeasurementError::new(
            CODE,
            format!("{path}: objective is invalid: {error}; found {stated}"),
        )
    })?;
    // EA admits any finite bound. A `zero` objective's bound is a tolerance
    // around zero, and a negative tolerance means nothing — so it is refused
    // here rather than read through `abs()` as some other tolerance.
    if objective.direction() == Direction::Zero
        && objective
            .bound()
            .is_some_and(|bound| bound.partial_cmp(&0.0) == Some(Ordering::Less))
    {
        return Err(MeasurementError::new(
            CODE,
            format!(
                "{path}: objective is invalid: a `zero` objective's bound is a tolerance and \
                 must not be negative; found {stated}"
            ),
        ));
    }
    Ok(Some(objective))
}
