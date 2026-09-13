// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The plain-text authored-argument render: `render_authored_argument`
//! (FR-047).
//!
//! Presentation only — every status it prints was decided in
//! [`super::build`].

use crate::argument::js_trim_end;

use super::view::{AuthoredArgumentView, ChallengeViewStatus, ViewStatus};

/// Render the view as deterministic markdown.
///
/// Byte-for-byte with the retained `renderAuthoredArgument`, including the
/// closing `trimEnd()` over JavaScript's whitespace set and the single newline
/// that follows it.
#[must_use]
pub fn render_authored_argument(view: &AuthoredArgumentView) -> String {
    let mut lines: Vec<String> = vec![
        format!("# {}: {}", view.argument.id, view.argument.title),
        String::new(),
        format!(
            "**{} {}** — {}",
            mark(view.top_claim.status == ViewStatus::Supported),
            view_status_upper(view.top_claim.status),
            view.top_claim.statement
        ),
        String::new(),
        format!("Subject: {}", view.top_claim.subject),
        format!("Owner: {}", view.argument.owner),
        format!("Evaluated as of: {}", view.as_of),
        String::new(),
    ];

    if !view.top_claim.reasons.is_empty() {
        lines.push("## Open reasons".to_owned());
        lines.push(String::new());
        for reason in &view.top_claim.reasons {
            lines.push(format!("- {reason}"));
        }
        lines.push(String::new());
    }

    lines.push("## Reasoning and sufficiency".to_owned());
    lines.push(String::new());
    for reasoning in &view.reasoning {
        lines.push(format!(
            "### {} {}",
            mark(reasoning.status == ViewStatus::Supported),
            reasoning.id
        ));
        lines.push(String::new());
        lines.push(reasoning.statement.clone());
        lines.push(String::new());
        for criterion in &reasoning.criteria {
            lines.push(format!(
                "- {} {}{}",
                mark(criterion.status == ViewStatus::Supported),
                criterion.criterion,
                suffix(criterion.reason.as_deref())
            ));
        }
        lines.push(String::new());
    }

    lines.push("## Assumptions".to_owned());
    lines.push(String::new());
    if view.assumptions.is_empty() {
        lines.push("_None._".to_owned());
        lines.push(String::new());
    }
    for assumption in &view.assumptions {
        lines.push(format!(
            "- {} `{}` ({}): {}{}",
            mark(assumption.status == ViewStatus::Supported),
            assumption.id,
            assumption.owner,
            assumption.statement,
            suffix(assumption.reason.as_deref())
        ));
    }
    if !view.assumptions.is_empty() {
        lines.push(String::new());
    }

    lines.push("## Challenges".to_owned());
    lines.push(String::new());
    if view.challenges.is_empty() {
        lines.push("_None._".to_owned());
        lines.push(String::new());
    }
    for challenge in &view.challenges {
        lines.push(format!(
            "- {} `{}` ({}): {}{}",
            mark(challenge.status == ChallengeViewStatus::Resolved),
            challenge.id,
            challenge.owner,
            challenge.statement,
            suffix(challenge.reason.as_deref())
        ));
    }
    if !view.challenges.is_empty() {
        lines.push(String::new());
    }

    lines.push("## Participants and authority".to_owned());
    lines.push(String::new());
    for participant in &view.participants {
        lines.push(format!(
            "- `{}` — {}; authority: {}; independence: {}",
            participant.id, participant.role, participant.authority, participant.independence
        ));
    }

    let body = lines.join("\n");
    format!("{}\n", js_trim_end(&body))
}

/// `state === "supported" || state === "resolved" ? "✓" : "◇"`.
///
/// Not `crate::render`'s mark, which is `✅`. Two renderers, two glyphs, and
/// the difference is in the retained source rather than an inconsistency to
/// tidy away.
fn mark(supported: bool) -> &'static str {
    if supported { "✓" } else { "◇" }
}

/// `status.toUpperCase()` over the two values the field can hold.
fn view_status_upper(status: ViewStatus) -> &'static str {
    match status {
        ViewStatus::Supported => "SUPPORTED",
        ViewStatus::Open => "OPEN",
    }
}

