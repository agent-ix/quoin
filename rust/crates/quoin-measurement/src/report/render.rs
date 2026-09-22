// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The measurement report as markdown.
//!
//! Ports `renderMeasurementReport` (`report.ts:105-200`), byte for byte,
//! including the places it emits nothing rather than a placeholder.
//!
//! # The three cells shared with the portfolio view
//!
//! `portfolio.ts:182-201` is a transcription of `report.ts:119-135`: the same
//! dimension suffix, the same metric label, the same four-way observation cell,
//! differing only in two fallback sentences. Porting it twice would land a
//! known duplication in new code, so the metric label and the observation cell
//! are written once here and [`crate::portfolio::render`] calls them with its
//! own two sentences.

use std::collections::BTreeMap;

use quoin_store::{JsonObject, JsonValue};

use crate::common::scalar::{js_f64_string, js_string};
use crate::error::MeasurementError;
use crate::intervention::report::render_intervention_report;
use crate::json_bridge::to_serde;
use crate::operational::report::render_operational_report;
use crate::report::build::{CurrentRow, MeasurementReport};
use crate::report::verdict::{RatchetOutcome, StageVerdict, TargetOutcome};
use crate::types::observation::{Dimensions, MeasurementObservation, MeasurementState};

/// The metrics that carry a factual attention sentence of their own.
///
/// `report.ts:176-196` is an `else if` ladder over three metric spellings. A
/// ladder over `&str` is what the Stage 6 plan §13.3 asks to be a type: parsed
/// once here, matched exhaustively below, so a fourth attention metric is a
/// compile error rather than a branch nobody added.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum AttentionMetric {
    /// `finding_recall` — a zero recall means no seeded defect was detected.
    FindingRecall,
    /// `finding_precision.unadjudicated` — firings with no corpus ruling.
    UnadjudicatedPrecision,
    /// `actionability_rate` — findings that name a row or a line.
    ActionabilityRate,
}

impl AttentionMetric {
    /// Every metric, in the order the retained ladder tests them.
    const ALL: [Self; 3] = [
        Self::FindingRecall,
        Self::UnadjudicatedPrecision,
        Self::ActionabilityRate,
    ];

    /// The metric name as a plan spells it.
    const fn as_str(self) -> &'static str {
        match self {
            Self::FindingRecall => "finding_recall",
            Self::UnadjudicatedPrecision => "finding_precision.unadjudicated",
            Self::ActionabilityRate => "actionability_rate",
        }
    }

    /// Recover a metric from its name.
    fn of(metric: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == metric)
    }
}

/// `${key}=${value}` for every dimension, joined with `, `.
///
/// `report.ts:120-122`. Empty when there are no dimensions, which is what
/// decides whether the metric label carries a `[…]` suffix at all.
///
/// # Errors
///
/// [`crate::MeasurementErrorCode::Store`] when a dimension value cannot cross
/// [`crate::json_bridge`] — a non-finite number, which no retained store holds.
pub(crate) fn dimension_suffix(
    dimensions: &BTreeMap<String, JsonValue>,
) -> Result<String, MeasurementError> {
    if dimensions.is_empty() {
        return Ok(String::new());
    }
    let mut object = JsonObject::new();
    for (name, value) in dimensions {
        object.set(name.clone(), value.clone());
    }
    let Some(members) = to_serde(&JsonValue::Object(object))?.as_object().cloned() else {
        return Ok(String::new());
    };
    Ok(members
        .iter()
        .map(|(name, value)| format!("{name}={}", js_string(Some(value))))
        .collect::<Vec<_>>()
        .join(", "))
}

/// `metric` or `metric [k=v, …]`.
///
/// `report.ts:123-124`, `report.ts:190`, `portfolio.ts:186-188` and
/// `portfolio.ts:329-331` — one expression, written four times there and once
/// here.
///
/// # Errors
///
/// As [`dimension_suffix`].
pub(crate) fn metric_label(
    metric: &str,
    dimensions: &BTreeMap<String, JsonValue>,
) -> Result<String, MeasurementError> {
    let suffix = dimension_suffix(dimensions)?;
    Ok(if suffix.is_empty() {
        metric.to_owned()
    } else {
        format!("{metric} [{suffix}]")
    })
}

/// The label for one report row, whose dimensions come from its observation.
pub(crate) fn row_label(row: &CurrentRow) -> Result<String, MeasurementError> {
    match &row.observation {
        Some(observation) => metric_label(&row.metric, observation.dimensions.entries()),
        None => metric_label(&row.metric, Dimensions::ABSENT.entries()),
    }
}

