// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The read-only discrepancy report (FR-154, FR-155).
//!
//! It compares each residual risk's recorded decision with the region its own
//! policy lists for the estimate's levels, using the one lookup rule
//! `quoin risk advise` uses. A finding says two things a person wrote disagree;
//! it never states a decision, a region or a remedy of its own.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Serialize;

use crate::finding::Token;
use crate::links::Links;
use crate::policy::lookup;
use crate::records::{Choice, Phase, Records};

const ACCEPTED: &str = "accepted";
const NOT_ACCEPTED: &str = "not_accepted";
const UNDECIDED: &str = "undecided";

/// A finding token of FR-155.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiscrepancyToken {
    /// Accepted outside the acceptable region with no covering benefit-risk record.
    AcceptedOutsideAcceptableRegion,
    /// Accepted outside the region; the covering records are undecided.
    BenefitRiskUndecided,
    /// Accepted outside the region; every covering record is not accepted.
    BenefitRiskNotAccepted,
    /// A decision was recorded but no region resolves.
    DecisionWithoutRegion,
    /// Not accepted inside the acceptable region.
    NotAcceptedInsideAcceptableRegion,
    /// The residual risk is undecided.
    ResidualRiskUndecided,
    /// A post-control estimate that no residual risk references.
    PostControlEstimateWithoutDecision,
}

impl DiscrepancyToken {
    fn as_str(self) -> &'static str {
        match self {
            Self::AcceptedOutsideAcceptableRegion => "accepted-outside-acceptable-region",
            Self::BenefitRiskUndecided => "benefit-risk-undecided",
            Self::BenefitRiskNotAccepted => "benefit-risk-not-accepted",
            Self::DecisionWithoutRegion => "decision-without-region",
            Self::NotAcceptedInsideAcceptableRegion => "not-accepted-inside-acceptable-region",
            Self::ResidualRiskUndecided => "residual-risk-undecided",
            Self::PostControlEstimateWithoutDecision => "post-control-estimate-without-decision",
        }
    }
}

/// What a finding names; every key is present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Refs {
    /// The residual-risk id, or `null` for an estimate with no residual risk.
    pub residual_risk: Option<String>,
    /// The post-control estimate id.
    pub estimate: Option<String>,
    /// The record's `decision` as written, or `null`.
    pub decision: Option<String>,
    /// The policy's region name, or `null`.
    pub region: Option<String>,
    /// The policy id from the residual risk's own reference, or `null`.
    pub policy: Option<String>,
    /// The ids of the covering benefit-risk records.
    pub benefit_risk: Vec<String>,
    /// Why a `decision-without-region` has no region, else `null`.
    pub region_missing: Option<&'static str>,
}

/// One place where a recorded decision disagrees with the policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Discrepancy {
    /// The token.
    pub finding: DiscrepancyToken,
    /// What the finding names.
    pub refs: Refs,
}

/// The whole report; its fields are the JSON keys, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Discrepancies {
    /// Where regions and decisions come from.
    pub attribution: &'static str,
    /// The disagreements, ordered by estimate, residual risk, then token.
    pub findings: Vec<Discrepancy>,
}

/// The benefit-risk records that list each record id.
fn coverage(records: &Records) -> BTreeMap<&str, Vec<&Choice>> {
    let mut covering: BTreeMap<&str, Vec<&Choice>> = BTreeMap::new();
    for record in records.benefit_risks.values() {
        for target in &record.references {
            covering.entry(target.as_str()).or_default().push(record);
        }
    }
    covering
}

fn token_for(
    decision: &str,
    outside: Option<bool>,
    covering: &[&Choice],
) -> Option<DiscrepancyToken> {
    match (decision, outside) {
        (UNDECIDED, _) => Some(DiscrepancyToken::ResidualRiskUndecided),
        (ACCEPTED | NOT_ACCEPTED, None) => Some(DiscrepancyToken::DecisionWithoutRegion),
        (ACCEPTED, Some(true)) => {
            let any = |wanted: &str| covering.iter().any(|record| record.decision == wanted);
            if covering.is_empty() {
                Some(DiscrepancyToken::AcceptedOutsideAcceptableRegion)
            } else if any(ACCEPTED) {
                None
            } else if any(UNDECIDED) {
                Some(DiscrepancyToken::BenefitRiskUndecided)
            } else {
                Some(DiscrepancyToken::BenefitRiskNotAccepted)
            }
        }
        (NOT_ACCEPTED, Some(false)) => Some(DiscrepancyToken::NotAcceptedInsideAcceptableRegion),
        _ => None,
    }
}

