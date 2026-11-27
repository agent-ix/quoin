// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The risk-management document, rendering the FR-169/FR-170 chain and FR-145
//! decision queue without deriving risk values or decisions (FR-199).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use quoin_anomaly::{Derived, Signature};
use quoin_risk_advice::{Advice, HarmText, Row, RowToken, Summary, Token};

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, line, prose};
use crate::order::dossier_cmp;
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
                    render_authored(
                        &mut body,
                        row.hazard_title.as_deref(),
                        "Condition",
                        row.hazard_text.as_deref(),
                    );
                }
                None => body.push_str("## Hazard not stated\n\n"),
            }
            current_hazard = Some(hazard);
        }
        render_row(&mut body, &mut gaps, row, &policy_findings, index);
    }
    render_benefit_risks(&mut body, summary);
    render_evaluations(&mut body, summary, advice);
    render_decision_queue(&mut body, advice);
    render_attribution(&mut body, advice);
    if summary.overall_evaluations.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoOverallEvaluation,
            "this document",
            "the corpus holds no overall residual risk evaluation record",
        ));
    }
    for evaluation in &summary.overall_evaluations {
        if evaluation.decision == "undecided" {
            gaps.push(
                Gap::new(
                    GapKind::OverallEvaluationUndecided,
                    evaluation.evaluation.as_str(),
                    "the overall residual risk evaluation has no recorded decision",
                )
                .for_element(evaluation.evaluation.as_str()),
            );
        }
    }
    gaps.extend(decision_gaps(advice));
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
    Generated {
        body,
        document_version: None,
        gaps,
    }
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
        render_authored(
            body,
            row.situation_title.as_deref(),
            "Description",
            row.situation_text.as_deref(),
        );
    } else if row.hazard.is_none() {
        body.push_str("### Situation not stated\n\n");
    }
    for harm in &row.harm_texts {
        render_harm(body, harm);
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
        body.push_str(
            "| Residual risk | Decision | Decider | Rationale |\n| --- | --- | --- | --- |\n",
        );
        for risk in &row.residual_risks {
            let _ = writeln!(
                body,
                "| {} | {} | {} | {} |",
                cell(&risk.residual_risk),
                cell(&risk.decision),
                cell(risk.decided_by.as_deref().unwrap_or(NOT_STATED)),
                cell(risk.rationale.as_deref().unwrap_or(NOT_STATED))
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

const NOT_STATED: &str = "not stated";

/// An authored title and text, shown as the record wrote them.
fn render_authored(body: &mut String, title: Option<&str>, label: &str, text: Option<&str>) {
    let _ = writeln!(
        body,
        "Title: {}\n",
        title.map_or_else(|| NOT_STATED.to_owned(), line)
    );
    let _ = writeln!(body, "{label}:\n");
    let _ = writeln!(
        body,
        "{}\n",
        text.map_or_else(|| NOT_STATED.to_owned(), prose)
    );
}

fn render_harm(body: &mut String, harm: &HarmText) {
    let _ = writeln!(body, "#### Harm {}\n", line(&harm.harm));
    render_authored(
        body,
        harm.title.as_deref(),
        "Description",
        harm.text.as_deref(),
    );
}

fn render_benefit_risks(body: &mut String, summary: &Summary) {
    body.push_str("## Benefit-risk records\n\n");
    if summary.benefit_risks.is_empty() {
        body.push_str("none\n\n");
        return;
    }
    let mut records: Vec<_> = summary.benefit_risks.iter().collect();
    records.sort_by(|a, b| dossier_cmp(&a.benefit_risk, &b.benefit_risk));
    body.push_str("| Record | Benefit | Decision | Decider | Rationale | Residual risks |\n| --- | --- | --- | --- | --- | --- |\n");
    for record in records {
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} | {} | {} |",
            cell_text(&record.benefit_risk),
            cell_text(record.benefit.as_deref().unwrap_or(NOT_STATED)),
            cell_text(&record.decision),
            cell_text(record.decided_by.as_deref().unwrap_or(NOT_STATED)),
            cell_text(record.rationale.as_deref().unwrap_or(NOT_STATED)),
            list_cell(&record.residual_risks)
        );
    }
    body.push('\n');
}

fn render_evaluations(body: &mut String, summary: &Summary, advice: &Advice) {
    body.push_str("## Overall residual risk evaluation\n\n");
    if summary.overall_evaluations.is_empty() {
        body.push_str("none\n\n");
        return;
    }
    let mut records: Vec<_> = summary.overall_evaluations.iter().collect();
    records.sort_by(|a, b| dossier_cmp(&a.evaluation, &b.evaluation));
    body.push_str("| Record | Product | Decision | Decider | Rationale | References |\n| --- | --- | --- | --- | --- | --- |\n");
    for record in records {
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} | {} | {} |",
            cell_text(&record.evaluation),
            cell_text(record.product.as_deref().unwrap_or(NOT_STATED)),
            cell_text(&record.decision),
            cell_text(record.decided_by.as_deref().unwrap_or(NOT_STATED)),
            cell_text(record.rationale.as_deref().unwrap_or(NOT_STATED)),
            list_cell(&record.references)
        );
    }
    body.push('\n');
    for check in &advice.overall_evaluation_checks {
        for finding in &check.findings {
            let _ = writeln!(
                body,
                "- {}: `{}`: {}",
                line(&check.evaluation),
                finding_name(finding.finding),
                line(&finding.names.join(", "))
            );
        }
    }
    if advice
        .overall_evaluation_checks
        .iter()
        .any(|check| !check.findings.is_empty())
    {
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
    body.push_str("| Control | Implementing requirements | Retired requirements | Verifications |\n| --- | --- | --- | --- |\n");
    for control in controls {
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} |",
            cell_text(&control.control),
            list_cell(&control.requirements),
            list_cell(&control.retired_requirements),
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
                entry.estimate.as_deref().map_or_default(cell_text),
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

/// The gaps of the FR-213 and FR-214 source findings (FR-199).
fn decision_gaps(advice: &Advice) -> Vec<Gap> {
    let mut gaps = Vec::new();
    for check in &advice.overall_evaluation_checks {
        for finding in &check.findings {
            let (kind, detail) = match finding.finding {
                Token::ResidualRiskNotConsidered => (
                    GapKind::ResidualRiskNotConsidered,
                    "the overall residual risk evaluation does not consider residual risks",
                ),
                Token::EvaluationAcceptedWithOpenRisk => (
                    GapKind::EvaluationAcceptedWithOpenRisk,
                    "the overall residual risk evaluation is decided over considered residual risks that are not decided as it is",
                ),
                _ => continue,
            };
            gaps.push(
                Gap::new(
                    kind,
                    check.evaluation.as_str(),
                    format!("{detail}: {}", finding.names.join(", ")),
                )
                .for_element(check.evaluation.as_str()),
            );
        }
    }
    for entry in &advice.decision_attribution {
        for finding in &entry.findings {
            let (kind, detail) = match finding.finding {
                Token::DecidedByDiffersFromCommitAuthor => (
                    GapKind::DecisionSignerMismatch,
                    "the typed decider differs from the author of the commit that set the decision",
                ),
                Token::DecidedByDiffersFromVerifiedSigner => (
                    GapKind::DecisionSignerMismatch,
                    "the typed decider differs from the verified signer of the commit that set the decision",
                ),
                Token::DecisionCommitNotSigned => (
                    GapKind::DecisionCommitUnsigned,
                    "the commit that set the decision carries no valid signature",
                ),
                _ => continue,
            };
            gaps.push(
                Gap::new(kind, entry.record.as_str(), detail).for_element(entry.record.as_str()),
            );
        }
    }
    gaps
}

fn render_attribution(body: &mut String, advice: &Advice) {
    body.push_str("\n## Decision attribution\n\n");
    if advice.decision_attribution.is_empty() {
        body.push_str("none\n");
        return;
    }
    let mut entries: Vec<_> = advice.decision_attribution.iter().collect();
    entries.sort_by(|a, b| dossier_cmp(&a.record, &b.record));
    body.push_str("| Record | Kind | Decision | Decider | Commit author | Signature | Findings |\n| --- | --- | --- | --- | --- | --- | --- |\n");
    for entry in entries {
        let (author, signature) = match &entry.decision_change {
            Derived::Present(change) => (
                change.author.as_str(),
                match change.signature {
                    Signature::Signed => "signed",
                    Signature::Unsigned => "unsigned",
                    Signature::Unknown => "unknown",
                },
            ),
            _ => (NOT_STATED, NOT_STATED),
        };
        let findings = entry
            .findings
            .iter()
            .map(|finding| finding_name(finding.finding))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} | {} | {} | {} |",
            cell_text(&entry.record),
            cell_text(entry.kind),
            cell_text(&entry.decision),
            cell_text(entry.decided_by.as_deref().unwrap_or(NOT_STATED)),
            cell_text(author),
            cell_text(signature),
            cell_text(&findings)
        );
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
        quoin_risk_advice::Token::ResidualRiskNotConsidered => "residual-risk-not-considered",
        quoin_risk_advice::Token::EvaluationAcceptedWithOpenRisk => {
            "evaluation-accepted-with-open-risk"
        }
        quoin_risk_advice::Token::DecidedByDiffersFromCommitAuthor => {
            "decided-by-differs-from-commit-author"
        }
        quoin_risk_advice::Token::DecidedByDiffersFromVerifiedSigner => {
            "decided-by-differs-from-verified-signer"
        }
        quoin_risk_advice::Token::DecisionCommitNotSigned => "decision-commit-not-signed",
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
        ControlEntry, EstimateEntry, EvaluationEntry, NoRecord, Queue, QueueEntry, ResidualEntry,
        RowFinding, RowToken,
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
            overall_evaluation_checks: Vec::new(),
            decision_attribution: Vec::new(),
            unreadable: Vec::new(),
        }
    }

    fn decided_evaluation() -> EvaluationEntry {
        EvaluationEntry {
            evaluation: "ORE-1".to_owned(),
            product: Some("Pump".to_owned()),
            decision: "accepted".to_owned(),
            decided_by: Some("A. Person".to_owned()),
            rationale: None,
            references: vec!["RR-1".to_owned()],
        }
    }

    fn complete_row() -> Row {
        Row {
            hazard: Some("HAZ-1".to_owned()),
            hazard_title: Some("Overdose".to_owned()),
            hazard_text: Some("The pump delivers too much.".to_owned()),
            situation: Some("SIT-1".to_owned()),
            situation_title: None,
            situation_text: None,
            harms: vec!["HARM-1".to_owned()],
            harm_texts: vec![HarmText {
                harm: "HARM-1".to_owned(),
                title: Some("Tissue damage".to_owned()),
                text: None,
            }],
            pre_control: vec![EstimateEntry {
                estimate: "EST-PRE".to_owned(),
                severity: "serious".to_owned(),
                probability: "likely".to_owned(),
                policy_region: Some("high".to_owned()),
            }],
            controls: vec![ControlEntry {
                control: "CTRL-1".to_owned(),
                requirements: vec!["FR-9".to_owned()],
                retired_requirements: Vec::new(),
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
                rationale: Some("Benefit outweighs the risk.".to_owned()),
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
            benefit_risks: Vec::new(),
            overall_evaluations: vec![decided_evaluation()],
            unreadable: Vec::new(),
            unusable: Vec::new(),
        };
        let advice = advice();
        let first = render(&summary, &advice);
        let second = render(&summary, &advice);
        assert_eq!(first, second);
        assert!(first.body.contains("## HAZ-1"));
        assert!(first.body.contains("#### Harm HARM-1"));
        assert!(
            first
                .body
                .contains("| EST-PRE | pre_control | serious | likely | high |  |")
        );
        assert!(
            first
                .body
                .contains("| EST-POST | post_control | minor | unlikely | low |  |")
        );
        assert!(first.body.contains("| CTRL-1 | FR-9 | none | TC-9 |"));
        assert!(
            first
                .body
                .contains("| RR-1 | accepted | A. Person | Benefit outweighs the risk. |")
        );
        assert_eq!(first.gaps, [] as [crate::gap::Gap; 0]);
        assert!(!first.body.to_ascii_lowercase().contains("score"));
        assert!(!first.body.to_ascii_lowercase().contains("rank"));
    }

    #[trace("FR-199-AC-1", "FR-199-AC-17")]
    #[test]
    fn retired_requirements_render_only_for_the_control_that_has_them() {
        let mut row = complete_row();
        row.controls[0].retired_requirements = vec!["FR-9".to_owned()];
        row.controls.push(ControlEntry {
            control: "CTRL-2".to_owned(),
            requirements: vec!["FR-10".to_owned()],
            retired_requirements: Vec::new(),
            verifications: vec!["TC-10".to_owned()],
        });
        let summary = Summary {
            attribution: quoin_risk_advice::ATTRIBUTION,
            rows: vec![row],
            benefit_risks: Vec::new(),
            overall_evaluations: vec![decided_evaluation()],
            unreadable: Vec::new(),
            unusable: Vec::new(),
        };
        let out = render(&summary, &advice());
        assert!(out.body.contains("| CTRL-1 | FR-9 | FR-9 | TC-9 |"));
        assert!(out.body.contains("| CTRL-2 | FR-10 | none | TC-10 |"));
    }

    fn summary_with_evaluation() -> Summary {
        Summary {
            attribution: quoin_risk_advice::ATTRIBUTION,
            rows: vec![complete_row()],
            benefit_risks: Vec::new(),
            overall_evaluations: vec![decided_evaluation()],
            unreadable: Vec::new(),
            unusable: Vec::new(),
        }
    }

    fn finding(token: Token, names: &[&str]) -> quoin_risk_advice::Finding {
        quoin_risk_advice::Finding {
            finding: token,
            names: names.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    #[trace("FR-199-AC-18")]
    #[test]
    fn overall_evaluation_findings_become_their_own_gaps_and_a_clean_one_none() {
        let mut advice = advice();
        let clean = render(&summary_with_evaluation(), &advice);
        assert_eq!(clean.gaps, [] as [crate::gap::Gap; 0]);
        advice.overall_evaluation_checks = vec![quoin_risk_advice::OverallCheck {
            evaluation: "ORE-1".to_owned(),
            product: Some("Pump".to_owned()),
            decision: "accepted".to_owned(),
            findings: vec![
                finding(Token::ResidualRiskNotConsidered, &["RR-2"]),
                finding(Token::EvaluationAcceptedWithOpenRisk, &["RR-1"]),
            ],
        }];
        let out = render(&summary_with_evaluation(), &advice);
        let kinds: Vec<_> = out.gaps.iter().map(|gap| gap.gap_kind).collect();
        assert_eq!(
            kinds,
            [
                GapKind::ResidualRiskNotConsidered,
                GapKind::EvaluationAcceptedWithOpenRisk
            ]
        );
        assert_eq!(out.gaps[0].subject, "ORE-1");
        assert!(out.gaps[0].detail.contains("RR-2"));
        assert!(out.gaps[1].detail.contains("RR-1"));
        assert!(
            out.body
                .contains("- ORE-1: `evaluation-accepted-with-open-risk`: RR-1")
        );
    }

    fn attribution_entry(
        findings: Vec<quoin_risk_advice::Finding>,
        change: Derived,
    ) -> quoin_risk_advice::Attribution {
        quoin_risk_advice::Attribution {
            record: "RR-1".to_owned(),
            kind: "residual_risk",
            decision: "accepted".to_owned(),
            decided_by: Some("Lee".to_owned()),
            decision_change: change,
            signer_principal: None,
            findings,
        }
    }

    fn present(signature: Signature) -> Derived {
        Derived::Present(quoin_anomaly::Change {
            commit: "abc123".to_owned(),
            author: "Pat".to_owned(),
            authored_at: "2026-01-01T00:00:00+00:00".to_owned(),
            signature,
            signature_code: 'N',
        })
    }

    #[trace("FR-199-AC-19", "FR-199-AC-20")]
    #[test]
    fn decision_attribution_renders_signer_columns_and_maps_findings_to_gaps() {
        let mut advice = advice();
        let none = render(&summary_with_evaluation(), &advice);
        assert!(none.body.contains("## Decision attribution\n\nnone\n"));
        advice.decision_attribution = vec![
            attribution_entry(
                vec![
                    finding(Token::DecidedByDiffersFromCommitAuthor, &["Lee", "Pat"]),
                    finding(Token::DecisionCommitNotSigned, &["abc123"]),
                ],
                present(Signature::Unsigned),
            ),
            quoin_risk_advice::Attribution {
                record: "BR-1".to_owned(),
                ..attribution_entry(Vec::new(), Derived::Pending)
            },
            quoin_risk_advice::Attribution {
                record: "ORE-1".to_owned(),
                findings: vec![finding(
                    Token::DecidedByDiffersFromVerifiedSigner,
                    &["Lee", "Morgan"],
                )],
                ..attribution_entry(Vec::new(), present(Signature::Signed))
            },
            quoin_risk_advice::Attribution {
                record: "RR-2".to_owned(),
                findings: Vec::new(),
                ..attribution_entry(Vec::new(), present(Signature::Signed))
            },
        ];
        let out = render(&summary_with_evaluation(), &advice);
        assert!(out.body.contains(
            "| RR-1 | residual_risk | accepted | Lee | Pat | unsigned | decided-by-differs-from-commit-author, decision-commit-not-signed |"
        ));
        assert!(
            out.body
                .contains("| BR-1 | residual_risk | accepted | Lee | not stated | not stated |  |")
        );
        assert!(out.body.contains(
            "| ORE-1 | residual_risk | accepted | Lee | Pat | signed | decided-by-differs-from-verified-signer |"
        ));
        let kinds: Vec<_> = out.gaps.iter().map(|gap| gap.gap_kind).collect();
        assert_eq!(
            kinds,
            [
                GapKind::DecisionSignerMismatch,
                GapKind::DecisionCommitUnsigned,
                GapKind::DecisionSignerMismatch
            ]
        );
        assert_eq!(out.gaps[0].subject, "RR-1");
        assert_eq!(out.gaps[1].subject, "RR-1");
        assert_eq!(out.gaps[2].subject, "ORE-1");
        assert!(!out.gaps.iter().any(|gap| gap.subject == "RR-2"));
        let lower = out.body.to_ascii_lowercase();
        assert!(!lower.contains("approv"));
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
        ];
        let mut hazard_without_situation = complete_row();
        hazard_without_situation.hazard = Some("HAZ-EMPTY".to_owned());
        hazard_without_situation.situation = None;
        hazard_without_situation.harms.clear();
        hazard_without_situation.pre_control.clear();
        hazard_without_situation.controls.clear();
        hazard_without_situation.post_control.clear();
        hazard_without_situation.residual_risks.clear();
        hazard_without_situation.findings = vec![RowFinding {
            finding: RowToken::HazardWithoutSituation,
            names: vec!["HAZ-EMPTY".to_owned()],
        }];
        let rendered = render(
            &Summary {
                attribution: quoin_risk_advice::ATTRIBUTION,
                rows: vec![row, hazard_without_situation],
                benefit_risks: Vec::new(),
                overall_evaluations: vec![decided_evaluation()],
                unreadable: Vec::new(),
                unusable: Vec::new(),
            },
            &advice(),
        );
        assert!(
            rendered
                .body
                .contains("| RR-1 | accepted | not stated | Benefit outweighs the risk. |")
        );
        assert!(rendered.body.contains("## HAZ-EMPTY"));
        assert!(
            rendered
                .body
                .contains("`hazard-without-situation`: HAZ-EMPTY")
        );
        assert!(
            rendered
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::ControlUnverified && gap.subject == "CTRL-1")
        );
        assert_eq!(
            rendered
                .gaps
                .iter()
                .filter(|gap| gap.subject == "CTRL-1")
                .count(),
            1
        );
        assert!(
            rendered
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::ResidualRiskUndecided && gap.subject == "RR-1")
        );
        assert_eq!(
            rendered
                .gaps
                .iter()
                .filter(|gap| gap.subject == "RR-1")
                .count(),
            1
        );
        assert!(rendered.gaps.iter().any(|gap| {
            gap.gap_kind == GapKind::RiskChainIncomplete && gap.subject == "HAZ-EMPTY"
        }));
    }

    #[trace("FR-199-AC-6")]
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
                benefit_risks: Vec::new(),
                overall_evaluations: vec![decided_evaluation()],
                unreadable: Vec::new(),
                unusable: Vec::new(),
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

    #[trace("FR-199-AC-9", "FR-199-AC-10", "FR-199-AC-11", "FR-199-AC-12")]
    #[test]
    fn every_absent_authored_value_reads_not_stated() {
        let mut row = complete_row();
        row.hazard_title = None;
        row.hazard_text = None;
        row.harm_texts[0].title = None;
        row.residual_risks[0].rationale = None;
        row.residual_risks[0].decided_by = None;
        let mut evaluation = decided_evaluation();
        evaluation.product = None;
        evaluation.decided_by = None;
        evaluation.rationale = None;
        let summary = Summary {
            attribution: quoin_risk_advice::ATTRIBUTION,
            rows: vec![row],
            benefit_risks: vec![quoin_risk_advice::BenefitRiskEntry {
                benefit_risk: "BR-1".to_owned(),
                benefit: None,
                decision: "undecided".to_owned(),
                decided_by: None,
                rationale: None,
                residual_risks: Vec::new(),
            }],
            overall_evaluations: vec![evaluation],
            unreadable: Vec::new(),
            unusable: Vec::new(),
        };
        let body = render(&summary, &advice()).body;
        assert!(body.contains("Title: not stated\n\nCondition:\n\nnot stated\n"));
        assert!(body.contains("#### Harm HARM-1\n\nTitle: not stated\n"));
        assert!(body.contains("| RR-1 | accepted | not stated | not stated |"));
        assert!(
            body.contains("| BR-1 | not stated | undecided | not stated | not stated | none |")
        );
        assert!(
            body.contains("| ORE-1 | not stated | accepted | not stated | not stated | RR-1 |")
        );
    }
}
