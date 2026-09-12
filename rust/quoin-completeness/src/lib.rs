// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! Declared-vocabulary completeness and its verdict policy (FR-037).
//!
//! Rust port of `src/completeness/` (issue #378, EPIC #373 Stage 3).
//!
//! The split ADR-0011 names: **quire-rs computes coverage over the spec corpus,
//! quoin decides what a gap is worth and whether an excuse was earned.** This
//! crate is the second half. It reads the same `traceability.vocabulary_coverage`
//! declaration the engine reads, so the two cannot disagree about how many
//! values a vocabulary has, and it applies the policy the engine deliberately
//! does not: an exclusion is a claim about the product and must carry a written
//! reason.
//!
//! # Parity with the TypeScript it replaces
//!
//! The TypeScript is the oracle and was consulted once:
//! `scripts/capture-semantic-goldens.mjs` recorded its answers into
//! `tests/goldens/`. The tests read those files and never invoke Node.
//!
//! This crate compiles no JSON Schema, so the ajv/`jsonschema` divergence
//! recorded in `../quoin-semantic/DIVERGENCE.md` does not reach it. The one
//! divergence that does is YAML: see §6 there.

#![forbid(unsafe_code)]

pub mod assess;
pub mod bundle;
pub mod declarations;
pub mod error;
pub mod ids;
pub mod run;

pub use assess::{
    assess_vocabulary, verdict_for, written_reason_for, Assessment, CompletenessFinding,
    CompletenessReport, DocumentClaims, FindingKind, Severity, Verdict, VocabularyRollup,
};
pub use bundle::{
    claims_for, read_bundle_claims, read_bundle_frontmatter, BundleDocument, BundleRead,
    FrontmatterRead, UnreadableDocument,
};
pub use declarations::{
    load_vocabulary_coverage, locate_module_root, UnresolvedDeclaration, VocabularyDeclaration,
    VocabularyDeclarations,
};
pub use error::{CompletenessError, CompletenessErrorCode};
pub use ids::{ArtifactTypeName, FrontmatterField, VocabularyName, VocabularyValue};
pub use run::{assess_bundle, AssessOptions, BundleAssessment};
