// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `quire` domain's request and payload shapes, and the ceilings they are
//! read under.
//!
//! # Nothing here is a new vocabulary
//!
//! Both payloads are built out of types that already cross the boundary:
//! [`Obligation`] and [`CoverageDiagnostic`] from `quoin-quire-types`, which
//! `auditor.audit` and `auditor.advise` already read, and [`PropertyShape`]
//! from `quoin-auditor`, which `auditor.advise` already accepts. Declaring a
//! second `Obligation` here — one for the payload, one for the request — is
//! exactly the hand-written duplicate FR-097-CON-1 forbids, and nothing would
//! compare the two.
//!
//! That reuse is what makes the wiring honest end to end: the obligation
//! `quire.coverage` emits is byte-identical, by construction, to the one
//! `auditor.audit` reads back a moment later.

use std::collections::BTreeMap;

use quoin_auditor::advise::PropertyShape;
use quoin_quire_types::{CoverageDiagnostic, Obligation};
use serde::{Deserialize, Serialize};

/// The largest `quire.coverage` or `quire.properties` request, in bytes.
///
/// A request here carries a scope, a module list and a document list — paths,
/// nothing more — so 1 MiB is far past anything a real invocation sends and
/// still refuses a stream (rust-style §11). The *reports* are unbounded and do
/// not travel inbound at all, which is the whole reason the walk happens on
/// this side of the boundary.
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// The largest single path field, in bytes.
pub const MAX_SCALAR_BYTES: usize = 4096;

/// The request accepted by `quire.coverage`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CoverageRequest {
    /// The repository root to derive obligations from.
    pub scope: String,
    /// Module roots supplying the traceability model, in the order given.
    ///
    /// Empty is not "no modules": it is ambient discovery, the resolution
    /// `quire coverage` performs with no `--module`. A closed set replaces
    /// discovery rather than adding to it (quire-rs#405).
    #[serde(default)]
    pub modules: Vec<String>,
}

/// The payload `quire.coverage` writes to stdout.
///
/// Two fields of the engine's report, and that is the whole of what the
/// retained TypeScript read: six commands parsed `quire coverage --json` and
/// between them touched `obligations` and `diagnostics` and nothing else. The
/// rest of `CoverageReport` — totals, rows, symbols, the status census — is
/// rendered by `quire` itself and was never quoin's to carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CoveragePayload {
    /// The obligations the run derived, in the engine's order.
    pub obligations: Vec<Obligation>,
    /// The run's diagnostics, in the engine's order.
    pub diagnostics: Vec<CoverageDiagnostic>,
}

/// The request accepted by `quire.properties`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PropertiesRequest {
    /// The repository root documents resolve and report relative to.
    pub scope: String,
    /// Module roots supplying the archetypes and the `property_idioms`
    /// registry. Empty means ambient discovery, as in [`CoverageRequest`].
    #[serde(default)]
    pub modules: Vec<String>,
    /// Documents to classify, scope-relative.
    ///
    /// Named by the caller rather than discovered here, because a glob is a
    /// filesystem walk and this half performs none. `quoin_quire::properties::
    /// documents_under_spec` is the host's way to answer the `spec/**/*.md`
    /// the retained `quoin advise` passed; an empty list asks for exactly that.
    #[serde(default)]
    pub documents: Vec<String>,
}

/// The payload `quire.properties` writes to stdout.
///
/// The **shape map**, not the classification report. `quoin advise` was the one
/// caller, and it walked every document and every criterion to build
/// `Map<row_id, {property, archetype}>` — so the map is the answer, and
/// `PropertiesReport`, `Document`, `Criterion`, `AcShape`, `Extraction` and
/// `Span` never needed to cross at all. `shapes` is keyed by obligation id and
/// feeds `auditor.advise`'s `shapes` field unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PropertiesPayload {
    /// Obligation id → what the classifier made of that criterion.
    pub shapes: BTreeMap<String, PropertyShape>,
    /// Documents that resolved to no archetype, scope-relative.
    ///
    /// Reported rather than raised: `quire properties` exits 1 when ANY
    /// document fails to resolve while still classifying every one that did,
    /// and the retained `propertyShapes` swallowed that exit deliberately so
    /// two untyped assets could not cost the whole shape axis. Here the
    /// partial result and the list of what was skipped are one answer, which
    /// is what an exit status could not say.
    pub unresolved: Vec<String>,
}