/// The `Plan` cell: `id (path)`, with the plan's ground-truth kind inside the
/// parentheses when it states one (PLAT-960).
///
/// `pub(crate)` because [`crate::portfolio::render`] quotes the plan the same
/// way. A plan that states no kind renders the bytes it did before PLAT-960.
pub(crate) fn plan_cell(row: &CurrentRow) -> String {
    match row.plan_ground_truth_kind {
        Some(kind) => format!(
            "{} ({}; ground truth: {})",
            row.plan_id,
            row.plan_path,
            kind.as_str()
        ),
        None => format!("{} ({})", row.plan_id, row.plan_path),
    }
}

/// The four-way `Current` cell.
///
/// `report.ts:125-133` and `portfolio.ts:189-196` differ only in the two
/// sentences a caller supplies: what to say when a `not_computed` observation
/// gave no reason, and what to say when there is no observation at all.
pub(crate) fn observation_cell(row: &CurrentRow, no_reason: &str, no_observation: &str) -> String {
    let Some(observation) = &row.observation else {
        return no_observation.to_owned();
    };
    if observation.definition_version.as_str() != row.plan_definition_version {
        return format!(
            "incomparable: definition {}; active plan {}",
            observation.definition_version, row.plan_definition_version
        );
    }
    match observation.state {
        MeasurementState::Measured => format!(
            "{} {}",
            value_text(observation.value),
            observation.unit.as_str()
        ),
        MeasurementState::NotComputed => format!(
            "not_computed: {}",
            observation.reason.as_deref().unwrap_or(no_reason)
        ),
    }
}

/// `${observation.value}` — `String(x)` for a value that may be `null`.
fn value_text(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), js_f64_string)
}

/// The provenance bullet's trailing clause naming every artifact this
/// repository could not verify locally, or nothing (PLAT-969's ruling: a
/// label is admitted, but the report states it beside the collection it came
/// from rather than staying silent).
///
/// `pub(crate)` because [`crate::portfolio::render`] shares the same clause on
/// its own "Provenance:" line rather than spelling it a second way.
pub(crate) fn unverified_artifacts_suffix(names: &[String]) -> String {
    if names.is_empty() {
        String::new()
    } else {
        format!("; digest not checked: {}", names.join(", "))
    }
}

/// Render the report.
///
/// `renderMeasurementReport` (`report.ts:105-200`). There is no trailing
/// newline: the retained function joins with `"\n"` and the last pushed element
/// is the empty string.
///
/// # Errors
///
/// As [`dimension_suffix`].
pub fn render_measurement_report(report: &MeasurementReport) -> Result<String, MeasurementError> {
    let mut lines: Vec<String> = vec![
        "# QA measurement report".to_owned(),
        String::new(),
        "## What this repository measures".to_owned(),
        String::new(),
    ];
    if report.plans.is_empty() {
        lines.push("No MeasurementPlan is authored.".to_owned());
        lines.push(String::new());
    } else {
        lines.push("| Metric | Plan | Stage | Current |".to_owned());
        lines.push("| --- | --- | --- | --- |".to_owned());
        for row in &report.current {
            lines.push(format!(
                "| {} | {} | {} | {} |",
                row_label(row)?,
                plan_cell(row),
                row.stage.as_str(),
                observation_cell(
                    row,
                    "producer did not compute it",
                    "not_computed: no record"
                )
            ));
        }
        lines.push(String::new());
        let verdicts = stage_verdict_table(&report.current)?;
        if !verdicts.is_empty() {
            lines.push("## Stage verdicts".to_owned());
            lines.push(String::new());
            lines.extend(verdicts);
            lines.push(String::new());
        }
    }

    lines.push("## Current evidence".to_owned());
    lines.push(String::new());
    lines.push(report.corpus_gaps.map_or_else(
        || "Corpus gaps: not_computed".to_owned(),
        |gaps| format!("Corpus gaps: {}", js_f64_string(gaps)),
    ));
    lines.push(String::new());

    // `new Map(entries)` keeps the first insertion's position and the last
    // insertion's value; every row quoting one collection quotes the same one,
    // so only the position matters and a first-seen list reproduces it.
    let mut seen: Vec<&str> = Vec::new();
    for row in &report.current {
        let Some(collection) = &row.collection else {
            continue;
        };
        if seen.contains(&collection.collection_id.as_str()) {
            continue;
        }
        seen.push(&collection.collection_id);
        lines.push(format!(
            "- {} — {} {}; source {}; corpus {}; config {}{}",
            collection.timestamp,
            collection.tool_identity,
            collection.tool_version,
            collection.source_revision,
            collection.corpus_revision.as_deref().unwrap_or("n/a"),
            collection.config_digest,
            unverified_artifacts_suffix(&collection.unverified_artifacts),
        ));
    }
    if seen.is_empty() {
        lines.push("No measurement collection recorded.".to_owned());
    }
    lines.push(String::new());

    lines.push("## Attention".to_owned());
    lines.push(String::new());
    let attention = attention(report)?;
    if attention.is_empty() {
        lines.push("No factual attention items.".to_owned());
    } else {
        lines.extend(attention.into_iter().map(|item| format!("- {item}")));
    }
    lines.push(String::new());

    let intervention = render_intervention_report(&report.interventions);
    if !intervention.is_empty() {
        lines.push(intervention);
    }
    let operational = render_operational_report(&report.operational);
    if !operational.is_empty() {
        lines.push(operational);
    }
    Ok(lines.join("\n"))
}

