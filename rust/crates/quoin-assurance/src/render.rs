// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rendering an assurance case (quoin#384, ported from `src/assurance/render.ts`).
//!
//! Deterministic markdown plus a mermaid graph. **A store view, never a
//! hand-maintained document** — every re-render over unchanged inputs produces
//! byte-identical output, so a diff is a change in the evidence and not in
//! somebody's prose.
//!
//! # The type budget, measured a second time
//!
//! [`crate::case`] records that sizing from the import list overstated
//! `build_case` by a factor of five. This module is the same instrument failing
//! the other way: `render.ts` imports one line, both of whose types already
//! existed, and then reads fourteen fields across three types it never names.
//! See [`quoin_evidence_types`] for the table.
//!
//! The lesson is in [`crate::case::AssuranceCase`]'s type parameters rather
//! than only in a comment: producer trust is opaque to `build_case` and read by
//! this module, and a reader asking which applies gets the answer from the
//! signature.

use quoin_evidence_types::{IndependenceAssessment, TrustAssessment};

use crate::case::{AssuranceCase, CaseNode, NodeKind, NodeStatus};

/// The label budget, in CODE POINTS. See [`label`].
const MAX_LABEL_LENGTH: usize = 80;

/// The case as read by this module: both assessments are real types here.
pub type RenderableCase = AssuranceCase<TrustAssessment, IndependenceAssessment>;

/// The internal view has two states and makes every unsupported branch open.
fn mark(status: NodeStatus) -> &'static str {
    match status {
        NodeStatus::Supported => "✅",
        NodeStatus::Open => "◇",
    }
}

/// Render the case as markdown.
#[must_use]
pub fn render_case(assurance: &RenderableCase) -> String {
    let mut lines: Vec<String> = vec!["# Assurance case".to_owned(), String::new()];

    if assurance.claims.is_empty() {
        // Not an empty case — no case. The distinction matters: a reader who
        // sees an empty document concludes there is nothing to argue, and the
        // truth is that nothing declared itself a claim.
        lines.push(
            "No document declares itself a top-level claim, so there is no case to".to_owned(),
        );
        lines.push("argue. Declare an `StR` (or pass `--claim-type`) and re-run.".to_owned());
        lines.push(String::new());
    }

    let open = assurance
        .claims
        .iter()
        .filter(|c| c.status == NodeStatus::Open)
        .count();
    if !assurance.claims.is_empty() {
        lines.push(format!(
            "{} claim(s), {open} with an open branch.",
            assurance.claims.len()
        ));
        lines.push(String::new());
    }

    for claim in &assurance.claims {
        lines.push(format!(
            "## {} {} — {}",
            mark(claim.status),
            claim.id,
            claim.statement
        ));
        lines.push(String::new());
        lines.extend(render_node(claim, 0));
        lines.push(String::new());
        lines.push("```mermaid".to_owned());
        lines.extend(mermaid_for(claim));
        lines.push("```".to_owned());
        lines.push(String::new());
    }

    if !assurance.unreachable.is_empty() {
        lines.push("## Reachable from no claim".to_owned());
        lines.push(String::new());
        lines.push(
            "These carry obligations and no claim argues over them. A case rendered".to_owned(),
        );
        lines.push("only over the reachable half would read as complete.".to_owned());
        lines.push(String::new());
        lines.extend(assurance.unreachable.iter().map(|id| format!("- {id}")));
        lines.push(String::new());
    }

    if !assurance.unreadable.is_empty() {
        lines.push("## Unreadable".to_owned());
        lines.push(String::new());
        lines.extend(
            assurance
                .unreadable
                .iter()
                .map(|u| format!("- {}: {}", u.path, u.reason)),
        );
        lines.push(String::new());
    }

    lines.extend(producer_trust_section(&assurance.producer_trust));

    // `(x?.length ?? 0) > 0` in the retained source: an absent list and an
    // EMPTY list both skip the section, so `Some(vec![])` must not emit a
    // heading with nothing under it. `unwrap_or(&[])` collapses the two
    // cases the way `?.` does, rather than branching on `is_some()`.
    lines.extend(independence_section(
        assurance.evidence_independence.as_deref().unwrap_or(&[]),
    ));
    lines.join("\n")
}

/// The producer-reliance section, or nothing when there is no reliance.
fn producer_trust_section(decisions: &[TrustAssessment]) -> Vec<String> {
    if decisions.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        "## Evidence-producer reliance".to_owned(),
        String::new(),
        "These decisions are context for the case; they do not make a claim supported.".to_owned(),
        String::new(),
    ];
    for decision in decisions {
        // `join(", ")` over a list of STRINGS. `TrustTrigger` is a string
        // union in the retained source, not an object — worth reading rather
        // than assuming, because a list of objects would render as
        // `[object Object]` there and the port would have had to reproduce it.
        let triggered = if decision.triggered_by.is_empty() {
            String::new()
        } else {
            format!(" — revalidate: {}", decision.triggered_by.join(", "))
        };
        lines.push(format!(
            "- **{} / {}** — {}{triggered}",
            decision.id, decision.use_id, decision.status
        ));
        if !decision.limitations.is_empty() {
            lines.push(format!(
                "  - limitations: {}",
                decision.limitations.join("; ")
            ));
        }
    }
    lines.push(String::new());
    lines
}

