// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! EA-26: the build resolves an engineering-assurance revision that provides
//! `DecisionRule::holds_on_interval` (FR-108-AC-13).
//!
//! `quoin-modules`' `tc_1032_pin_agreement` holds the three pin copies to one
//! revision and one version label; this holds that version to `0.5.0` and
//! calls the capability itself, so a pin that resolved an older revision
//! fails here on a missing method's behaviour, not on a string. `make
//! rust-deny` (no new `bans.skip`) is a gate, not a test.
//!
//! Provenance: EA-26

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use engineering_assurance::measurement::{
    Comparator, ConfidenceLevel, DecisionRule, Interval, RuleEvaluationError,
};

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");
const CARGO_MANIFEST: &str = include_str!("../../../Cargo.toml");

/// Trace: FR-108-AC-13, TC-1959
/// Provenance: EA-26
#[test]
fn tc_1959_the_resolved_engineering_assurance_decides_on_an_interval() {
    let dependency = CARGO_MANIFEST
        .lines()
        .find(|line| line.starts_with("engineering-assurance = "))
        .expect("rust/Cargo.toml declares the engineering-assurance dependency");
    assert!(
        dependency.contains("version = \"=0.5.0\""),
        "the crate is pinned at =0.5.0: {dependency}"
    );
    assert!(
        DEFAULT_MODULES.contains("version: \"0.5.0\""),
        "default-modules.yaml's module entry is at 0.5.0"
    );

    let level = ConfidenceLevel::new(0.95).unwrap();
    let rule = DecisionRule::against_threshold(Comparator::Ge, 0.8)
        .unwrap()
        .with_interval_level(level)
        .unwrap();
    let interval = Interval::new(0.75, 0.99, level, "wilson").unwrap();
    // The point estimate 0.9 passes `ge 0.8`; the unfavourable bound 0.75 does
    // not, and `holds` refuses to stand in for it.
    assert_eq!(
        rule.holds_on_interval(0.9, Some(&interval), None),
        Ok(false)
    );
    assert!(matches!(
        rule.holds(0.9, None),
        Err(RuleEvaluationError::IntervalRequired { .. })
    ));
}
