// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `semantic` domain's wire shapes, and the ceilings they are read under.
//!
//! Request and payload types only: what a caller may say, and what it gets
//! back. No decision is taken here. The operations that take them live in
//! [`super`], and the exit-taxonomy mapping in [`super::taxonomy`].
//!
//! The size ceilings sit with the shapes rather than with the operations
//! because each is a property of a field and not of the work: they bound what
//! may be read off stdin at all, and [`super`] applies them before any host is
//! consulted. stdin is untrusted (rust-style §11).

use serde::{Deserialize, Serialize};

use quoin_semantic::{SemanticBlock, SemanticDiagnostic, SweepReport};

/// The largest `semantic.read_blocks` request this domain will decide over, in
/// bytes.
///
/// A request is a list of module-root paths. `loadCatalog` asks about every
/// installed module in one call, which is why the whole request is bounded as
/// well as each path: a caller that sends one path repeatedly is bounded by
/// this, and a caller that sends one enormous path is bounded by
/// [`MAX_SCALAR_BYTES`].
pub const MAX_READ_BLOCKS_BYTES: usize = 1 << 20;

/// The largest `semantic.sweep_corpus` request this domain will decide over,
/// in bytes.
///
/// The corpus itself never rides on the request — the roots are walked by the
/// host — so this bounds a list of directory names and two identity strings,
/// not the artifacts.
pub const MAX_SWEEP_CORPUS_BYTES: usize = 1 << 20;

/// The largest single path or identity string this domain will accept, in
/// bytes.
///
/// A module root, a corpus root, a repository name, a revision, a package
/// identity and a timestamp are all one of these. 4 KiB is past every
/// platform's `PATH_MAX` and still refuses a stream.
pub const MAX_SCALAR_BYTES: usize = 4 * 1024;

/// A request with no arguments.
///
/// Deserialised rather than ignored so a caller that sends a field learns that
/// this operation takes none, instead of having it silently dropped.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyRequest {}

/// The request accepted by `semantic.read_blocks`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReadBlocksRequest {
    /// The module roots to read, each a directory holding a `manifest.yaml`.
    ///
    /// A list rather than one root per call: `loadCatalog` reads every
    /// installed module on every `quoin write`, and one subprocess per module
    /// would make the cost of the boundary proportional to the module set.
    pub roots: Vec<String>,
}

/// The payload `semantic.read_blocks` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReadBlocksPayload {
    /// One entry per requested root, in the order they were requested.
    pub modules: Vec<ModuleSemanticView>,
}

/// What one module root's `semantic` block came to.
///
/// The `data_schema` resolutions `quoin_semantic::SemanticModule` also carries
/// are deliberately NOT here. Nothing on the TypeScript side reads them — they
/// exist so the diagnostics below can be produced — and a payload field with no
/// reader is a wire shape nobody maintains and every future change has to keep
/// working.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModuleSemanticView {
    /// The module root this entry answers for, echoed back.
    ///
    /// Echoed rather than left to positional correlation: the caller pairs the
    /// answer with its own module record, and a list that says which root each
    /// entry is for cannot be mis-paired by a change to either side.
    pub root: String,
    /// The parsed block, absent when the manifest declares none or its block
    /// was refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<SemanticBlock>,
    /// Every diagnostic reading the block produced, in the order produced.
    pub diagnostics: Vec<SemanticDiagnostic>,
}

/// One corpus root a sweep should walk.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SweepRootRequest {
    /// The directory to walk.
    pub root: String,
    /// The repository name recorded against every finding under it.
    pub repository: String,
    /// The revision recorded in the report's `corpus` block.
    pub revision: String,
}

/// The request accepted by `semantic.sweep_corpus`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SweepCorpusRequest {
    /// The roots to walk, in the order the report should list them.
    pub roots: Vec<SweepRootRequest>,
    /// The semantic package identity the report is for.
    pub package: String,
    /// The module version the report is for.
    pub version: String,
    /// RFC 3339 UTC, supplied by the caller.
    ///
    /// The clock is the caller's, not the boundary's: a report whose timestamp
    /// came from inside this process could not be asserted, and
    /// `quoin_semantic::sweep_corpus` takes the same parameter for the same
    /// reason.
    pub generated_at: String,
}

/// The payload `semantic.sweep_corpus` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SweepCorpusPayload {
    /// The report, in the shape `sweep-report.schema.json` describes.
    pub report: SweepReport,
}

/// The payload `semantic.migration_example` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MigrationExamplePayload {
    /// The migration guidance a legacy-form diagnostic cites (FR-074).
    pub example: String,
}
