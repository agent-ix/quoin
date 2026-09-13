// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What this projection is handed, and the one thing it normalises on the way
//! in.
//!
//! Ports `GraphCollectionRead`, `InjectedStructuralGraph`,
//! `GraphPortfolioRepositoryInput`, `GraphPortfolioGap` and
//! `NormalizedStructuralGraph` (`graph-portfolio.ts:26-62,169-200`).
//!
//! # The graph stays opaque
//!
//! `premises`, `fanOut`, `churn` and `changeImpact` are declared `unknown` in
//! the retained types and are [`JsonValue`] here for the same reason:
//! `graph-portfolio.ts:41-43` states that the portfolio "retains object
//! identity and never owns or interprets the graph report contract". FR-062 is
//! the sole owner of graph semantics; this crate carries the bytes.

use quoin_measurement::portfolio::PortfolioRepositoryReport;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_store::JsonValue;

use crate::availability::{CollectionUnavailability, GapAvailability, GapSubject};

/// One attempt to read one retained collection, as this projection sees it.
///
/// `graph-portfolio.ts:34-39`. The retained interface carries `collection`,
/// `availability` and `error` as three independent optional members, so it
/// admits a read that produced a collection *and* an error. This does not:
/// the two states are variants, and the retained `??` defaults are applied by
/// [`Self::refused`] where the value is built rather than at each of the two
/// places that read it.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphCollectionRead {
    /// The collection was read.
    Read {
        /// Where the record lives.
        path: String,
        /// What was read. Boxed: a collection is far larger than a refusal,
        /// and an un-boxed variant would size every read by the bigger one.
        collection: Box<MeasurementCollection>,
    },
    /// The collection was not read, for this stated reason.
    Refused {
        /// Where the record lives, or was expected to.
        path: String,
        /// Why it could not be used.
        availability: CollectionUnavailability,
        /// The sentence to report.
        reason: String,
    },
}

impl GraphCollectionRead {
    /// The default `graph-portfolio.ts:498` applies to an absent
    /// `availability`.
    pub const DEFAULT_REFUSAL: CollectionUnavailability = CollectionUnavailability::Unknown;

    /// The default `graph-portfolio.ts:501` applies to an absent `error`.
    pub const DEFAULT_REASON: &'static str = "collection was not readable";

    /// A read that produced a collection.
    #[must_use]
    pub fn read(path: impl Into<String>, collection: MeasurementCollection) -> Self {
        Self::Read {
            path: path.into(),
            collection: Box::new(collection),
        }
    }

    /// A read that did not, with the retained defaults applied.
    #[must_use]
    pub fn refused(
        path: impl Into<String>,
        availability: Option<CollectionUnavailability>,
        error: Option<String>,
    ) -> Self {
        Self::Refused {
            path: path.into(),
            availability: availability.unwrap_or(Self::DEFAULT_REFUSAL),
            reason: error.unwrap_or_else(|| Self::DEFAULT_REASON.to_owned()),
        }
    }

    /// Where the record lives.
    #[must_use]
    pub fn path(&self) -> &str {
        match *self {
            Self::Read { ref path, .. } | Self::Refused { ref path, .. } => path,
        }
    }

    /// The collection, when there is one.
    #[must_use]
    pub fn collection(&self) -> Option<&MeasurementCollection> {
        match *self {
            Self::Read { ref collection, .. } => Some(collection),
            Self::Refused { .. } => None,
        }
    }
}

/// Opaque FR-062 results, as the caller injects them.
/// `graph-portfolio.ts:45-57`.
#[derive(Clone, Debug, PartialEq)]
pub enum InjectedStructuralGraph {
    /// The export, its premises and the two analyses were produced.
    Available {
        /// Where the export came from, when the caller said.
        path: Option<String>,
        /// The premises document, uninterpreted.
        premises: JsonValue,
        /// The fan-out analysis, uninterpreted.
        fan_out: JsonValue,
        /// The churn analysis, uninterpreted.
        churn: JsonValue,
        /// The change-impact analyses, when a changed seed was requested.
        change_impact: Option<Vec<JsonValue>>,
    },
    /// It was not, for this stated reason.
    Unavailable {
        /// Why not.
        availability: GapAvailability,
        /// Where the export was expected, when the caller said.
        path: Option<String>,
        /// The sentence to report.
        reason: String,
    },
}

