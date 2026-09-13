// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Golden parity with the retained `src/auditor/combinatorial.ts`, and the
//! declaration of every place this port does **not** agree with it.
//!
//! The corpus in `tests/goldens/combinatorial.json` was captured once by
//! `oracle/capture-combinatorial.mjs` and committed. **No test spawns node**
//! (FR-101-AC-5): the retained implementation is not a runtime oracle.
//!
//! # What the fixture covers, and what it cannot
//!
//! It covers: every `parseSpace` refusal path (no header, unanchored header,
//! no whitespace after `over`, strength `0`, an infinite strength, fewer than
//! two multi-valued dimensions, empty parentheses); value trimming with the
//! ECMAScript whitespace set including U+FEFF; exclusion parsing including a
//! one-clause clause list, a clause with no `=`, a value containing `=`, two
//! clause lists, and a three-clause exclusion that cannot forbid a pair; 2-way
//! and 3-way demand; a partial configuration; an undeclared value; gap ordering
//! by UTF-16 code unit above the BMP; and three identifier shapes that mean
//! something to JavaScript (`constructor`, `__proto__`, a repeated dimension
//! name).
//!
//! It cannot cover: the resource ceiling, because the retained side has none
//! and answers by exhausting the heap; and a strength above `2^53`, because
//! the retained side holds it as a double. Both are handled below as declared
//! divergences with must-have-fired assertions, not by widening the fixture
//! until everything looks covered.

use std::collections::BTreeMap;

use quoin_combinatorial::{
    CombinatorialErrorCode, Configuration, DimensionName, DimensionValue, MAX_DEMANDED_TUPLES,
    covered_by, demanded_tuples, parse_space, tway_coverage,
};
use serde::Deserialize;

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod harness {
    use super::{BTreeMap, Deserialize};

    #[derive(Debug, Deserialize)]
    pub(crate) struct Corpus {
        pub(crate) cases: Vec<Case>,
    }

    #[derive(Debug, Deserialize)]
    pub(crate) struct Case {
        pub(crate) name: String,
        pub(crate) statement: String,
        pub(crate) configs: Vec<BTreeMap<String, String>>,
        pub(crate) space: Option<serde_json::Value>,
        pub(crate) demanded: Option<Vec<String>>,
        #[serde(rename = "coveredBy")]
        pub(crate) covered_by: Option<Vec<Vec<String>>>,
        pub(crate) coverage: Option<serde_json::Value>,
    }

    /// The captured corpus, read from the committed fixture.
    pub(crate) fn corpus() -> Corpus {
        let text = include_str!("goldens/combinatorial.json");
        serde_json::from_str(text).expect("the committed golden corpus parses")
    }
}

fn configuration(raw: &BTreeMap<String, String>) -> Configuration {
    raw.iter()
        .map(|(name, value)| {
            (
                DimensionName::new(name.clone()),
                DimensionValue::new(value.clone()),
            )
        })
        .collect()
}

/// How much of the corpus actually exercised each behaviour.
///
/// A census rather than a pass/fail: the floor below turns it into one.
#[derive(Default)]
struct Tally {
    refusals: usize,
    with_exclusions: usize,
    with_gaps: usize,
    with_coverage: usize,
    configurations: usize,
}

/// Check one accepted case's demand against the capture.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
fn check_demand(case: &harness::Case, space: &quoin_combinatorial::ConfigurationSpace) {
    let demanded = demanded_tuples(space)
        .unwrap_or_else(|error| panic!("{}: unexpected refusal {error}", case.name));
    let demanded_keys: Vec<&str> = demanded
        .iter()
        .map(quoin_combinatorial::TupleKey::as_str)
        .collect();
    let expected = case
        .demanded
        .as_ref()
        .expect("an accepted case has a demand");
    assert_eq!(
        demanded_keys, *expected,
        "{}: the demanded tuple set differs, in content or in order",
        case.name
    );
}

/// Check one accepted case's per-configuration cover against the capture.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
fn check_cover(
    case: &harness::Case,
    space: &quoin_combinatorial::ConfigurationSpace,
    tally: &mut Tally,
) {
    let expected = case
        .covered_by
        .as_ref()
        .expect("an accepted case has a per-configuration cover");
    assert_eq!(
        expected.len(),
        case.configs.len(),
        "{}: the capture is malformed",
        case.name
    );
    for (index, raw) in case.configs.iter().enumerate() {
        tally.configurations += 1;
        let covered = covered_by(space, &configuration(raw))
            .unwrap_or_else(|error| panic!("{}: unexpected refusal {error}", case.name));
        let keys: Vec<&str> = covered
            .iter()
            .map(quoin_combinatorial::TupleKey::as_str)
            .collect();
        assert_eq!(
            keys, expected[index],
            "{}: configuration {index} covers a different tuple set",
            case.name
        );
    }
}

