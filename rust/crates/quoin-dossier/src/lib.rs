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

mod engine;
mod error;
mod export;
mod gap;
mod order;
mod registry;
mod report;
#[cfg(feature = "test-stubs")]
mod stub;
#[cfg(test)]
mod testing;
mod writer;

pub use engine::{check, generate};
pub use error::{DossierError, DossierErrorCode, QuireFailure, SourceFailure};
pub use export::{ExportOptions, export};
pub use gap::{Gap, GapKind};
pub use registry::{DocumentSpec, Generated, Generator, Inputs, Registry};
pub use report::{Comparison, DocumentReport, Report, Status};

/// The registry `quoin dossier` runs against.
///
/// # Errors
///
/// Returns [`DossierErrorCode::RegistryConflict`] when two generators claim one
/// kind or one id, so a generator added later is never dropped silently.
pub fn builtin_registry() -> Result<Registry, DossierError> {
    registry_of(builtin_generators())
}

/// A registry holding `generators` in the order given.
///
/// # Errors
///
/// Returns [`DossierErrorCode::RegistryConflict`] for the first generator that
/// collides with one registered before it.
pub fn registry_of(generators: Vec<Box<dyn Generator>>) -> Result<Registry, DossierError> {
    let mut registry = Registry::new();
    for generator in generators {
        registry.register(generator)?;
    }
    Ok(registry)
}

/// Every generator this build ships, in bundle order. The real generators are
/// pushed here by the tickets that own their content; under the `test-stubs`
/// feature the stubs follow.
fn builtin_generators() -> Vec<Box<dyn Generator>> {
    #[allow(
        unused_mut,
        reason = "the list is empty until a generator or a stub is added"
    )]
    let mut generators: Vec<Box<dyn Generator>> = Vec::new();
    #[cfg(feature = "test-stubs")]
    generators.extend(stub::stubs());
    generators
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use crate::testing::doc;
    use ix_trace_rs::trace;

    #[trace("FR-191")]
    #[test]
    fn a_colliding_generator_is_an_error_not_a_dropped_document() {
        let error = registry_of(vec![
            Box::new(doc("srs", "DOS-SRS", false)),
            Box::new(doc("srs", "DOS-OTHER", false)),
        ])
        .err()
        .unwrap();
        assert_eq!(error.code(), DossierErrorCode::RegistryConflict);
        let ok = registry_of(vec![
            Box::new(doc("srs", "DOS-SRS", false)),
            Box::new(doc("rmm", "DOS-RMM", false)),
        ])
        .unwrap();
        assert_eq!(
            ok.specs().map(|spec| spec.id).collect::<Vec<_>>(),
            ["DOS-SRS", "DOS-RMM"]
        );
    }

    /// This frame ships no real generator: the shipped registry is empty until
    /// PLAT-1437 and PLAT-1438 register theirs, and every run says so and exits 2.
    #[cfg(not(feature = "test-stubs"))]
    #[trace("FR-191")]
    #[test]
    fn the_shipped_registry_is_empty_and_every_run_is_refused_without_writing() {
        use crate::testing::inputs;
        let registry = builtin_registry().unwrap();
        assert_eq!(registry.specs().count(), 0);
        let work = tempfile::tempdir().unwrap();
        let out = work.path().join("dossier");
        let error = generate(&registry, &[], &out, &inputs()).unwrap_err();
        assert_eq!(error.code(), DossierErrorCode::NoDocuments);
        assert!(!out.exists());
        assert!(check(&registry, &[], &out, &inputs()).is_err());
    }

    #[cfg(feature = "test-stubs")]
    #[trace("FR-191")]
    #[test]
    fn the_shipped_stubs_do_not_collide() {
        assert_eq!(builtin_registry().unwrap().specs().count(), 5);
    }
}