/// Why there is no change-impact analysis. `graph-portfolio.ts:183-199`.
///
/// The two sentences are stated once here rather than at the two places
/// [`crate::build::normalize_graph`] would otherwise write them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChangeImpactAbsence {
    /// The caller asked for no changed requirement seed.
    NoSeedRequested,
    /// There is no accepted graph export to analyse.
    NoAcceptedExport,
}

impl ChangeImpactAbsence {
    /// Every reason, in declaration order.
    pub const ALL: [Self; 2] = [Self::NoSeedRequested, Self::NoAcceptedExport];

    /// The stated sentence.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NoSeedRequested => "no changed requirement seed was requested",
            Self::NoAcceptedExport => "change-impact requires an accepted graph export",
        }
    }
}

/// The change-impact member of a normalised graph. `graph-portfolio.ts:178-180`.
#[derive(Clone, Debug, PartialEq)]
pub enum ChangeImpact {
    /// The analyses the caller supplied.
    Analyses(Vec<JsonValue>),
    /// None, for this reason.
    NotApplicable(ChangeImpactAbsence),
}

/// An injected graph with its `changeImpact` member settled.
/// `graph-portfolio.ts:169-200`.
///
/// The unavailable variant carries no `changeImpact` field because there is
/// only one value it can hold — [`ChangeImpactAbsence::NoAcceptedExport`] —
/// and a field that can hold one value is a field that can be set wrong.
#[derive(Clone, Debug, PartialEq)]
pub enum NormalizedStructuralGraph {
    /// The export was accepted.
    Available {
        /// Where the export came from, when the caller said.
        path: Option<String>,
        /// The premises document, uninterpreted.
        premises: JsonValue,
        /// The fan-out analysis, uninterpreted.
        fan_out: JsonValue,
        /// The churn analysis, uninterpreted.
        churn: JsonValue,
        /// The change-impact analyses, or why there are none.
        change_impact: ChangeImpact,
    },
    /// It was not.
    Unavailable {
        /// Why not.
        availability: GapAvailability,
        /// Where the export was expected, when the caller said.
        path: Option<String>,
        /// The sentence to report.
        reason: String,
    },
}

impl NormalizedStructuralGraph {
    /// The wire spelling of this graph's availability.
    #[must_use]
    pub const fn availability(&self) -> &'static str {
        match *self {
            Self::Available { .. } => "available",
            Self::Unavailable { availability, .. } => availability.as_str(),
        }
    }
}

/// Something the governed view needed and did not have.
/// `graph-portfolio.ts:26-33`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphPortfolioGap {
    /// Why the evidence could not be used.
    pub availability: GapAvailability,
    /// What the gap is in.
    pub subject: GapSubject,
    /// Where the missing evidence was expected, when that is known.
    pub path: Option<String>,
    /// The sentence to report.
    pub reason: String,
    /// Who owns it, from the active plan's governance fields.
    pub owner: Option<String>,
    /// What to do about it, from the same.
    pub action: Option<String>,
}

/// One repository's inputs to the governed view. `graph-portfolio.ts:59-64`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphPortfolioRepositoryInput {
    /// The repository's ungoverned portfolio entry.
    pub portfolio: PortfolioRepositoryReport,
    /// Its measurement plans, governance fields included.
    pub plans: Vec<MeasurementPlan>,
    /// Every attempt to read one of its collections.
    pub collections: Vec<GraphCollectionRead>,
    /// Its structural graph, as the caller loaded it.
    pub graph: InjectedStructuralGraph,
}
