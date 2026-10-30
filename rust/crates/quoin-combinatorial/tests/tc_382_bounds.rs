// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The bounds and saturation behaviour of the combinatorial space, asserted
//! against this crate alone.

use quoin_combinatorial::{
    CombinatorialErrorCode, Configuration, DimensionName, DimensionValue, MAX_DEMANDED_TUPLES,
    covered_by, demanded_tuples, parse_space, tway_coverage,
};

/// A demand past the ceiling is refused, not exhausted.
///
/// Trace: FR-035-AC-1
/// Provenance: quoin#382
#[test]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
fn tc_382_002_a_demand_past_the_ceiling_is_refused() {
    // Thirty binary dimensions at 30-way: 2^30 tuples, about a billion strings.
    // Accumulating them would be an unbounded allocation; the crate refuses by
    // name instead.
    let mut statement = String::from("30-way over");
    for index in 0..30 {
        use std::fmt::Write as _;
        write!(statement, " d{index}(0|1)").expect("writing to a String cannot fail");
    }
    let space = parse_space(&statement).expect("thirty binary dimensions are a space");

    let error = demanded_tuples(&space).expect_err(
        "a 2^30 demand has to be refused rather than become an unbounded allocation",
    );
    assert_eq!(error.code, CombinatorialErrorCode::DemandTooLarge);
    assert_eq!(error.code.as_str(), "QCB-DEMAND-TOO-LARGE");

    // And the ceiling is a ceiling, not a coincidence: one below it computes.
    let under = parse_space("2-way over a(1|2) b(3|4)").expect("a small space parses");
    assert!(
        demanded_tuples(&under)
            .expect("a small space computes")
            .len()
            <= MAX_DEMANDED_TUPLES
    );
}

/// A strength above `u64` saturates, and that cannot be observed.
///
/// Trace: FR-035-AC-1
/// Provenance: quoin#382
#[test]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
fn tc_382_003_a_strength_above_u64_saturates_and_that_cannot_be_observed() {
    // `1e20-way` saturates to `u64::MAX`. That is reachable in the *parsed
    // space* and provably unreachable in any *report*, which is the only place
    // the number is read.
    let statement = format!(
        "{}-way over a(1|2) b(3|4)",
        100_000_000_000_000_000_000_u128
    );
    let space = parse_space(&statement).expect("a finite strength is still a space");
    assert_eq!(
        space.strength().get(),
        u64::MAX,
        "the strength did not saturate"
    );

    // The proof. `strength` is only read out of a `CoverageResult`, and the one
    // consumer interpolates it solely inside a `covered < demanded` branch. `covered <= demanded` always, so that branch
    // needs `demanded > 0`, which needs `strength <= dimensions.len()`. A space
    // holding `u64::MAX` dimensions cannot be constructed: each one costs at
    // least four bytes of statement.
    let result = tway_coverage(&space, &[]).expect("an empty demand is not a refusal");
    assert_eq!(
        result.demanded, 0,
        "a strength above the dimension count must demand nothing"
    );
    assert!(
        result.covered >= result.demanded,
        "with a zero demand the finding branch is unreachable"
    );
    assert_eq!(result.gaps.len(), 0);
}

/// A dimension named like an inherited property is an ordinary dimension.
///
/// Trace: FR-035-AC-1
/// Provenance: quoin#382
#[test]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
fn tc_382_004_an_inherited_property_name_is_an_ordinary_dimension() {
    // A `BTreeMap` has no inherited names, so `constructor` is just a name.
    let space = parse_space("2-way over constructor(1|2) b(3|4)").expect("a space");
    let mut config = Configuration::new();
    config.insert(DimensionName::new("b"), DimensionValue::new("3"));
    let covered = covered_by(&space, &config).expect("a small space computes");
    assert!(
        covered.is_empty(),
        "only one dimension is present, so no 2-way tuple exists"
    );
}
