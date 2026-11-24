// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The safety-module records the report reads, as their authors wrote them.

use std::collections::BTreeMap;
use std::path::PathBuf;

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
    /// The file the record was read from.
    pub(crate) path: PathBuf,
    pub(crate) decision: String,
    /// The `Decided by` cell as written; `None` when empty or absent.
    pub(crate) decided_by: Option<String>,
    /// The `Rationale` cell as written; `None` when empty or absent.
    pub(crate) rationale: Option<String>,
    /// The `Benefit` cell of a benefit-risk record or the `Product` cell of an
    /// evaluation, as written; `None` when empty or absent.
    pub(crate) subject: Option<String>,
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
    /// The frontmatter `title`, when one is present.
    pub(crate) title: Option<String>,
    /// The `## Condition` section text of a hazard, as written.
    pub(crate) condition: Option<String>,
    /// The `## Description` section text, as written.
    pub(crate) description: Option<String>,
    /// The exact authored status, when one is present.
    pub(crate) status: Option<String>,
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

/// The text of a cell or section, `None` when it holds nothing but whitespace.
fn non_blank(text: String) -> Option<String> {
    (!text.trim().is_empty()).then_some(text)
}

/// A section's text without the blank lines around it, `None` when empty.
fn section(doc: &Document, heading: &str) -> Option<String> {
    let text = doc.sections.get(heading)?;
    non_blank(text.trim_matches(['\r', '\n']).to_owned())
}

fn choice(doc: &Document) -> Choice {
    let determination = doc.tables.get("Determination");
    let decided = |name: &str| determination.and_then(|table| cell(table, 0, name));
    let decision = decided("Decision").unwrap_or_default();
    Choice {
        id: doc.id.clone(),
        path: doc.path.clone(),
        decision,
        decided_by: decided("Decided by").and_then(non_blank),
        rationale: decided("Rationale").and_then(non_blank),
        subject: decided("Benefit")
            .or_else(|| decided("Product"))
            .and_then(non_blank),
        estimate: edges(doc, "estimate").into_iter().next(),
        references: edges(doc, "references"),
    }
}

fn plain(doc: &Document) -> Plain {
    let is_requirement = matches!(doc.kind.as_str(), "FR" | "NFR");
    Plain {
        id: doc.id.clone(),
        kind: doc.kind.clone(),
        title: doc
            .frontmatter
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        condition: section(doc, "Condition"),
        description: section(doc, "Description"),
        status: doc
            .frontmatter
            .get("status")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
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

    fn document(kind: &str, headers: &[&str], cells: &[&str]) -> Document {
        let mut tables = BTreeMap::new();
        tables.insert(
            "Determination".to_owned(),
            Table {
                headers: headers.iter().map(|h| (*h).to_owned()).collect(),
                rows: vec![cells.iter().map(|c| (*c).to_owned()).collect()],
            },
        );
        Document {
            path: "R-1.md".into(),
            id: "R-1".to_owned(),
            kind: kind.to_owned(),
            frontmatter: serde_json::Map::new(),
            edges: Vec::new(),
            tables,
            sections: BTreeMap::from([
                ("Condition".to_owned(), "\n  \n".to_owned()),
                (
                    "Description".to_owned(),
                    "\nline one\n\nline two\n".to_owned(),
                ),
            ]),
            raw: String::new(),
            contract: Contract::NotChecked,
        }
    }

    #[trace("FR-169-AC-30", "FR-169-AC-31", "FR-169-AC-32")]
    #[test]
    fn an_empty_rationale_benefit_product_or_decider_cell_is_none() {
        let benefit = choice(&document(
            "benefit_risk",
            &["Benefit", "Decision", "Rationale", "Decided by"],
            &["", "undecided", " ", ""],
        ));
        assert_eq!(benefit.subject, None);
        assert_eq!(benefit.rationale, None);
        assert_eq!(benefit.decided_by, None);
        assert_eq!(benefit.decision, "undecided");
        let overall = choice(&document(
            "overall_residual_risk_evaluation",
            &["Product", "Decision", "Rationale", "Decided by"],
            &["", "undecided", "", ""],
        ));
        assert_eq!(overall.subject, None);
        assert_eq!(overall.rationale, None);
        let filled = choice(&document(
            "overall_residual_risk_evaluation",
            &["Product", "Decision", "Rationale", "Decided by"],
            &["Pump", "accepted", "why", "Pat"],
        ));
        assert_eq!(filled.subject.as_deref(), Some("Pump"));
        assert_eq!(filled.rationale.as_deref(), Some("why"));
    }

    #[trace("FR-169-AC-29")]
    #[test]
    fn a_blank_section_is_none_and_a_text_keeps_its_inner_line_breaks() {
        let doc = document("hazard", &["Decision"], &["x"]);
        assert_eq!(section(&doc, "Condition"), None);
        assert_eq!(section(&doc, "Missing"), None);
        assert_eq!(
            section(&doc, "Description").as_deref(),
            Some("line one\n\nline two")
        );
    }
}
