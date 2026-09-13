// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Tuple keys, index combinations, demand and cover.
//!
//! Ports `combinations` … `coveredBy` of `src/auditor/combinatorial.ts`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::{CombinatorialError, CombinatorialErrorCode};
use crate::js::cmp_js;
use crate::space::{ConfigurationSpace, DimensionName, DimensionValue};

/// The largest demand set this crate will build before refusing.
///
/// The retained implementation accumulates into an unbounded `Set<string>`, so
/// a statement naming forty dimensions at `20-way` is a heap exhaustion with no
/// diagnostic. A named refusal at a ceiling is this workspace's rule for every
/// accumulator. See `DIVERGENCE.md`.
pub const MAX_DEMANDED_TUPLES: usize = 1_000_000;

/// The strength at which the ceiling is unreachable-by-construction.
///
/// Every [`Dimension`](crate::Dimension) carries at least two values by
/// construction, so a `t`-dimension combination demands at least `2^t` tuples.
/// `2^20 = 1_048_576`, which already exceeds [`MAX_DEMANDED_TUPLES`], so a
/// strength of 20 or more that is *reachable at all* (that is, no larger than
/// the dimension count) refuses whatever the values are. Checking it up front
/// bounds the value-walk's recursion depth at 19 rather than at the number of
/// dimensions, which the statement alone controls.
const REFUSED_STRENGTH: u64 = 20;

const _: () = assert!(
    (1_usize << REFUSED_STRENGTH) > MAX_DEMANDED_TUPLES,
    "REFUSED_STRENGTH must be the point where the ceiling is already exceeded"
);

/// A stable key for one t-way tuple, so covered and demanded compare directly.
///
/// Ordered by UTF-16 code unit, because the retained gap list is produced by
/// `Array.prototype.sort` with its default comparator. Holding that ordering in
/// the type means a [`BTreeSet<TupleKey>`] *is* the sorted gap list, rather than
/// a set someone must remember to sort correctly at each use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TupleKey(String);

impl TupleKey {
    /// The key's spelling: `d1=v1 & d2=v2`, dimensions in name order.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Build the key for one assignment set.
    ///
    /// The sort is by dimension name only and is **stable**, exactly as
    /// `Array.prototype.sort` is: two dimensions sharing a name keep the order
    /// the walk produced them in.
    fn of(pairs: &[(&DimensionName, &DimensionValue)]) -> Self {
        let mut sorted: Vec<&(&DimensionName, &DimensionValue)> = pairs.iter().collect();
        sorted.sort_by(|left, right| cmp_js(left.0.as_str(), right.0.as_str()));
        let mut key = String::new();
        for (index, (name, value)) in sorted.into_iter().enumerate() {
            if index > 0 {
                key.push_str(" & ");
            }
            key.push_str(name.as_str());
            key.push('=');
            key.push_str(value.as_str());
        }
        Self(key)
    }
}

impl PartialOrd for TupleKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TupleKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        cmp_js(&self.0, &other.0)
    }
}

impl std::fmt::Display for TupleKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// `C(n, size)`, saturating at one past the ceiling rather than overflowing.
fn binomial_bounded(n: usize, size: usize) -> u128 {
    if size > n {
        return 0;
    }
    let ceiling = MAX_DEMANDED_TUPLES as u128 + 1;
    let size = size.min(n - size);
    let mut result: u128 = 1;
    for step in 0..size {
        // `result * (n - step) / (step + 1)` is exact at every step because the
        // running product is always a binomial coefficient of a smaller `n`.
        result = result.saturating_mul((n - step) as u128) / (step as u128 + 1);
        if result >= ceiling {
            return ceiling;
        }
    }
    result
}

