// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! t-way coverage over a declared configuration space (FR-035).
//!
//! quire mints the obligation — *"2-way over these dimensions"* — and states
//! the space in the obligation's own statement (quire-rs FR-061). This computes
//! what a run actually reached, and names what it did not.
//!
//! **quoin computes; it neither declares nor generates.** The dimensions come
//! from the spec, the executed configurations come from the consumer's CI, and
//! the covering array itself is nobody's job here (ADR-0011 invariant 1).
//!
//! # Why this is its own crate
//!
//! The retained `src/auditor/combinatorial.ts` imports nothing. It is 195 lines
//! of pure algebra over strings and sets: no clock, no filesystem, no store, no
//! process. Keeping that shape in the port is what lets the covering-array
//! invariants be stated as properties over generated spaces
//! (`tests/tc_382_properties.rs`) instead of as examples over fixtures — a
//! function that could read the world could not be quantified over that way.
//!
//! Whether the algebra should instead live in `agent-ix/engineering-assurance`
//! was asked and answered on quoin#382: no, for now, with the reason recorded
//! and a gap ticket opened (`agent-ix/engineering-assurance#105`) against the
//! day a second consumer appears.
//!
//! # Entry points
//!
//! * [`parse_space`] — statement → [`ConfigurationSpace`], or `None` when the
//!   statement is not a combinatorial obligation at all.
//! * [`demanded_tuples`] / [`covered_by`] — the two sets.
//! * [`tway_coverage`] — the measurement a report carries.

#![forbid(unsafe_code)]

pub mod coverage;
pub mod error;
pub mod js;
pub mod space;
pub mod tuple;

pub use coverage::{Configuration, CoverageResult, tway_coverage};
pub use error::{CombinatorialError, CombinatorialErrorCode};
pub use space::{
    ConfigurationSpace, Dimension, DimensionName, DimensionValue, Exclusion, Strength, parse_space,
};
pub use tuple::{MAX_DEMANDED_TUPLES, TupleKey, covered_by, demanded_tuples};
