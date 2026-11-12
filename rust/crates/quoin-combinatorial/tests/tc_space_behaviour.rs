// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Behaviour of the declared configuration space, asserted on live code.
//!
//! Every expectation here is derived from the space the test declares (a count
//! worked out from the dimensions, a tuple named from its own values). Nothing
//! is replayed from a recorded run.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_combinatorial::{
    Configuration, DimensionName, DimensionValue, TupleKey, demanded_tuples, parse_space,
    tway_coverage,
};

fn config(pairs: &[(&str, &str)]) -> Configuration {
    pairs
        .iter()
        .map(|(name, value)| (DimensionName::new(*name), DimensionValue::new(*value)))
        .collect()
}

/// A statement that is not a configuration space yields `None`; that is the
/// whole test for whether an obligation is combinatorial.
#[trace("FR-035-AC-2")]
#[test]
fn a_statement_that_is_not_a_configuration_space_yields_none() {
    for prose in [
        "The system SHALL reject malformed input.",
        "2-way interactions are tested by the team",
        "over a(1|2) b(3|4)",
        "",
    ] {
        assert!(
            parse_space(prose).is_none(),
            "{prose:?} is not a declared space"
        );
    }
    assert!(parse_space("2-way over a(1|2) b(3|4)").is_some());
}

/// An exclusion is read as an exclusion, never as a dimension.
#[trace("FR-035-AC-3")]
#[test]
fn an_exclusion_is_not_mistaken_for_a_dimension() {
    let space = parse_space("2-way over a(1|2) b(3|4) excluding[a=1,b=3]").unwrap();
    let names: Vec<&str> = space
        .dimensions()
        .iter()
        .map(|dimension| dimension.name().as_str())
        .collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(space.exclusions().len(), 1);
    let pairs: Vec<(&str, &str)> = space.exclusions()[0]
        .assignments()
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    assert_eq!(pairs, [("a", "1"), ("b", "3")]);
}

/// The demanded-tuple count is the number of value pairs across dimension
/// pairs, which this test counts by enumeration rather than by formula.
#[trace("FR-035-AC-4")]
#[test]
fn the_demanded_count_is_the_number_of_value_combinations() {
    let sizes = [3usize, 2, 2];
    let space = parse_space("2-way over a(1|2|3) b(x|y) c(p|q)").unwrap();
    let mut expected = 0usize;
    for first in 0..sizes.len() {
        for second in (first + 1)..sizes.len() {
            for _ in 0..sizes[first] {
                for _ in 0..sizes[second] {
                    expected += 1;
                }
            }
        }
    }
    assert_eq!(demanded_tuples(&space).unwrap().len(), expected);
    assert_eq!(tway_coverage(&space, &[]).unwrap().demanded, expected);
}

/// A combination the space forbids is not demanded, and is not a gap.
#[trace("FR-035-AC-5")]
#[test]
fn a_forbidden_combination_is_not_demanded() {
    let allowed = parse_space("2-way over a(1|2) b(3|4)").unwrap();
    let forbidden = parse_space("2-way over a(1|2) b(3|4) excluding[a=1,b=3]").unwrap();
    let demanded = demanded_tuples(&forbidden).unwrap();
    assert_eq!(demanded.len(), demanded_tuples(&allowed).unwrap().len() - 1);
    assert!(!demanded.iter().any(|key| key.as_str() == "a=1 & b=3"));
    let result = tway_coverage(&forbidden, &[]).unwrap();
    assert!(!result.gaps.iter().any(|key| key.as_str() == "a=1 & b=3"));
}

/// The result names the combinations that never ran, not only how many.
#[trace("FR-035-AC-6")]
#[test]
fn the_result_names_the_combinations_that_never_ran() {
    let space = parse_space("2-way over a(1|2) b(3|4)").unwrap();
    let ran = [
        config(&[("a", "1"), ("b", "3")]),
        config(&[("a", "2"), ("b", "4")]),
    ];
    let result = tway_coverage(&space, &ran).unwrap();
    assert_eq!((result.demanded, result.covered), (4, 2));
    let named: Vec<&str> = result.gaps.iter().map(TupleKey::as_str).collect();
    assert_eq!(named, ["a=1 & b=4", "a=2 & b=3"]);
}

/// A configuration exercising values the spec never declared covers nothing.
#[trace("FR-035-AC-7")]
#[test]
fn values_the_spec_never_declared_cover_nothing() {
    let space = parse_space("2-way over a(1|2) b(3|4)").unwrap();
    let undeclared = [
        config(&[("a", "9"), ("b", "9")]),
        config(&[("zzz", "1"), ("b", "3")]),
    ];
    let result = tway_coverage(&space, &undeclared).unwrap();
    assert_eq!(result.covered, 0);
    assert_eq!(result.gaps.len(), result.demanded);
}

/// Full coverage reports an empty gap list.
#[trace("FR-035-AC-8")]
#[test]
fn full_coverage_reports_an_empty_gap_list() {
    let space = parse_space("2-way over a(1|2) b(3|4)").unwrap();
    let all = [
        config(&[("a", "1"), ("b", "3")]),
        config(&[("a", "1"), ("b", "4")]),
        config(&[("a", "2"), ("b", "3")]),
        config(&[("a", "2"), ("b", "4")]),
    ];
    let result = tway_coverage(&space, &all).unwrap();
    assert_eq!(result.covered, result.demanded);
    assert_eq!(result.gaps.len(), 0);
}
