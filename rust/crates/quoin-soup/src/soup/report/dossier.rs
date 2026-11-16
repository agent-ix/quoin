// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Dossier presentation values from the same FR-144 source (FR-197).

use super::{Listed, Report, load_report, render};
use crate::SoupError;
use crate::soup::advisory::{Resolution, Tool};
use quoin_core::runtime::RuntimeSettings;
use std::path::Path;

/// The SOUP source's rows and appended items, with its own status decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DossierSource {
    /// Rows in the source's priority order.
    pub rows: Vec<DossierRow>,
    /// Unmatched findings and scan states, exactly as the source renders them.
    pub appended: String,
}

/// One source row's presentation and source-owned gap facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DossierRow {
    /// Component, version, serves, anomalies, advisories, policy, support and end date.
    pub cells: [String; 8],
    /// Required fields whose source value is absent or unknown.
    pub not_stated: Vec<&'static str>,
    /// Each unresolved finding's source-owned description.
    pub unresolved_findings: Vec<String>,
    /// The source's past-end-of-support flag, independent of priority group.
    pub past_end_of_support: bool,
}

/// Read the FR-144 source for a dossier without reading an SBOM.
///
/// # Errors
/// Returns a source diagnostic when loading or assembling the report fails.
pub fn dossier_source(
    repo: &Path,
    records: Option<&Path>,
    binding: &Path,
    as_of: &str,
    support: Option<&Path>,
    settings: &RuntimeSettings,
) -> Result<DossierSource, SoupError> {
    let default_records = repo.join("spec/soup");
    let report = load_report(
        repo,
        records.unwrap_or(&default_records),
        &binding.display().to_string(),
        as_of,
        support,
        settings,
    )
    .map_err(SoupError::from)?;
    Ok(presentation(&report))
}

fn presentation(report: &Report) -> DossierSource {
    let osv_database = report
        .scans
        .iter()
        .find(|state| state.tool == Tool::OsvScanner)
        .and_then(|state| state.ruleset.as_deref());
    let rows = report
        .rows
        .iter()
        .map(|row| {
            let mut advisories = Tool::ALL
                .into_iter()
                .flat_map(|tool| render::tool_lines(row, tool, osv_database))
                .collect::<Vec<_>>();
            if render::has_disposition(row) {
                advisories.push(render::DISPOSITION_SCOPE.to_owned());
            }
            let mut policy = row
                .policy
                .findings
                .iter()
                .map(render::policy_finding_text)
                .collect::<Vec<_>>();
            if let Some(status) = render::policy_status_text(&row.policy.status) {
                policy.push(status);
            }
            let mut not_stated = Vec::new();
            if row
                .version
                .as_deref()
                .is_none_or(|value| matches!(value, "unknown" | "not stated"))
            {
                not_stated.push("version");
            }
            if matches!(row.serves, Listed::Absent) {
                not_stated.push("serves");
            }
            if matches!(row.anomalies, Listed::Absent) {
                not_stated.push("known_anomalies");
            }
            let support = render::resolved_text(&row.support.support_status, |status| {
                status.as_str().to_owned()
            });
            let end = render::resolved_text(&row.support.end_of_support, |date| date.to_string());
            if support == "unknown" {
                not_stated.push("support_status");
            }
            if end == "unknown" {
                not_stated.push("end_of_support");
            }
            let unresolved_findings = row
                .join
                .advisories
                .iter()
                .filter(|finding| matches!(finding.resolution, Resolution::Unresolved(_)))
                .map(|finding| format!("{} {}", finding.tool.name(), finding.rule_id))
                .chain(
                    row.policy
                        .findings
                        .iter()
                        .filter(|finding| matches!(finding.resolution, Resolution::Unresolved(_)))
                        .map(|finding| format!("{} {}", finding.tool.name(), finding.rule_id)),
                )
                .collect();
            DossierRow {
                cells: [
                    row.join.purl.clone(),
                    row.version
                        .clone()
                        .unwrap_or_else(|| render::NOT_STATED.to_owned()),
                    render::listed_text(&row.serves, Clone::clone),
                    render::listed_text(&row.anomalies, render::anomaly_text),
                    advisories.join("; "),
                    policy.join("; "),
                    support,
                    end,
                ],
                not_stated,
                unresolved_findings,
                past_end_of_support: row.support.past_end_of_support,
            }
        })
        .collect();
    DossierSource {
        rows,
        appended: render::appended(report, "###"),
    }
}
