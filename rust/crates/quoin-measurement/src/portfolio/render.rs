// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The portfolio report as markdown.
//!
//! Ports `renderPortfolioReport` and `renderComparison`
//! (`portfolio.ts:142-217,333-367`).
//!
//! # What is shared with the per-repository report
//!
//! `portfolio.ts:194-206` is `report.ts:119-133` with two different fallback
//! sentences, so the metric label and the observation cell come from
//! [`crate::report::render`] rather than being transcribed a second time. The
//! two sentences are the arguments.

use crate::common::scalar::js_f64_string;
use crate::error::MeasurementError;
use crate::portfolio::ranking::{PortfolioRanking, RANKING_ADVISORY_NOTE, rank_portfolio};
use crate::portfolio::types::{PortfolioReport, PortfolioRepositoryReport};
use crate::report::render::{
    metric_label, observation_cell, plan_cell, row_label, unverified_artifacts_suffix,
};
use crate::report::verdict_render::stage_verdict_table;

/// Render the portfolio as markdown.
///
/// `renderPortfolioReport` (`portfolio.ts:142-217`). No trailing newline: the
/// retained function joins with `"\n"` and the last pushed element is empty.
///
/// # Errors
///
/// As [`crate::report::render_measurement_report`].
pub fn render_portfolio_report(report: &PortfolioReport) -> Result<String, MeasurementError> {
    let mut lines = vec![
        "# QA portfolio report".to_owned(),
        String::new(),
        "No cross-repository metric is summed, averaged, or assigned a quality verdict.".to_owned(),
        String::new(),
    ];
    for repository in &report.repositories {
        lines.push(format!("## {}", repository.name));
        lines.push(String::new());
        lines.push(format!("Root: {}", repository.root));
        if let Some(error) = repository.status.error() {
            lines.push(format!("Status: {} — {error}", repository.status.as_str()));
            lines.push(String::new());
            continue;
        }
        lines.extend(readable(repository)?);
    }
    lines.extend(ranking_section(&rank_portfolio(report)?));
    Ok(lines.join("\n"))
}

/// The "Priority ranking" section: every rankable plan, scored and ordered,
/// then every plan the ranking could not score, with why. Present even when
/// both lists are empty, so a reader always sees [`RANKING_ADVISORY_NOTE`].
fn ranking_section(ranking: &PortfolioRanking) -> Vec<String> {
    let mut lines = vec![
        "## Priority ranking".to_owned(),
        String::new(),
        RANKING_ADVISORY_NOTE.to_owned(),
        String::new(),
    ];
    lines.push("| Repository | Plan | Metric | Score | Gap | Weight | Decay | Budget |".to_owned());
    lines.push("| --- | --- | --- | ---: | ---: | ---: | ---: | --- |".to_owned());
    if ranking.ranked.is_empty() {
        lines.push("| n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |".to_owned());
    } else {
        for entry in &ranking.ranked {
            lines.push(format!(
                "| {} | {} ({}) | {} | {} | {} | {} | {} | {} |",
                entry.repository,
                entry.plan_id,
                entry.plan_path,
                entry.label,
                js_f64_string(entry.score),
                js_f64_string(entry.gap),
                js_f64_string(entry.weight),
                js_f64_string(entry.decay_factor),
                entry.budget.map_or_else(|| "n/a".to_owned(), js_f64_string),
            ));
        }
    }
    lines.push(String::new());
    if !ranking.unranked.is_empty() {
        lines.push("Not ranked:".to_owned());
        lines.push(String::new());
        for plan in &ranking.unranked {
            lines.push(format!(
                "- {} — {} ({}): {}",
                plan.repository,
                plan.plan_id,
                plan.plan_path,
                plan.reason.as_str()
            ));
        }
        lines.push(String::new());
    }
    lines
}

