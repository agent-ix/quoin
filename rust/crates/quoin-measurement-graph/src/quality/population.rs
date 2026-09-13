// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What the extractor walked, and the census it kept while walking.
//!
//! Ports `censusItemSchema`, `dimensionSchema` and `populationSchema`
//! (`src/measurement/graph-adapters.ts:211-262`).

use serde::Deserialize;

use crate::scalars::NonEmptyText;

/// One `(key, count)` row of a census.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CensusItem {
    /// What was counted.
    pub key: NonEmptyText,
    /// How many.
    pub count: u64,
}

/// The axes a graph-quality fact can be sliced along.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    /// Not sliced.
    Overall,
    /// By source language.
    Language,
    /// By node kind.
    NodeKind,
    /// By relation kind.
    RelationKind,
    /// By which resolver tier answered.
    ResolverTier,
}

impl Dimension {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Overall => "overall",
            Self::Language => "language",
            Self::NodeKind => "node_kind",
            Self::RelationKind => "relation_kind",
            Self::ResolverTier => "resolver_tier",
        }
    }
}

/// Whether the extractor measured anything, and if not, why not.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationState {
    /// Files were walked and results were produced.
    Measured,
    /// There was nothing to walk.
    Empty,
    /// What there was could not be read.
    Unreadable,
    /// What there was is in no supported language.
    Unsupported,
}

impl PopulationState {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Empty => "empty",
            Self::Unreadable => "unreadable",
            Self::Unsupported => "unsupported",
        }
    }
}

/// The four census axes the extractor keeps.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Census {
    /// By language.
    pub languages: Vec<CensusItem>,
    /// By node kind.
    pub node_kinds: Vec<CensusItem>,
    /// By relation kind.
    pub relation_kinds: Vec<CensusItem>,
    /// By resolver tier.
    pub resolver_tiers: Vec<CensusItem>,
}

impl Census {
    /// The four axes, each with the dimension name the transcription slices it
    /// under.
    ///
    /// The retained loop is `Object.entries(record.population.census)`, so the
    /// dimension names are the **member names of the census object** —
    /// `languages`, not `language` — and the order is the object's own. Stated
    /// here as a list rather than derived, because that is what makes the
    /// order a fact of this port rather than of whichever iterator ran.
    #[must_use]
    pub fn axes(&self) -> [(&'static str, &[CensusItem]); 4] {
        [
            ("languages", &self.languages),
            ("node_kinds", &self.node_kinds),
            ("relation_kinds", &self.relation_kinds),
            ("resolver_tiers", &self.resolver_tiers),
        ]
    }
}

/// What the extractor walked.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Population {
    /// Whether anything was measured.
    pub state: PopulationState,
    /// How many files were seen at all.
    pub files_seen: u64,
    /// How many of those were in a supported language.
    pub supported_files: u64,
    /// How many could not be read.
    pub unreadable_files: u64,
    /// How many were in no supported language.
    pub unsupported_files: u64,
    /// The census kept while walking.
    pub census: Census,
}

impl Population {
    /// The four file counts, in the order the transcription emits them.
    ///
    /// `graph-adapters.ts:604-610` names them in this order and emits one
    /// observation per name; a `BTreeMap` or a derive would emit a different
    /// one, and the order is observable in the sorted output only by accident.
    #[must_use]
    pub fn file_counts(&self) -> [(&'static str, u64); 4] {
        [
            ("files_seen", self.files_seen),
            ("supported_files", self.supported_files),
            ("unreadable_files", self.unreadable_files),
            ("unsupported_files", self.unsupported_files),
        ]
    }
}
