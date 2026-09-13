// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What a set of executed configurations reached, and what it did not.
//!
//! Ports `CoverageResult` and `twayCoverage` of
//! `src/auditor/combinatorial.ts`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::CombinatorialError;
use crate::space::{ConfigurationSpace, DimensionName, DimensionValue, Strength};
use crate::tuple::{TupleKey, covered_by, demanded_tuples};

/// One executed configuration: a value per dimension it exercised.
pub type Configuration = BTreeMap<DimensionName, DimensionValue>;

/// The result of measuring a run against a declared space.
///
/// # The gap list is the point
///
/// A percentage tells someone how much is missing; the list tells them *which
/// combinations to run*, which is the difference between a number and an
/// action. `gaps` is therefore the whole list, not a sample, and it is ordered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageResult {
    /// The `t` the space declared.
    pub strength: Strength,
    /// How many tuples the space demands.
    pub demanded: usize,
    /// How many of those the executed configurations reached.
    pub covered: usize,
    /// Tuples no executed configuration reached, in stable order.
    pub gaps: Vec<TupleKey>,
}

/// Measure executed configurations against a declared space.
///
/// # Errors
///
/// [`CombinatorialErrorCode::DemandTooLarge`](crate::CombinatorialErrorCode::DemandTooLarge)
/// when the space demands more than
/// [`MAX_DEMANDED_TUPLES`](crate::MAX_DEMANDED_TUPLES) tuples.
pub fn tway_coverage(
    space: &ConfigurationSpace,
    configs: &[Configuration],
) -> Result<CoverageResult, CombinatorialError> {
    let demanded = demanded_tuples(space)?;
    let mut covered = std::collections::BTreeSet::new();
    for config in configs {
        for key in covered_by(space, config)? {
            // Only tuples the space actually demands count. A run exercising a
            // value the spec never declared covers nothing it was asked for —
            // and counting it would let a coverage number rise by testing
            // something else.
            if demanded.contains(&key) {
                covered.insert(key);
            }
        }
    }
    let gaps: Vec<TupleKey> = demanded
        .iter()
        .filter(|key| !covered.contains(*key))
        .cloned()
        .collect();
    Ok(CoverageResult {
        strength: space.strength(),
        demanded: demanded.len(),
        covered: covered.len(),
        gaps,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Configuration, tway_coverage};
    use crate::space::{DimensionName, DimensionValue, parse_space};

    fn config(pairs: &[(&str, &str)]) -> Configuration {
        pairs
            .iter()
            .map(|(name, value)| (DimensionName::new(*name), DimensionValue::new(*value)))
            .collect()
    }

    #[test]
    fn an_undeclared_value_raises_no_coverage_number() {
        let space = parse_space("2-way over a(1|2) b(3|4)").unwrap();
        let result = tway_coverage(&space, &[config(&[("a", "9"), ("b", "9")])]).unwrap();
        assert_eq!(result.demanded, 4);
        assert_eq!(result.covered, 0);
        assert_eq!(result.gaps.len(), 4);
    }

    #[test]
    fn a_full_array_leaves_no_gap_and_an_empty_run_leaves_every_one() {
        let space = parse_space("2-way over a(1|2) b(3|4)").unwrap();
        let full = [
            config(&[("a", "1"), ("b", "3")]),
            config(&[("a", "1"), ("b", "4")]),
            config(&[("a", "2"), ("b", "3")]),
            config(&[("a", "2"), ("b", "4")]),
        ];
        let result = tway_coverage(&space, &full).unwrap();
        assert_eq!((result.demanded, result.covered), (4, 4));
        assert!(result.gaps.is_empty());

        let empty = tway_coverage(&space, &[]).unwrap();
        assert_eq!((empty.demanded, empty.covered), (4, 0));
        assert_eq!(empty.gaps.len(), 4);
    }

    #[test]
    fn a_forbidden_tuple_is_neither_demanded_nor_a_gap() {
        let space = parse_space("2-way over a(1|2) b(3|4) excluding[a=1,b=3]").unwrap();
        let result = tway_coverage(&space, &[config(&[("a", "1"), ("b", "3")])]).unwrap();
        assert_eq!(result.demanded, 3);
        assert_eq!(result.covered, 0);
        assert!(
            !result.gaps.iter().any(|gap| gap.as_str() == "a=1 & b=3"),
            "an excluded tuple must never appear as a gap"
        );
    }
}
