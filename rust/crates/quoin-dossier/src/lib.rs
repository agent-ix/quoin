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
    use ix_trace_rs::trace;

    struct Fixed(DocumentSpec);

    impl Generator for Fixed {
        fn spec(&self) -> DocumentSpec {
            self.0
        }

        fn generate(&self, _: &Inputs) -> Result<Generated, SourceFailure> {
            Ok(Generated {
                body: String::new(),
                gaps: Vec::new(),
            })
        }
    }

    fn fixed(kind: &'static str, id: &'static str) -> Box<dyn Generator> {
        Box::new(Fixed(DocumentSpec {
            kind,
            id,
            title: "T",
            needs_as_of: false,
        }))
    }

    #[trace("FR-191")]
    #[test]
    fn a_colliding_generator_is_an_error_not_a_dropped_document() {
        let error = registry_of(vec![fixed("srs", "DOS-SRS"), fixed("srs", "DOS-OTHER")])
            .err()
            .unwrap();
        assert_eq!(error.code(), DossierErrorCode::RegistryConflict);
        let ok = registry_of(vec![fixed("srs", "DOS-SRS"), fixed("rmm", "DOS-RMM")]).unwrap();
        assert_eq!(
            ok.specs().map(|spec| spec.id).collect::<Vec<_>>(),
            ["DOS-SRS", "DOS-RMM"]
        );
    }

    #[cfg(feature = "test-stubs")]
    #[trace("FR-191")]
    #[test]
    fn the_shipped_stubs_do_not_collide() {
        assert_eq!(builtin_registry().unwrap().specs().count(), 5);
    }
}
