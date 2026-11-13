// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The read-only risk summary (FR-169, FR-170): the hazard-to-decision chain,
//! one row per hazard and hazardous situation.
//!
//! Every value is one a person entered or a link that exists in the records.
//! The region comes from the lookup `quoin risk advise` uses, and the findings
//! come from the rules of `quoin risk advise` and `quoin risk discrepancies`;
//! nothing here restates them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Serialize;

use crate::discrepancies::{self, Discrepancy, DiscrepancyToken};
use crate::evidence::{assumed_controls, control_evidence};
use crate::finding::Token;
use crate::links::Links;
use crate::policy::lookup;
use crate::records::{Estimate, Phase, Records};

/// A finding token of FR-170, in the order of its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RowToken {
    /// A hazard with no situation.
    HazardWithoutSituation,
    /// A situation with no hazard.
    SituationWithoutHazard,
    /// A situation with no harm.
    SituationWithoutHarm,
    /// No requirement satisfies the control.
    ControlWithoutRequirement,
    /// Nothing verifies the control.
    ControlWithoutVerification,
    /// The post-control estimate assumes no control.
    PostControlWithoutControl,
    /// No residual risk references the post-control estimate.
    PostControlEstimateWithoutDecision,
    /// The residual risk is undecided.
    ResidualRiskUndecided,
    /// A decision was recorded but no region resolves.
    DecisionWithoutRegion,
    /// Accepted outside the acceptable region with no covering record.
    AcceptedOutsideAcceptableRegion,
    /// Accepted outside the region; the covering records are undecided.
    BenefitRiskUndecided,
    /// Accepted outside the region; the covering records are not accepted.
    BenefitRiskNotAccepted,
    /// Not accepted inside the acceptable region.
    NotAcceptedInsideAcceptableRegion,
}

impl RowToken {
    /// Whether the finding names the covering benefit-risk records too.
    fn names_coverage(self) -> bool {
        matches!(
            self,
            Self::AcceptedOutsideAcceptableRegion
                | Self::BenefitRiskUndecided
                | Self::BenefitRiskNotAccepted
        )
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::HazardWithoutSituation => "hazard-without-situation",
            Self::SituationWithoutHazard => "situation-without-hazard",
            Self::SituationWithoutHarm => "situation-without-harm",
            Self::ControlWithoutRequirement => "control-without-requirement",
            Self::ControlWithoutVerification => "control-without-verification",
            Self::PostControlWithoutControl => "post-control-without-control",
            Self::PostControlEstimateWithoutDecision => "post-control-estimate-without-decision",
            Self::ResidualRiskUndecided => "residual-risk-undecided",
            Self::DecisionWithoutRegion => "decision-without-region",
            Self::AcceptedOutsideAcceptableRegion => "accepted-outside-acceptable-region",
            Self::BenefitRiskUndecided => "benefit-risk-undecided",
            Self::BenefitRiskNotAccepted => "benefit-risk-not-accepted",
            Self::NotAcceptedInsideAcceptableRegion => "not-accepted-inside-acceptable-region",
        }
    }
}

impl RowToken {
    /// The row token of a per-control token of `quoin risk advise`.
    fn from_control(token: Token) -> Option<Self> {
        match token {
            Token::ControlWithoutRequirement => Some(Self::ControlWithoutRequirement),
            Token::ControlWithoutVerification => Some(Self::ControlWithoutVerification),
            _ => None,
        }
    }
}

impl From<DiscrepancyToken> for RowToken {
    fn from(token: DiscrepancyToken) -> Self {
        match token {
            DiscrepancyToken::AcceptedOutsideAcceptableRegion => {
                Self::AcceptedOutsideAcceptableRegion
            }
            DiscrepancyToken::BenefitRiskUndecided => Self::BenefitRiskUndecided,
            DiscrepancyToken::BenefitRiskNotAccepted => Self::BenefitRiskNotAccepted,
            DiscrepancyToken::DecisionWithoutRegion => Self::DecisionWithoutRegion,
            DiscrepancyToken::NotAcceptedInsideAcceptableRegion => {
                Self::NotAcceptedInsideAcceptableRegion
            }
            DiscrepancyToken::ResidualRiskUndecided => Self::ResidualRiskUndecided,
            DiscrepancyToken::PostControlEstimateWithoutDecision => {
                Self::PostControlEstimateWithoutDecision
            }
        }
    }
}

