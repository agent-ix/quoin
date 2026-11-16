// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The unresolved-anomalies document, rendering the FR-162/FR-163 source
//! report without its git-derived change fields (FR-198).

use std::fmt::Write as _;

use quoin_anomaly::{GateMode, Report as AnomalyReport, build_report};

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, line};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The unresolved-anomalies dossier generator.
pub(crate) struct AnomaliesGenerator;

impl Generator for AnomaliesGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "anomalies",
            id: "DOS-ANOM",
            title: "Unresolved anomalies",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let report =
            build_report(&inputs.scope, &[GateMode::UnjustifiedImpacting]).map_err(|error| {
                SourceFailure {
                    source: "quoin anomaly report".to_owned(),
                    detail: error.to_string(),
                }
            })?;
        if !report.unreadable.is_empty() {
            let detail = report
                .unreadable
                .iter()
                .map(|item| format!("{}: {}", item.path, item.code))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(SourceFailure {
                source: "quoin anomaly report".to_owned(),
                detail: format!("unreadable anomaly documents: {detail}"),
            });
        }
        if report.gate.is_none() {
            return Err(SourceFailure {
                source: "quoin anomaly report".to_owned(),
                detail: "the FR-164 gate result is absent".to_owned(),
            });
        }
        Ok(render(&report))
    }
}

/// Render only authored anomaly values, groups, ungrouped rows and FR-164
/// failures. Git-derived `rationaleChange` and `fixingChange` never enter the
/// dossier bytes.
fn render(report: &AnomalyReport) -> Generated {
    let mut body = String::from("# Unresolved anomalies\n\n## By hazard\n\n");
    render_groups(&mut body, "Hazard", &report.by_hazard);
    body.push_str("\n## By requirement\n\n");
    render_groups(
        &mut body,
        "Requirement or criterion",
        &report.by_requirement,
    );
    body.push_str("\n## Ungrouped\n\n");
    if report.ungrouped.is_empty() {
        body.push_str("none\n");
    } else {
        body.push_str("| Anomaly | Reason |\n| --- | --- |\n");
        for row in &report.ungrouped {
            let _ = writeln!(body, "| {} | {} |", cell(&row.anomaly), cell(row.reason));
        }
    }
    body.push_str("\n## Anomalies\n");
    if report.anomalies.is_empty() {
        body.push_str("\nnone\n");
    } else {
        for anomaly in &report.anomalies {
            let title = anomaly.title.as_deref().unwrap_or("not stated");
            let _ = write!(
                body,
                "\n### {}: {}\n\nStatus: {}\n\nResolution: {}\n\nSeverity: {}\n\nSafety impact: {}\n\n",
                line(&anomaly.id),
                line(title),
                line(&anomaly.status),
                line(anomaly.resolution.as_deref().unwrap_or("not stated")),
                line(anomaly.severity.as_deref().unwrap_or("not stated")),
                line(&anomaly.safety_impact),
            );
            render_assessment(&mut body, &anomaly.assessment);
            body.push_str("\nAffected targets:\n\n");
            render_affected(&mut body, &anomaly.affected);
            body.push_str("\nRelease rationale:\n\n");
            if anomaly.release_rationale.is_empty() {
                body.push_str("none\n");
            } else {
                body.push_str("| Author | Rationale |\n| --- | --- |\n");
                for entry in &anomaly.release_rationale {
                    let _ = writeln!(
                        body,
                        "| {} | {} |",
                        cell(&entry.author),
                        cell(&entry.rationale)
                    );
                }
            }
        }
    }

    let mut gaps = Vec::new();
    for anomaly in &report.anomalies {
        if anomaly.unassessed {
            gaps.push(Gap::new(
                GapKind::AnomalyUnassessed,
                &anomaly.id,
                "safety impact is unassessed",
            ));
        }
    }
    if let Some(gate) = &report.gate {
        for failure in &gate.failures {
            if failure.mode == GateMode::UnjustifiedImpacting {
                gaps.push(Gap::new(
                    GapKind::AnomalyNoReleaseRationale,
                    &failure.anomaly,
                    "no release rationale",
                ));
            }
        }
    }
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

fn render_groups(body: &mut String, target_heading: &str, groups: &[quoin_anomaly::report::Group]) {
    if groups.is_empty() {
        body.push_str("none\n");
        return;
    }
    let _ = writeln!(
        body,
        "| {target_heading} | Anomaly | Impact |\n| --- | --- | --- |"
    );
    for group in groups {
        for member in &group.anomalies {
            let _ = writeln!(
                body,
                "| {} | {} | {} |",
                cell(&group.target),
                cell(&member.anomaly),
                cell(&member.impact)
            );
        }
    }
}

fn render_assessment(body: &mut String, rows: &[quoin_anomaly::report::AssessmentEntry]) {
    body.push_str("| Kind | Target | Impact | Rationale |\n| --- | --- | --- | --- |\n");
    if rows.is_empty() {
        body.push_str("| none | not stated | not stated | not stated |\n");
        return;
    }
    for row in rows {
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} |",
            cell(&row.kind),
            cell(row.target.as_deref().unwrap_or("not stated")),
            cell(&row.impact),
            cell(&row.rationale)
        );
    }
}

fn render_affected(body: &mut String, rows: &[quoin_anomaly::report::Affected]) {
    if rows.is_empty() {
        body.push_str("none\n");
        return;
    }
    body.push_str("| Kind | Target | Impact |\n| --- | --- | --- |\n");
    for row in rows {
        let _ = writeln!(
            body,
            "| {} | {} | {} |",
            cell(&row.kind),
            cell(&row.target),
            cell(&row.impact)
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "the fixture contains exactly one anomaly row"
)]
mod tests {
    use super::*;

    use ix_trace_rs::trace;

    #[trace("FR-198-AC-4")]
    #[test]
    fn git_derived_change_values_do_not_change_dossier_bytes() {
        let mut report = AnomalyReport {
            anomalies: vec![quoin_anomaly::report::ReportRow {
                id: "AN-001".to_owned(),
                title: Some("Example".to_owned()),
                path: "spec/anomalies/AN-001.md".to_owned(),
                status: "open".to_owned(),
                resolution: None,
                severity: Some("minor".to_owned()),
                safety_impact: "assessed".to_owned(),
                unassessed: false,
                assessment: Vec::new(),
                affected: Vec::new(),
                release_rationale: Vec::new(),
                fixing_change: quoin_anomaly::Derived::Absent,
                rationale_change: quoin_anomaly::Derived::Absent,
            }],
            by_hazard: Vec::new(),
            by_requirement: Vec::new(),
            ungrouped: Vec::new(),
            unreadable: Vec::new(),
            gate: Some(quoin_anomaly::Gate {
                modes: vec![GateMode::UnjustifiedImpacting],
                failures: Vec::new(),
            }),
        };
        let before = render(&report);
        report.anomalies[0].fixing_change = quoin_anomaly::Derived::Pending;
        report.anomalies[0].rationale_change = quoin_anomaly::Derived::NotApplicable;
        assert_eq!(render(&report), before);
        assert!(!before.body.contains("rationaleChange"));
        assert!(!before.body.contains("fixingChange"));
    }
}