/// Every `size`-element subset of `0..n`, lexicographic.
///
/// Generated iteratively rather than by the retained recursive walk: the
/// recursion's depth is `size`, which a statement controls, and a stack
/// overflow is not a diagnostic. The sequence is identical.
fn combinations(n: usize, size: usize) -> Result<Vec<Vec<usize>>, CombinatorialError> {
    if size < 1 || size > n {
        return Ok(Vec::new());
    }
    let count = binomial_bounded(n, size);
    if count > MAX_DEMANDED_TUPLES as u128 {
        return Err(CombinatorialError::new(
            CombinatorialErrorCode::DemandTooLarge,
            format!(
                "a {size}-way demand over {n} dimensions needs more than {MAX_DEMANDED_TUPLES} index combinations"
            ),
        ));
    }

    let mut out = Vec::new();
    let mut current: Vec<usize> = (0..size).collect();
    loop {
        out.push(current.clone());
        // Advance to the lexicographically next combination: find the rightmost
        // slot that has not reached its own ceiling, bump it, and reset every
        // slot to its right to the smallest legal value.
        let mut slot = size;
        let advanced = loop {
            if slot == 0 {
                break false;
            }
            slot -= 1;
            let Some(value) = current.get(slot) else {
                break false;
            };
            if *value < n - size + slot {
                break true;
            }
        };
        if !advanced {
            return Ok(out);
        }
        if let Some(value) = current.get_mut(slot) {
            *value += 1;
        }
        for index in slot + 1..size {
            let previous = current.get(index - 1).copied().unwrap_or(0);
            if let Some(value) = current.get_mut(index) {
                *value = previous + 1;
            }
        }
    }
}

/// Do these assignments contain every assignment of some declared exclusion?
fn forbidden(space: &ConfigurationSpace, pairs: &[(&DimensionName, &DimensionValue)]) -> bool {
    space.exclusions().iter().any(|exclusion| {
        exclusion.assignments().iter().all(|(name, value)| {
            pairs
                .iter()
                .any(|(pair_name, pair_value)| *pair_name == name && *pair_value == value)
        })
    })
}

/// Every t-way tuple the space demands, as stable keys.
///
/// # Errors
///
/// [`CombinatorialErrorCode::DemandTooLarge`] when the declared space demands
/// more than [`MAX_DEMANDED_TUPLES`].
pub fn demanded_tuples(
    space: &ConfigurationSpace,
) -> Result<BTreeSet<TupleKey>, CombinatorialError> {
    let strength = space.strength().get();
    let dimension_count = space.dimensions().len();
    let Ok(size) = usize::try_from(strength) else {
        // A strength past `usize` cannot be no larger than the dimension count,
        // so the retained side produces an empty demand here and so does this.
        return Ok(BTreeSet::new());
    };
    if size > dimension_count {
        return Ok(BTreeSet::new());
    }
    if strength >= REFUSED_STRENGTH {
        return Err(CombinatorialError::new(
            CombinatorialErrorCode::DemandTooLarge,
            format!(
                "a {strength}-way demand over dimensions of at least two values each needs more than {MAX_DEMANDED_TUPLES} tuples"
            ),
        ));
    }

    let mut out = BTreeSet::new();
    let mut leaves: usize = 0;
    for indices in combinations(dimension_count, size)? {
        let selected: Vec<_> = indices
            .iter()
            .filter_map(|index| space.dimensions().get(*index))
            .collect();
        let mut accumulator: Vec<(&DimensionName, &DimensionValue)> = Vec::with_capacity(size);
        walk_values(space, &selected, 0, &mut accumulator, &mut leaves, &mut out)?;
    }
    Ok(out)
}

/// The retained inner `walk`: the cross product of the selected dimensions.
///
/// Depth is bounded by [`REFUSED_STRENGTH`], which the caller has already
/// enforced.
fn walk_values<'space>(
    space: &'space ConfigurationSpace,
    selected: &[&'space crate::space::Dimension],
    slot: usize,
    accumulator: &mut Vec<(&'space DimensionName, &'space DimensionValue)>,
    leaves: &mut usize,
    out: &mut BTreeSet<TupleKey>,
) -> Result<(), CombinatorialError> {
    let Some(dimension) = selected.get(slot) else {
        *leaves += 1;
        if *leaves > MAX_DEMANDED_TUPLES {
            return Err(CombinatorialError::new(
                CombinatorialErrorCode::DemandTooLarge,
                format!("the declared space demands more than {MAX_DEMANDED_TUPLES} tuples"),
            ));
        }
        if !forbidden(space, accumulator) {
            out.insert(TupleKey::of(accumulator));
        }
        return Ok(());
    };
    for value in dimension.values() {
        accumulator.push((dimension.name(), value));
        let result = walk_values(space, selected, slot + 1, accumulator, leaves, out);
        accumulator.pop();
        result?;
    }
    Ok(())
}

