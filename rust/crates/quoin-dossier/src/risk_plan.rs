// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The authored-policy risk management plan document (FR-200, PLAT-1656).

use std::fmt::Write as _;

use quoin_quire::records::{Corpus, Document, Table};

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, unusable_input};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// Reads all selected acceptability policy documents.
pub(crate) struct RiskPlanGenerator(pub(crate) fn(&Inputs) -> Result<Corpus, SourceFailure>);

impl Generator for RiskPlanGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "risk-management-plan",
            id: "DOS-RMP",
            title: "Risk management plan",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let corpus = (self.0)(inputs)?;
        if let Some(failure) = unusable_input("acceptability policy", &corpus) {
            return Err(failure);
        }
        let policies: Vec<_> = corpus
            .documents
            .iter()
            .filter(|document| document.kind == "acceptability_policy")
            .collect();
        render(&policies)
    }
}

fn render(policies: &[&Document]) -> Result<Generated, SourceFailure> {
    let mut policies = policies.to_vec();
    policies.sort_by(|left, right| left.id.cmp(&right.id));
    let mut body = String::from(
        "# Risk management plan\n\n## Scope\n\nNo authored plan record exists.\n\n\
         ## Responsibilities\n\nNo authored plan record exists.\n\n\
         ## Verification of risk controls\n\nNo authored plan record exists.\n\n\
         ## Acceptability policy\n",
    );
    let mut gaps = Vec::new();
    if policies.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoAcceptabilityPolicy,
            "this document",
            "no acceptability policy record exists in scope",
        ));
    }
    gaps.push(Gap::new(
        GapKind::NoPlanRecord,
        "this document",
        "no authored risk management plan record exists",
    ));
    for policy in policies {
        let severity = required_table(policy, "Severity levels")?;
        let probability = required_table(policy, "Probability levels")?;
        let regions = required_table(policy, "Regions")?;
        let cells = required_table(policy, "Cells")?;
        validate_table(policy, "Severity levels", severity, &["Severity level"])?;
        validate_table(
            policy,
            "Probability levels",
            probability,
            &["Probability level"],
        )?;
        validate_table(
            policy,
            "Regions",
            regions,
            &["Region", "Outside acceptable"],
        )?;
        validate_table(
            policy,
            "Cells",
            cells,
            &["Severity", "Probability", "Region"],
        )?;
        if regions.rows.iter().any(|row| {
            row.get(1)
                .is_none_or(|flag| !matches!(flag.as_str(), "true" | "false"))
        }) {
            return Err(invalid_policy(
                policy,
                "a region has no authored outside_acceptable flag",
            ));
        }
        let _ = write!(body, "\n### {}\n\n", cell(&policy.id));
        render_table(&mut body, "Severity levels", severity);
        render_table(&mut body, "Probability levels", probability);
        render_table(&mut body, "Regions", regions);
        render_table(&mut body, "Cells", cells);
    }
    Ok(Generated { body, gaps })
}

fn required_table<'a>(document: &'a Document, name: &str) -> Result<&'a Table, SourceFailure> {
    document
        .tables
        .get(name)
        .ok_or_else(|| invalid_policy(document, &format!("missing {name} table")))
}

fn validate_table(
    document: &Document,
    name: &str,
    table: &Table,
    expected: &[&str],
) -> Result<(), SourceFailure> {
    if table
        .headers
        .iter()
        .map(String::as_str)
        .ne(expected.iter().copied())
        || table.rows.iter().any(|row| row.len() != expected.len())
    {
        return Err(invalid_policy(document, &format!("invalid {name} table")));
    }
    Ok(())
}

fn invalid_policy(document: &Document, detail: &str) -> SourceFailure {
    SourceFailure {
        source: "acceptability policy".to_owned(),
        detail: format!("{}: {detail}", document.path.display()),
    }
}

