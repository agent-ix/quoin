// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The configuration space, parsed out of an obligation statement (FR-035).
//!
//! Ports the `Dimension` … `parseSpace` block of
//! `src/auditor/combinatorial.ts`.
//!
//! # Parse, don't validate
//!
//! [`parse_space`] is the **only** constructor of a [`ConfigurationSpace`], and
//! every invariant the retained function enforces by `continue`/`return null`
//! is an invariant of the returned value rather than a fact a later reader has
//! to re-check:
//!
//! * a [`Strength`] is at least 1;
//! * a [`Dimension`] carries at least two values;
//! * a space carries at least two dimensions;
//! * an [`Exclusion`] carries at least two assignments.
//!
//! Returning `None` for a statement that is not a combinatorial obligation is
//! the retained contract and is load-bearing: it is how a caller tells a
//! combinatorial obligation from every other kind without a second flag to keep
//! in agreement with the first. `src/advisor/advise.ts:459` reads it exactly
//! that way, to mint `configuration-matrix` structurally.

use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::{JS_WHITESPACE_CLASS, js_trim};

/// A dimension's name, as the statement spelled it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DimensionName(String);

/// One value a dimension may take, as the statement spelled it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DimensionValue(String);

macro_rules! string_newtype {
    ($name:ident) => {
        impl $name {
            /// Wrap a spelling read out of a statement or a run entry.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The spelling.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_newtype!(DimensionName);
string_newtype!(DimensionValue);

/// The `t` of a t-way covering array: at least 1, by construction.
///
/// # Why it is a `u64` and why that cannot be observed
///
/// `Number.parseInt(header[1], 10)` yields a double, so a header naming
/// `99999999999999999999999-way` produces `1e23` on the retained side and
/// saturates to [`u64::MAX`] here. That difference is unobservable: the only
/// readers of the value are the `size > n` guard in
/// [`combinations`](crate::tuple::combinations) — which rejects every strength
/// above the dimension count identically on both sides — and
/// [`CoverageResult::strength`](crate::CoverageResult), which the auditor
/// interpolates only inside a `covered < demanded` branch that a zero demand
/// set cannot enter. `tests/tc_382_parity.rs` proves the unreachability rather
/// than declaring the difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Strength(u64);

impl Strength {
    /// The strength as a number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Read a strength off the decimal digits a header matched.
    ///
    /// The retained guard is `!Number.isFinite(strength) || strength < 1`, and
    /// **both halves are load-bearing**: `Number.parseInt` of a 400-digit
    /// header yields `Infinity`, so the retained parser calls such a statement
    /// *not a combinatorial obligation at all* rather than one with an enormous
    /// strength. Reading the digits as an `f64` first reproduces that exactly —
    /// Rust and ECMAScript both round decimal to the nearest double — instead
    /// of approximating the boundary with a digit count.
    fn parse_digits(digits: &str) -> Option<Self> {
        let as_double = digits.parse::<f64>().ok()?;
        if !as_double.is_finite() || as_double < 1.0 {
            return None;
        }
        Some(Self(digits.parse::<u64>().unwrap_or(u64::MAX)))
    }
}

impl std::fmt::Display for Strength {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// One dimension parsed back out of an obligation statement.
///
/// A `Dimension` exists only with **two or more** values: the retained parser
/// skips a one-value dimension, on the ground that a dimension with a single
/// value is not a dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimension {
    name: DimensionName,
    values: Vec<DimensionValue>,
}

impl Dimension {
    /// The dimension's name.
    #[must_use]
    pub fn name(&self) -> &DimensionName {
        &self.name
    }

    /// The values it may take, in the order the statement listed them.
    #[must_use]
    pub fn values(&self) -> &[DimensionValue] {
        &self.values
    }

    /// Build a dimension, refusing one that does not span at least two values.
    fn parse(name: &str, values: Vec<DimensionValue>) -> Option<Self> {
        (values.len() >= 2).then(|| Self {
            name: DimensionName::new(name),
            values,
        })
    }
}

/// A forbidden combination: assignments that cannot co-occur.
///
/// An `Exclusion` exists only with **two or more** assignments, matching the
/// retained `assignments.length >= 2` guard: a single assignment would forbid a
/// value outright, which is a narrower dimension rather than an exclusion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusion {
    assignments: Vec<(DimensionName, DimensionValue)>,
}

impl Exclusion {
    /// The assignments that may not co-occur.
    #[must_use]
    pub fn assignments(&self) -> &[(DimensionName, DimensionValue)] {
        &self.assignments
    }

    fn parse(assignments: Vec<(DimensionName, DimensionValue)>) -> Option<Self> {
        (assignments.len() >= 2).then_some(Self { assignments })
    }
}

/// A declared configuration space: a strength, its dimensions and its
/// exclusions.
///
/// Constructed only by [`parse_space`], so a value of this type is already a
/// well-formed space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationSpace {
    strength: Strength,
    dimensions: Vec<Dimension>,
    exclusions: Vec<Exclusion>,
}

impl ConfigurationSpace {
    /// The `t` of the t-way demand.
    #[must_use]
    pub const fn strength(&self) -> Strength {
        self.strength
    }

    /// The dimensions, in statement order. At least two.
    #[must_use]
    pub fn dimensions(&self) -> &[Dimension] {
        &self.dimensions
    }

    /// The declared exclusions, in statement order.
    #[must_use]
    pub fn exclusions(&self) -> &[Exclusion] {
        &self.exclusions
    }
}

/// `^(\d+)-way over\s` — the header that marks a statement as one of these.
fn header() -> &'static Regex {
    static HEADER: OnceLock<Regex> = OnceLock::new();
    HEADER.get_or_init(|| {
        Regex::new(&format!(r"^([0-9]+)-way over[{JS_WHITESPACE_CLASS}]"))
            .unwrap_or_else(|error| unreachable!("the header pattern is a literal: {error}"))
    })
}

