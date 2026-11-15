// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Architecture document generator (FR-193, PLAT-1437).

use std::collections::{BTreeMap, HashSet};

use quoin_quire::coverage::Request;
use quoin_quire::model::DesignElement;

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::order::dossier_cmp;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The Architecture generator.
pub struct ArchitectureGenerator;

impl Generator for ArchitectureGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "architecture",
            id: "DOS-ARCH",
            title: "Architecture",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let mut gaps = Vec::new();
        let mut body = String::from("# Architecture\n\n");

        // Load modules
        let modules = inputs
            .modules
            .iter()
            .map(|path| quoin_quire::ids::ModuleRoot::open(path))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SourceFailure {
                source: "modules".to_owned(),
                detail: format!("failed to open module: {error}"),
            })?;

        let module_selection = quoin_quire::ModuleSelection::Closed(modules);

        // Create request for design matrix
        let scope =
            quoin_quire::ids::ScopeRoot::open(&inputs.scope).map_err(|error| SourceFailure {
                source: "scope".to_owned(),
                detail: format!("failed to open scope: {error}"),
            })?;

        let request = Request {
            scope: scope.clone(),
            modules: module_selection.clone(),
        };

        // Load design matrix which contains both coverage and design elements
        let design_outcome =
            quoin_quire::design_matrix::compute(&request).map_err(|error| SourceFailure {
                source: "design_matrix".to_owned(),
                detail: format!("failed to compute design matrix: {error}"),
            })?;

        // Load ADR documents
        let corpus = quoin_quire::records::read(&inputs.scope, &module_selection, &["ADR"])
            .map_err(|error| SourceFailure {
                source: "adrs".to_owned(),
                detail: format!("failed to read ADR documents: {error}"),
            })?;

        let mut adrs: Vec<_> = corpus
            .documents
            .iter()
            .filter(|doc| doc.kind == "ADR")
            .collect();
        adrs.sort_by(|a, b| dossier_cmp(&a.id, &b.id));

        // Collect all design elements and group requirements by element
        let mut element_requirements: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut all_elements: BTreeMap<String, DesignElement> = BTreeMap::new();

        for entry in &design_outcome.design_matrix {
            if let Some(blocks) = &entry.design {
                for block in blocks {
                    for element in &block.elements {
                        all_elements
                            .entry(element.id.clone())
                            .or_insert_with(|| element.clone());

                        // Map requirements to this element
                        if let Some(req_id) = &entry.id {
                            element_requirements
                                .entry(element.id.clone())
                                .or_default()
                                .push(req_id.clone());
                        }
                    }
                }
            }
        }

        // Render Parts section (design elements)
        body.push_str("## Parts\n\n");
        if all_elements.is_empty() {
            body.push_str("None.\n\n");
        } else {
            body.push_str("| Part | Ports | Allocated or realized requirements |\n");
            body.push_str("|------|-------|-------------------------------------|\n");

            let mut sorted_elements: Vec<_> = all_elements.values().collect();
            sorted_elements.sort_by(|a, b| dossier_cmp(&a.id, &b.id));

            for element in sorted_elements {
                let requirements = element_requirements
                    .get(&element.id)
                    .map(|reqs| {
                        let mut sorted = reqs.clone();
                        sorted.sort_by(|a, b| dossier_cmp(a, b));
                        sorted.dedup();
                        sorted.join(", ")
                    })
                    .unwrap_or_default();

                body.push_str(&format!(
                    "| {} | | {} |\n",
                    markdown_escape(&element.id),
                    markdown_escape(&requirements)
                ));
            }
            body.push_str("\n");
        }

        // Render Connections section (empty for now)
        body.push_str("## Connections\n\nNone.\n\n");

        // Render Decisions section
        body.push_str("## Decisions\n\n");
        if adrs.is_empty() {
            body.push_str("None.\n\n");
        } else {
            body.push_str("| ID | Title | Status |\n");
            body.push_str("|----|-------|--------|\n");
            for adr in &adrs {
                let title = adr
                    .frontmatter
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let status = adr
                    .frontmatter
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                body.push_str(&format!(
                    "| {} | {} | {} |\n",
                    markdown_escape(&adr.id),
                    markdown_escape(title),
                    markdown_escape(status)
                ));
            }
            body.push_str("\n");
        }

        // Find requirements without design elements
        let mut requirements_with_design: HashSet<String> = HashSet::new();
        let mut all_requirements: HashSet<String> = HashSet::new();

        for entry in &design_outcome.design_matrix {
            if let Some(req_id) = &entry.id {
                all_requirements.insert(req_id.clone());
                if entry.design.is_some() {
                    requirements_with_design.insert(req_id.clone());
                }
            }
        }

        let mut requirements_without_design: Vec<String> = all_requirements
            .difference(&requirements_with_design)
            .cloned()
            .collect();
        requirements_without_design.sort_by(|a, b| dossier_cmp(a, b));

        // Render Requirements without a design element section
        body.push_str("## Requirements without a design element\n\n");
        if requirements_without_design.is_empty() {
            body.push_str("None.\n\n");
        } else {
            for req_id in &requirements_without_design {
                body.push_str(&format!("- {}\n", markdown_escape(req_id)));
                gaps.push(
                    Gap::new(
                        GapKind::NoDesignElement,
                        req_id,
                        "has no realizing design element",
                    )
                    .for_requirement(req_id),
                );
            }
            body.push_str("\n");
        }

        Ok(Generated { body, gaps })
    }
}

/// Escape Markdown table cell content so it cannot break table syntax.
fn markdown_escape(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
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

    #[trace("FR-193")]
    #[test]
    fn architecture_generator_has_correct_spec() {
        let generator = ArchitectureGenerator;
        let spec = generator.spec();
        assert_eq!(spec.kind, "architecture");
        assert_eq!(spec.id, "DOS-ARCH");
        assert_eq!(spec.title, "Architecture");
        assert!(!spec.needs_as_of);
    }
}
