// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The safety-module records the report reads, as their authors wrote them.

use std::collections::BTreeMap;

use quoin_anomaly::{Record as Anomaly, parse_record};

use crate::corpus::Doc;

/// Whether an estimate was made before or after the controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
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
pub(crate) struct DecisionRecord {
    pub(crate) id: String,
    pub(crate) decision: String,
    /// `estimate` for a residual risk.
    pub(crate) estimate: Option<String>,
    /// `references` edges: the policy of a residual risk, the covered or
    /// considered records of the other two.
    pub(crate) references: Vec<String>,
}

/// A document the report only needs to know by kind and id.
pub(crate) struct Plain {
    pub(crate) id: String,
    pub(crate) kind: String,
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
    pub(crate) residual_risks: BTreeMap<String, DecisionRecord>,
    pub(crate) benefit_risks: BTreeMap<String, DecisionRecord>,
    pub(crate) evaluations: BTreeMap<String, DecisionRecord>,
    pub(crate) others: BTreeMap<String, Plain>,
    pub(crate) anomalies: Vec<Anomaly>,
}

impl Records {
    pub(crate) fn read(docs: &[Doc]) -> Self {
        let mut records = Self::default();
        for doc in docs {
            match doc.kind.as_str() {
                "risk_estimate" => {
                    if let Some(estimate) = estimate(doc) {
                        records.estimates.insert(doc.id.clone(), estimate);
                    }
                }
                "acceptability_policy" => {
                    records.policies.insert(doc.id.clone(), policy(doc));
                }
                "residual_risk" => {
                    records.residual_risks.insert(doc.id.clone(), decision(doc));
                }
                "benefit_risk" => {
                    records.benefit_risks.insert(doc.id.clone(), decision(doc));
                }
                "overall_residual_risk_evaluation" => {
                    records.evaluations.insert(doc.id.clone(), decision(doc));
                }
                "Anomaly" => {
                    if let Ok(anomaly) = parse_record(&doc.text) {
                        records.anomalies.push(anomaly);
                    }
                }
                _ => {}
            }
            records.others.insert(doc.id.clone(), plain(doc));
        }
        records
    }
}

fn estimate(doc: &Doc) -> Option<Estimate> {
    let phase = match doc.front_str("phase")? {
        "pre_control" => Phase::PreControl,
        "post_control" => Phase::PostControl,
        _ => return None,
    };
    let rows = doc.table("Judgement", &["Severity", "Probability", "Rationale"]);
    let row = rows.first()?;
    Some(Estimate {
        id: doc.id.clone(),
        phase,
        severity: row.first()?.clone(),
        probability: row.get(1)?.clone(),
        situations: doc.edges("estimates"),
        controls: doc.edges("assumes"),
    })
}

fn column(doc: &Doc, heading: &str, name: &str) -> Vec<String> {
    doc.table(heading, &[name])
        .into_iter()
        .filter_map(|row| row.into_iter().next())
        .collect()
}

fn policy(doc: &Doc) -> Policy {
    let regions = doc
        .table("Regions", &["Region", "Outside acceptable"])
        .into_iter()
        .filter_map(|row| {
            let [name, flag] = <[String; 2]>::try_from(row).ok()?;
            Some((name, flag.parse::<bool>().ok()?))
        })
        .collect();
    let cells = doc
        .table("Cells", &["Severity", "Probability", "Region"])
        .into_iter()
        .filter_map(|row| {
            let [severity, probability, region] = <[String; 3]>::try_from(row).ok()?;
            Some(((severity, probability), region))
        })
        .collect();
    Policy {
        id: doc.id.clone(),
        severities: column(doc, "Severity levels", "Severity level"),
        probabilities: column(doc, "Probability levels", "Probability level"),
        regions,
        cells,
    }
}

fn decision(doc: &Doc) -> DecisionRecord {
    let word = doc
        .table("Determination", &["Decision", "Rationale", "Decided by"])
        .into_iter()
        .chain(doc.table(
            "Determination",
            &["Benefit", "Decision", "Rationale", "Decided by"],
        ))
        .chain(doc.table(
            "Determination",
            &["Product", "Decision", "Rationale", "Decided by"],
        ))
        .next()
        .and_then(|row| {
            let at = usize::from(row.len() > 3);
            row.into_iter().nth(at)
        })
        .unwrap_or_default();
    DecisionRecord {
        id: doc.id.clone(),
        decision: word,
        estimate: doc.edges("estimate").into_iter().next(),
        references: doc.edges("references"),
    }
}

fn plain(doc: &Doc) -> Plain {
    let prefix = format!("{}-AC-", doc.id);
    let criteria = if matches!(doc.kind.as_str(), "FR" | "NFR") {
        doc.text
            .lines()
            .filter_map(|line| {
                let first = line.trim_start().strip_prefix('|')?.split('|').next()?;
                first
                    .trim()
                    .starts_with(&prefix)
                    .then(|| first.trim().to_owned())
            })
            .collect()
    } else {
        Vec::new()
    };
    Plain {
        id: doc.id.clone(),
        kind: doc.kind.clone(),
        arises_from: doc.edges("arises_from"),
        satisfies: doc.edges("satisfies"),
        verifies: doc.edges("verifies"),
        criteria,
    }
}