/// A token and the ids it names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RowFinding {
    /// The token.
    pub finding: RowToken,
    /// The ids the token names.
    pub names: Vec<String>,
}

/// An estimate as entered, and the policy's region name for its levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EstimateEntry {
    /// The estimate id.
    pub estimate: String,
    /// The severity as entered.
    pub severity: String,
    /// The probability as entered.
    pub probability: String,
    /// The policy's region name, or `null`.
    pub policy_region: Option<String>,
}

/// A control a post-control estimate assumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControlEntry {
    /// The control id.
    pub control: String,
    /// Ids of the `FR` and `NFR` documents that satisfy it.
    pub requirements: Vec<String>,
    /// Ids of the documents that verify it.
    pub verifications: Vec<String>,
}

/// A residual-risk record as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResidualEntry {
    /// The residual-risk id.
    pub residual_risk: String,
    /// The decision word as written.
    pub decision: String,
    /// The decider as written, or `null`.
    pub decided_by: Option<String>,
}

/// One hazard and situation pairing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// The hazard id, or `null`.
    pub hazard: Option<String>,
    /// The situation id, or `null`.
    pub situation: Option<String>,
    /// Ids of the harms that arise from the situation.
    pub harms: Vec<String>,
    /// The pre-control estimates of the situation.
    pub pre_control: Vec<EstimateEntry>,
    /// The controls the post-control estimates assume.
    pub controls: Vec<ControlEntry>,
    /// The post-control estimates of the situation.
    pub post_control: Vec<EstimateEntry>,
    /// The residual risks on the post-control estimates.
    pub residual_risks: Vec<ResidualEntry>,
    /// The completeness findings naming this row.
    pub findings: Vec<RowFinding>,
}

/// The whole report; its fields are the JSON keys, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    /// Where regions and decisions come from.
    pub attribution: &'static str,
    /// One row per hazard and situation pairing.
    pub rows: Vec<Row>,
    /// The files the reader could not load, each as `path: reason`.
    #[serde(skip)]
    pub unreadable: Vec<String>,
}

/// Residual-risk findings by residual-risk id, and the estimates with no
/// residual risk, from the rules of `quoin risk discrepancies`.
struct Decisions {
    by_residual: BTreeMap<String, Vec<RowFinding>>,
    undecided_estimates: BTreeSet<String>,
}

impl Decisions {
    fn build(records: &Records, links: &Links) -> Self {
        let mut by_residual: BTreeMap<String, Vec<RowFinding>> = BTreeMap::new();
        let mut undecided_estimates = BTreeSet::new();
        for found in discrepancies::build(records, links) {
            let Discrepancy { finding, refs } = found;
            let token = RowToken::from(finding);
            match (refs.residual_risk, refs.estimate) {
                (Some(risk), _) => {
                    let mut names = vec![risk.clone()];
                    if token.names_coverage() {
                        names.extend(refs.benefit_risk);
                    }
                    by_residual.entry(risk).or_default().push(RowFinding {
                        finding: token,
                        names,
                    });
                }
                (None, Some(estimate)) => {
                    undecided_estimates.insert(estimate);
                }
                (None, None) => {}
            }
        }
        Self {
            by_residual,
            undecided_estimates,
        }
    }
}

struct Chain<'a> {
    records: &'a Records,
    links: &'a Links,
    decisions: Decisions,
    /// Estimate id to the policy's region name.
    regions: BTreeMap<&'a str, Option<String>>,
}

fn is_hazard(records: &Records, id: &str) -> bool {
    records.others.get(id).is_some_and(|doc| doc.hazard)
}

impl<'a> Chain<'a> {
    fn new(records: &'a Records, links: &'a Links) -> Self {
        let regions = records
            .estimates
            .values()
            .map(|estimate| {
                let policies = links.policies_of(records, estimate);
                let found = lookup(
                    records,
                    &policies,
                    &estimate.severity,
                    &estimate.probability,
                );
                (estimate.id.as_str(), found.region.map(|(region, _)| region))
            })
            .collect();
        Self {
            records,
            links,
            decisions: Decisions::build(records, links),
            regions,
        }
    }

