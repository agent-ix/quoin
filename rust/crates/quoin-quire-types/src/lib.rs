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
/// Both fields here are required, for the reason in the module header: they
/// are read unconditionally, so their absence must be an error rather than a
/// silent empty.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Obligation {
    /// The obligation id, e.g. `FR-001-AC-1` or `NFR-010-M-2`.
    pub id: String,
    /// The criterion's statement, in the spec's own words.
    pub statement: String,
}
