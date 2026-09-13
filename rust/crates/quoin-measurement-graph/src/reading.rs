// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What a governed graph reading is, and what two of them compared are.
//!
//! Ports `GraphPartitionRow`, `GraphQualityHistoryRow`,
//! `GraphCompatibilityCode`, `GraphCompatibilityReason`,
//! `GraphQualityComparisonRow` and `GraphQualityComparison`
//! (`graph-portfolio.ts:102-166`).

use quoin_measurement::types::observation::{MeasurementShape, MeasurementState};
use quoin_store::JsonValue;

use crate::availability::HistoryAvailability;

/// The three producer-supplied names one graph partition is sliced by.
///
/// `Pick<GraphPartitionRow, "measure" | "dimension" | "key">`
/// (`graph-portfolio.ts:770-778`): a partition row and a comparison row carry
/// the same three, so they are one type rather than six fields declared twice.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartitionIdentity {
    /// What was measured.
    pub measure: String,
    /// Which axis it was sliced on.
    pub dimension: String,
    /// Which value of that axis.
    pub key: String,
}

impl PartitionIdentity {
    /// The value each of the three takes when the observation did not state it.
    /// `graph-portfolio.ts:772-776`.
    pub const UNKNOWN: &'static str = "unknown";
}

/// One measured graph partition. `graph-portfolio.ts:102-111`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphPartitionRow {
    /// What it is a partition of.
    pub identity: PartitionIdentity,
    /// Whether a value was produced.
    pub state: MeasurementState,
    /// The value, when one was.
    pub value: Option<f64>,
    /// The unit the value is in.
    pub unit: String,
    /// What kind of quantity it is.
    pub shape: MeasurementShape,
    /// Why no value was produced. Absent rather than `null`: the retained
    /// object spreads it in only when it is truthy
    /// (`graph-portfolio.ts:765-767`).
    pub reason: Option<String>,
}

/// The plan a collection says it measured against.
/// `graph-portfolio.ts:119`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryPlanRef {
    /// The plan's identity.
    pub id: String,
    /// The definition version the producer measured against.
    pub definition_version: String,
}

/// One retained collection, as a governed graph reading.
/// `graph-portfolio.ts:113-131`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphQualityHistoryRow {
    /// The collection's identity.
    pub id: String,
    /// Where the record lives.
    pub path: String,
    /// When it was produced.
    pub timestamp: String,
    /// Whether this reading may be used as current evidence.
    pub availability: HistoryAvailability,
    /// Why not, when it may not.
    pub reason: Option<String>,
    /// The plan the collection's first graph partition names.
    pub plan: Option<HistoryPlanRef>,
    /// Which tool produced it.
    pub tool_identity: String,
    /// Which version of that tool.
    pub tool_version: String,
    /// The digest of the configuration it ran under.
    pub config_digest: String,
    /// The revision of the measured source.
    pub source_revision: String,
    /// The revision of the measured corpus, when it stated one.
    pub corpus_revision: Option<String>,
    /// The producer-defined population identity, uninterpreted.
    pub population_identity: Option<JsonValue>,
    /// The producer record, uninterpreted.
    pub producer: Option<JsonValue>,
    /// The producer record's own observation id.
    pub producer_record_digest: Option<String>,
    /// The digest of the retained scorer attachment.
    pub scorer_digest: Option<String>,
    /// Its graph partitions, in identity order.
    pub partitions: Vec<GraphPartitionRow>,
}

/// Why two readings may not be compared. `graph-portfolio.ts:133-142`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GraphCompatibilityCode {
    /// The two observations name different plans.
    PlanChanged,
    /// They name different definition versions.
    DefinitionChanged,
    /// The two collections ran under different configurations.
    ConfigurationChanged,
    /// They were produced by different tools or tool versions.
    ToolChanged,
    /// They measured different corpus revisions.
    CorpusChanged,
    /// At least one population is not declared complete.
    PopulationIncomplete,
    /// The two populations have different identities.
    PopulationChanged,
    /// One of the two collections is not current-compatible at all.
    CollectionIncompatible,
}

impl GraphCompatibilityCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 8] = [
        Self::PlanChanged,
        Self::DefinitionChanged,
        Self::ConfigurationChanged,
        Self::ToolChanged,
        Self::CorpusChanged,
        Self::PopulationIncomplete,
        Self::PopulationChanged,
        Self::CollectionIncompatible,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlanChanged => "plan_changed",
            Self::DefinitionChanged => "definition_changed",
            Self::ConfigurationChanged => "configuration_changed",
            Self::ToolChanged => "tool_changed",
            Self::CorpusChanged => "corpus_changed",
            Self::PopulationIncomplete => "population_incomplete",
            Self::PopulationChanged => "population_changed",
            Self::CollectionIncompatible => "collection_incompatible",
        }
    }

    /// Recover a code from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// One stated incompatibility. `graph-portfolio.ts:144-148`.
///
/// The retained interface declares `blocking: true` — the literal, not the
/// type — so every value of it is blocking and there is no member here to
/// hold a `false` that cannot occur. The JSON view states it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphCompatibilityReason {
    /// Which incompatibility.
    pub code: GraphCompatibilityCode,
    /// The sentence to report.
    pub message: String,
}

impl GraphCompatibilityReason {
    /// The one value `blocking` takes. `graph-portfolio.ts:146`.
    pub const BLOCKING: bool = true;
}

/// Whether one partition could be compared. `graph-portfolio.ts:157`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ComparisonStatus {
    /// Both readings exist and nothing blocks the comparison.
    Comparable,
    /// Both readings exist and something blocks it.
    Incomparable,
    /// One of the two readings computed no value.
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

/// One partition, before and after. `graph-portfolio.ts:150-158`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphQualityComparisonRow {
    /// What it is a partition of.
    pub identity: PartitionIdentity,
    /// The earlier value.
    pub before: Option<f64>,
    /// The later value.
    pub after: Option<f64>,
    /// `after - before`, only when the comparison is [`ComparisonStatus::Comparable`].
    pub delta: Option<f64>,
    /// Whether it could be compared.
    pub status: ComparisonStatus,
    /// Why not, when it could not.
    pub reasons: Vec<GraphCompatibilityReason>,
}

/// Two readings, compared. `graph-portfolio.ts:160-164`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphQualityComparison {
    /// The earlier reading.
    pub before: GraphQualityHistoryRow,
    /// The later reading.
    pub after: GraphQualityHistoryRow,
    /// The compared partitions, in identity order.
    pub observations: Vec<GraphQualityComparisonRow>,
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use super::{ComparisonStatus, GraphCompatibilityCode};

    #[test]
    fn every_compatibility_code_round_trips_and_is_unique() {
        let mut spellings: Vec<&str> = Vec::new();
        for code in GraphCompatibilityCode::ALL {
            assert_eq!(GraphCompatibilityCode::from_wire(code.as_str()), Some(code));
            spellings.push(code.as_str());
        }
        spellings.sort_unstable();
        let count = spellings.len();
        spellings.dedup();
        assert_eq!(spellings.len(), count);
        assert_eq!(count, 8, "anti-vacuity floor: the retained union has eight");
    }

    #[test]
    fn every_comparison_status_round_trips() {
        for status in ComparisonStatus::ALL {
            assert_eq!(ComparisonStatus::from_wire(status.as_str()), Some(status));
        }
    }
}
