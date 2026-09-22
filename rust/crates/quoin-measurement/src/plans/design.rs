// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A plan's `ground_truth_kind` and `statistical_design` members (PLAT-960).
//!
//! Split from [`super`] so the plan walk stays under this crate's module-size
//! ceiling. See [`super`]'s module header for what a malformed member does.

use std::num::NonZeroU32;

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

/// Read the optional `statistical_design` block's `minimum_population` and
/// `repetitions`, or `None` when the document declares no such block.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] when the block is present but is not
/// an object, or when either member is present but is not a whole number from
/// 1 to [`u32::MAX`].
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
    Ok(Some(StatisticalDesign {
        minimum_population: count("minimum_population")?,
        repetitions: count("repetitions")?,
    }))
}
