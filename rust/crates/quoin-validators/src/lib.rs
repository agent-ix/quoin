// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deterministic repository QA-gate validators for quoin.
//!
//! Stage 2 of the Rust burn-down (quoin#377, parent quoin#373): the port of
//! `src/validators/` at quoin `4d27dcf`. One capability today — [`inspect_empty_gates`],
//! which finds a declared, wired shell gate that counts forbidden matches but
//! never asserts the count (quoin#224).
//!
//! # Boundary
//!
//! The crate takes a repository root and returns a [`GateReport`]. It does not
//! print, does not exit, and does not know what a CLI flag is: [`GateReport::verdict`]
//! names the three states the `validate` command encodes inline, and
//! [`GateReport::human_lines`] produces the operator-facing text. That split is
//! what lets `quoin-core <domain>.<op>` wrap this without the command's opinions
//! leaking into the library.
//!
//! Every refusal is a [`ValidatorError`] with a stable code. Nothing degrades to
//! an empty result: an empty [`GateReport`] means the validator walked the whole
//! tree and found nothing.
//!
//! ```no_run
//! use std::path::Path;
//! use quoin_validators::{GateReport, Verdict, inspect_empty_gates};
//!
//! # fn main() -> Result<(), quoin_validators::ValidatorError> {
//! let report = GateReport::new(inspect_empty_gates(Path::new("."))?);
//! for line in report.human_lines() {
//!     println!("{line}");
//! }
//! assert_eq!(report.verdict(false), Verdict::Clean);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod error;
mod finding;
mod gates;
mod ids;
mod repo;

pub use error::{ErrorCode, ValidatorError};
pub use finding::{EmptyGateFinding, FindingKind, GateReport, Verdict};
pub use gates::inspect_empty_gates;
pub use ids::{LineNumber, ObligationId, RepoPath};
