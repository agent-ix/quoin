// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The covering-array invariants, as properties over generated spaces.
//!
//! quoin#382 asks for properties **and not only example cases**, and the reason
//! is specific: the golden corpus can only say that thirty-one particular
//! statements produce thirty-one particular answers. It cannot say that a full
//! factorial run always closes every gap, or that adding a configuration never
//! *removes* coverage — statements about all spaces, which no list of examples
//! makes.
//!
//! Each property below is an algebraic fact about covering arrays rather than a
//! restatement of the implementation. Where a property could be satisfied
//! vacuously (an empty demand satisfies almost everything), the test carries an
//! explicit anti-vacuity assertion that fails at zero.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use proptest::prelude::{Just, Strategy, prop};
use proptest::{prop_assert, prop_assert_eq, prop_assume, proptest};
use quoin_combinatorial::{
    Configuration, ConfigurationSpace, DimensionName, DimensionValue, TupleKey, covered_by,
    demanded_tuples, parse_space, tway_coverage,
};

/// A generated space: the statement that declares it, and the space itself.
#[derive(Debug, Clone)]
struct Generated {
    statement: String,
    space: ConfigurationSpace,
}

/// Every configuration the space admits, exclusions and all.
///
/// Deliberately **not** exclusion-aware: it is the unconstrained cross product,
/// so a property using it exercises the excluded region too.
fn full_factorial(space: &ConfigurationSpace) -> Vec<Configuration> {
    let mut out = vec![Configuration::new()];
    for dimension in space.dimensions() {
        let mut next = Vec::new();
        for partial in &out {
            for value in dimension.values() {
                let mut extended = partial.clone();
                extended.insert(dimension.name().clone(), value.clone());
                next.push(extended);
            }
        }
        out = next;
    }
    out
}

/// Statements of two to four dimensions, two to three values each, strength
/// within the dimension count, and up to two exclusions over distinct
/// dimensions.
///
/// Small on purpose: the invariants are not size-dependent, and a generator
/// that spent its budget on enormous spaces would test the resource ceiling
/// (`tc_382_parity.rs` does that) instead of the algebra.
fn generated_space(max_exclusions: usize) -> impl Strategy<Value = Generated> {
    (2_usize..=4, prop::collection::vec(2_usize..=3, 4))
        .prop_flat_map(move |(dimension_count, value_counts)| {
            let strength = 1_usize..=dimension_count;
            let exclusions = prop::collection::vec(
                (
                    0_usize..dimension_count,
                    0_usize..dimension_count,
                    0_usize..3,
                    0_usize..3,
                ),
                0..=max_exclusions,
            );
            (
                Just(dimension_count),
                Just(value_counts),
                strength,
                exclusions,
            )
        })
        .prop_filter_map(
            "the drawn exclusions must name two distinct dimensions and legal values",
            |(dimension_count, value_counts, strength, exclusions)| {
                let mut statement = format!("{strength}-way over");
                for index in 0..dimension_count {
                    let values = *value_counts.get(index)?;
                    write!(statement, " d{index}(").map_err(|_| ()).ok()?;
                    for value in 0..values {
                        if value > 0 {
                            statement.push('|');
                        }
                        write!(statement, "v{value}").map_err(|_| ()).ok()?;
                    }
                    statement.push(')');
                }
                for (left, right, left_value, right_value) in exclusions {
                    if left == right {
                        continue;
                    }
                    if left_value >= *value_counts.get(left)?
                        || right_value >= *value_counts.get(right)?
                    {
                        continue;
                    }
                    write!(
                        statement,
                        " excluding[d{left}=v{left_value},d{right}=v{right_value}]"
                    )
                    .map_err(|_| ())
                    .ok()?;
                }
                let space = parse_space(&statement)?;
                Some(Generated { statement, space })
            },
        )
}

