// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The dossier SOUP list, rendering the shared FR-144 source (FR-197).

use crate::markdown::{cell, line};
use crate::{DocumentSpec, Gap, GapKind, Generated, Generator, Inputs, SourceFailure};
use quoin_soup::DossierSource;
use std::fmt::Write as _;

pub(crate) struct SoupGenerator(pub(crate) fn(&Inputs) -> Result<DossierSource, SourceFailure>);

impl Generator for SoupGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "soup",
            id: "DOS-SOUP",
            title: "SOUP list",
            needs_as_of: true,
        }
    }

    fn requires_as_of(&self, inputs: &Inputs) -> bool {
        inputs.soup_binding.is_some()
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        if inputs.soup_binding.is_none() {
            return Ok(Generated {
                body: "# SOUP list\n\n".to_owned(),
                gaps: vec![Gap::new(
                    GapKind::NoClauseBinding,
                    "this document",
                    "no --soup-binding was supplied for quoin soup report",
                )],
            });
        }
        let source = (self.0)(inputs)?;
        let mut body = String::from("# SOUP list\n\n");
        if let Some(instant) = &inputs.as_of {
            let _ = write!(body, "As of: {}\n\n", line(instant));
        }
        body.push_str("| Component (purl) | Version | Serves | Known anomalies | Advisories and disposition | Policy findings and disposition | Support status | End of support |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n");
        let mut gaps = Vec::new();
        for row in source.rows {
            let _ = writeln!(
                body,
                "| {} |",
                row.cells
                    .iter()
                    .map(|text| cell(text))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
            let component = row.cells.first().map(String::as_str).unwrap_or_default();
            for finding in row.unresolved_findings {
                gaps.push(Gap::new(
                    GapKind::SoupUnresolvedAdvisory,
                    component,
                    finding,
                ));
            }
            if row.past_end_of_support {
                gaps.push(Gap::new(
                    GapKind::SoupPastEndOfSupport,
                    component,
                    "past end of support",
                ));
            }
            for field in row.not_stated {
                gaps.push(Gap::new(
                    GapKind::SoupFieldNotStated,
                    component,
                    format!("{field}: not stated or unknown"),
                ));
            }
        }
        body.push_str("\n## Unmatched findings and scans not run\n");
        // The shared source owns the appended sections and every status word.
        body.push_str(&source.appended);
        body.push('\n');
        Ok(Generated { body, gaps })
    }
}
