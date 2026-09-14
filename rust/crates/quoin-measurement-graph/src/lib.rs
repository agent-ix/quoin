// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
#![forbid(unsafe_code)]
//! Retained graph evidence: the FR-066 versioned adapters that admit it
//! (`src/measurement/graph-adapters.ts`, quoin#475) and the FR-067 governed
//! portfolio projection that reports it
//! (`src/measurement/graph-portfolio.ts`, quoin#476), with the filesystem
//! boundary that feeds the projection
//! (`src/measurement/graph-portfolio-load.ts`, quoin#477).
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
//! | `Buffer.from(x, "base64")` both ways | [`base64`] |
//!
//! # The governed portfolio (FR-067)
//!
//! The adapters answer *may this evidence be admitted*. The portfolio answers
//! *what does the evidence say, and where is it missing* — for one metric,
//! `graph_quality`, across many repositories. It is the **governed sibling** of
//! [`quoin_measurement::portfolio`]: that view reports what each repository
//! measured and how stale the reading is; this one additionally holds every
//! reading to the active plan and re-checks the retained scorer attachment
//! against the digest its producer declared.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `graph-portfolio.ts:18-24`, the four `Exclude<…>` narrowings | [`availability`] |
//! | `graph-portfolio.ts:26-64`, `169-200` | [`input`] |
//! | `graph-portfolio.ts:102-164` | [`reading`] |
//! | `graph-portfolio.ts:166-207` | [`report`] |
//! | `graph-portfolio.ts:209-268`, `772-838` | [`mapping`] |
//! | `graph-portfolio.ts:270-296`, `410-565` | [`build`] |
//! | `graph-portfolio.ts:567-641`, `757-779` | [`history`] |
//! | `graph-portfolio.ts:795-836` | [`scorer`] |
//! | `graph-portfolio.ts:298-324`, `519-556`, `643-712` | [`compare`] |
//! | `graph-portfolio.ts:326-329` | [`render_json`] |
//! | `graph-portfolio.ts:331-408` | [`render`] |
//! | `compare`, `compareInstants` | [`order`] |
//! | `markdownCell`, `inlineText` | [`text`] |
//! | `asRecord`, `stringField` | [`fields`] |
//!
//! # The loader (FR-067, quoin#477)
//!
//! [`build`] is a pure projection over inputs somebody hands it. [`loader`] is
//! the only module that decides *which bytes* those are, and [`structural`] is
//! the only one that turns a resolved mapping into an FR-062 result. Together
//! they are `graph-portfolio-load.ts`, and they read nothing themselves: the
//! collections are [`quoin_measurement::read_measurement_collection_results`]'s,
//! the plans [`quoin_measurement::load_measurement_plans`]'s, the ungoverned
//! entry [`quoin_measurement::portfolio::build_portfolio_report_from_collections`]'s
//! and the three graph documents [`quoin_graph_analysis`]'s behind its one
//! [`quoin_graph_analysis::GraphInputReader`] seam.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `graph-portfolio-load.ts:20-93`, `buildGovernedGraphPortfolio` | [`loader`] |
//! | `graph-portfolio-load.ts:95-149`, `loadStructuralGraph` | [`structural`] |
//! | `graph-portfolio-load.ts:151-161`, `pathFor` | [`structural`] |
//!
//! ## What the portfolio refuses to do
//!
//! - **It owns no graph semantics.** `premises`, `fanOut`, `churn` and
//!   `changeImpact` are opaque FR-062 results carried as
//!   [`quoin_store::JsonValue`], compared by nothing and interpreted by
//!   nothing. `graph-portfolio.ts:41-43` says so and [`input`] is that promise
//!   in the type system.
//! - **It aggregates nothing.** No partition, repository or metric is summed,
//!   averaged or ranked, and the rendered report says so in its second line.
//! - **It re-declares nothing [`quoin_measurement`] already owns.** The
//!   portfolio entry, the plan, the collection, the observation, the ECMA-262
//!   number spelling and the one RFC 3339 grammar all come from there, and
//!   [`render_json`] *flattens*
//!   [`quoin_measurement::portfolio::render_json::RepositoryWire`] because the
//!   TypeScript spreads that very object.
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
//! # Declared divergences
//!
//! Every place the two implementations would answer differently is declared in
//! this crate's `DIVERGENCE.md`, which is the index over the module
//! documentation below and over `DECLARED_DIVERGENCES` in
//! `tests/tc_475_parity.rs`.
//!
//! The port is not bit-identical to the retained modules on every input. The
//! places it is not are stated, measured against a captured TypeScript oracle,
//! and asserted to have *actually fired* wherever firing is observable: the
//! base64 decoder's strictness (quoin#465, see [`base64`]), `Date.parse`'s
//! implementation-defined heuristic (see [`attestation`] and [`order`]), and a
//! non-string `dimensions` member (see [`history::partition_identity`]).
//!
//! A third difference — the observation identity sorts member names by UTF-16
//! code unit where the retained module sorts by code point (see
//! [`quality::identity`]) — is **not** declared, because a declaration that
//! cannot fire is a licence rather than a measurement. It is proved unreachable
//! instead: every member name an admissible record may carry is schema-fixed
//! ASCII, and `tests/tc_475_parity.rs` measures that over the corpus.
//!
//! quoin#477 adds one more of each. Declared: the sentence a graph refusal
//! carries when an operating system or a schema validator wrote it
//! (`DIVERGENCE.md` §9), which `tests/tc_477_graph_loader.rs` compares by
//! verdict rather than by byte. Proved unreachable rather than declared: the
//! `portfolio omitted resolved repository` refusal `graph-portfolio-load.ts:74`
//! throws (`DIVERGENCE.md` §10).
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
pub mod availability;
pub mod base64;
pub mod build;
pub mod canonical;
pub mod compare;
pub mod error;
pub mod fields;
pub mod history;
pub mod input;
pub mod loader;
pub mod mapping;
pub mod order;
pub mod quality;
pub mod reading;
pub mod render;
pub mod render_json;
pub mod report;
pub mod scalars;
pub mod scorer;
pub mod structural;
pub mod text;
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

pub use build::build_governed_graph_portfolio_from;
pub use compare::compare_graph_quality_collections;
pub use input::{
    GraphCollectionRead, GraphPortfolioGap, GraphPortfolioRepositoryInput, InjectedStructuralGraph,
    NormalizedStructuralGraph,
};
pub use loader::build_governed_graph_portfolio;
pub use mapping::{GraphPortfolioMappingOptions, ResolvedMapping, parse_graph_portfolio_mappings};
pub use render::render_governed_graph_portfolio;
pub use render_json::{canonical_graph_portfolio_json, canonical_graph_portfolio_json_bytes};
pub use report::{GovernedGraphPortfolioReport, GovernedGraphRepositoryReport};
pub use structural::{load_structural_graph, load_structural_graph_with};
