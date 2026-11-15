// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The machinery behind `quoin dossier` (FR-191, PLAT-1614): the document
//! registry, the gap model and its deterministic order, the `DossierDocument`
//! frontmatter and `Gaps` writer, `generate` / `check`, and the export
//! hand-off to `quire export`.
//!
//! The per-document generators (SRS, architecture, trace matrix, RMM and the
//! later documents) are not here: each registers itself in [`builtin_registry`]
//! in the ticket that owns its content, in the fixed bundle order.

pub mod engine;
pub mod error;
pub mod export;
pub mod gap;
pub mod order;
pub mod registry;
pub mod report;
#[cfg(feature = "test-stubs")]
mod stub;
pub mod writer;

pub use engine::{check, generate};
pub use error::{DossierError, DossierErrorCode, QuireFailure, SourceFailure};
pub use export::{ExportOptions, export};
pub use gap::{Gap, GapKind};
pub use registry::{DocumentSpec, Generated, Generator, Inputs, Registry};
pub use report::{Comparison, DocumentReport, Report, Status};

/// The registry `quoin dossier` runs against.
///
/// Real generators are registered here, in bundle order. Under the
/// `test-stubs` feature the stub generators follow.
#[must_use]
pub fn builtin_registry() -> Registry {
    #[allow(
        unused_mut,
        reason = "the registry is empty until a generator or a stub registers"
    )]
    let mut registry = Registry::new();
    #[cfg(feature = "test-stubs")]
    for generator in stub::stubs() {
        // The stubs use distinct kinds and ids; a conflict would be a defect
        // in this file and surfaces as a missing kind in the tests.
        let _ = registry.register(generator);
    }
    registry
}
