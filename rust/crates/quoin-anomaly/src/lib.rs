// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Anomaly lifecycle check over git history and assessment reverse references
//! (FR-136).
//!
//! The `Anomaly` archetype, its vocabularies and its tables belong to
//! `spec-artifacts-process`. This crate owns only what no module mapping can
//! express: the `statusHistory` derived from git, the check of its status
//! moves, the missing-module report for hazard links, and the reverse
//! references from a requirement or hazard to the anomalies that name it.

#![forbid(unsafe_code)]

pub mod check;
pub mod derived;
pub mod error;
pub mod history;
pub mod record;
pub mod refs;
pub mod report;
pub mod status;

pub use check::{Checked, Context, Diagnostic, DiagnosticCode, Severity, check_document};
pub use derived::{Change, Derived, fixing_change, rationale_change};
pub use error::{AnomalyError, ErrorCode};
pub use history::{History, Signature, StatusEntry, read_history};
pub use record::{
    MappingError, MappingErrorCode, RationaleEntry, Record, Row, parse_record, status_line,
};
pub use refs::{RefEntry, acceptance_criteria, anomaly_documents, is_hazard, references_to};
pub use report::{Gate, GateMode, Report, build_report};
pub use status::Status;
