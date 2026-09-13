// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
//! Deterministic, read-only assurance-graph projections (FR-062, quoin#385).
//!
//! The Rust successor to `src/graph-analysis/` — 1,373 lines across five files,
//! all of which compute and none of which decide policy. Three views over one
//! input:
//!
//! | view | question | retained |
//! |---|---|---|
//! | [`analyze_fan_out`] | which suites carry which live obligations | `analysis.ts:154` |
//! | [`analyze_churn`] | which obligations were re-affirmed, by whom | `analysis.ts:197` |
//! | [`analyze_change_impact`] | what depends on these requirements | `analysis.ts:263` |
//!
//! # One IO seam, and it is the whole filesystem surface
//!
//! [`analyze_fan_out`], [`analyze_churn`] and [`analyze_change_impact`] take an
//! in-memory [`GraphAnalysisInput`] and touch nothing. The only reads are in
//! [`load`], behind [`GraphInputReader`]: an existence probe and two byte
//! reads, exactly the three calls `src/graph-analysis/load.ts` makes. No
//! subprocess, no network, no environment and no working directory.
//!
//! # There is no canonical JSON in this crate
//!
//! `renderGraphAnalysisJson` is `canonicalJson`, and here it is
//! [`quoin_store::canonical_json_bytes`] — called, never reimplemented.
//! Everywhere the retained source writes `compare(left, right)` it means
//! JavaScript `<` on strings, which is UTF-16 code-unit order;
//! `quoin_store::json::order::cmp_utf16` is that comparator and the id
//! newtypes in [`ids`] derive their `Ord` from it, so a `BTreeMap` keyed on
//! one iterates in the order the oracle sorts in. `tests/tc_385_one_canonical_json.rs`
//! asserts that no second implementation of either has appeared.
//!
//! # "corpus" here means the spec corpus
//!
//! `AssuranceRelation::Corpus` is an edge between two documents in the spec
//! corpus. It has nothing to do with evidence-corpus accounting, which is
//! engineering-assurance's, and `analysis::impact::selected_corpus_edges`
//! carries that warning at its own definition because the name has already
//! misled once (quoin#385).

pub mod analysis;
pub mod error;
pub mod ids;
pub mod input;
mod json;
pub mod load;
pub mod model;
pub mod render;

pub use analysis::{analyze_change_impact, analyze_churn, analyze_fan_out};
pub use error::{GraphError, GraphErrorCode, GraphInput, Result};
pub use ids::{ArtifactId, Author, Commit, DocumentPath, ObligationId, RelationKind, SuiteId};
pub use input::{
    check_accepted_premises, check_audit_identity, parse_accepted_premises, parse_assurance_export,
    parse_audit_envelope,
};
pub use load::{GraphInputReader, GraphLoadOptions, OsGraphInputReader, load_graph_analysis_input};
pub use model::{
    AcceptedPremises, AuditEnvelope, Binding, BindingInput, ChangeImpactAnalysis, ChurnAnalysis,
    DEFAULT_RELATION_KINDS, FanOutAnalysis, GraphAnalysis, GraphAnalysisInput, GraphAnalysisState,
    GraphGap, GraphGapKind,
};
pub use render::{render_graph_analysis, render_graph_analysis_json};