/// `reason ? ` — ${reason}` : ""`.
fn suffix(reason: Option<&str>) -> String {
    match reason {
        Some(text) => format!(" — {text}"),
        None => String::new(),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::super::fixtures::{AS_OF, argument, build, decision, with};
    use super::render_authored_argument;

    /// Trace: FR-047-AC-6
    /// Provenance: quoin#445
    #[test]
    fn tc_1136_renders_open_reasons_and_explicit_decision_state_deterministically() {
        let view = build(argument(), vec![], AS_OF);
        let first = render_authored_argument(&view);
        assert_eq!(first, render_authored_argument(&view));
        // The retained implementation's own bytes, recorded from `dist/` at
        // this revision. A `contains` assertion would have passed over a
        // renderer that lost a heading or a blank line.
        assert_eq!(
            first,
            "# AA-900: Synthetic widget release decision\n\
             \n\
             **◇ OPEN** — The bounded synthetic widget change is acceptable.\n\
             \n\
             Subject: widget revision 0123456789abcdef\n\
             Owner: release-owner\n\
             Evaluated as of: 2026-08-15T00:00:00.000Z\n\
             \n\
             ## Open reasons\n\
             \n\
             - one or more sufficiency criteria are open\n\
             \n\
             ## Reasoning and sufficiency\n\
             \n\
             ### ◇ ARG-900\n\
             \n\
             Argue from the explicitly reviewed clause disposition.\n\
             \n\
             - ◇ Every binding synthetic clause has a current disposition. — no sufficiency decision\n\
             \n\
             ## Assumptions\n\
             \n\
             - ✓ `ASM-900` (release-owner): The test environment represents the bounded target.\n\
             \n\
             ## Challenges\n\
             \n\
             - ✓ `CH-900` (release-owner): A bounded recovery case needed review.\n\
             \n\
             ## Participants and authority\n\
             \n\
             - `reviewer-900` — decision reviewer; authority: may accept or reject this synthetic release; independence: did not produce the implementation evidence\n"
        );
    }

    /// The supported render, which drops the whole "Open reasons" section.
    ///
    /// Trace: FR-047-AC-6
    /// Provenance: quoin#445
    #[test]
    fn tc_1136_the_supported_render_carries_no_open_reasons_section() {
        let rendered = render_authored_argument(&build(argument(), vec![decision()], AS_OF));
        assert_eq!(
            rendered,
            "# AA-900: Synthetic widget release decision\n\
             \n\
             **✓ SUPPORTED** — The bounded synthetic widget change is acceptable.\n\
             \n\
             Subject: widget revision 0123456789abcdef\n\
             Owner: release-owner\n\
             Evaluated as of: 2026-08-15T00:00:00.000Z\n\
             \n\
             ## Reasoning and sufficiency\n\
             \n\
             ### ✓ ARG-900\n\
             \n\
             Argue from the explicitly reviewed clause disposition.\n\
             \n\
             - ✓ Every binding synthetic clause has a current disposition.\n\
             \n\
             ## Assumptions\n\
             \n\
             - ✓ `ASM-900` (release-owner): The test environment represents the bounded target.\n\
             \n\
             ## Challenges\n\
             \n\
             - ✓ `CH-900` (release-owner): A bounded recovery case needed review.\n\
             \n\
             ## Participants and authority\n\
             \n\
             - `reviewer-900` — decision reviewer; authority: may accept or reject this synthetic release; independence: did not produce the implementation evidence\n"
        );
    }

    /// The empty-section text, which no fixture in the retained suite reached.
    ///
    /// Trace: FR-047-AC-6
    /// Provenance: quoin#445
    #[test]
    fn tc_1136_empty_assumptions_and_challenges_render_the_none_placeholder() {
        let bare = with(
            &with(&argument(), "assumptions", serde_json::json!([])),
            "challenges",
            serde_json::json!([]),
        );
        let rendered = render_authored_argument(&build(bare, vec![decision()], AS_OF));
        assert_eq!(
            rendered,
            "# AA-900: Synthetic widget release decision\n\
             \n\
             **✓ SUPPORTED** — The bounded synthetic widget change is acceptable.\n\
             \n\
             Subject: widget revision 0123456789abcdef\n\
             Owner: release-owner\n\
             Evaluated as of: 2026-08-15T00:00:00.000Z\n\
             \n\
             ## Reasoning and sufficiency\n\
             \n\
             ### ✓ ARG-900\n\
             \n\
             Argue from the explicitly reviewed clause disposition.\n\
             \n\
             - ✓ Every binding synthetic clause has a current disposition.\n\
             \n\
             ## Assumptions\n\
             \n\
             _None._\n\
             \n\
             ## Challenges\n\
             \n\
             _None._\n\
             \n\
             ## Participants and authority\n\
             \n\
             - `reviewer-900` — decision reviewer; authority: may accept or reject this synthetic release; independence: did not produce the implementation evidence\n"
        );
    }
}
