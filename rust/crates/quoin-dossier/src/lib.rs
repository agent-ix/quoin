// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The machinery behind `quoin dossier` (FR-191, PLAT-1614): the document
//! registry, the gap model and its deterministic order, the `DossierDocument`
//! frontmatter and `Gaps` writer, `generate` / `check`, and the export
//! hand-off to `quire export`.
//!
//! The SRS, architecture, trace matrix, RMM, verification report, SOUP and
//! unresolved-anomalies documents register in [`builtin_registry`] in fixed
//! bundle order. The host supplies the matrix and discharge source adapters so
//! their evidence assembly stays shared.

mod anomalies;
mod architecture;
mod engine;
mod error;
mod export;
mod gap;
mod markdown;
mod order;
mod registry;
mod release_record;
pub mod release_snapshot;
mod report;
mod risk_management;
mod risk_plan;
mod rmm;
mod soup;
mod sources;
mod srs;
#[cfg(feature = "test-stubs")]
mod stub;
#[cfg(test)]
mod testing;
mod threat_model;
mod trace_matrix;
mod verification_report;
mod writer;

pub use engine::{check, generate};
pub use error::{DossierError, DossierErrorCode, QuireFailure, SourceFailure};
pub use export::{ExportOptions, export};
pub use gap::{Gap, GapKind};
pub use registry::{DocumentSpec, Generated, Generator, Inputs, Registry};
pub use report::{Comparison, DocumentReport, Report, Status};
pub use sources::{MatrixSource, RmmSource, Sources, ThreatModelSource};

/// The registry `quoin dossier` runs against.
///
/// # Errors
///
/// Returns [`DossierErrorCode::RegistryConflict`] when two generators claim one
/// kind or one id, so a generator added later is never dropped silently.
pub fn builtin_registry(sources: Sources) -> Result<Registry, DossierError> {
    registry_of(builtin_generators(sources))
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

/// Every generator this build ships, in bundle order; test stubs follow.
fn builtin_generators(sources: Sources) -> Vec<Box<dyn Generator>> {
    let generators: Vec<Box<dyn Generator>> = vec![
        Box::new(srs::SrsGenerator),
        Box::new(architecture::ArchitectureGenerator),
        Box::new(trace_matrix::TraceMatrixGenerator(std::sync::Arc::clone(
            &sources.matrix,
        ))),
        Box::new(rmm::RmmGenerator(sources.rmm)),
        Box::new(verification_report::VerificationReportGenerator::new(
            sources.matrix,
        )),
        Box::new(soup::SoupGenerator(sources.soup)),
        Box::new(anomalies::AnomaliesGenerator),
        Box::new(risk_management::RiskManagementGenerator),
        Box::new(risk_plan::RiskPlanGenerator(sources.policy)),
        Box::new(release_record::ReleaseRecordGenerator),
        Box::new(threat_model::ThreatModelGenerator(sources.threat_model)),
    ];
    #[cfg(feature = "test-stubs")]
    let generators = {
        let mut generators = generators;
        generators.extend(stub::stubs());
        generators
    };
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

    /// Shipped documents register in bundle order.
    #[cfg(not(feature = "test-stubs"))]
    #[trace("FR-191")]
    #[test]
    fn the_shipped_registry_holds_the_eleven_documents() {
        let registry = builtin_registry(crate::testing::sources()).unwrap();
        assert_eq!(registry.specs().count(), 11);
        let kinds: Vec<_> = registry.specs().map(|spec| spec.kind).collect();
        assert_eq!(
            kinds,
            vec![
                "srs",
                "architecture",
                "trace-matrix",
                "rmm",
                "verification-report",
                "soup",
                "anomalies",
                "risk-management",
                "risk-management-plan",
                "release-record",
                "threat-model"
            ]
        );
    }

    #[cfg(feature = "test-stubs")]
    #[trace("FR-191")]
    #[test]
    fn the_shipped_stubs_do_not_collide() {
        assert_eq!(
            builtin_registry(crate::testing::sources())
                .unwrap()
                .specs()
                .count(),
            16
        );
    }
}
