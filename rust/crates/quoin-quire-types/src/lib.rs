// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deserialize-only readers for quire's wire formats (quoin#384).
//!
//! # This crate defines nothing
//!
//! **quire owns every format in this crate.** These types are projections of
//! what `quire coverage --json` emits, restricted to the fields quoin actually
//! reads. They are not a definition, they do not compete with quire's own
//! types, and quoin is not entitled to change the shape by changing this file.
//!
//! The alternative was a dependency on quire-rs's crate. Rejected: it buys a
//! version coupling across the whole burn-down for two fields, and the
//! burn-down is the thing that has to stay able to move.
//!
//! # What that obliges of anyone editing this crate
//!
//! Adding a field here is adding a field to a **reader**. Check quire's
//! emitter first, and match its spelling exactly — a reader that invents a
//! name reads nothing and reports empty, which is the silent-drift failure
//! this whole program exists to catch.
//!
//! The coupling is recorded from quire's side too, so it is visible to the
//! owner rather than only to us.
//!
//! # How drift is made loud
//!
//! Two mechanisms, because either alone is insufficient:
//!
//! 1. **Fields quoin reads are required, never `Option`.** A rename or removal
//!    upstream fails deserialization at the boundary instead of reading as an
//!    empty string and rendering a blank row.
//! 2. **The fixture is captured from the pinned quire binary**, never written
//!    by hand — a hand-written fixture asserts against bytes the test itself
//!    invented, which FR-101 names as a self-fixture tautology. See
//!    `tests/coverage_reader.rs`.
//!
//! There is deliberately **no** `deny_unknown_fields`: [`Obligation`] reads
//! two of the nine fields quire emits, so unknown fields are the normal case
//! and not an error.

pub mod clauses;

pub use clauses::{
    ClauseBinding, ClauseBindingOutcome, ClauseBindingReason, ClauseBindingReport,
    ClauseBindingSchemaVersion, ClauseForce, ClauseSetKey, EngineProvenance,
};

use serde::Deserialize;

/// One obligation, as quoin reads it off `quire coverage --json`.
///
/// # Two fields of nine
///
/// quire emits `source`, `id`, `document`, `statement`, `statement_hash`,
/// `method`, `parameters`, `criticality` and `target_ids`. The assurance view
/// reads `id` (to derive the owning requirement, and as the solution node's
/// id) and `statement` (as the node's statement). It reads none of the other
/// seven — not even `document`, which a reader might reasonably expect a view
/// to cite, and does not.
///
/// The three required fields here are required for the reason in the module
/// header: they are read unconditionally, so their absence must be an error
/// rather than a silent empty. `statement_hash` joined the set when the
/// evidence store arrived (agent-ix/quoin#456): a binding stamps it, and a
/// default would make every binding agree with every statement.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Obligation {
    /// The obligation id, e.g. `FR-001-AC-1` or `NFR-010-M-2`.
    pub id: String,
    /// The criterion's statement, in the spec's own words.
    pub statement: String,
    /// quire's hash of the statement, as it stands at the read.
    ///
    /// quoin never computes this and only ever compares it: suspect detection
    /// is `stamped != current`, so a hash quoin derived itself would compare
    /// equal to itself and detect nothing.
    pub statement_hash: String,
    /// Test-case ids the criterion's method cell names, when it names any.
    ///
    /// The indirection an agent-eval report and every other Test-Matrix-keyed
    /// tool arrives on: the tool reports `TC-EV-057`, the row says that test
    /// case verifies this criterion, and quire-rs FR-053-AC-11 carries the join
    /// here rather than making quoin re-parse the table (agent-ix/quoin#144).
    #[serde(default)]
    pub target_ids: Option<Vec<String>>,
    /// The authored `Verification` cell, verbatim.
    ///
    /// # Why the three below are `Option` when the header says fields quoin reads are required
    ///
    /// The rule in the module header is about **drift**: a field quoin reads
    /// unconditionally must fail loudly if quire renames it, rather than read
    /// as an empty string. These three are not read unconditionally — quire
    /// emits them as `method?: string | null`, `parameters?` and
    /// `criticality?: string | null`, and every reader here branches on the
    /// absence first. `unknownMethodFinding` returns `null` without a method;
    /// `multiplicityFinding` and `mutationFinding` return `null` without a
    /// criticality. An absence is an answer, so it must be representable.
    ///
    /// `null` and the missing key both read as [`None`], which is what the
    /// retained `!obligation.method` guard does with either.
    #[serde(default)]
    pub method: Option<String>,
    /// The obligation's declared criticality, verbatim (`P0`, `high`, …).
    ///
    /// Carried, never interpreted. CR-008 deleted a hardcoded `["P0"]`
    /// precisely so the engine does not decide which values count as high.
    #[serde(default)]
    pub criticality: Option<String>,
    /// The obligation's structured parameters, as quire emits them —
    /// `{"target": "< 4 min", "threshold": "< 5 min"}` on an NFR row.
    ///
    /// A `BTreeMap` and not a `Value`: the advisor asks whether the keys
    /// `target` or `threshold` are present, which an opaque value would put a
    /// cast in front of at the one read site.
    #[serde(default)]
    pub parameters: Option<std::collections::BTreeMap<String, String>>,
}

/// One coverage diagnostic, restricted to the two fields the advisor joins on.
///
/// # Two fields of seven
///
/// quire emits `declaration`, `reason`, `message`, `path`, `line`, `value` and
/// a subject field. The advisor reads `reason` — to select
/// `uncatalogued-verification-method` — and `value`, which quire-rs CR-091
/// guarantees is byte-equal to the [`Obligation::method`] it is about. It
/// renders none of the rest, so none of the rest is declared.
///
/// [`CoverageDiagnostic::value`] is the one `Option` that carries meaning by
/// being absent: an engine predating CR-091 emits the reason with no value,
/// and the advisor must degrade to two-state behaviour rather than misread
/// silence as "every authored method is catalogued".
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CoverageDiagnostic {
    /// The open machine vocabulary quire classifies the diagnostic under
    /// (quire-rs FR-055 leaves it open, so this is a `String`).
    pub reason: String,
    /// The catalog or vocabulary value the diagnostic is about, verbatim.
    #[serde(default)]
    pub value: Option<String>,
}