/// `([^\s(]+)\(([^)]*)\)` — one `name(a|b|c)` group.
fn dimension() -> &'static Regex {
    static DIMENSION: OnceLock<Regex> = OnceLock::new();
    DIMENSION.get_or_init(|| {
        Regex::new(&format!(r"([^{JS_WHITESPACE_CLASS}(]+)\(([^)]*)\)"))
            .unwrap_or_else(|error| unreachable!("the dimension pattern is a literal: {error}"))
    })
}

/// `excluding\[([^\]]*)\]` — one exclusion clause list.
fn exclusion() -> &'static Regex {
    static EXCLUSION: OnceLock<Regex> = OnceLock::new();
    EXCLUSION.get_or_init(|| {
        Regex::new(r"excluding\[([^\]]*)\]")
            .unwrap_or_else(|error| unreachable!("the exclusion pattern is a literal: {error}"))
    })
}

/// Parse the space out of a combinatorial obligation's statement.
///
/// `None` for any statement that is not one.
#[must_use]
pub fn parse_space(statement: &str) -> Option<ConfigurationSpace> {
    let captured = header().captures(statement)?;
    let strength = Strength::parse_digits(captured.get(1)?.as_str())?;

    // Dimensions are read from the part before any `excluding[...]`, so an
    // exclusion naming `dim=value` is never mistaken for a dimension.
    let head = statement
        .split_once("excluding[")
        .map_or(statement, |(before, _)| before);

    let mut dimensions = Vec::new();
    for group in dimension().captures_iter(head) {
        let (Some(name), Some(values)) = (group.get(1), group.get(2)) else {
            continue;
        };
        let values: Vec<DimensionValue> = values
            .as_str()
            .split('|')
            .map(js_trim)
            .filter(|value| !value.is_empty())
            .map(DimensionValue::new)
            .collect();
        if let Some(parsed) = Dimension::parse(name.as_str(), values) {
            dimensions.push(parsed);
        }
    }
    if dimensions.len() < 2 {
        return None;
    }

    let mut exclusions = Vec::new();
    for group in exclusion().captures_iter(statement) {
        let Some(clauses) = group.get(1) else {
            continue;
        };
        let assignments: Vec<(DimensionName, DimensionValue)> = clauses
            .as_str()
            .split(',')
            .map(js_trim)
            .filter(|clause| !clause.is_empty())
            .filter_map(parse_assignment)
            .collect();
        if let Some(parsed) = Exclusion::parse(assignments) {
            exclusions.push(parsed);
        }
    }

    Some(ConfigurationSpace {
        strength,
        dimensions,
        exclusions,
    })
}

