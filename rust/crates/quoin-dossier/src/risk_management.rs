// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The risk-management document, rendering the FR-169/FR-170 chain and FR-145
//! decision queue without deriving risk values or decisions (FR-199).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use quoin_risk_advice::{Advice, Row, RowToken, Summary};

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, line};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The risk-management dossier generator.
pub(crate) struct RiskManagementGenerator;

impl Generator for RiskManagementGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "risk-management",
            id: "DOS-RISK",
            title: "Risk management document",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let modules = inputs
            .modules
            .iter()
            .map(quoin_quire::ids::ModuleRoot::open)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SourceFailure {
                source: "modules".to_owned(),
                detail: format!("failed to open module: {error}"),
            })?;
        let selection = if modules.is_empty() {
            quoin_quire::ModuleSelection::Ambient
        } else {
            quoin_quire::ModuleSelection::Closed(modules)
        };
        let summary = quoin_risk_advice::summarize(&inputs.scope, &selection).map_err(|error| {
            SourceFailure {
                source: "quoin risk-summary".to_owned(),
                detail: error.message,
            }
        })?;
        let advice = quoin_risk_advice::advise(&inputs.scope, &selection).map_err(|error| {
            SourceFailure {
                source: "quoin risk advise".to_owned(),
                detail: error.message,
            }
        })?;
        let unreadable = summary
            .unreadable
            .iter()
            .chain(&advice.unreadable)
            .collect::<std::collections::BTreeSet<_>>();
        if !unreadable.is_empty() {
            return Err(SourceFailure {
                source: "risk records".to_owned(),
                detail: unreadable
                    .into_iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("; "),
            });
        }
        Ok(render(&summary, &advice))
    }
}

fn render(summary: &Summary, advice: &Advice) -> Generated {
    let mut body = String::from("# Risk management document\n\n");
    let policy_findings: BTreeMap<&str, &quoin_risk_advice::PolicyCell> = advice
        .policy_cells
        .iter()
        .map(|cell| (cell.estimate.as_str(), cell))
        .collect();
    let mut current_hazard: Option<Option<&str>> = None;
    let mut gaps = Vec::new();

    for (index, row) in summary.rows.iter().enumerate() {
        let hazard = row.hazard.as_deref();
        if current_hazard != Some(hazard) {
            match hazard {
                Some(id) => {
                    let _ = writeln!(body, "## {}\n", line(id));
                }
                None => body.push_str("## Hazard not stated\n\n"),
            }
            current_hazard = Some(hazard);
        }
        render_row(&mut body, &mut gaps, row, &policy_findings, index);
    }
    render_decision_queue(&mut body, advice);
    if advice.policy_cells.iter().any(|cell| {
        cell.findings
            .iter()
            .any(|finding| finding.finding == quoin_risk_advice::Token::PolicyNotFound)
    }) {
        gaps.push(Gap::new(
            GapKind::NoAcceptabilityPolicy,
            "this document",
            "the source report found no policy for one or more risk estimates",
        ));
    }
    Generated { body, gaps }
}