/// The lines for a repository that could be read.
fn readable(repository: &PortfolioRepositoryReport) -> Result<Vec<String>, MeasurementError> {
    let mut lines = vec![
        format!(
            "Status: readable; measurement store: {}",
            repository.store.as_str()
        ),
        format!("Profiles: {}", profiles(repository)),
    ];
    let latest = repository.latest_collection.as_ref();
    lines.push(latest.map_or_else(
        || "Latest: not_computed — no measurement collection".to_owned(),
        |collection| {
            format!(
                "Latest: {} — {} ({}); {}, {} days behind portfolio latest",
                collection.timestamp,
                collection.id,
                collection.path,
                repository.staleness.as_str(),
                repository
                    .staleness
                    .age_days()
                    .map_or_else(|| "null".to_owned(), |days| days.to_string())
            )
        },
    ));
    if let Some(collection) = latest {
        lines.push(format!(
            "Provenance: source {}; corpus {}; tool {} {}; config {}{}",
            collection.source_revision,
            collection.corpus_revision.as_deref().unwrap_or("n/a"),
            collection.tool_identity,
            collection.tool_version,
            collection.config_digest,
            unverified_artifacts_suffix(&collection.unverified_artifacts),
        ));
    }

    let measurements = repository.measurements.as_ref();
    let gaps = measurements.and_then(|report| report.corpus_gaps);
    lines.push(match (gaps, latest) {
        (Some(gaps), Some(collection)) => format!(
            "Corpus gaps: {} ({}; raw evidence, no metric plan)",
            js_f64_string(gaps),
            collection.path
        ),
        _ => "Corpus gaps: not_computed".to_owned(),
    });
    lines.push(String::new());
    lines.push("| Metric | State/value | Plan | Collection |".to_owned());
    lines.push("| --- | --- | --- | --- |".to_owned());

    let rows = measurements
        .map(|report| report.current.as_slice())
        .unwrap_or_default();
    if rows.is_empty() {
        lines.push("| not_computed | no active MeasurementPlan | n/a | n/a |".to_owned());
    } else {
        for row in rows {
            lines.push(format!(
                "| {} | {} | {} | {} |",
                row_label(row)?,
                observation_cell(
                    row,
                    "producer supplied no reason",
                    "not_computed: no collection"
                ),
                plan_cell(row),
                row.collection.as_ref().map_or_else(
                    || "n/a".to_owned(),
                    |collection| format!("{} ({})", collection.collection_id, collection.path)
                )
            ));
        }
    }
    lines.push(String::new());
    // Present only when a plan carries a `ratchet`/`target` objective
    // (PLAT-958); a repository with none renders the bytes it did before.
    let vanished = measurements
        .map(|report| report.vanished_slices.as_slice())
        .unwrap_or_default();
    let verdicts = stage_verdict_table(rows, vanished)?;
    if !verdicts.is_empty() {
        lines.extend(verdicts);
        lines.push(String::new());
    }
    lines.extend(comparison(repository)?);
    Ok(lines)
}

/// `Profiles: …` (`portfolio.ts:159-167`).
fn profiles(repository: &PortfolioRepositoryReport) -> String {
    if repository.profiles.is_empty() {
        return "none".to_owned();
    }
    repository
        .profiles
        .iter()
        .map(|profile| format!("{} ({})", profile.id, profile.path))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `renderComparison` (`portfolio.ts:333-367`).
fn comparison(repository: &PortfolioRepositoryReport) -> Result<Vec<String>, MeasurementError> {
    let Some(comparison) = repository.comparison.as_ref() else {
        return Ok(vec![
            "Comparison: not_computed — fewer than two collections".to_owned(),
            String::new(),
        ]);
    };
    let mut lines = vec![
        format!(
            "Comparison: {} ({}) → {} ({})",
            comparison.before.id,
            comparison.before.path,
            comparison.after.id,
            comparison.after.path
        ),
        String::new(),
        "| Metric | Status | Before | After | Reasons | Plan |".to_owned(),
        "| --- | --- | ---: | ---: | --- | --- |".to_owned(),
    ];
    for observation in &comparison.observations {
        let plan = repository
            .plans
            .iter()
            .find(|candidate| candidate.metric.as_str() == observation.metric);
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            metric_label(&observation.metric, &observation.dimensions)?,
            observation.status.as_str(),
            cell(observation.before),
            cell(observation.after),
            reasons(observation),
            plan.map_or_else(
                || "not_computed: no active plan".to_owned(),
                |found| format!("{} ({})", found.id, found.path)
            )
        ));
    }
    lines.push(String::new());
    Ok(lines)
}

/// `${value ?? "n/a"}`.
fn cell(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), js_f64_string)
}

/// `reasons.map(…).join("; ") || "none"` (`portfolio.ts:362`).
fn reasons(observation: &crate::types::comparison::MeasurementComparison) -> String {
    let joined = observation
        .reasons
        .iter()
        .map(|reason| format!("{}: {}", reason.code.as_str(), reason.message))
        .collect::<Vec<_>>()
        .join("; ");
    if joined.is_empty() {
        "none".to_owned()
    } else {
        joined
    }
}