    fn entry(&self, estimate: &Estimate) -> EstimateEntry {
        EstimateEntry {
            estimate: estimate.id.clone(),
            severity: estimate.severity.clone(),
            probability: estimate.probability.clone(),
            policy_region: self.regions.get(estimate.id.as_str()).cloned().flatten(),
        }
    }

    /// The estimates of `phase` that estimate the situation, ascending by id.
    fn estimates_of(&self, situation: &str, phase: Phase) -> Vec<&'a Estimate> {
        self.records
            .estimates
            .values()
            .filter(|estimate| estimate.phase == phase)
            .filter(|estimate| estimate.situations.iter().any(|id| id == situation))
            .collect()
    }

    fn harms_of(&self, situation: &str) -> Vec<String> {
        self.records
            .others
            .values()
            .filter(|doc| doc.kind == "harm" && doc.arises_from.iter().any(|id| id == situation))
            .map(|doc| doc.id.clone())
            .collect()
    }

    fn row(&self, hazard: Option<&str>, situation: Option<&str>) -> Row {
        let mut findings = BTreeSet::new();
        let mut add = |finding: RowToken, names: Vec<String>| {
            findings.insert(RowFinding { finding, names });
        };
        let Some(situation) = situation else {
            if let Some(id) = hazard {
                add(RowToken::HazardWithoutSituation, vec![id.to_owned()]);
            }
            return Row {
                hazard: hazard.map(str::to_owned),
                situation: None,
                harms: Vec::new(),
                pre_control: Vec::new(),
                controls: Vec::new(),
                post_control: Vec::new(),
                residual_risks: Vec::new(),
                findings: findings.into_iter().collect(),
            };
        };
        if hazard.is_none() {
            add(RowToken::SituationWithoutHazard, vec![situation.to_owned()]);
        }
        let harms = self.harms_of(situation);
        if harms.is_empty() {
            add(RowToken::SituationWithoutHarm, vec![situation.to_owned()]);
        }
        let pre = self.estimates_of(situation, Phase::PreControl);
        let post = self.estimates_of(situation, Phase::PostControl);

        let mut control_ids = BTreeSet::new();
        let mut residual_ids = BTreeSet::new();
        for estimate in &post {
            let assumed = assumed_controls(self.records, estimate);
            if assumed.is_empty() {
                add(
                    RowToken::PostControlWithoutControl,
                    vec![estimate.id.clone()],
                );
            }
            control_ids.extend(assumed.into_iter().cloned());
            let risks = self.links.residual_risks_of(&estimate.id);
            if self.decisions.undecided_estimates.contains(&estimate.id) {
                add(
                    RowToken::PostControlEstimateWithoutDecision,
                    vec![estimate.id.clone()],
                );
            }
            residual_ids.extend(
                risks
                    .iter()
                    .filter(|risk| self.records.residual_risks.contains_key(*risk))
                    .cloned(),
            );
        }
        let mut controls = Vec::new();
        for id in &control_ids {
            let evidence = control_evidence(self.links, id);
            for found in &evidence.findings {
                if let Some(token) = RowToken::from_control(found.finding) {
                    add(token, vec![id.clone()]);
                }
            }
            controls.push(ControlEntry {
                control: evidence.control,
                requirements: evidence.requirements,
                verifications: evidence.verifications,
            });
        }
        let residual_risks = residual_ids
            .iter()
            .filter_map(|id| self.records.residual_risks.get(id))
            .map(|risk| ResidualEntry {
                residual_risk: risk.id.clone(),
                decision: risk.decision.clone(),
                decided_by: risk.decided_by.clone(),
            })
            .collect();
        for id in &residual_ids {
            for found in self.decisions.by_residual.get(id).into_iter().flatten() {
                add(found.finding, found.names.clone());
            }
        }
        Row {
            hazard: hazard.map(str::to_owned),
            situation: Some(situation.to_owned()),
            harms,
            pre_control: pre.into_iter().map(|e| self.entry(e)).collect(),
            controls,
            post_control: post.into_iter().map(|e| self.entry(e)).collect(),
            residual_risks,
            findings: findings.into_iter().collect(),
        }
    }
}

