// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The risk management plan document: the authored plan record and the authored
//! acceptability policy (FR-200, PLAT-1656, PLAT-1834).

use std::fmt::Write as _;

use quoin_quire::records::{Contract, Corpus, Document, Table};

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, unusable_input};
use crate::order::dossier_cmp;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The source named when a plan or policy record is unusable.
const SOURCE: &str = "acceptability policy and risk management plan";

/// The source to name for an unusable corpus: the one kind whose documents
/// broke their contract, or both kinds when files are unreadable or both fail.
fn unusable_source(corpus: &Corpus) -> &'static str {
    if !corpus.unreadable.is_empty() || !corpus.malformed.is_empty() {
        return SOURCE;
    }
    let mut kinds = corpus
        .documents
        .iter()
        .filter(|document| matches!(document.contract, Contract::Violated(_)))
        .map(|document| document.kind.as_str())
        .collect::<Vec<_>>();
    kinds.sort_unstable();
    kinds.dedup();
    match kinds.as_slice() {
        ["risk_management_plan"] => "risk management plan",
        ["acceptability_policy"] => "acceptability policy",
        _ => SOURCE,
    }
}

/// Reads every selected acceptability policy and risk management plan record.
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
        if let Some(failure) = unusable_input(unusable_source(&corpus), &corpus) {
            return Err(failure);
        }
        let of_kind = |kind: &str| -> Vec<&Document> {
            corpus
                .documents
                .iter()
                .filter(|document| document.kind == kind)
                .collect()
        };
        render(
            &of_kind("acceptability_policy"),
            &of_kind("risk_management_plan"),
        )
    }
}

/// One plan section: its heading in the document, the plan table that fills
/// it, and the columns that table must carry.
struct PlanSection {
    heading: &'static str,
    table: &'static str,
    columns: &'static [&'static str],
}

const PLAN_SECTIONS: [PlanSection; 4] = [
    PlanSection {
        heading: "Scope",
        table: "Scope",
        columns: &["Scope"],
    },
    PlanSection {
        heading: "Responsibilities",
        table: "Responsibilities",
        columns: &["Responsibility", "Assigned to"],
    },
    PlanSection {
        heading: "Verification of risk controls",
        table: "Control verification methods",
        columns: &["Method", "Applies to"],
    },
    PlanSection {
        heading: "Production and post-production methods",
        table: "Production and post-production methods",
        columns: &["Activity", "Method"],
    },
];

fn render(policies: &[&Document], plans: &[&Document]) -> Result<Generated, SourceFailure> {
    let mut policies = policies.to_vec();
    policies.sort_by(|left, right| dossier_cmp(&left.id, &right.id));
    let mut plans = plans.to_vec();
    plans.sort_by(|left, right| dossier_cmp(&left.id, &right.id));
    let mut body = String::from("# Risk management plan\n");
    let mut gaps = Vec::new();
    for section in &PLAN_SECTIONS {
        let _ = write!(body, "\n## {}\n", section.heading);
        if plans.is_empty() {
            body.push_str("\nNo authored plan record exists.\n");
        }
        for plan in &plans {
            let table = required_table(plan, section.table)?;
            validate_plan_table(plan, section, table)?;
            let _ = write!(body, "\n### {}\n\n", cell(&plan.id));
            render_table(&mut body, section.table, table);
        }
    }
    body.push_str("\n## Acceptability policy\n");
    if policies.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoAcceptabilityPolicy,
            "this document",
            "no acceptability policy record exists in scope",
        ));
    }
    if plans.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoPlanRecord,
            "this document",
            "no authored risk management plan record exists",
        ));
    }
    for plan in &plans {
        let referenced = referenced_policies(plan, &policies);
        if referenced.is_empty() {
            gaps.push(Gap::new(
                GapKind::NoAcceptabilityPolicy,
                plan.id.as_str(),
                "the plan references no acceptability policy record in scope",
            ));
        } else {
            let ids: Vec<_> = referenced.iter().map(|id| cell(id)).collect();
            let _ = write!(
                body,
                "\nPlan {} references the acceptability criteria {}.\n",
                cell(&plan.id),
                ids.join(", ")
            );
        }
    }
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
            return Err(invalid_record(
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
    Ok(Generated {
        body,
        document_version: None,
        gaps,
    })
}

/// The ids of the policies in scope that the plan names with a `references`
/// edge, in id order. The plan holds a reference to criteria people wrote and
/// never the criteria themselves.
fn referenced_policies<'a>(plan: &Document, policies: &[&'a Document]) -> Vec<&'a str> {
    policies
        .iter()
        .filter(|policy| {
            plan.edges
                .iter()
                .any(|(target, kind)| kind == "references" && *target == policy.id)
        })
        .map(|policy| policy.id.as_str())
        .collect()
}

