// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The plain-text discharge render: `render_discharge_report` (FR-046).
//!
//! Presentation only — every value it prints was decided in
//! [`super::build`].

use super::report::{ClauseDischarge, DischargeReport};
use crate::argument::js_trim_end;

/// Render the same complete partition as compact, deterministic Markdown.
#[must_use]
pub fn render_discharge_report(report: &DischargeReport) -> String {
    let mut lines = vec![
        format!(
            "# Clause discharge: {}/{}@{}",
            report.clause_set.authority, report.clause_set.id, report.clause_set.version
        ),
        String::new(),
        format!("- Clause-set digest: `{}`", report.clause_set_digest),
        format!("- Evaluated as of: `{}`", report.as_of),
        String::new(),
    ];
    section(&mut lines, "Direct evidence", &report.binding.direct);
    section(
        &mut lines,
        "Approved dispositions",
        &report.binding.dispositions,
    );
    section(&mut lines, "Open binding clauses", &report.binding.open);
    section(&mut lines, "Unresolved applicability", &report.unresolved);
    section(&mut lines, "Not binding", &report.not_binding);
    if !report.unused_facts.is_empty() {
        lines.push("## Unused facts".to_owned());
        lines.push(String::new());
        for fact in &report.unused_facts {
            lines.push(format!(
                "- `{}` ({}): {}",
                fact.clause_id,
                fact.kind.as_str(),
                fact.reason.as_str()
            ));
        }
        lines.push(String::new());
    }
    let joined = lines.join("\n");
    format!("{}\n", js_trim_end(&joined))
}

/// One rendered section, `_None._` when the population is empty.
fn section(lines: &mut Vec<String>, title: &str, entries: &[ClauseDischarge]) {
    lines.push(format!("## {title}"));
    lines.push(String::new());
    if entries.is_empty() {
        // An empty population is stated, never omitted. FR-046's report exists
        // to expose every input population; a section that disappears when it
        // is empty reads as a complete report over a narrower input.
        lines.push("_None._".to_owned());
        lines.push(String::new());
        return;
    }
    for item in entries {
        let outputs = if item.expected_outputs.is_empty() {
            "no declared outputs".to_owned()
        } else {
            item.expected_outputs
                .iter()
                .map(|value| format!("`{value}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let reason = match &item.reason {
            Some(reason) => format!("; {reason}"),
            None => String::new(),
        };
        lines.push(format!(
            "- `{}` — {}; {outputs}{reason}",
            item.clause_id,
            item.force.as_str()
        ));
    }
    lines.push(String::new());
}
