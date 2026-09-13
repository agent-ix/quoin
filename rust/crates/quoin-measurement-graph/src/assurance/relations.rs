// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The edges an `assurance-v1` export carries, and how fresh they are.
//!
//! Ports `freshnessSchema`, `relationKindSchema`, the three relation arms and
//! `relationObservationSchema` (`src/measurement/graph-adapters.ts:113-171`).

use serde::Deserialize;

use crate::scalars::{BareDigest, NonEmptyText};
use crate::wire::absent_or;

/// How current an edge is believed to be.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// Observed at the measured revision.
    Current,
    /// Observed, but something under it moved.
    Suspect,
    /// Not established.
    Unknown,
    /// Freshness is not a question for this edge.
    NotApplicable,
}

/// Where a declared relation kind was learned from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKindSource {
    /// A module's vocabulary declared it.
    ModuleVocabulary,
    /// A required-relation rule declared it.
    RequiredRelation,
    /// A trace binding declared it.
    TraceBinding,
    /// It was observed in the corpus.
    Observed,
}

/// One relation kind the export says it can resolve.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationKind {
    /// The kind's name.
    pub kind: NonEmptyText,
    /// Always `available`: an unavailable kind is not declared here.
    pub availability: crate::wire::AvailableLiteral,
    /// Where it was learned from, non-empty.
    pub sources: Vec<RelationKindSource>,
}

/// Whether a corpus edge found its target.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// The target exists.
    Resolved,
    /// The target does not.
    Dangling,
}

/// Which tagging form a `verifies` edge was recovered from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifiesProvenance {
    /// The current tagging form.
    Canonical,
    /// A form kept readable for history.
    Legacy,
}

/// One document-to-document edge read out of the corpus.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusRelation {
    /// The edge's source.
    pub source: NonEmptyText,
    /// The edge's target.
    pub target: NonEmptyText,
    /// Which edge type it is.
    pub edge_type: NonEmptyText,
    /// Whether the target was found.
    pub resolution: Resolution,
    /// Where it was read from.
    pub locator: super::entities::Locator,
    /// How current it is.
    pub freshness: Freshness,
}

/// One symbol-verifies-obligation edge.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiesRelation {
    /// The verifying symbol's identity.
    pub source: BareDigest,
    /// The obligation it verifies.
    pub target: NonEmptyText,
    /// The tagging form it was written in.
    pub form: NonEmptyText,
    /// Which tagging form it was recovered from.
    pub provenance: VerifiesProvenance,
    /// Where it was read from.
    pub locator: super::entities::Locator,
    /// How current it is.
    pub freshness: Freshness,
}

/// One symbol-implements-obligation edge.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImplementsRelation {
    /// The implementing symbol's identity.
    pub source: BareDigest,
    /// The obligation it implements.
    pub target: NonEmptyText,
    /// The tagging form it was written in.
    pub form: NonEmptyText,
    /// Where it was read from.
    pub locator: super::entities::Locator,
    /// How current it is.
    pub freshness: Freshness,
}

/// The three-armed `kind`-discriminated relation union.
///
/// `z.discriminatedUnion("kind", …)` becomes serde's internally tagged enum:
/// both read `kind` first and then apply exactly one arm, so an unknown `kind`
/// is one refusal rather than three.
///
/// The three `z.literal("corpus" | "verifies" | "implements")` discriminants
/// are the *variant tags* here rather than members of the three structs —
/// serde consumes the tag before the arm sees the object — so they are the
/// three `z.literal(` calls `tests/tc_475_schema_census.rs` accounts for as
/// `DISCRIMINANT_LITERALS`.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Relation {
    /// A document-to-document edge.
    Corpus(CorpusRelation),
    /// A symbol-verifies-obligation edge.
    Verifies(VerifiesRelation),
    /// A symbol-implements-obligation edge.
    Implements(ImplementsRelation),
}

/// Whether a declared relation was found at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationAvailability {
    /// Present.
    Available,
    /// Declared and absent.
    Missing,
    /// Not a question for this declaration.
    NotApplicable,
    /// Not established.
    Unknown,
}

/// What the extractor saw when it looked for one declared relation.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationObservation {
    /// The declaration that was looked for.
    pub declaration: NonEmptyText,
    /// The subject it was looked for on, when there is one.
    #[serde(default, deserialize_with = "absent_or")]
    pub subject: Option<NonEmptyText>,
    /// Whether it was found.
    pub availability: ObservationAvailability,
    /// How current the answer is.
    pub freshness: Freshness,
    /// Why, when the extractor said.
    #[serde(default, deserialize_with = "absent_or")]
    pub reason: Option<NonEmptyText>,
}
