// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What the analysis reads, and what it reports.
//!
//! Split by **who owns the shape**, not by size: [`premises`] and [`audit`]
//! are the two strict input contracts, [`binding`] is the retained store read
//! at a stricter boundary than the store's own reader uses, and [`report`] is
//! the output every consumer sees.

pub mod audit;
pub mod binding;
pub mod premises;
pub mod report;

pub use audit::{AuditEnvelope, SourceIdentity};
pub use binding::{Affirmation, Binding, BindingInput};
pub use premises::{AcceptedPremises, ModulePremise, SchemaPremise};
pub use report::{
    AuditorVerdict, ChangeImpactAnalysis, ChangeImpactRow, ChurnAnalysis, ChurnEvent, ChurnRow,
    ExportIdentity, FanOutAnalysis, FanOutRow, GraphAnalysis, GraphAnalysisInput,
    GraphAnalysisState, GraphGap, GraphGapKind, GraphReportBase, GraphView, ImpactBinding,
    ImpactObligation, ImpactPath, ImpactPathEdge, OwnedObligation,
};

/// The relationship kinds a change-impact walk uses when the caller names none
/// (`analysis.ts:22`).
///
/// Eight, in the order the retained constant lists them — which is already
/// sorted, and is re-sorted anyway by the caller, because the caller also has
/// to sort a set the user supplied.
pub const DEFAULT_RELATION_KINDS: [&str; 8] = [
    "depends_on",
    "derives_from",
    "implements",
    "mitigates",
    "refines",
    "requires",
    "satisfies",
    "traces_to",
];

/// The artifact types a change-impact walk treats as requirements
/// (`analysis.ts:33`).
pub const REQUIREMENT_ARTIFACT_TYPES: [&str; 4] = ["StR", "US", "FR", "NFR"];
