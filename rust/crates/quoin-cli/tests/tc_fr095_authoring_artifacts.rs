// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Ordinary authoring contract assertions for the installed specification skill.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test failures should identify the broken authoring contract"
)]

use ix_trace_rs::trace;

const SPECIFY: &str = include_str!("../../../../skills/specify/SKILL.md");
const EVALS: &str = include_str!("../../../../spec/evals.md");

/// An ordinary contained FR request keeps assurance artifacts opt-in.
#[trace("FR-095-AC-2")]
#[test]
fn tc_fr095_ordinary_authoring_emits_no_unrequested_assurance_artifact() {
    let opt_in = SPECIFY
        .find("artifacts are opt-in")
        .expect("the authoring contract states assurance artifacts are opt-in");
    let no_unsolicited_artifacts = SPECIFY[opt_in..]
        .find("does\nnot receive an unsolicited profile, architecture description, or measurement plan")
        .expect("ordinary requests do not receive unsolicited assurance artifacts");
    assert!(no_unsolicited_artifacts > 0);

    let ordinary_eval = EVALS
        .lines()
        .find(|line| line.starts_with("| TC-EV-059 |"))
        .expect("TC-EV-059 exercises ordinary authoring with assurance installed");
    for expected in [
        "exactly one validated FR",
        "no assurance, architecture, measurement, or review artifact",
    ] {
        assert!(
            ordinary_eval.contains(expected),
            "TC-EV-059 must preserve the ordinary authoring expectation: {expected}"
        );
    }
}
