// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Ordinary `/specify` requests create only requested spec artifact types (FR-095).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a broken workflow contract is a failing test"
)]

use ix_trace_rs::trace;
use std::path::Path;

const OPT_IN_RULE: &str = "Assurance artifacts are opt-in: a low-impact request that asks for an ordinary requirement does not receive an unsolicited profile, architecture description, or measurement plan.";

fn ordinary_authoring_keeps_assurance_opt_in(source: &str) -> bool {
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized.contains("Author **only the artifact types the request calls for** — nothing more.")
        && normalized.contains(OPT_IN_RULE)
        && normalized.contains("\"Add the FR that satisfies US-003\" → one FR (traced to US-003).")
}

#[trace("FR-095-AC-2")]
#[test]
fn ordinary_authoring_does_not_add_unsolicited_assurance_artifacts() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../skills/specify/SKILL.md");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "read the shipped specify skill at {}: {error}",
            path.display()
        )
    });

    assert!(
        ordinary_authoring_keeps_assurance_opt_in(&source),
        "the shipped authoring workflow must scope output to requested artifact types and keep assurance artifacts opt-in"
    );

    let normalized_source = source.split_whitespace().collect::<Vec<_>>().join(" ");
    let without_the_opt_in_rule = normalized_source.replace(
        OPT_IN_RULE,
        "Assurance artifacts are created for ordinary requests.",
    );
    assert!(
        !ordinary_authoring_keeps_assurance_opt_in(&without_the_opt_in_rule),
        "the contract check must fail when the ordinary-request exclusion is removed"
    );
}
