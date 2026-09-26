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
///
/// # Why there is an `unmodelled` map
///
/// The retained code never rebuilds this object: `compare.ts:106` keys on
/// `JSON.stringify(observation.population)` and `report.ts` re-serialises the
/// observation it read. Both see **every** stored member, so a model that kept
/// only the four named ones would compare and re-serialise a different value
/// than the oracle does. The retained corpus is not hypothetical about this —
/// 25 of the 14,644 observations under `spec/evidence/measurements/` carry
/// `exclusions` and `namedMisses` — so the rest is kept verbatim rather than
/// dropped (quoin#473).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeasurementPopulation {
    /// How many candidates were considered.
    pub examined: Option<f64>,
    /// How many of them matched.
    pub matched: Option<f64>,
    /// Whether the producer considers the population complete.
    pub complete: Option<bool>,
    /// How many repeated runs produced the observation (PLAT-960), exactly
    /// as stored. Intake requires a whole number of at least 1 and holds it
    /// against the plan's `statistical_design.repetitions`; the stored value
    /// is kept verbatim so a report shows what a retained collection carries.
    pub repetitions: Option<JsonValue>,
    /// A producer-defined identity for the population.
    pub identity: Option<JsonValue>,
    /// Every other member of the stored object, kept as it was stored.
    pub unmodelled: BTreeMap<String, JsonValue>,
}

impl MeasurementPopulation {
    /// The members this type reads by name; everything else is
    /// [`unmodelled`](Self::unmodelled).
    pub const MODELLED: [&'static str; 5] =
        ["examined", "matched", "complete", "repetitions", "identity"];

    /// A population that declares nothing.
    ///
    /// `compare.ts:79-87` reads an absent `population` through `?.`, so every
    /// member reads as absent rather than the whole thing being a special case.
    pub const EMPTY: Self = Self {
        examined: None,
        matched: None,
        complete: None,
        repetitions: None,
        identity: None,
        unmodelled: BTreeMap::new(),
    };
}

/// The `dimensions` member of a stored observation.
///
/// # Why absence is modelled rather than flattened to an empty map
///
/// `report.ts:202-204` re-serialises the observation it read, and
/// `JSON.stringify` drops an absent member while emitting a stated empty
/// object as `{}`. The two are therefore different bytes, and 305 of the
/// 14,644 observations under `spec/evidence/measurements/` state no
/// `dimensions` at all — so a model that could not tell them apart would
/// render a byte the oracle does not (quoin#473).
///
/// Every *reader* still sees `dimensions ?? {}`, which is what `compare.ts:99`
/// and `report.ts:120` do; that is [`entries`](Self::entries). Only the
/// re-serialiser asks whether the member was stated, through
/// [`stated_entries`](Self::stated_entries).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dimensions(Option<BTreeMap<String, JsonValue>>);

/// The map [`Dimensions::entries`] hands back when the member was not stated.
static NO_ENTRIES: BTreeMap<String, JsonValue> = BTreeMap::new();

impl Dimensions {
    /// The member was not stated.
    pub const ABSENT: Self = Self(None);

    /// The member was stated, with these entries.
    #[must_use]
    pub const fn stated(entries: BTreeMap<String, JsonValue>) -> Self {
        Self(Some(entries))
    }

    /// The entries every reader sees — empty when the member was not stated.
    #[must_use]
    pub fn entries(&self) -> &BTreeMap<String, JsonValue> {
        self.0.as_ref().unwrap_or(&NO_ENTRIES)
    }

    /// The entries, only when the member was stated.
    #[must_use]
    pub const fn stated_entries(&self) -> Option<&BTreeMap<String, JsonValue>> {
        self.0.as_ref()
    }
}

/// The metric-name suffix a constant-predictor producer appends to the
/// plan's own metric to name its per-item retained observations (PLAT-1016).
///
/// A per-item observation is never itself a run of the plan: the checker's
/// run-building filter matches on the plan's own `metric` exactly, so an item
/// observation (whose metric always carries this suffix) is never mistaken
/// for another slice of the governed metric. Intake admits it under the
/// governed metric's own plan ([`constant_predictor_governed_metric`]), and
/// `compare` leaves it out of slice-by-slice comparison. See
/// `crate::verify::constant_predictor` for the reader and `quoin-jev`'s
/// `constant_predictor` module for the producer.
pub const CONSTANT_PREDICTOR_ITEM_METRIC_SUFFIX: &str = ".constant-predictor-item";

/// The per-item metric name a constant-predictor producer writes for
/// `metric`'s governed measurement.
#[must_use]
pub fn constant_predictor_item_metric(metric: &str) -> String {
    format!("{metric}{CONSTANT_PREDICTOR_ITEM_METRIC_SUFFIX}")
}

/// The governed metric `metric` is a constant-predictor item metric of, or
/// `None` when `metric` does not carry
/// [`CONSTANT_PREDICTOR_ITEM_METRIC_SUFFIX`] (or is nothing but the suffix).
#[must_use]
pub fn constant_predictor_governed_metric(metric: &str) -> Option<&str> {
    metric
        .strip_suffix(CONSTANT_PREDICTOR_ITEM_METRIC_SUFFIX)
        .filter(|governed| !governed.is_empty())
}

/// `dimensions` keys a constant-predictor per-item observation carries
/// (PLAT-1016). Every key's value is a string except
/// [`CONSTANT_PREDICTOR_DIM_CONTESTED`], which is a JSON array of strings.
pub mod constant_predictor_dims {
    /// The answer-space family this item belongs to (the plan's own
    /// Population grouping — MP-222's rule is "do not invent a new grouping
    /// here").
    pub const FAMILY: &str = "family";
    /// A producer-defined identity for the item, unique within the
    /// collection's per-item observations for this metric.
    pub const ITEM_ID: &str = "item_id";
    /// The primary recorded (ground-truth) label.
    pub const EXPECTED: &str = "expected";
    /// Every reading the corpus recorded as defensible, [`EXPECTED`]
    /// included, as a JSON array of strings.
    pub const CONTESTED: &str = "contested";
    /// What the graded tool returned for this item, for traceability. Not
    /// read by the constant-predictor baseline formula, which depends only
    /// on the corpus's own labels.
    pub const ACTUAL: &str = "actual";
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
    pub dimensions: Dimensions,
    /// Why no value was produced.
    pub reason: Option<String>,
    /// The stated uncertainty `interval` (EA-26), exactly as stored, a
    /// malformed value included: intake refuses a malformed one, but a
    /// retained collection that carries one is still shown as it was stored.
    /// Read it through [`crate::interval::reading`], never by hand.
    pub interval: Option<JsonValue>,
}

impl MeasurementObservation {
    /// The dimensions as the retained code keys them: sorted by name.
    ///
    /// `compare.ts:95-101` and `validate.ts:185-191` both build this key by
    /// sorting the entries, so a [`BTreeMap`] is the same key by construction.
    #[must_use]
    pub fn identity(&self) -> (&str, &BTreeMap<String, JsonValue>) {
        (self.metric.as_str(), self.dimensions.entries())
    }
}