proptest! {
    /// Every gap is a demanded tuple no run reached, and the three numbers
    /// account for each other exactly.
    ///
    /// Trace: FR-035-AC-1
    /// Provenance: quoin#382
    #[test]
    fn tc_382_010_demanded_equals_covered_plus_gaps(
        generated in generated_space(2),
        take in 0_usize..16,
    ) {
        let space = &generated.space;
        let configs: Vec<Configuration> = full_factorial(space).into_iter().take(take).collect();
        let result = tway_coverage(space, &configs).map_err(|error| {
            proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
        })?;

        prop_assert_eq!(
            result.demanded,
            result.covered + result.gaps.len(),
            "{}: the three numbers must account for each other",
            generated.statement
        );
        prop_assert!(result.covered <= result.demanded);

        // Anti-vacuity: a space always demands something, because the generator
        // only produces strengths within the dimension count. If this ever
        // holds at zero, every assertion above is trivially true.
        prop_assert!(result.demanded > 0, "{}: demanded nothing", generated.statement);
    }

    /// The gap list is ordered, and by UTF-16 code unit specifically.
    ///
    /// Trace: FR-035-AC-1
    /// Provenance: quoin#382
    #[test]
    fn tc_382_011_gaps_are_strictly_ordered(generated in generated_space(2)) {
        let result = tway_coverage(&generated.space, &[]).map_err(|error| {
            proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
        })?;
        prop_assert!(!result.gaps.is_empty(), "{}: no gaps to order", generated.statement);
        for pair in result.gaps.windows(2) {
            let (Some(left), Some(right)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            prop_assert!(left < right, "{left} is not before {right}");
        }
    }

    /// A full factorial run closes every gap.
    ///
    /// This is the covering-array statement proper: the exhaustive array is a
    /// covering array of every strength, so a space measured against it has no
    /// gap at all — whatever the strength, whatever the exclusions.
    ///
    /// Trace: FR-035-AC-1
    /// Provenance: quoin#382
    #[test]
    fn tc_382_012_the_full_factorial_covers_everything_demanded(generated in generated_space(2)) {
        let space = &generated.space;
        let result = tway_coverage(space, &full_factorial(space)).map_err(|error| {
            proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
        })?;
        prop_assert!(
            result.gaps.is_empty(),
            "{}: the exhaustive array left {} gaps",
            generated.statement,
            result.gaps.len()
        );
        prop_assert_eq!(result.covered, result.demanded);
        prop_assert!(result.demanded > 0, "{}: demanded nothing", generated.statement);
    }

    /// Coverage is monotone: adding a configuration never uncovers a tuple.
    ///
    /// Trace: FR-035-AC-1
    /// Provenance: quoin#382
    #[test]
    fn tc_382_013_adding_a_configuration_never_removes_coverage(
        generated in generated_space(2),
        prefix in 0_usize..16,
        extra in 0_usize..16,
    ) {
        let space = &generated.space;
        let all = full_factorial(space);
        let head: Vec<Configuration> = all.iter().take(prefix).cloned().collect();
        let grown: Vec<Configuration> = all.iter().take(prefix + extra).cloned().collect();
        prop_assume!(grown.len() > head.len());

        let before = tway_coverage(space, &head).map_err(|error| {
            proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
        })?;
        let after = tway_coverage(space, &grown).map_err(|error| {
            proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
        })?;

        prop_assert!(after.covered >= before.covered);
        prop_assert!(after.gaps.len() <= before.gaps.len());
        let remaining: BTreeSet<&TupleKey> = after.gaps.iter().collect();
        for gap in &after.gaps {
            prop_assert!(
                before.gaps.contains(gap),
                "{gap} became a gap by adding configurations"
            );
        }
        prop_assert_eq!(remaining.len(), after.gaps.len(), "the gap list repeats a tuple");
    }

    /// No excluded combination is ever demanded, and so none is ever a gap.
    ///
    /// A gap list is a work list. Naming a combination the spec forbids would
    /// ask someone to run a configuration that is not allowed to exist.
    ///
    /// Trace: FR-035-AC-1
    /// Provenance: quoin#382
    #[test]
    fn tc_382_014_an_excluded_combination_is_never_demanded(generated in generated_space(2)) {
        let space = &generated.space;
        prop_assume!(!space.exclusions().is_empty());
        let demanded = demanded_tuples(space).map_err(|error| {
            proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
        })?;
        prop_assume!(!demanded.is_empty());

        let mut checked = 0_usize;
        for key in &demanded {
            let assignments: BTreeSet<&str> = key.as_str().split(" & ").collect();
            for exclusion in space.exclusions() {
                let forbidden: Vec<String> = exclusion
                    .assignments()
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect();
                if forbidden.iter().all(|pair| assignments.contains(pair.as_str())) {
                    return Err(proptest::test_runner::TestCaseError::fail(format!(
                        "{}: the demanded tuple {key} contains every assignment of a declared exclusion",
                        generated.statement
                    )));
                }
                checked += 1;
            }
        }
        prop_assert!(checked > 0, "no demanded tuple was checked against any exclusion");
    }

    /// With no exclusions, the demand is exactly the sum over t-subsets of the
    /// product of their value counts.
    ///
    /// The counting formula is computed here from the dimensions directly, not
    /// from the implementation's own walk, so agreement is evidence rather than
    /// a tautology.
    ///
    /// Trace: FR-035-AC-1
    /// Provenance: quoin#382
    #[test]
    fn tc_382_015_an_unconstrained_demand_matches_the_counting_formula(
        generated in generated_space(0),
    ) {
        let space = &generated.space;
        prop_assert!(space.exclusions().is_empty());
        let strength = usize::try_from(space.strength().get()).unwrap_or(usize::MAX);
        let sizes: Vec<usize> = space
            .dimensions()
            .iter()
            .map(|dimension| dimension.values().len())
            .collect();

        let mut expected = 0_usize;
        // Every subset of the dimension indices, by bit mask; the generator
        // caps the dimension count at four, so this is at most sixteen.
        for mask in 0_u32..(1_u32 << sizes.len()) {
            if mask.count_ones() as usize != strength {
                continue;
            }
            let mut product = 1_usize;
            for (index, size) in sizes.iter().enumerate() {
                if mask & (1 << index) != 0 {
                    product *= size;
                }
            }
            expected += product;
        }

        let demanded = demanded_tuples(space).map_err(|error| {
            proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
        })?;
        prop_assert_eq!(demanded.len(), expected, "{}", generated.statement);
        prop_assert!(expected > 0, "{}: the formula predicted nothing", generated.statement);
    }

    /// A configuration covers only tuples built from dimensions it actually
    /// named with a declared value.
    ///
    /// Trace: FR-035-AC-1
    /// Provenance: quoin#382
    #[test]
    fn tc_382_016_a_cover_never_names_a_value_the_configuration_did_not_run(
        generated in generated_space(2),
        take in 1_usize..8,
    ) {
        let space = &generated.space;
        let mut checked = 0_usize;
        for config in full_factorial(space).into_iter().take(take) {
            let covered = covered_by(space, &config).map_err(|error| {
                proptest::test_runner::TestCaseError::fail(format!("unexpected refusal: {error}"))
            })?;
            for key in &covered {
                for assignment in key.as_str().split(" & ") {
                    let Some((name, value)) = assignment.split_once('=') else {
                        return Err(proptest::test_runner::TestCaseError::fail(format!(
                            "{key} is not a list of assignments"
                        )));
                    };
                    let ran = config.get(&DimensionName::new(name));
                    prop_assert_eq!(
                        ran,
                        Some(&DimensionValue::new(value)),
                        "{}: {} claims a value the configuration did not run",
                        generated.statement,
                        key
                    );
                    checked += 1;
                }
            }
        }
        prop_assert!(checked > 0, "{}: no covered assignment was checked", generated.statement);
    }
}

/// The generator itself must produce spaces, or every property above is
/// vacuous.
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
fn tc_382_017_the_generator_produces_spaces_with_and_without_exclusions() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let strategy = generated_space(2);
    let mut with_exclusions = 0_usize;
    let mut strengths: BTreeSet<u64> = BTreeSet::new();
    for _ in 0..256 {
        let generated = strategy
            .new_tree(&mut runner)
            .expect("the generator produces a space")
            .current();
        if !generated.space.exclusions().is_empty() {
            with_exclusions += 1;
        }
        strengths.insert(generated.space.strength().get());
    }
    assert!(
        with_exclusions > 0,
        "no generated space ever declared an exclusion"
    );
    assert!(
        strengths.len() > 1,
        "every generated space had the same strength {strengths:?}"
    );
}