fn render_row(
    body: &mut String,
    gaps: &mut Vec<Gap>,
    row: &Row,
    policy_findings: &BTreeMap<&str, &quoin_risk_advice::PolicyCell>,
    index: usize,
) {
    if let Some(situation) = &row.situation {
        let _ = writeln!(body, "### Situation {}\n", line(situation));
    } else if row.hazard.is_none() {
        body.push_str("### Situation not stated\n\n");
    }
    if !row.harms.is_empty() {
        let _ = writeln!(
            body,
            "Harms: {}\n",
            row.harms
                .iter()
                .map(|id| line(id))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    render_estimates(
        body,
        "Pre-control estimates",
        &row.pre_control,
        policy_findings,
    );
    render_controls(body, &row.controls);
    render_estimates(
        body,
        "Post-control estimates",
        &row.post_control,
        policy_findings,
    );
    if !row.residual_risks.is_empty() {
        body.push_str("| Residual risk | Decision | Decider |\n| --- | --- | --- |\n");
        for risk in &row.residual_risks {
            let decider = risk.decided_by.as_deref().unwrap_or("not stated");
            let _ = writeln!(
                body,
                "| {} | {} | {} |",
                cell(&risk.residual_risk),
                cell(&risk.decision),
                cell(decider)
            );
        }
        body.push('\n');
    }
    if !row.findings.is_empty() {
        body.push_str("Source findings:\n\n");
        for finding in &row.findings {
            let names = finding.names.join(", ");
            let _ = writeln!(
                body,
                "- `{}`{}",
                row_token(finding.finding),
                if names.is_empty() {
                    String::new()
                } else {
                    format!(": {}", line(&names))
                }
            );
            let subject = finding.names.first().cloned().unwrap_or_else(|| {
                row.hazard
                    .clone()
                    .or_else(|| row.situation.clone())
                    .unwrap_or_else(|| format!("row {}", index + 1))
            });
            let gap = match finding.finding {
                RowToken::ControlWithoutVerification => Some((
                    GapKind::ControlUnverified,
                    "the control has no verification",
                )),
                RowToken::ResidualRiskUndecided => Some((
                    GapKind::ResidualRiskUndecided,
                    "the residual risk has no recorded decision or decider",
                )),
                _ => Some((
                    GapKind::RiskChainIncomplete,
                    "the source reports an incomplete risk chain",
                )),
            };
            if let Some((kind, detail)) = gap {
                let gap = Gap::new(kind, subject, detail);
                gaps.push(if let Some(hazard) = row.hazard.as_deref() {
                    gap.for_element(hazard)
                } else {
                    gap
                });
            }
        }
        body.push('\n');
    }
}

fn render_estimates(
    body: &mut String,
    heading: &str,
    estimates: &[quoin_risk_advice::EstimateEntry],
    policy_findings: &BTreeMap<&str, &quoin_risk_advice::PolicyCell>,
) {
    if estimates.is_empty() {
        return;
    }
    let _ = writeln!(body, "{heading}:\n");
    body.push_str("| Estimate | Phase | Severity | Probability | Region | Policy findings |\n| --- | --- | --- | --- | --- | --- |\n");
    let phase = if heading.starts_with("Pre-") {
        "pre_control"
    } else {
        "post_control"
    };
    for estimate in estimates {
        let cell = policy_findings.get(estimate.estimate.as_str()).copied();
        let findings = cell.map_or_else(String::new, |cell| {
            cell.findings
                .iter()
                .map(|finding| finding_name(finding.finding))
                .collect::<Vec<_>>()
                .join(", ")
        });
        let region = estimate.policy_region.as_deref().unwrap_or("not stated");
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} | {} | {} |",
            cell_text(&estimate.estimate),
            phase,
            cell_text(&estimate.severity),
            cell_text(&estimate.probability),
            cell_text(region),
            cell_text(&findings)
        );
    }
    body.push('\n');
}

fn render_controls(body: &mut String, controls: &[quoin_risk_advice::ControlEntry]) {
    if controls.is_empty() {
        return;
    }
    body.push_str("| Control | Implementing requirements | Verifications |\n| --- | --- | --- |\n");
    for control in controls {
        let _ = writeln!(
            body,
            "| {} | {} | {} |",
            cell_text(&control.control),
            list_cell(&control.requirements),
            list_cell(&control.verifications)
        );
    }
    body.push('\n');
}