pub(crate) fn build(records: &Records, links: &Links) -> Vec<Row> {
    let chain = Chain::new(records, links);
    let situations: Vec<&str> = records
        .others
        .values()
        .filter(|doc| doc.kind == "hazardous_situation")
        .map(|doc| doc.id.as_str())
        .collect();
    let hazards_of = |situation: &str| -> Vec<&str> {
        records
            .others
            .get(situation)
            .into_iter()
            .flat_map(|doc| &doc.arises_from)
            .filter(|id| is_hazard(records, id))
            .map(String::as_str)
            .collect()
    };
    let mut rows = Vec::new();
    for hazard in records.others.values().filter(|doc| doc.hazard) {
        let mine: Vec<&str> = situations
            .iter()
            .copied()
            .filter(|situation| hazards_of(situation).contains(&hazard.id.as_str()))
            .collect();
        if mine.is_empty() {
            rows.push(chain.row(Some(&hazard.id), None));
        }
        for situation in mine {
            rows.push(chain.row(Some(&hazard.id), Some(situation)));
        }
    }
    for situation in situations {
        if hazards_of(situation).is_empty() {
            rows.push(chain.row(None, Some(situation)));
        }
    }
    rows
}

/// A copied value in a table cell: a pipe would end the cell, so it is escaped.
fn esc(value: &str) -> String {
    value.replace('|', "\\|")
}

fn ids(list: &[String]) -> String {
    if list.is_empty() {
        "[]".to_owned()
    } else {
        list.iter().map(|id| esc(id)).collect::<Vec<_>>().join(", ")
    }
}

fn or_null(value: Option<&String>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| esc(value))
}

fn entries<T>(list: &[T], render: impl Fn(&T) -> String) -> String {
    if list.is_empty() {
        "[]".to_owned()
    } else {
        list.iter().map(render).collect::<Vec<_>>().join("<br>")
    }
}

fn estimate_cell(entry: &EstimateEntry) -> String {
    let region = entry.policy_region.as_ref().map_or_else(
        || "policy_region=null".to_owned(),
        |region| format!("the policy marks this cell as {}", esc(region)),
    );
    format!(
        "estimate={}; severity={}; probability={}; {region}",
        esc(&entry.estimate),
        esc(&entry.severity),
        esc(&entry.probability)
    )
}

fn control_cell(entry: &ControlEntry) -> String {
    format!(
        "control={}; requirements={}; verifications={}",
        esc(&entry.control),
        ids(&entry.requirements),
        ids(&entry.verifications)
    )
}

fn residual_cell(entry: &ResidualEntry) -> String {
    format!(
        "residual_risk={}; decision={}; decided_by={}",
        esc(&entry.residual_risk),
        esc(&entry.decision),
        or_null(entry.decided_by.as_ref())
    )
}

fn finding_cell(entry: &RowFinding) -> String {
    format!(
        "finding={}; names={}",
        entry.finding.as_str(),
        ids(&entry.names)
    )
}

const COLUMNS: [&str; 8] = [
    "hazard",
    "situation",
    "harms",
    "pre_control",
    "controls",
    "post_control",
    "residual_risks",
    "findings",
];

pub(crate) fn markdown(summary: &Summary) -> String {
    let mut out = format!("{}\n\n", summary.attribution);
    let _ = writeln!(out, "| {} |", COLUMNS.join(" | "));
    let _ = writeln!(out, "|{}", " --- |".repeat(COLUMNS.len()));
    for row in &summary.rows {
        let cells = [
            or_null(row.hazard.as_ref()),
            or_null(row.situation.as_ref()),
            ids(&row.harms),
            entries(&row.pre_control, estimate_cell),
            entries(&row.controls, control_cell),
            entries(&row.post_control, estimate_cell),
            entries(&row.residual_risks, residual_cell),
            entries(&row.findings, finding_cell),
        ];
        let _ = writeln!(out, "| {} |", cells.join(" | "));
    }
    out
}