/// A plan table must carry its columns and at least one row with no blank
/// cell: an unfilled plan row is refused, never rendered as if it were planned.
fn validate_plan_table(
    document: &Document,
    section: &PlanSection,
    table: &Table,
) -> Result<(), SourceFailure> {
    validate_table(document, section.table, table, section.columns)?;
    if table.rows.is_empty()
        || table
            .rows
            .iter()
            .flatten()
            .any(|value| value.trim().is_empty())
    {
        return Err(invalid_record(
            document,
            &format!("the {} table has no row or a blank cell", section.table),
        ));
    }
    Ok(())
}

fn required_table<'a>(document: &'a Document, name: &str) -> Result<&'a Table, SourceFailure> {
    document
        .tables
        .get(name)
        .ok_or_else(|| invalid_record(document, &format!("missing {name} table")))
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
        return Err(invalid_record(document, &format!("invalid {name} table")));
    }
    Ok(())
}

fn invalid_record(document: &Document, detail: &str) -> SourceFailure {
    let source = if document.kind == "risk_management_plan" {
        "risk management plan"
    } else {
        "acceptability policy"
    };
    SourceFailure {
        source: source.to_owned(),
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
        generated_with(docs, &[])
    }

    fn generated_with(
        policies: &[Document],
        plans: &[Document],
    ) -> Result<Generated, SourceFailure> {
        let policies: Vec<_> = policies.iter().collect();
        let plans: Vec<_> = plans.iter().collect();
        render(&policies, &plans)
    }

    fn plan(id: &str, references: &[&str]) -> Document {
        let mut document = policy(id, "false");
        document.kind = "risk_management_plan".to_owned();
        document.edges = references
            .iter()
            .map(|target| ((*target).to_owned(), "references".to_owned()))
            .collect();
        document.tables = BTreeMap::from([
            (
                "Scope".to_owned(),
                table(&["Scope"], &[&["Brake controller"]]),
            ),
            (
                "Responsibilities".to_owned(),
                table(
                    &["Responsibility", "Assigned to"],
                    &[&["Keep the risk file current", "Safety lead"]],
                ),
            ),
            (
                "Control verification methods".to_owned(),
                table(
                    &["Method", "Applies to"],
                    &[&["Test against the control requirements", "Every control"]],
                ),
            ),
            (
                "Production and post-production methods".to_owned(),
                table(
                    &["Activity", "Method"],
                    &[&["Review field reports", "Quarterly review"]],
                ),
            ),
        ]);
        document
    }

    #[trace("FR-200-AC-1")]
    #[test]
    fn renders_authored_levels_regions_flags_and_cells_with_the_plan_gap() {
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
            4
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

    #[trace("FR-200-AC-7")]
    #[test]
    fn an_authored_plan_renders_its_four_tables_and_clears_the_plan_gap() {
        let output =
            generated_with(&[policy("POL-1", "true")], &[plan("RMP-1", &["POL-1"])]).unwrap();
        assert!(output.body.contains("### RMP-1"));
        assert!(output.body.contains("| Brake controller |"));
        assert!(
            output
                .body
                .contains("| Keep the risk file current | Safety lead |")
        );
        assert!(
            output
                .body
                .contains("| Test against the control requirements | Every control |")
        );
        assert!(
            output
                .body
                .contains("| Review field reports | Quarterly review |")
        );
        assert!(!output.body.contains("No authored plan record exists."));
        assert!(output.gaps.is_empty());
    }

    #[trace("FR-200-AC-8")]
    #[test]
    fn the_plan_references_its_criteria_and_does_not_copy_or_choose_them() {
        let output = generated_with(
            &[policy("POL-A", "true"), policy("POL-B", "false")],
            &[plan("RMP-1", &["POL-B"])],
        )
        .unwrap();
        assert!(
            output
                .body
                .contains("Plan RMP-1 references the acceptability criteria POL-B.")
        );
        assert!(!output.body.contains("criteria POL-A"));
        assert!(output.body.find("### POL-A").unwrap() < output.body.find("### POL-B").unwrap());
    }

    #[trace("FR-200-AC-9")]
    #[test]
    fn a_plan_that_references_no_policy_in_scope_has_a_gap_naming_the_plan() {
        let output = generated_with(&[policy("POL-1", "true")], &[plan("RMP-1", &[])]).unwrap();
        assert_eq!(output.gaps.len(), 1);
        assert_eq!(output.gaps[0].gap_kind, GapKind::NoAcceptabilityPolicy);
        assert_eq!(output.gaps[0].subject, "RMP-1");
        let dangling =
            generated_with(&[policy("POL-1", "true")], &[plan("RMP-1", &["POL-9"])]).unwrap();
        assert_eq!(dangling.gaps.len(), 1);
        assert!(
            !dangling
                .body
                .contains("references the acceptability criteria")
        );
    }

    #[trace("FR-200-AC-10")]
    #[test]
    fn an_unfilled_plan_fails_instead_of_rendering() {
        let mut blank = plan("RMP-1", &["POL-1"]);
        blank.tables.insert(
            "Responsibilities".to_owned(),
            table(
                &["Responsibility", "Assigned to"],
                &[&["Keep the risk file", ""]],
            ),
        );
        let error = generated_with(&[policy("POL-1", "true")], &[blank]).unwrap_err();
        assert!(error.detail.contains("Responsibilities"));
        assert_eq!(error.source, "risk management plan");
        let mut missing = plan("RMP-1", &["POL-1"]);
        missing.tables.remove("Scope");
        assert!(generated_with(&[policy("POL-1", "true")], &[missing]).is_err());
        let mut empty = plan("RMP-1", &["POL-1"]);
        empty
            .tables
            .insert("Scope".to_owned(), table(&["Scope"], &[]));
        assert!(generated_with(&[policy("POL-1", "true")], &[empty]).is_err());
    }

    #[trace("FR-200-AC-11")]
    #[test]
    fn plans_render_in_id_order_deterministically_without_a_verdict() {
        let docs = [plan("RMP-Z", &["POL-1"]), plan("RMP-A", &["POL-1"])];
        let policies = [policy("POL-1", "false")];
        let first = generated_with(&policies, &docs).unwrap();
        let second = generated_with(&policies, &docs).unwrap();
        assert_eq!(first.body.as_bytes(), second.body.as_bytes());
        assert!(first.body.find("### RMP-A").unwrap() < first.body.find("### RMP-Z").unwrap());
        for phrase in [
            "in force",
            "approved",
            "risk is acceptable",
            "risk is accepted",
        ] {
            assert!(!first.body.contains(phrase), "{phrase}");
        }
    }

    #[trace("FR-200-AC-13")]
    #[test]
    fn plans_and_policies_sort_in_dossier_order_not_byte_order() {
        let policies = [policy("POL-10", "false"), policy("POL-2", "false")];
        let plans = [plan("RMP-10", &["POL-2"]), plan("RMP-2", &["POL-10"])];
        let output = generated_with(&policies, &plans).unwrap();
        assert!(output.body.find("### RMP-2").unwrap() < output.body.find("### RMP-10").unwrap());
        assert!(output.body.find("### POL-2").unwrap() < output.body.find("### POL-10").unwrap());
        assert!(
            output.body.find("Plan RMP-2 references").unwrap()
                < output.body.find("Plan RMP-10 references").unwrap()
        );
        let unreferenced = [plan("RMP-10", &[]), plan("RMP-2", &[])];
        let gaps = generated_with(&policies, &unreferenced).unwrap().gaps;
        let subjects: Vec<_> = gaps.iter().map(|gap| gap.subject.as_str()).collect();
        assert_eq!(subjects, ["RMP-2", "RMP-10"]);
    }

    #[trace("FR-200-AC-9")]
    #[test]
    fn a_plan_with_no_policy_in_scope_has_the_document_gap_then_the_plan_gap() {
        let output = generated_with(&[], &[plan("RMP-1", &[])]).unwrap();
        let gaps: Vec<_> = output
            .gaps
            .iter()
            .map(|gap| (gap.gap_kind, gap.subject.as_str()))
            .collect();
        assert_eq!(
            gaps,
            [
                (GapKind::NoAcceptabilityPolicy, "this document"),
                (GapKind::NoAcceptabilityPolicy, "RMP-1"),
            ]
        );
    }
}