fn render_table(body: &mut String, heading: &str, table: &Table) {
    let _ = writeln!(body, "#### {heading}\n");
    body.push('|');
    for header in &table.headers {
        let _ = write!(body, " {} |", cell(header));
    }
    body.push_str("\n|");
    for _ in &table.headers {
        body.push_str(" --- |");
    }
    body.push('\n');
    for row in &table.rows {
        body.push('|');
        for value in row {
            let _ = write!(body, " {} |", cell(value));
        }
        body.push('\n');
    }
    body.push('\n');
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
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use ix_trace_rs::trace;
    use quoin_quire::records::{Contract, Document, Table};
    use serde_json::Map;

    use super::*;

    fn table(headers: &[&str], rows: &[&[&str]]) -> Table {
        Table {
            headers: headers.iter().map(|value| (*value).to_owned()).collect(),
            rows: rows
                .iter()
                .map(|row| row.iter().map(|value| (*value).to_owned()).collect())
                .collect(),
        }
    }

    fn policy(id: &str, outside: &str) -> Document {
        Document {
            path: PathBuf::from(format!("spec/{id}.md")),
            id: id.to_owned(),
            kind: "acceptability_policy".to_owned(),
            frontmatter: Map::new(),
            edges: Vec::new(),
            tables: BTreeMap::from([
                (
                    "Severity levels".to_owned(),
                    table(&["Severity level"], &[&["minor"], &["critical"]]),
                ),
                (
                    "Probability levels".to_owned(),
                    table(&["Probability level"], &[&["unlikely"], &["likely"]]),
                ),
                (
                    "Regions".to_owned(),
                    table(&["Region", "Outside acceptable"], &[&["review", outside]]),
                ),
                (
                    "Cells".to_owned(),
                    table(
                        &["Severity", "Probability", "Region"],
                        &[&["critical", "likely", "review"]],
                    ),
                ),
            ]),
            sections: BTreeMap::new(),
            raw: String::new(),
            contract: Contract::NotChecked,
        }
    }

    fn generated(docs: &[Document]) -> Result<Generated, SourceFailure> {
        let refs: Vec<_> = docs.iter().collect();
        render(&refs)
    }

    #[trace("FR-200-AC-1")]
    #[test]
    fn renders_authored_levels_regions_flags_and_cells_with_permanent_gap() {
        let output = generated(&[policy("POL-1", "true")]).unwrap();
        assert!(output.body.contains("| critical | likely | review |"));
        assert!(output.body.contains("| review | true |"));
        assert!(output.body.contains("| minor |"));
        assert!(output.body.contains("| unlikely |"));
        assert_eq!(output.gaps.len(), 1);
        assert_eq!(output.gaps[0].gap_kind, GapKind::NoPlanRecord);
    }

    #[trace("FR-200-AC-2")]
    #[test]
    fn missing_policy_has_two_gaps_and_no_example_levels_or_cells() {
        let output = generated(&[]).unwrap();
        assert_eq!(
            output
                .gaps
                .iter()
                .map(|gap| gap.gap_kind)
                .collect::<Vec<_>>(),
            [GapKind::NoAcceptabilityPolicy, GapKind::NoPlanRecord]
        );
        assert!(!output.body.contains("| Severity level |"));
        assert!(!output.body.contains("| Severity | Probability | Region |"));
    }

    #[trace("FR-200-AC-3")]
    #[test]
    fn every_policy_renders_in_id_order_without_selection_language() {
        let output = generated(&[policy("POL-Z", "false"), policy("POL-A", "true")]).unwrap();
        assert!(output.body.find("### POL-A").unwrap() < output.body.find("### POL-Z").unwrap());
        assert!(!output.body.contains("in force"));
        assert!(!output.body.contains("selected policy"));
    }

    #[trace("FR-200-AC-4")]
    #[test]
    fn missing_outside_flag_fails_instead_of_defaulting() {
        let error = generated(&[policy("POL-1", "")]).unwrap_err();
        assert!(error.detail.contains("no authored outside_acceptable flag"));
    }

    #[trace("FR-200-AC-5")]
    #[test]
    fn each_plan_section_states_the_same_missing_source() {
        let output = generated(&[]).unwrap();
        assert_eq!(
            output
                .body
                .matches("No authored plan record exists.")
                .count(),
            3
        );
    }

    #[trace("FR-200-AC-6")]
    #[test]
    fn authored_policy_output_is_byte_deterministic_and_has_no_risk_verdict() {
        let docs = [policy("POL-1", "false")];
        let first = generated(&docs).unwrap();
        let second = generated(&docs).unwrap();
        assert_eq!(first.body.as_bytes(), second.body.as_bytes());
        assert!(!first.body.contains("risk is acceptable"));
        assert!(!first.body.contains("risk is accepted"));
    }
}
