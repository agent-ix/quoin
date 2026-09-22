// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What a portfolio report is made of.
//!
//! Ports `portfolio.ts:26-75`. The two places the retained interfaces carry a
//! string beside the state it explains — `status`/`error` and
//! `staleness.status`/`ageDays` — are one value here, for the reason
//! `portfolio.ts:151-157` shows: the renderer reads `repository.error ??
//! "unknown read error"`, a fallback that exists only because the type admits a
//! repository that is not readable and says nothing about why.

use crate::report::build::MeasurementReport;
use crate::types::comparison::MeasurementComparison;
use crate::types::plan::MeasurementPlan;
use crate::types::profile::AssuranceProfileSummary;

/// How many days behind the portfolio's newest collection a repository may be
/// before it is stale. `portfolio.ts:27`.
pub const PORTFOLIO_STALE_AFTER_DAYS: i64 = 30;

/// Milliseconds in a day. `portfolio.ts:26`.
pub(crate) const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Whether a repository could be read, and why not when it could not.
///
/// `portfolio.ts:49-50`'s `status` and `error` pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryStatus {
    /// The repository was read.
    Readable,
    /// Nothing exists at the location.
    Missing(String),
    /// Something is there and could not be read, for this stated reason.
    Unreadable(String),
}

impl RepositoryStatus {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::Readable => "readable",
            Self::Missing(_) => "missing",
            Self::Unreadable(_) => "unreadable",
        }
    }

    /// The stated reason, which only a refusal has.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        match *self {
            Self::Readable => None,
            Self::Missing(ref error) | Self::Unreadable(ref error) => Some(error),
        }
    }
}

/// What the repository's measurement store holds. `portfolio.ts:51`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum StoreState {
    /// The directory exists and holds collections.
    Present,
    /// The directory exists and holds none.
    Empty,
    /// The directory does not exist.
    Missing,
    /// The repository could not be read at all.
    Unreadable,
}

impl StoreState {
    /// Every state, in declaration order.
    pub const ALL: [Self; 4] = [Self::Present, Self::Empty, Self::Missing, Self::Unreadable];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Empty => "empty",
            Self::Missing => "missing",
            Self::Unreadable => "unreadable",
        }
    }

    /// Recover a state from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// How far behind the portfolio's newest collection a repository is.
///
/// `portfolio.ts:57-62`'s four members, with the three that are only ever
/// stated together carried by the variant that states them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Staleness {
    /// No collection to measure against.
    NotComputed,
    /// Within the threshold, this many whole days behind `relative_to`.
    Current {
        /// Whole days behind the portfolio's newest collection.
        age_days: i64,
        /// The timestamp that age is relative to.
        relative_to: String,
    },
    /// Past the threshold.
    Stale {
        /// Whole days behind the portfolio's newest collection.
        age_days: i64,
        /// The timestamp that age is relative to.
        relative_to: String,
    },
}

impl Staleness {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::NotComputed => "not_computed",
            Self::Current { .. } => "current",
            Self::Stale { .. } => "stale",
        }
    }

    /// Whole days behind, when there is a collection to measure against.
    #[must_use]
    pub const fn age_days(&self) -> Option<i64> {
        match *self {
            Self::NotComputed => None,
            Self::Current { age_days, .. } | Self::Stale { age_days, .. } => Some(age_days),
        }
    }

    /// The timestamp the age is relative to.
    #[must_use]
    pub fn relative_to(&self) -> Option<&str> {
        match *self {
            Self::NotComputed => None,
            Self::Current {
                ref relative_to, ..
            }
            | Self::Stale {
                ref relative_to, ..
            } => Some(relative_to),
        }
    }

    /// The staleness of a repository `age_days` behind `relative_to`.
    ///
    /// `portfolio.ts:128` — the threshold is `>`, so a repository exactly
    /// [`PORTFOLIO_STALE_AFTER_DAYS`] days behind is current.
    #[must_use]
    pub fn of(age_days: i64, relative_to: String) -> Self {
        if age_days > PORTFOLIO_STALE_AFTER_DAYS {
            Self::Stale {
                age_days,
                relative_to,
            }
        } else {
            Self::Current {
                age_days,
                relative_to,
            }
        }
    }
}

/// A collection, as the portfolio view quotes it. `portfolio.ts:29-38`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioCollectionRef {
    /// The collection's identity.
    pub id: String,
    /// Where the record lives.
    pub path: String,
    /// When it was produced.
    pub timestamp: String,
    /// The revision of the measured source.
    pub source_revision: String,
    /// Which tool produced it.
    pub tool_identity: String,
    /// Which version of that tool.
    pub tool_version: String,
    /// The digest of the configuration it ran under.
    pub config_digest: String,
    /// The measured corpus revision, when the collection states a non-empty
    /// one — `portfolio.ts:327-329` spreads it only when it is truthy.
    pub corpus_revision: Option<String>,
    /// `verificationStack.artifacts` names with no local filesystem entry,
    /// sorted; empty when there are none or no verification stack at all
    /// (PLAT-969's ruling — see [`crate::report::build::CollectionSummary`]).
    pub unverified_artifacts: Vec<String>,
}

/// Two of a repository's collections, compared. `portfolio.ts:40-44`.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioComparison {
    /// The older collection.
    pub before: PortfolioCollectionRef,
    /// The newer collection.
    pub after: PortfolioCollectionRef,
    /// The compared slices.
    pub observations: Vec<MeasurementComparison>,
}

/// One repository's portfolio entry. `portfolio.ts:46-63`.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioRepositoryReport {
    /// The repository's directory name, or its root when it has none.
    pub name: String,
    /// The resolved repository root.
    pub root: String,
    /// Whether it could be read.
    pub status: RepositoryStatus,
    /// What its measurement store holds.
    pub store: StoreState,
    /// Its active assurance profiles.
    pub profiles: Vec<AssuranceProfileSummary>,
    /// Its active measurement plans.
    pub plans: Vec<MeasurementPlan>,
    /// Its own measurement report, when it could be built.
    pub measurements: Option<MeasurementReport>,
    /// Its newest collection, when it has one.
    pub latest_collection: Option<PortfolioCollectionRef>,
    /// Its two newest collections compared, when it has two.
    pub comparison: Option<PortfolioComparison>,
    /// How far behind the portfolio it is.
    pub staleness: Staleness,
}

/// The whole portfolio. `portfolio.ts:65-70`.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioReport {
    /// The newest collection timestamp across every repository.
    pub newest_collection_timestamp: Option<String>,
    /// The repositories, in resolved-root order.
    pub repositories: Vec<PortfolioRepositoryReport>,
}

impl PortfolioReport {
    /// The report's schema version. `portfolio.ts:66,135`: always 1.
    pub const SCHEMA_VERSION: i64 = 1;
}

/// A caller's already-read store snapshot. `portfolio.ts:72-75`.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioCollectionSnapshot {
    /// The repository root the collections were read from.
    pub root: std::path::PathBuf,
    /// Every collection under it, oldest first.
    pub collections: Vec<crate::types::collection::MeasurementCollection>,
}
