// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `graph` domain's wire shapes, and the ceilings they are read under.
//!
//! Request and payload types only: what a caller may say, and what it gets
//! back. No decision is taken here. The operations that take them live in
//! [`super`], and the exit-taxonomy mapping in [`super::taxonomy`].
//!
//! The four input paths are spelled out twice — once in [`ViewRequest`] and
//! once in [`ChangeImpactRequest`] — rather than shared through a flattened
//! struct. `#[serde(flatten)]` silently disables `deny_unknown_fields` on the
//! outer type, and a misspelled `premisis_path` accepted in silence is exactly
//! the condition rust-style §11 forbids: a field the boundary drops is a field
//! the caller believes it sent.

use serde::{Deserialize, Serialize};

/// The largest `graph.*` request this domain will decide over, in bytes.
///
/// A request is four paths, a flag, and — for change impact — a list of
/// requirement ids and an optional relationship vocabulary. The assurance
/// export, the premises, the audit envelope and the bindings store are all
/// READ by the granted reader and never ride on stdin, so this bounds names,
/// not documents.
pub const MAX_GRAPH_REQUEST_BYTES: usize = 1 << 20;

/// The largest single path, requirement id or relationship kind this domain
/// will accept, in bytes.
///
/// 4 KiB is past every platform's `PATH_MAX` and still refuses a stream. The
/// same number `ops::semantic` uses, for the same reason.
pub const MAX_SCALAR_BYTES: usize = 4 * 1024;

/// Where the four inputs are, and how the answer should be spelled.
///
/// The shape `quoin graph fan-out` and `quoin graph churn` send.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ViewRequest {
    /// The repository root. The retained bindings store is found under it.
    pub repo: String,
    /// The existing quire assurance-v1 export.
    pub export_path: String,
    /// The accepted assurance format/module/schema premises.
    pub premises_path: String,
    /// The source-bound FR-032 audit envelope.
    pub audit_path: String,
    /// Canonical JSON rather than markdown.
    pub json: bool,
}

/// As [`ViewRequest`], plus the seeds and vocabulary a change-impact walk takes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChangeImpactRequest {
    /// The repository root. The retained bindings store is found under it.
    pub repo: String,
    /// The existing quire assurance-v1 export.
    pub export_path: String,
    /// The accepted assurance format/module/schema premises.
    pub premises_path: String,
    /// The source-bound FR-032 audit envelope.
    pub audit_path: String,
    /// Canonical JSON rather than markdown.
    pub json: bool,
    /// The changed requirement ids the walk seeds from.
    pub requirements: Vec<String>,
    /// The relationship vocabulary replacing the default selection.
    ///
    /// `None` means the eight defaults; `Some([])` means none at all, which is
    /// a walk with no edges rather than a walk with the defaults. The two are
    /// not the same answer and `analyze_change_impact` distinguishes them, so
    /// the absent case stays absent on the wire rather than being normalised
    /// into an empty list here.
    #[serde(default)]
    pub relations: Option<Vec<String>>,
}

/// The payload every `graph.*` operation writes to stdout.
///
/// One member, because a graph command prints one document. The report OBJECT
/// is deliberately not on the wire: both spellings a caller can ask for —
/// canonical JSON and markdown — are rendered by `quoin-graph-analysis`, and a
/// payload that carried the object as well would be a second encoding of the
/// same report for a reader to disagree with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderedPayload {
    /// The report, rendered as the request asked for it.
    pub rendered: String,
}