/// `dim=value` → the pair, splitting at the FIRST `=` and trimming both halves.
///
/// A clause with no `=` is dropped, matching the retained `at === -1` branch.
fn parse_assignment(clause: &str) -> Option<(DimensionName, DimensionValue)> {
    let at = clause.find('=')?;
    let (name, value) = clause.split_at(at);
    Some((
        DimensionName::new(js_trim(name)),
        // `slice(at + 1)`: the `=` itself is one byte, so the arithmetic is
        // exact rather than a UTF-16-versus-UTF-8 hazard.
        DimensionValue::new(js_trim(&value[1..])),
    ))
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
    use super::{Strength, parse_space};

    #[test]
    fn a_statement_without_the_header_is_not_a_space() {
        assert!(parse_space("The system shall do the thing.").is_none());
        assert!(parse_space("way over a(1|2) b(3|4)").is_none());
        // Anchored: a header in the middle is not a header.
        assert!(parse_space("see: 2-way over a(1|2) b(3|4)").is_none());
    }

    #[test]
    fn strength_zero_is_refused_and_leading_zeros_are_read_as_decimal() {
        assert!(parse_space("0-way over a(1|2) b(3|4)").is_none());
        let space = parse_space("007-way over a(1|2) b(3|4)").unwrap();
        assert_eq!(space.strength(), Strength(7));
    }

    #[test]
    fn a_single_value_dimension_is_skipped_and_two_are_still_required() {
        // `c(only)` contributes nothing, leaving two — still a space.
        let space = parse_space("2-way over a(1|2) b(3|4) c(only)").unwrap();
        assert_eq!(space.dimensions().len(), 2);
        // With only one surviving dimension there is no space at all.
        assert!(parse_space("2-way over a(1|2) c(only)").is_none());
    }

    #[test]
    fn an_exclusion_naming_a_dimension_is_never_read_as_one() {
        let space = parse_space("2-way over a(1|2) b(3|4) excluding[a=1,b=3]").unwrap();
        assert_eq!(space.dimensions().len(), 2);
        assert_eq!(space.exclusions().len(), 1);
        assert_eq!(space.exclusions()[0].assignments().len(), 2);
        assert_eq!(space.exclusions()[0].assignments()[0].0.as_str(), "a");
        assert_eq!(space.exclusions()[0].assignments()[0].1.as_str(), "1");
    }

    #[test]
    fn a_one_clause_exclusion_is_dropped_rather_than_narrowing_a_dimension() {
        let space = parse_space("2-way over a(1|2) b(3|4) excluding[a=1]").unwrap();
        assert!(space.exclusions().is_empty());
    }

    #[test]
    fn a_value_containing_an_equals_sign_keeps_everything_after_the_first() {
        let space = parse_space("2-way over a(1|2) b(3|4) excluding[a=x=y,b=3]").unwrap();
        assert_eq!(space.exclusions()[0].assignments()[0].1.as_str(), "x=y");
    }

    #[test]
    fn values_are_trimmed_with_the_ecmascript_whitespace_set() {
        let space = parse_space("2-way over a( 1 |\u{feff}2\u{feff}) b(3|4)").unwrap();
        let values: Vec<&str> = space.dimensions()[0]
            .values()
            .iter()
            .map(super::DimensionValue::as_str)
            .collect();
        assert_eq!(values, ["1", "2"]);
    }
}
