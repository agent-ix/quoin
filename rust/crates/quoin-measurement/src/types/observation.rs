// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One measured value, and the population it was measured over.
//!
//! Ports the `MeasurementObservation`, `MeasurementState`, `MeasurementShape`
//! and `MeasurementPopulation` members of `src/measurement/types.ts:4-25`.

use std::collections::BTreeMap;

use quoin_store::JsonValue;

use crate::types::ids::NonEmptyText;

/// Whether an observation carries a value or records why it does not.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MeasurementState {
    /// A value was produced.
    Measured,
    /// No value was produced; `value` is null and `reason` says why.
    NotComputed,
}

impl MeasurementState {
    /// Every state, in declaration order.
    pub const ALL: [Self; 2] = [Self::Measured, Self::NotComputed];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::NotComputed => "not_computed",
        }
    }

    /// Recover a state from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// What kind of quantity an observation is.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MeasurementShape {
    /// A bare quantity.
    Scalar,
    /// A proportion of `matched` over `examined`.
    Ratio,
    /// A cardinality.
    Count,
}

impl MeasurementShape {
    /// Every shape, in declaration order.
    pub const ALL: [Self; 3] = [Self::Scalar, Self::Ratio, Self::Count];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Ratio => "ratio",
            Self::Count => "count",
        }
    }

    /// Recover a shape from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// What was examined to produce an observation.
///
/// Every member is optional and none is validated by the retained validator,
/// so none is narrowed here either. `identity` is producer-defined and stays
/// opaque.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeasurementPopulation {
    /// How many candidates were considered.
    pub examined: Option<f64>,
    /// How many of them matched.
    pub matched: Option<f64>,
    /// Whether the producer considers the population complete.
    pub complete: Option<bool>,
    /// A producer-defined identity for the population.
    pub identity: Option<JsonValue>,
}

impl MeasurementPopulation {
    /// A population that declares nothing.
    ///
    /// `compare.ts:79-87` reads an absent `population` through `?.`, so every
    /// member reads as absent rather than the whole thing being a special case.
    pub const EMPTY: Self = Self {
        examined: None,
        matched: None,
        complete: None,
        identity: None,
    };
}

/// One observation within a collection.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementObservation {
    /// The metric this observation reports.
    pub metric: NonEmptyText,
    /// The `MeasurementPlan` the producer believes governs it.
    pub plan_id: NonEmptyText,
    /// The definition version the producer measured against.
    pub definition_version: NonEmptyText,
    /// Whether a value was produced.
    pub state: MeasurementState,
    /// The value, present exactly when `state` is
    /// [`MeasurementState::Measured`].
    pub value: Option<f64>,
    /// The unit the value is in.
    pub unit: NonEmptyText,
    /// What kind of quantity it is.
    pub shape: MeasurementShape,
    /// What was examined, when the producer said.
    pub population: Option<MeasurementPopulation>,
    /// The dimensions this observation is sliced by.
    ///
    /// `types.ts:22` declares `Record<string, string>` but nothing validates
    /// the member type, and `compare.ts:99` stringifies whatever is there. The
    /// port keeps what the validator checks, not what the declaration claims.
    pub dimensions: BTreeMap<String, JsonValue>,
    /// Why no value was produced.
    pub reason: Option<String>,
}

impl MeasurementObservation {
    /// The dimensions as the retained code keys them: sorted by name.
    ///
    /// `compare.ts:95-101` and `validate.ts:185-191` both build this key by
    /// sorting the entries, so a [`BTreeMap`] is the same key by construction.
    #[must_use]
    pub fn identity(&self) -> (&str, &BTreeMap<String, JsonValue>) {
        (self.metric.as_str(), &self.dimensions)
    }
}