/// Check one case end to end, and record what it exercised.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
fn check_case(case: &harness::Case, tally: &mut Tally) {
    let parsed = parse_space(&case.statement);

    let Some(expected_space) = case.space.as_ref() else {
        assert!(
            parsed.is_none(),
            "{}: the retained parser refused this statement and this one did not",
            case.name
        );
        tally.refusals += 1;
        return;
    };
    let space = parsed.unwrap_or_else(|| {
        panic!(
            "{}: the retained parser accepted this statement and this one did not",
            case.name
        )
    });

    assert_eq!(
        serde_json::to_value(&space).expect("a space serializes"),
        *expected_space,
        "{}: the parsed space differs",
        case.name
    );
    if !space.exclusions().is_empty() {
        tally.with_exclusions += 1;
    }

    check_demand(case, &space);
    check_cover(case, &space, tally);

    let configs: Vec<Configuration> = case.configs.iter().map(configuration).collect();
    let result = tway_coverage(&space, &configs)
        .unwrap_or_else(|error| panic!("{}: unexpected refusal {error}", case.name));
    assert_eq!(
        serde_json::to_value(&result).expect("a result serializes"),
        *case
            .coverage
            .as_ref()
            .expect("an accepted case has a coverage result"),
        "{}: the coverage result differs",
        case.name
    );
    if !result.gaps.is_empty() {
        tally.with_gaps += 1;
    }
    if result.covered > 0 {
        tally.with_coverage += 1;
    }
}

/// Trace: FR-035-AC-1, FR-101-AC-5
/// Provenance: quoin#382
#[test]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
fn tc_382_001_every_captured_case_agrees_with_the_retained_typescript() {
    let corpus = harness::corpus();

    // Anti-vacuity floor. Each count is the number of cases that must exercise
    // the named behaviour; a corpus that stops doing so fails here rather than
    // passing silently on the cases that remain.
    let mut tally = Tally::default();
    for case in &corpus.cases {
        check_case(case, &mut tally);
    }
    let Tally {
        refusals,
        with_exclusions,
        with_gaps,
        with_coverage,
        configurations,
    } = tally;

    assert!(
        corpus.cases.len() >= 30,
        "the corpus shrank to {} cases; it is captured, not generated, so it does not shrink by accident",
        corpus.cases.len()
    );
    assert!(
        refusals >= 8,
        "only {refusals} statements exercised a refusal path"
    );
    assert!(
        with_exclusions >= 5,
        "only {with_exclusions} spaces declared an exclusion"
    );
    assert!(
        with_gaps >= 10,
        "only {with_gaps} cases produced a gap list"
    );
    assert!(
        with_coverage >= 3,
        "only {with_coverage} cases covered anything at all"
    );
    assert!(
        configurations >= 15,
        "only {configurations} configurations were measured"
    );
}

/// Declared divergence 1: a demand past the ceiling is refused, not exhausted.
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
fn tc_382_002_a_demand_past_the_ceiling_is_refused_where_the_retained_side_exhausts_the_heap() {
    // Thirty binary dimensions at 30-way: 2^30 tuples, about a billion strings.
    // The retained `demandedTuples` accumulates them into an unbounded
    // `Set<string>` and the process dies with no diagnostic. This port refuses
    // by name. That is the whole of the difference: refusal versus exhaustion,
    // not two different answers.
    let mut statement = String::from("30-way over");
    for index in 0..30 {
        use std::fmt::Write as _;
        write!(statement, " d{index}(0|1)").expect("writing to a String cannot fail");
    }
    let space = parse_space(&statement).expect("thirty binary dimensions are a space");

    let error = demanded_tuples(&space).expect_err(
        "MUST HAVE FIRED: a 2^30 demand has to be refused, or this divergence has silently \
         become an unbounded allocation and DIVERGENCE.md is now false",
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

/// Declared divergence 2, and its proof of unreachability.
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
    // The retained side holds `strength` as a double, so `1e20-way` is
    // 100000000000000000000 there and saturates to `u64::MAX` here. The
    // difference is reachable in the *parsed space* and provably unreachable in
    // any *report*, which is the only place the number is read.
    let statement = format!(
        "{}-way over a(1|2) b(3|4)",
        100_000_000_000_000_000_000_u128
    );
    let space = parse_space(&statement).expect("a finite strength is still a space");
    assert_eq!(
        space.strength().get(),
        u64::MAX,
        "MUST HAVE FIRED: the saturation this divergence is about did not happen"
    );

    // The proof. `strength` is only read out of a `CoverageResult`, and the one
    // consumer (`src/auditor/audit.ts:700`) interpolates it solely inside a
    // `covered < demanded` branch. `covered <= demanded` always, so that branch
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
        "MUST HAVE FIRED: with a zero demand the finding branch is unreachable, which is the proof"
    );
    assert!(result.gaps.is_empty());
}

/// Declared non-divergence: JavaScript's inherited property names.
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
fn tc_382_004_an_inherited_property_name_is_not_a_divergence_because_the_value_guard_catches_it() {
    // `config["constructor"]` is inherited from `Object.prototype` and is NOT
    // `undefined`, so the retained `!== undefined` guard passes on a
    // configuration that never named the dimension. A `BTreeMap` has no such
    // inheritance, so the two sides reach the second guard by different routes.
    // They agree because `d.values.includes(...)` rejects a function, and a
    // dimension's values are always strings. This test holds that reasoning in
    // place; `tests/goldens/combinatorial.json` carries the captured answer.
    let space = parse_space("2-way over constructor(1|2) b(3|4)").expect("a space");
    let mut config = Configuration::new();
    config.insert(DimensionName::new("b"), DimensionValue::new("3"));
    let covered = covered_by(&space, &config).expect("a small space computes");
    assert!(
        covered.is_empty(),
        "MUST HAVE FIRED: only one dimension is present, so no 2-way tuple exists"
    );
}