/// Every t-way tuple one executed configuration covers.
///
/// A configuration entry naming a dimension the space does not declare, or a
/// value that dimension cannot take, contributes nothing — the retained filter,
/// kept because a run exercising an undeclared value has covered nothing it was
/// asked for.
///
/// # Errors
///
/// [`CombinatorialErrorCode::DemandTooLarge`] when the present dimensions admit
/// more than [`MAX_DEMANDED_TUPLES`] index combinations.
pub fn covered_by(
    space: &ConfigurationSpace,
    config: &std::collections::BTreeMap<DimensionName, DimensionValue>,
) -> Result<BTreeSet<TupleKey>, CombinatorialError> {
    let present: Vec<(&DimensionName, &DimensionValue)> = space
        .dimensions()
        .iter()
        .filter_map(|dimension| {
            let value = config.get(dimension.name())?;
            dimension
                .values()
                .contains(value)
                .then_some((dimension.name(), value))
        })
        .collect();

    let Ok(size) = usize::try_from(space.strength().get()) else {
        return Ok(BTreeSet::new());
    };
    let mut out = BTreeSet::new();
    for indices in combinations(present.len(), size)? {
        let pairs: Vec<(&DimensionName, &DimensionValue)> = indices
            .iter()
            .filter_map(|index| present.get(*index).copied())
            .collect();
        out.insert(TupleKey::of(&pairs));
    }
    Ok(out)
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
    use super::{MAX_DEMANDED_TUPLES, binomial_bounded, combinations, demanded_tuples};
    use crate::space::parse_space;

    #[test]
    fn combinations_are_lexicographic_and_match_the_retained_walk() {
        let produced = combinations(4, 2).unwrap();
        assert_eq!(
            produced,
            vec![
                vec![0, 1],
                vec![0, 2],
                vec![0, 3],
                vec![1, 2],
                vec![1, 3],
                vec![2, 3]
            ]
        );
        assert!(combinations(2, 3).unwrap().is_empty());
        assert!(combinations(3, 0).unwrap().is_empty());
        assert_eq!(combinations(3, 3).unwrap(), vec![vec![0, 1, 2]]);
    }

    #[test]
    fn the_binomial_saturates_instead_of_overflowing() {
        assert_eq!(binomial_bounded(4, 2), 6);
        assert_eq!(binomial_bounded(2, 3), 0);
        // C(200, 100) is astronomically large; it must report the ceiling, not
        // wrap, and not take a century to compute.
        assert_eq!(binomial_bounded(200, 100), MAX_DEMANDED_TUPLES as u128 + 1);
    }

    #[test]
    fn a_strength_above_the_dimension_count_demands_nothing() {
        let space = parse_space("5-way over a(1|2) b(3|4)").unwrap();
        assert!(demanded_tuples(&space).unwrap().is_empty());
    }

    #[test]
    fn an_exclusion_removes_exactly_the_tuples_it_names() {
        let space = parse_space("2-way over a(1|2) b(3|4) excluding[a=1,b=3]").unwrap();
        let keys: Vec<String> = demanded_tuples(&space)
            .unwrap()
            .iter()
            .map(|key| key.as_str().to_owned())
            .collect();
        assert_eq!(keys, ["a=1 & b=4", "a=2 & b=3", "a=2 & b=4"]);
    }

    #[test]
    fn a_demand_past_the_ceiling_is_refused_by_name() {
        // Twenty-five binary dimensions at 25-way: 2^25 tuples.
        let mut dimensions = String::new();
        for index in 0..25 {
            use std::fmt::Write as _;
            write!(dimensions, "d{index}(0|1) ").unwrap();
        }
        let space = parse_space(&format!("25-way over {dimensions}")).unwrap();
        let error = demanded_tuples(&space).unwrap_err();
        assert_eq!(error.code.as_str(), "QCB-DEMAND-TOO-LARGE");
    }
}
