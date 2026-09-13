// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `quire-assurance` format version 1, and the adapter that admits one.
//!
//! Ports `quireAssuranceSchema` and `adaptQuireAssurance`
//! (`src/measurement/graph-adapters.ts:172-201, 423-448`).

pub mod adapt;
pub mod entities;
pub mod premise;
pub mod relations;

use serde::Deserialize;

use crate::wire::{FormatVersion1, QuireAssuranceFormat};
use entities::{Artifact, Obligation, Symbol};
use premise::{ModulePremise, SourcePremise};
use relations::{Relation, RelationKind, RelationObservation};

/// One `quire-assurance` export, at format version 1.
///
/// Minted only by [`adapt::adapt_quire_assurance`]: holding one is the proof
/// that the published schema accepted it *and* that its premises are the ones
/// the caller agreed to. Parse, don't validate.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuireAssuranceV1 {
    /// Always `quire-assurance`.
    pub format: QuireAssuranceFormat,
    /// Always `1`.
    pub format_version: FormatVersion1,
    /// Where the export was taken from.
    pub source: SourcePremise,
    /// Which modules were active.
    pub modules: Vec<ModulePremise>,
    /// The spec artifacts.
    pub artifacts: Vec<Artifact>,
    /// The obligations.
    pub obligations: Vec<Obligation>,
    /// The symbols.
    pub symbols: Vec<Symbol>,
    /// The relation kinds the extractor could resolve.
    pub relation_kinds: Vec<RelationKind>,
    /// The edges.
    pub relations: Vec<Relation>,
    /// What was seen when each declared relation was looked for.
    pub relation_observations: Vec<RelationObservation>,
}
