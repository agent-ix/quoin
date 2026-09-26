// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The "Stage verdicts" table (PLAT-958), shared by the text report and the
//! portfolio.
//!
//! Split from [`crate::report::render`] so both stay under this crate's
//! module-size ceiling.

use crate::common::scalar::js_f64_string;
use crate::error::MeasurementError;
use crate::report::build::CurrentRow;
use crate::report::render::{metric_label, plan_cell, plan_cell_of, row_label};
use crate::report::vanished::VanishedSlice;
use crate::report::verdict::{GateOutcome, RatchetOutcome, StageVerdict, TargetOutcome};

/// The stage-verdict table, one row per report row whose plan carries a
/// verdict (PLAT-958), or nothing at all when none does — so a report with no
/// `ratchet`/`target` objective renders the bytes it did before.
///
/// `pub(crate)` because [`crate::portfolio::render`] prints the same table
/// under each repository.
///
/// # Errors
///
/// As [`crate::report::render::dimension_suffix`].
pub(crate) fn stage_verdict_table(
    rows: &[CurrentRow],
    vanished: &[VanishedSlice],
) -> Result<Vec<String>, MeasurementError> {
    let mut cells: Vec<(String, String, &StageVerdict)> = Vec::new();
    for row in rows {
        if let Some(verdict) = &row.stage_verdict {
            cells.push((row_label(row)?, plan_cell(row), verdict));
        }
    }
    for slice in vanished {
        cells.push((
            metric_label(&slice.metric, &slice.dimensions)?,
            plan_cell_of(
                &slice.plan_id,
                &slice.plan_path,
                slice.plan_ground_truth_kind,
            ),
            &slice.stage_verdict,
        ));
    }
    if cells.is_empty() {
        return Ok(Vec::new());
    }
    let mut lines = vec![
        "| Metric | Plan | Stage | Objective | Verdict | Detail |".to_owned(),
        "| --- | --- | --- | --- | --- | --- |".to_owned(),
    ];
    for (label, plan, verdict) in cells {
        lines.push(format!(
            "| {label} | {plan} | {} | {} | {} | {} |",
            verdict.stage().as_str(),
            objective_cell(verdict),
            verdict.as_str(),
            verdict_detail(verdict)
        ));
    }
    Ok(lines)
}

/// `higher`, or `higher; bound 0.9` when the objective states a bound.
fn objective_cell(verdict: &StageVerdict) -> String {
    let objective = verdict.objective();
    match objective.bound() {
        Some(bound) => format!("{}; bound {}", objective.direction(), js_f64_string(bound)),
        None => objective.direction().to_string(),
    }
}

/// The numbers behind a verdict, or the reason there is none.
fn verdict_detail(verdict: &StageVerdict) -> String {
    match verdict {
        StageVerdict::Ratchet {
            outcome:
                RatchetOutcome::Held {
                    current,
                    best_prior,
                }
                | RatchetOutcome::Regressed {
                    current,
                    best_prior,
                },
            ..
        } => format!(
            "current {}; best prior {} ({})",
            js_f64_string(*current),
            js_f64_string(best_prior.value),
            best_prior.collection_id
        ),
        StageVerdict::Target {
            outcome:
                TargetOutcome::Measured {
                    current,
                    bound,
                    distance,
                    ..
                },
            ..
        } => format!(
            "current {}; bound {}; distance {}",
            js_f64_string(*current),
            js_f64_string(*bound),
            js_f64_string(*distance)
        ),
        StageVerdict::Gate {
            outcome:
                GateOutcome::Pass {
                    current,
                    baseline: Some(baseline),
                    ..
                }
                | GateOutcome::Fail {
                    current,
                    baseline: Some(baseline),
                    ..
                },
            ..
        } => format!(
            "current {}; baseline {}",
            js_f64_string(*current),
            js_f64_string(*baseline)
        ),
        StageVerdict::Gate {
            outcome:
                GateOutcome::Pass {
                    current,
                    baseline: None,
                    ..
                }
                | GateOutcome::Fail {
                    current,
                    baseline: None,
                    ..
                },
            ..
        } => format!("current {}", js_f64_string(*current)),
        StageVerdict::Ratchet {
            outcome: RatchetOutcome::Inconclusive(reason),
            ..
        }
        | StageVerdict::Target {
            outcome: TargetOutcome::Inconclusive(reason),
            ..
        }
        | StageVerdict::Gate {
            outcome: GateOutcome::Inconclusive(reason),
            ..
        } => format!("{}: {}", reason.as_str(), reason.sentence()),
    }
}