fn render_decision_queue(body: &mut String, advice: &Advice) {
    body.push_str("## Undecided decisions\n\n");
    let queue = &advice.decision_queue;
    if queue.queue.is_empty() && queue.no_record.is_empty() {
        body.push_str("none\n");
        return;
    }
    if !queue.queue.is_empty() {
        body.push_str("| Record | Kind | Estimate | Region | References | Findings |\n| --- | --- | --- | --- | --- | --- |\n");
        for entry in &queue.queue {
            let findings = entry
                .findings
                .iter()
                .map(|finding| finding_name(finding.finding))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                body,
                "| {} | {} | {} | {} | {} | {} |",
                cell_text(&entry.record),
                cell_text(entry.kind),
                entry.estimate.as_deref().map(cell_text).unwrap_or_default(),
                entry
                    .region
                    .as_deref()
                    .map_or_else(|| "not stated".to_owned(), cell_text),
                list_cell(&entry.references),
                cell_text(&findings)
            );
        }
        body.push('\n');
    }
    if !queue.no_record.is_empty() {
        body.push_str("Estimates without residual-risk records:\n\n");
        for entry in &queue.no_record {
            let _ = writeln!(
                body,
                "- {}: {}",
                line(&entry.estimate),
                entry
                    .findings
                    .iter()
                    .map(|finding| finding_name(finding.finding))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

fn row_token(token: RowToken) -> &'static str {
    match token {
        RowToken::HazardWithoutSituation => "hazard-without-situation",
        RowToken::SituationWithoutHazard => "situation-without-hazard",
        RowToken::SituationWithoutHarm => "situation-without-harm",
        RowToken::ControlWithoutRequirement => "control-without-requirement",
        RowToken::ControlWithoutVerification => "control-without-verification",
        RowToken::PostControlWithoutControl => "post-control-without-control",
        RowToken::PostControlEstimateWithoutDecision => "post-control-estimate-without-decision",
        RowToken::ResidualRiskUndecided => "residual-risk-undecided",
        RowToken::DecisionWithoutRegion => "decision-without-region",
        RowToken::AcceptedOutsideAcceptableRegion => "accepted-outside-acceptable-region",
        RowToken::BenefitRiskUndecided => "benefit-risk-undecided",
        RowToken::BenefitRiskNotAccepted => "benefit-risk-not-accepted",
        RowToken::NotAcceptedInsideAcceptableRegion => "not-accepted-inside-acceptable-region",
    }
}

fn finding_name(token: quoin_risk_advice::Token) -> &'static str {
    match token {
        quoin_risk_advice::Token::LevelNotInPolicy => "level-not-in-policy",
        quoin_risk_advice::Token::LevelNotAssessed => "level-not-assessed",
        quoin_risk_advice::Token::CellNotInPolicy => "cell-not-in-policy",
        quoin_risk_advice::Token::PolicyNotFound => "policy-not-found",
        quoin_risk_advice::Token::PolicyAmbiguous => "policy-ambiguous",
        quoin_risk_advice::Token::PostControlWithoutControl => "post-control-without-control",
        quoin_risk_advice::Token::ControlWithoutRequirement => "control-without-requirement",
        quoin_risk_advice::Token::ControlWithoutVerification => "control-without-verification",
        quoin_risk_advice::Token::EstimateWithoutSituation => "estimate-without-situation",
        quoin_risk_advice::Token::SituationWithoutHazard => "situation-without-hazard",
        quoin_risk_advice::Token::PostControlEstimateWithoutDecision => {
            "post-control-estimate-without-decision"
        }
    }
}

fn list_cell(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values
            .iter()
            .map(|value| cell_text(value))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn cell_text(value: &str) -> String {
    cell(value)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quoin_risk_advice::{
        ControlEntry, EstimateEntry, NoRecord, Queue, QueueEntry, ResidualEntry, RowFinding,
        RowToken,
    };

    fn advice() -> Advice {
        Advice {
            attribution: quoin_risk_advice::ATTRIBUTION,
            policy_cells: Vec::new(),
            evidence_gaps: Vec::new(),
            decision_queue: Queue {
                queue: Vec::<QueueEntry>::new(),
                no_record: Vec::<NoRecord>::new(),
            },
            unreadable: Vec::new(),
        }
    }

    fn complete_row() -> Row {
        Row {
            hazard: Some("HAZ-1".to_owned()),
            situation: Some("SIT-1".to_owned()),
            harms: vec!["HARM-1".to_owned()],
            pre_control: vec![EstimateEntry {
                estimate: "EST-PRE".to_owned(),
                severity: "serious".to_owned(),
                probability: "likely".to_owned(),
                policy_region: Some("high".to_owned()),
            }],
            controls: vec![ControlEntry {
                control: "CTRL-1".to_owned(),
                requirements: vec!["FR-9".to_owned()],
                verifications: vec!["TC-9".to_owned()],
            }],
            post_control: vec![EstimateEntry {
                estimate: "EST-POST".to_owned(),
                severity: "minor".to_owned(),
                probability: "unlikely".to_owned(),
                policy_region: Some("low".to_owned()),
            }],
            residual_risks: vec![ResidualEntry {
                residual_risk: "RR-1".to_owned(),
                decision: "accepted".to_owned(),
                decided_by: Some("A. Person".to_owned()),
            }],
            findings: Vec::new(),
        }
    }

    #[trace("FR-199-AC-1", "FR-199-AC-7")]
    #[test]
    fn complete_chain_renders_source_values_deterministically_without_derived_values() {
        let summary = Summary {
            attribution: quoin_risk_advice::ATTRIBUTION,
            rows: vec![complete_row()],
            unreadable: Vec::new(),
        };
        let advice = advice();
        let first = render(&summary, &advice);
        let second = render(&summary, &advice);
        assert_eq!(first, second);
        assert!(first.body.contains("## HAZ-1"));
        assert!(
            first
                .body
                .contains("| EST-PRE | pre_control | serious | likely | high |  |")
        );
        assert!(first.body.contains("| CTRL-1 | FR-9 | TC-9 |"));
        assert!(first.body.contains("| RR-1 | accepted | A. Person |"));
        assert!(first.gaps.is_empty());
        assert!(!first.body.to_ascii_lowercase().contains("score"));
        assert!(!first.body.to_ascii_lowercase().contains("rank"));
    }

    #[trace("FR-199-AC-2", "FR-199-AC-3", "FR-199-AC-4")]
    #[test]
    fn row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit() {
        let mut row = complete_row();
        row.controls[0].verifications.clear();
        row.residual_risks[0].decided_by = None;
        row.findings = vec![
            RowFinding {
                finding: RowToken::ControlWithoutVerification,
                names: vec!["CTRL-1".to_owned()],
            },
            RowFinding {
                finding: RowToken::ResidualRiskUndecided,
                names: vec!["RR-1".to_owned()],
            },
            RowFinding {
                finding: RowToken::HazardWithoutSituation,
                names: vec!["HAZ-1".to_owned()],
            },
        ];
        let rendered = render(
            &Summary {
                attribution: quoin_risk_advice::ATTRIBUTION,
                rows: vec![row],
                unreadable: Vec::new(),
            },
            &advice(),
        );
        assert!(rendered.body.contains("| RR-1 | accepted | not stated |"));
        assert!(
            rendered
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::ControlUnverified && gap.subject == "CTRL-1")
        );
        assert!(
            rendered
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::ResidualRiskUndecided && gap.subject == "RR-1")
        );
        assert!(
            rendered
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::RiskChainIncomplete && gap.subject == "HAZ-1")
        );
    }

    #[trace("FR-199-AC-5", "FR-199-AC-6")]
    #[test]
    fn policy_findings_and_undecided_entries_are_rendered_from_advice() {
        let mut row = complete_row();
        row.pre_control[0].policy_region = None;
        let mut advice = advice();
        advice.policy_cells.push(quoin_risk_advice::PolicyCell {
            estimate: "EST-PRE".to_owned(),
            phase: "pre_control",
            severity: "serious".to_owned(),
            probability: "likely".to_owned(),
            policy: None,
            cell: None,
            region: None,
            outside_acceptable: None,
            findings: vec![quoin_risk_advice::Finding {
                finding: quoin_risk_advice::Token::PolicyNotFound,
                names: Vec::new(),
            }],
        });
        advice.decision_queue.queue.push(QueueEntry {
            record: "BR-1".to_owned(),
            kind: "benefit_risk",
            estimate: None,
            policy: None,
            severity: None,
            probability: None,
            region: None,
            outside_acceptable: None,
            references: vec!["RR-1".to_owned()],
            findings: Vec::new(),
        });
        let rendered = render(
            &Summary {
                attribution: quoin_risk_advice::ATTRIBUTION,
                rows: vec![row],
                unreadable: Vec::new(),
            },
            &advice,
        );
        assert!(rendered.body.contains(
            "| EST-PRE | pre_control | serious | likely | not stated | policy-not-found |"
        ));
        assert!(rendered.body.contains("| BR-1 | benefit_risk |"));
        assert_eq!(
            rendered
                .gaps
                .iter()
                .filter(|gap| gap.gap_kind == GapKind::NoAcceptabilityPolicy)
                .count(),
            1
        );
    }
}
