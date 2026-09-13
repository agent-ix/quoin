// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
#![forbid(unsafe_code)]
//! The FR-066 versioned adapters for retained graph evidence, ported from
//! `src/measurement/graph-adapters.ts` (quoin#475).
//!
//! # What an adapter is for
//!
//! Two producers emit graph evidence and neither emits a measurement
//! collection. An adapter is the one place their evidence becomes governed
//! evidence, and it is versioned so that a producer changing its output is a
//! new adapter rather than a silent change of meaning.
//!
//! - [`assurance`] admits Quire's authoritative `quire-assurance` export
//!   **without translating it**: the export is already the contract, so the
//!   adapter's whole job is to refuse one taken under premises the caller did
//!   not agree to.
//! - [`quality`] **transcribes** `quire-code-rs`'s graph-quality observation
//!   into one [`quoin_measurement::MeasurementCollection`], because that
//!   producer's shape is its own and the measurement contract is quoin's.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `GRAPH_ADAPTER_NAMES`, `selectGraphAdapter` | [`adapter`] |
//! | `GraphAdapterError`, `GraphAdapterErrorCode` | [`error`] |
//! | the `z.string().regex(…)` scalars | [`scalars`] |
//! | `z.literal`, `.optional()`, `.nullable()` | [`wire`] |
//! | `quireAssuranceSchema`, `adaptQuireAssurance` | [`assurance`] |
//! | `graphQualityObservationSchema`, `adaptGraphQualityObservation` | [`quality`] |
//! | `invocationAttestationSchema` | [`attestation`] |
//! | `canonicalJson` call sites | [`canonical`] |
//! | `Buffer.from(x).toString("base64")` | [`base64`] |
//!
//! # What is deliberately absent
//!
//! - **No canonicalizer and no digest.** The bytes are
//!   [`quoin_store::canonical_json`] and [`quoin_store::canonical_bytes`]'s and
//!   the sha256 is [`quoin_store::digest_bytes_sha256`]'s (quoin#484). See
//!   [`canonical`] and [`quality::identity`].
//! - **No second `serde_json` ↔ [`quoin_store::JsonValue`] crossing.**
//!   [`quoin_measurement::json_bridge`] is the workspace's one crossing.
//!   [`canonical`] and [`quality::adapt`] both call it.
//! - **No second instant grammar.** [`quoin_measurement::Rfc3339DateTime`] is
//!   the one this workspace has, per the Stage 6 plan §5.
//! - **No schema validator of its own.** `assurance-v1` is checked by
//!   [`quoin_quire::validate_assurance`] (quoin#474) and the transcribed
//!   collection by [`quoin_measurement::validate::measurement_collection`].
//! - **No `bool`-returning validator.** Every check mints the type that proves
//!   it, which is the `parse, don't validate` rule
//!   [`quoin_store::RawFileSha256Digest::parse_stored`] models.
//!
//! # Two declared divergences, and one difference proved unreachable
//!
//! The port is not bit-identical to the retained module. The two places it is
//! not are stated, measured against a captured TypeScript oracle, and asserted
//! to have *actually fired*: the base64 decoder's strictness (quoin#465, see
//! [`base64`]) and `Date.parse`'s implementation-defined heuristic (see
//! [`attestation`]).
//!
//! A third difference — the observation identity sorts member names by UTF-16
//! code unit where the retained module sorts by code point (see
//! [`quality::identity`]) — is **not** declared, because a declaration that
//! cannot fire is a licence rather than a measurement. It is proved unreachable
//! instead: every member name an admissible record may carry is schema-fixed
//! ASCII, and `tests/tc_475_parity.rs` measures that over the corpus.
//!
//! # The TypeScript is retained
//!
//! This is a port wave. `src/measurement/graph-adapters.ts` is untouched, and
//! it is not a runtime oracle: the corpus under `tests/corpus/` was captured
//! from it once and committed, per FR-101 AC-5. The cutover is W11 and the
//! deletion is W12.

pub mod adapter;
pub mod assurance;
pub mod attestation;
pub mod base64;
pub mod canonical;
pub mod error;
pub mod quality;
pub mod scalars;
pub mod wire;

pub use adapter::{GRAPH_ADAPTER_NAMES, GraphAdapterName, select_graph_adapter};
pub use assurance::QuireAssuranceV1;
pub use assurance::adapt::adapt_quire_assurance;
pub use assurance::premise::AcceptedQuirePremises;
pub use attestation::InvocationAttestation;
pub use error::{GraphAdapterError, GraphAdapterErrorCode, Result};
pub use quality::GraphQualityObservationV1;
pub use quality::adapt::{
    AdaptGraphQualityInput, TranscribedCollection, adapt_graph_quality_observation,
};
pub use quality::identity::graph_quality_observation_id;
