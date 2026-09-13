// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The result of comparing two collections. Comparability only — this layer
//! deliberately emits no quality verdict.
//!
//! Ports `ComparisonReason` and `MeasurementComparison` from
//! `src/measurement/types.ts:79-99`.

use std::collections::BTreeMap;

use quoin_store::JsonValue;

/// Why two observations are, or are not, comparable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ComparisonReasonCode {
    /// The metric's definition version moved between the two collections.
    DefinitionChanged,
    /// The producer's configuration digest moved.
    ConfigurationChanged,
    /// One side's population is incomplete.
    IncompletePopulation,
    /// The populations differ. Not blocking: the delta is shown with a warning.
    PopulationChanged,
    /// The producing tool's version moved. Not blocking.
    ToolChanged,
}

impl ComparisonReasonCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::DefinitionChanged,
        Self::ConfigurationChanged,
        Self::IncompletePopulation,
        Self::PopulationChanged,
        Self::ToolChanged,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DefinitionChanged => "definition_changed",
            Self::ConfigurationChanged => "configuration_changed",
            Self::IncompletePopulation => "incomplete_population",
            Self::PopulationChanged => "population_changed",
            Self::ToolChanged => "tool_changed",
        }
    }

    /// Recover a code from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }

    /// Whether this reason blocks the comparison.
    ///
    /// `compare.ts:31-63` sets `blocking` per code and never varies it per
    /// call site, so it is a property of the code rather than a field a caller
    /// could get wrong. The struct keeps the field so the emitted shape is
    /// unchanged; this is where its value comes from.
    #[must_use]
    pub const fn is_blocking(self) -> bool {
        match self {
            Self::DefinitionChanged | Self::ConfigurationChanged | Self::IncompletePopulation => {
                true
            }
            Self::PopulationChanged | Self::ToolChanged => false,
        }
    }
}

/// One reason attached to a comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparisonReason {
    /// Which reason this is.
    pub code: ComparisonReasonCode,
    /// The rendered sentence. Not contractual; the code is.
    pub message: String,
    /// Whether it blocks. Always [`ComparisonReasonCode::is_blocking`].
    pub blocking: bool,
}

impl ComparisonReason {
    /// Build a reason, taking `blocking` from the code.
    pub fn new(code: ComparisonReasonCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            blocking: code.is_blocking(),
        }
    }
}

/// Whether two observations could be compared.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ComparisonStatus {
    /// Both sides measured and nothing blocking stood in the way.
    Comparable,
    /// Both sides measured but something blocking stood in the way.
    Incomparable,
    /// One side is absent or was not computed.
    NotComputed,
}

impl ComparisonStatus {
    /// Every status, in declaration order.
    pub const ALL: [Self; 3] = [Self::Comparable, Self::Incomparable, Self::NotComputed];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Comparable => "comparable",
            Self::Incomparable => "incomparable",
            Self::NotComputed => "not_computed",
        }
    }

    /// Recover a status from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// One metric-and-dimensions slice, compared across two collections.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementComparison {
    /// The metric.
    pub metric: String,
    /// The dimensions that identify this slice.
    pub dimensions: BTreeMap<String, JsonValue>,
    /// The earlier value, when there was one.
    pub before: Option<f64>,
    /// The later value, when there was one.
    pub after: Option<f64>,
    /// `after - before`, present only when the status is
    /// [`ComparisonStatus::Comparable`].
    pub delta: Option<f64>,
    /// Whether the two could be compared.
    pub status: ComparisonStatus,
    /// Why.
    pub reasons: Vec<ComparisonReason>,
}
