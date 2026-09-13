// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Whether a piece of graph evidence could be used, and the four narrowings
//! the retained types declare over that question.
//!
//! # Why five enums and not one with a comment
//!
//! `graph-portfolio.ts` declares `GraphAvailability` once and then narrows it
//! four separate times with `Exclude<…>`:
//!
//! | retained type | admits |
//! |---|---|
//! | `GraphAvailability` (`:18-24`) | all six |
//! | `GraphPortfolioGap.availability` (`:27`) | everything but `available` |
//! | `GraphCollectionRead.availability` (`:36`) | also not `not_applicable` |
//! | `GraphQualityHistoryRow.availability` (`:115`) | four, stated by hand |
//! | `GraphPortfolioResolvedMapping.status` (`:95`) | `missing`, `incompatible` |
//!
//! Those narrowings are load-bearing — a `not_applicable` history row or an
//! `available` gap would each be a contradiction — so they are types here
//! rather than a widened enum plus a review note. Every widening conversion
//! exists ([`From`]); no narrowing one does.

/// Whether a piece of graph evidence could be used. `graph-portfolio.ts:18-24`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GraphAvailability {
    /// The evidence was read and accepted.
    Available,
    /// Nothing is there.
    Missing,
    /// Something is there and could not be read.
    Unreadable,
    /// Whether it is usable could not be decided.
    Unknown,
    /// It was read and does not match what governs it.
    Incompatible,
    /// The question does not arise for this input.
    NotApplicable,
}