/// The factual attention items. `report.ts:158-198`.
fn attention(report: &MeasurementReport) -> Result<Vec<String>, MeasurementError> {
    let mut out = Vec::new();
    // `(report.corpusGaps ?? 0) > 0`.
    if report.corpus_gaps.unwrap_or(0.0) > 0.0 {
        out.push(format!(
            "{} mode-language cells are GAP; run the corpus bounds view to see each named cell.",
            js_f64_string(report.corpus_gaps.unwrap_or(0.0))
        ));
    }
    for row in &report.current {
        let Some(observation) = &row.observation else {
            out.push(format!(
                "{}: no collection has computed this authored plan.",
                row.metric
            ));
            continue;
        };
        let name = metric_label(&row.metric, observation.dimensions.entries())?;
        if let Some(item) = attention_for(row, observation, &name) {
            out.push(item);
        }
        if let Some(StageVerdict::Ratchet {
            outcome:
                RatchetOutcome::Regressed {
                    current,
                    best_prior,
                },
            ..
        }) = &row.stage_verdict
        {
            out.push(format!(
                "{name}: regressed to {} from the best prior {} ({}); plan {} does not hold its \
                 ratchet.",
                js_f64_string(*current),
                js_f64_string(best_prior.value),
                best_prior.collection_id,
                row.plan_id
            ));
        }
    }
    Ok(out)
}

/// One row's attention sentence, if it has one.
fn attention_for(
    row: &CurrentRow,
    observation: &MeasurementObservation,
    name: &str,
) -> Option<String> {
    if observation.state == MeasurementState::NotComputed {
        return Some(format!(
            "{name}: not computed — {}.",
            observation
                .reason
                .as_deref()
                .unwrap_or("no reason supplied")
        ));
    }
    if observation.definition_version.as_str() != row.plan_definition_version {
        return Some(format!(
            "{name}: stored definition {} does not match active plan {}; do not compare the \
             value.",
            observation.definition_version, row.plan_definition_version
        ));
    }
    match AttentionMetric::of(&row.metric)? {
        AttentionMetric::FindingRecall => (observation.value == Some(0.0)).then(|| {
            format!(
                "{name}: zero seeded defects detected; add or connect the detector named by the \
                 family."
            )
        }),
        AttentionMetric::UnadjudicatedPrecision => {
            (observation.value.unwrap_or(0.0) > 0.0).then(|| {
                format!(
                    "{name}: {} firings have no corpus ruling; adjudicate their cases.",
                    value_text(observation.value)
                )
            })
        }
        AttentionMetric::ActionabilityRate => {
            let examined = observation
                .population
                .as_ref()
                .and_then(|population| population.examined)?;
            let matched = observation
                .population
                .as_ref()
                .and_then(|population| population.matched)
                .unwrap_or(0.0);
            Some(format!(
                "{name}: {} of {} findings name a row or line; inspect emitted findings without a \
                 locus.",
                js_f64_string(matched),
                js_f64_string(examined)
            ))
        }
    }
}

/// The stage-verdict table, one row per report row whose plan carries a
/// verdict (PLAT-958), or nothing at all when none does — so a report with no
/// `ratchet`/`target` objective renders the bytes it did before.
///
/// `pub(crate)` because [`crate::portfolio::render`] prints the same table
/// under each repository.
///
/// # Errors
///
/// As [`dimension_suffix`].
pub(crate) fn stage_verdict_table(rows: &[CurrentRow]) -> Result<Vec<String>, MeasurementError> {
    let mut lines = Vec::new();
    for row in rows {
        let Some(verdict) = &row.stage_verdict else {
            continue;
        };
        if lines.is_empty() {
            lines.push("| Metric | Plan | Stage | Objective | Verdict | Detail |".to_owned());
            lines.push("| --- | --- | --- | --- | --- | --- |".to_owned());
        }
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            row_label(row)?,
            plan_cell(row),
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
        StageVerdict::Ratchet {
            outcome: RatchetOutcome::Inconclusive(reason),
            ..
        }
        | StageVerdict::Target {
            outcome: TargetOutcome::Inconclusive(reason),
            ..
        } => format!("{}: {}", reason.as_str(), reason.sentence()),
    }
}