/// Why the lookup resolved no region, in the spelling of FR-146's tokens.
fn region_missing(findings: &[crate::finding::Finding]) -> Option<&'static str> {
    findings.iter().find_map(|finding| match finding.finding {
        Token::LevelNotInPolicy => Some("level-not-in-policy"),
        Token::LevelNotAssessed => Some("level-not-assessed"),
        Token::CellNotInPolicy => Some("cell-not-in-policy"),
        Token::PolicyNotFound => Some("policy-not-found"),
        Token::PolicyAmbiguous => Some("policy-ambiguous"),
        _ => None,
    })
}

fn residual_finding(
    records: &Records,
    risk: &Choice,
    covering: &BTreeMap<&str, Vec<&Choice>>,
) -> Option<Discrepancy> {
    let estimate = risk
        .estimate
        .as_ref()
        .and_then(|id| records.estimates.get(id));
    let found = estimate.map(|estimate| {
        lookup(
            records,
            &risk.references,
            &estimate.severity,
            &estimate.probability,
        )
    });
    let (policy, region, missing) = match found {
        Some(found) => {
            let missing = region_missing(&found.findings);
            (found.policy, found.region, missing)
        }
        None => (None, None, Some("estimate-not-found")),
    };
    let covering = covering
        .get(risk.id.as_str())
        .map_or(&[][..], Vec::as_slice);
    let finding = token_for(
        &risk.decision,
        region.as_ref().map(|(_, outside)| *outside),
        covering,
    )?;
    let missing = if finding == DiscrepancyToken::DecisionWithoutRegion {
        missing
    } else {
        None
    };
    Some(Discrepancy {
        finding,
        refs: Refs {
            residual_risk: Some(risk.id.clone()),
            estimate: risk.estimate.clone(),
            decision: Some(risk.decision.clone()),
            region: region.map(|(name, _)| name),
            policy,
            benefit_risk: covering.iter().map(|record| record.id.clone()).collect(),
            region_missing: missing,
        },
    })
}

pub(crate) fn build(records: &Records, links: &Links) -> Vec<Discrepancy> {
    let covering = coverage(records);
    let mut findings: Vec<Discrepancy> = records
        .residual_risks
        .values()
        .filter_map(|risk| residual_finding(records, risk, &covering))
        .collect();
    findings.extend(
        records
            .estimates
            .values()
            .filter(|estimate| estimate.phase == Phase::PostControl)
            .filter(|estimate| links.residual_risks_of(&estimate.id).is_empty())
            .map(|estimate| Discrepancy {
                finding: DiscrepancyToken::PostControlEstimateWithoutDecision,
                refs: Refs {
                    residual_risk: None,
                    estimate: Some(estimate.id.clone()),
                    decision: None,
                    region: None,
                    policy: None,
                    benefit_risk: Vec::new(),
                    region_missing: None,
                },
            }),
    );
    findings.sort_by(|left, right| {
        (
            &left.refs.estimate,
            &left.refs.residual_risk,
            left.finding.as_str(),
        )
            .cmp(&(
                &right.refs.estimate,
                &right.refs.residual_risk,
                right.finding.as_str(),
            ))
    });
    findings
}

fn line(finding: &Discrepancy) -> String {
    let refs = &finding.refs;
    let mut text = format!("- {}", finding.finding.as_str());
    if let Some(estimate) = &refs.estimate {
        let _ = write!(text, ": estimate {estimate}");
    }
    if let Some(risk) = &refs.residual_risk {
        let _ = write!(text, ", residual risk {risk}");
    }
    if let Some(decision) = &refs.decision {
        let _ = write!(text, ", recorded decision {decision}");
    }
    match (&refs.region, refs.region_missing) {
        (Some(region), _) => {
            let _ = write!(text, ", the policy marks this cell as {region}");
        }
        (None, Some(reason)) => {
            let _ = write!(text, ", no region resolves ({reason})");
        }
        (None, None) => {}
    }
    if let Some(policy) = &refs.policy {
        let _ = write!(text, ", policy {policy}");
    }
    if !refs.benefit_risk.is_empty() {
        let _ = write!(text, ", benefit-risk {}", refs.benefit_risk.join(", "));
    }
    text.push('.');
    text
}

pub(crate) fn markdown(report: &Discrepancies) -> String {
    let mut out = format!(
        "# Risk discrepancies\n\n## attribution\n\n{}\n\n## findings\n\n",
        report.attribution
    );
    if report.findings.is_empty() {
        out.push_str("None.\n");
    }
    for finding in &report.findings {
        out.push_str(&line(finding));
        out.push('\n');
    }
    out
}