/// Declares an enum whose members are a subset of [`GraphAvailability`],
/// with the stable spelling, the census array and the widening conversion.
macro_rules! availability_subset {
    ($(#[$outer:meta])* $name:ident { $($(#[$inner:meta])* $variant:ident => $wire:literal),+ $(,)? }) => {
        $(#[$outer])*
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        pub enum $name {
            $($(#[$inner])* $variant),+
        }

        impl $name {
            /// Every member, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The stable wire spelling.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }

            /// Recover a member from its wire spelling.
            #[must_use]
            pub fn from_wire(value: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|known| known.as_str() == value)
            }
        }

        impl From<$name> for GraphAvailability {
            fn from(value: $name) -> Self {
                match value { $($name::$variant => Self::$variant),+ }
            }
        }
    };
}

availability_subset! {
    /// Why a gap is a gap: everything [`GraphAvailability`] admits except
    /// `available`. `graph-portfolio.ts:27`.
    GapAvailability {
        /// Nothing is there.
        Missing => "missing",
        /// Something is there and could not be read.
        Unreadable => "unreadable",
        /// Whether it is usable could not be decided.
        Unknown => "unknown",
        /// It was read and does not match what governs it.
        Incompatible => "incompatible",
        /// The question does not arise for this input.
        NotApplicable => "not_applicable",
    }
}

availability_subset! {
    /// Why a collection read did not yield a collection. `graph-portfolio.ts:36`.
    ///
    /// `not_applicable` is excluded as well as `available`: a read that did not
    /// produce a collection is never "the question does not arise".
    CollectionUnavailability {
        /// Nothing is there.
        Missing => "missing",
        /// Something is there and could not be read.
        Unreadable => "unreadable",
        /// Whether it is usable could not be decided.
        Unknown => "unknown",
        /// It was read and does not match what governs it.
        Incompatible => "incompatible",
    }
}

availability_subset! {
    /// Whether one retained collection is usable as current graph evidence.
    /// `graph-portfolio.ts:115`.
    HistoryAvailability {
        /// The collection is current-compatible and its attachment verified.
        Available => "available",
        /// It was read and does not match the active plan or itself.
        Incompatible => "incompatible",
        /// Its retained scorer attachment does not hash to its declared digest.
        Unreadable => "unreadable",
        /// Its producer record or scorer identity is not there to check.
        Unknown => "unknown",
    }
}

availability_subset! {
    /// Why a repository's structural graph could not be assembled from the
    /// supplied mappings. `graph-portfolio.ts:95-100`.
    MappingRefusal {
        /// No graph mapping at all was supplied for this repository.
        Missing => "missing",
        /// Some but not all three of export, premises and audit were supplied.
        Incompatible => "incompatible",
    }
}

impl HistoryAvailability {
    /// Whether this reading may be used as current evidence.
    ///
    /// `graph-portfolio.ts:474` and `:494` both ask exactly this, so it is one
    /// question with one spelling rather than two `==` comparisons.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }

    /// The gap this reading raises, or [`None`] when it raises none.
    ///
    /// `graph-portfolio.ts:493-499` pushes a gap for every history row that is
    /// not `available`, and the four-member history vocabulary narrows into
    /// the five-member gap vocabulary exactly.
    #[must_use]
    pub const fn as_gap(self) -> Option<GapAvailability> {
        match self {
            Self::Available => None,
            Self::Incompatible => Some(GapAvailability::Incompatible),
            Self::Unreadable => Some(GapAvailability::Unreadable),
            Self::Unknown => Some(GapAvailability::Unknown),
        }
    }
}

impl From<CollectionUnavailability> for GapAvailability {
    fn from(value: CollectionUnavailability) -> Self {
        match value {
            CollectionUnavailability::Missing => Self::Missing,
            CollectionUnavailability::Unreadable => Self::Unreadable,
            CollectionUnavailability::Unknown => Self::Unknown,
            CollectionUnavailability::Incompatible => Self::Incompatible,
        }
    }
}

impl From<MappingRefusal> for GapAvailability {
    fn from(value: MappingRefusal) -> Self {
        match value {
            MappingRefusal::Missing => Self::Missing,
            MappingRefusal::Incompatible => Self::Incompatible,
        }
    }
}

/// What a gap is a gap *in*. `graph-portfolio.ts:28`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GapSubject {
    /// The repository itself.
    Repository,
    /// One retained measurement collection.
    Collection,
    /// One retained attachment.
    Attachment,
    /// The structural graph export.
    GraphExport,
}

impl GapSubject {
    /// Every subject, in declaration order.
    pub const ALL: [Self; 4] = [
        Self::Repository,
        Self::Collection,
        Self::Attachment,
        Self::GraphExport,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::Collection => "collection",
            Self::Attachment => "attachment",
            Self::GraphExport => "graph_export",
        }
    }

    /// Recover a subject from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

impl GraphAvailability {
    /// Every availability, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Available,
        Self::Missing,
        Self::Unreadable,
        Self::Unknown,
        Self::Incompatible,
        Self::NotApplicable,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
            Self::Unreadable => "unreadable",
            Self::Unknown => "unknown",
            Self::Incompatible => "incompatible",
            Self::NotApplicable => "not_applicable",
        }
    }

    /// Recover an availability from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
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
    use super::{
        CollectionUnavailability, GapAvailability, GapSubject, GraphAvailability,
        HistoryAvailability, MappingRefusal,
    };

    /// Every narrowed member spells itself the way the union member it came
    /// from does, so a widening conversion cannot rename anything.
    #[test]
    fn every_subset_member_agrees_with_the_union_spelling() {
        let mut seen = 0_usize;
        for member in GapAvailability::ALL {
            assert_eq!(
                GraphAvailability::from(*member).as_str(),
                member.as_str(),
                "GapAvailability::{member:?}"
            );
            seen += 1;
        }
        for member in CollectionUnavailability::ALL {
            assert_eq!(GraphAvailability::from(*member).as_str(), member.as_str());
            assert_eq!(GapAvailability::from(*member).as_str(), member.as_str());
            seen += 1;
        }
        for member in HistoryAvailability::ALL {
            assert_eq!(GraphAvailability::from(*member).as_str(), member.as_str());
            seen += 1;
        }
        for member in MappingRefusal::ALL {
            assert_eq!(GraphAvailability::from(*member).as_str(), member.as_str());
            assert_eq!(GapAvailability::from(*member).as_str(), member.as_str());
            seen += 1;
        }
        // Anti-vacuity floor: 5 + 4 + 4 + 2.
        assert_eq!(seen, 15);
    }

    #[test]
    fn every_spelling_round_trips() {
        for member in GraphAvailability::ALL {
            assert_eq!(GraphAvailability::from_wire(member.as_str()), Some(member));
        }
        for subject in GapSubject::ALL {
            assert_eq!(GapSubject::from_wire(subject.as_str()), Some(subject));
        }
        assert_eq!(GraphAvailability::from_wire("nonsense"), None);
    }
}
