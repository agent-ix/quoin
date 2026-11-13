// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The safety-module records the report reads, as their authors wrote them.

use std::collections::BTreeMap;

use quoin_anomaly::{Record as Anomaly, acceptance_criteria, is_hazard, parse_record};
use quoin_quire::records::{Contract, Document, Table};

use crate::error::AdviceError;

/// The types the discrepancy report reads (FR-154): it checks no other.
pub(crate) const DISCREPANCY_CHECKED: [&str; 4] = [
    "risk_estimate",
    "acceptability_policy",
    "residual_risk",
    "benefit_risk",
];

/// The types whose declared `body_extraction` contract is checked.
pub(crate) const CHECKED: [&str; 5] = [
    "risk_estimate",
    "acceptability_policy",
    "residual_risk",
    "benefit_risk",
    "overall_residual_risk_evaluation",
];

/// Whether an estimate was made before or after the controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    PreControl,
    PostControl,
}

impl Phase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PreControl => "pre_control",
            Self::PostControl => "post_control",
        }
    }
}

pub(crate) struct Estimate {
    pub(crate) id: String,
    pub(crate) phase: Phase,
    pub(crate) severity: String,
    pub(crate) probability: String,
    pub(crate) situations: Vec<String>,
    pub(crate) controls: Vec<String>,
}

pub(crate) struct Policy {
    pub(crate) id: String,
    pub(crate) severities: Vec<String>,
    pub(crate) probabilities: Vec<String>,
    /// Region name to its `outside_acceptable` flag.
    pub(crate) regions: BTreeMap<String, bool>,
    /// Severity and probability to the region name.
    pub(crate) cells: BTreeMap<(String, String), String>,
}

/// A record that holds a decision: residual risk, benefit-risk or evaluation.
pub(crate) struct Choice {
    pub(crate) id: String,
    pub(crate) decision: String,
    /// `estimate` for a residual risk.
    pub(crate) estimate: Option<String>,
    /// `references` edges: the policy of a residual risk, the covered or
    /// considered records of the other two.
    pub(crate) references: Vec<String>,
}

/// A document the report only needs to know by kind and links.
pub(crate) struct Plain {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) hazard: bool,
    pub(crate) arises_from: Vec<String>,
    pub(crate) satisfies: Vec<String>,
    pub(crate) verifies: Vec<String>,
    /// The acceptance-criterion ids a requirement declares.
    pub(crate) criteria: Vec<String>,
}

#[derive(Default)]
pub(crate) struct Records {
    pub(crate) estimates: BTreeMap<String, Estimate>,
    pub(crate) policies: BTreeMap<String, Policy>,
    pub(crate) residual_risks: BTreeMap<String, Choice>,
    pub(crate) benefit_risks: BTreeMap<String, Choice>,
    pub(crate) evaluations: BTreeMap<String, Choice>,
    pub(crate) others: BTreeMap<String, Plain>,
    pub(crate) anomalies: Vec<Anomaly>,
}

impl Records {
    /// Build the records from the documents.
    ///
    /// An estimate that breaks its type's contract is skipped (FR-145): `quire
    /// validate` reports it. Any other checked record that breaks its contract
    /// stops the report, because dropping it would drop an undecided decision
    /// or a policy from the output without a word.
    pub(crate) fn read(docs: &[Document]) -> Result<Self, AdviceError> {
        let mut records = Self::default();
        for doc in docs {
            if let Contract::Violated(reason) = &doc.contract
                && doc.kind != "risk_estimate"
            {
                return Err(AdviceError::record(&doc.path, &doc.id, reason));
            }
            let satisfied = doc.contract != Contract::NotChecked
                && !matches!(doc.contract, Contract::Violated(_));
            match doc.kind.as_str() {
                "risk_estimate" if satisfied => {
                    if let Some(estimate) = estimate(doc) {
                        records.estimates.insert(doc.id.clone(), estimate);
                    }
                }
                "acceptability_policy" => {
                    records.policies.insert(doc.id.clone(), policy(doc));
                }
                "residual_risk" => {
                    records.residual_risks.insert(doc.id.clone(), choice(doc));
                }
                "benefit_risk" => {
                    records.benefit_risks.insert(doc.id.clone(), choice(doc));
                }
                "overall_residual_risk_evaluation" => {
                    records.evaluations.insert(doc.id.clone(), choice(doc));
                }
                "Anomaly" => {
                    if let Ok(anomaly) = parse_record(&doc.raw) {
                        records.anomalies.push(anomaly);
                    }
                }
                _ => {}
            }
            records.others.insert(doc.id.clone(), plain(doc));
        }
        Ok(records)
    }
}