/// The independence section, or nothing when no check was selected.
fn independence_section(assessments: &[IndependenceAssessment]) -> Vec<String> {
    if assessments.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        "## Evidence independence".to_owned(),
        String::new(),
        "These are profile-selected relationship checks; they do not make a claim supported by themselves.".to_owned(),
        String::new(),
    ];
    for assessment in assessments {
        lines.push(format!(
            "- **{} / {} / {}** — {}",
            assessment.profile, assessment.requirement, assessment.obligation, assessment.status
        ));
        lines.push(format!("  - {}", assessment.summary));
        for dimension in &assessment.dimensions {
            let values = if dimension.values.is_empty() {
                "no stated values".to_owned()
            } else {
                dimension.values.join(", ")
            };
            let missing = if dimension.missing_suites.is_empty() {
                String::new()
            } else {
                format!("; missing on {}", dimension.missing_suites.join(", "))
            };
            lines.push(format!("  - {}: {values}{missing}", dimension.dimension));
        }
    }
    lines.push(String::new());
    lines
}

fn render_node(node: &CaseNode, depth: usize) -> Vec<String> {
    let indent = "  ".repeat(depth);
    let label = if node.kind == NodeKind::Solution {
        format!(
            "{indent}- {} **{}** — {}",
            mark(node.status),
            node.id,
            node.statement
        )
    } else {
        format!(
            "{indent}- {} {} — {}",
            mark(node.status),
            node.id,
            node.statement
        )
    };
    // The claim's own line is the `##` heading above, so depth 0 contributes
    // no bullet — only its `because` and its children.
    let mut out: Vec<String> = if depth == 0 { Vec::new() } else { vec![label] };
    if let Some(because) = &node.because {
        out.push(format!("{indent}  - ↳ {because}"));
    }
    for child in &node.children {
        out.extend(render_node(child, depth + 1));
    }
    out
}

/// The claim's subtree as a mermaid flowchart.
///
/// Four authoring rules. The first three produce a diagram that **fails to
/// draw** rather than one that draws wrongly:
///
/// - **Ids are sanitised.** `FR-001-AC-1` contains `-`, and mermaid reads a
///   bare hyphenated token as an edge fragment.
/// - **Labels are quoted**, because a statement containing `(` or `)` ends the
///   node shape early.
/// - **No `;` in label text** — it terminates the statement.
///
/// The fourth governs length rather than punctuation and fails the other way,
/// quietly: labels truncate on a **code point**, never on a UTF-16 code unit
/// (quoin#432). That one is free here — Rust has no way to express the defect,
/// since `String` cannot hold a lone surrogate, which is precisely why the
/// retained implementation had to be fixed before this module could match it.
fn mermaid_for(claim: &CaseNode) -> Vec<String> {
    let mut lines = vec!["flowchart TD".to_owned()];
    let mut seen: Vec<String> = Vec::new();
    walk(claim, &mut lines, &mut seen);
    lines
}

fn walk(node: &CaseNode, lines: &mut Vec<String>, seen: &mut Vec<String>) {
    let id = node_id(&node.id);
    if !seen.contains(&id) {
        seen.push(id.clone());
        let (open, close) = if node.kind == NodeKind::Solution {
            ("([", "])")
        } else {
            ("[", "]")
        };
        lines.push(format!("  {id}{open}\"{}\"{close}", label(node)));
    }
    for child in &node.children {
        walk(child, lines, seen);
        lines.push(format!("  {id} --> {}", node_id(&child.id)));
    }
}

/// `id.replace(/[^A-Za-z0-9]/g, "_")`.
///
/// One underscore per UTF-16 CODE UNIT, not per character, and that is not
/// pedantry. A JavaScript regex without the `u` flag matches code units, so an
/// astral character — two units — becomes TWO underscores there and would have
/// become one here. `chars()` was the obvious port and the wrong one.
///
/// Same defect family as quoin#432 one function below, from the opposite
/// direction: there the retained code counted units where it should have
/// counted characters, here it counts units correctly and the port must too.
/// The unit is a fact about the retained implementation either way, never a
/// preference.
fn node_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for c in id.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            for _ in 0..c.len_utf16() {
                out.push('_');
            }
        }
    }
    out
}

fn label(node: &CaseNode) -> String {
    let text = format!("{}: {}", node.id, node.statement);
    let sanitised: String = text
        .chars()
        .map(|c| match c {
            '"' | '`' => '\'',
            ';' => ',',
            other => other,
        })
        .collect();
    // `Array.from(...).slice(0, 80).join("")` — code points, not code units.
    // `chars()` is the same unit, so this is a direct correspondence rather
    // than an approximation of one.
    let truncated: String = sanitised.chars().take(MAX_LABEL_LENGTH).collect();
    if node.status == NodeStatus::Open {
        format!("{truncated} ◇")
    } else {
        truncated
    }
}
