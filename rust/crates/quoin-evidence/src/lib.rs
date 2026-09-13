// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Quoin's evidence store and its producer adapters (quoin#456, FR-100).
//!
//! # What this crate owns
//!
//! The record vocabulary in [`types`], and the producer adapters in
//! [`adapters`] that transcribe a tool's own output into it. The store's
//! read/write surface follows in the same crate — one crate with module trees,
//! not a second crate for the adapters, because the adapters exist to produce
//! exactly the records the store holds and splitting them would put a version
//! boundary through one contract.
//!
//! # What this crate does NOT own
//!
//! Canonical JSON, JCS canonicalization, sha256 record identity, blake3 and
//! atomic replacement all live in `quoin-store`, and FR-100-CON-4 forbids a
//! second implementation anywhere — including here. Every byte this crate
//! writes goes out through `quoin_store`.
//!
//! `quoin-evidence-types` holds the assurance renderer's narrow *reader* view
//! of `TrustAssessment` and `IndependenceAssessment`. This crate owns the full
//! shapes those are a projection of; the two are deliberately not merged,
//! because a field nothing renders cannot be covered by a byte comparison and
//! declaring it there would add unobservable surface.
//!
//! # The adapter contract
//!
//! Pure. `parse(&str) -> Result<_, EvidenceError>` reads no file, spawns no
//! process, reaches no network — an adapter that could run the tool would make
//! `quoin evidence record` a test runner, and a transcript nobody can trust
//! (ADR-0011 invariant 1).

#![forbid(unsafe_code)]

pub mod adapters;
pub mod assurance_records;
pub mod error;
pub mod ids;
pub mod independence;
pub mod instant;
pub mod mock_inspection;
pub mod paths;
pub mod record;
pub mod source;
pub mod store;
pub mod trust;
pub mod types;

pub use error::{EvidenceError, EvidenceErrorCode};
pub use ids::{Commit, ObligationId, ProfileId, StatementHash, SuiteId, SymbolId, TrustDecisionId};
pub use source::{DiskEvidence, EvidenceSource, MemoryEvidence};
pub use types::{Finding, MUTATION_SCORE_METRIC, Outcome, RunEntry, STORE_SCHEMA_VERSION};