/// The target ids of the document's edges of type `verb`, sorted and distinct.
fn edges(doc: &Document, verb: &str) -> Vec<String> {
    let mut targets: Vec<String> = doc
        .edges
        .iter()
        .filter(|(_, edge)| edge == verb)
        .map(|(target, _)| target.clone())
        .collect();
    targets.sort();
    targets.dedup();
    targets
}

fn column(table: &Table, name: &str) -> Option<usize> {
    table.headers.iter().position(|header| header == name)
}

fn cell(table: &Table, row: usize, name: &str) -> Option<String> {
    table.rows.get(row)?.get(column(table, name)?).cloned()
}

fn estimate(doc: &Document) -> Option<Estimate> {
    let phase = match doc.frontmatter.get("phase")?.as_str()? {
        "pre_control" => Phase::PreControl,
        "post_control" => Phase::PostControl,
        _ => return None,
    };
    let table = doc.tables.get("Judgement")?;
    Some(Estimate {
        id: doc.id.clone(),
        phase,
        severity: cell(table, 0, "Severity")?,
        probability: cell(table, 0, "Probability")?,
        situations: edges(doc, "estimates"),
        controls: edges(doc, "assumes"),
    })
}

fn levels(doc: &Document, heading: &str, name: &str) -> Vec<String> {
    doc.tables.get(heading).map_or_else(Vec::new, |table| {
        (0..table.rows.len())
            .filter_map(|row| cell(table, row, name))
            .collect()
    })
}

fn policy(doc: &Document) -> Policy {
    let regions = doc
        .tables
        .get("Regions")
        .map_or_else(BTreeMap::new, |table| {
            (0..table.rows.len())
                .filter_map(|row| {
                    let flag = cell(table, row, "Outside acceptable")?
                        .parse::<bool>()
                        .ok()?;
                    Some((cell(table, row, "Region")?, flag))
                })
                .collect()
        });
    let cells = doc.tables.get("Cells").map_or_else(BTreeMap::new, |table| {
        (0..table.rows.len())
            .filter_map(|row| {
                Some((
                    (
                        cell(table, row, "Severity")?,
                        cell(table, row, "Probability")?,
                    ),
                    cell(table, row, "Region")?,
                ))
            })
            .collect()
    });
    Policy {
        id: doc.id.clone(),
        severities: levels(doc, "Severity levels", "Severity level"),
        probabilities: levels(doc, "Probability levels", "Probability level"),
        regions,
        cells,
    }
}

fn choice(doc: &Document) -> Choice {
    let decision = doc
        .tables
        .get("Determination")
        .and_then(|table| cell(table, 0, "Decision"))
        .unwrap_or_default();
    Choice {
        id: doc.id.clone(),
        decision,
        estimate: edges(doc, "estimate").into_iter().next(),
        references: edges(doc, "references"),
    }
}

fn plain(doc: &Document) -> Plain {
    let is_requirement = matches!(doc.kind.as_str(), "FR" | "NFR");
    Plain {
        id: doc.id.clone(),
        kind: doc.kind.clone(),
        hazard: is_hazard(&doc.frontmatter),
        arises_from: edges(doc, "arises_from"),
        satisfies: edges(doc, "satisfies"),
        verifies: edges(doc, "verifies"),
        criteria: if is_requirement {
            acceptance_criteria(&doc.raw, &doc.id)
        } else {
            Vec::new()
        },
    }
}
