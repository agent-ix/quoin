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
mod product;
mod records_of_processing;
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
mod vulnerability_management_plan;
mod writer;

pub use engine::{check, generate};
pub use error::{DossierError, DossierErrorCode, QuireFailure, SourceFailure};
pub use export::{ExportOptions, export};
pub use gap::{Gap, GapKind};
pub use registry::{DocumentSpec, Generated, Generator, Inputs, Registry};
pub use report::{Comparison, DocumentReport, Report, Status};
pub use sources::{MatrixSource, RecordsOfProcessingSource, RmmSource, Sources, ThreatModelSource};
pub use threat_model::render_threat_model;

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
        Box::new(product::SoftwareDescription),
        Box::new(product::DocumentationDecision),
        Box::new(product::SoftwareDesign),
        Box::new(product::DeviceDescription),
        Box::new(records_of_processing::RecordsOfProcessingGenerator(
            sources.records_of_processing,
        )),
        Box::new(vulnerability_management_plan::VulnerabilityManagementPlanGenerator),
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
    #[trace("FR-191-AC-23")]
    #[test]
    fn the_shipped_registry_holds_the_seventeen_documents() {
        let registry = builtin_registry(crate::testing::sources()).unwrap();
        assert_eq!(registry.specs().count(), 17);
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
                "threat-model",
                "software-description",
                "documentation-level-decision",
                "software-design-specification",
                "device-description",
                "records-of-processing",
                "vulnerability-management-plan"
            ]
        );
        let selected = registry
            .select(&[
                "device-description".to_owned(),
                "software-description".to_owned(),
                "software-design-specification".to_owned(),
                "documentation-level-decision".to_owned(),
            ])
            .unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|generator| generator.spec().id)
                .collect::<Vec<_>>(),
            ["DOS-SW", "DOS-DL", "DOS-SDS", "DOS-DEV"]
        );
    }

    #[cfg(not(feature = "test-stubs"))]
    #[test]
    fn the_m1_and_m2_registry_selects_and_exports_ten_documents_in_dossier_order() {
        use std::path::PathBuf;

        use crate::engine::Rendered;
        use crate::export::{ExportOptions, quire_args};
        use crate::registry::{DocumentSpec, Generated, Generator, Inputs};
        struct Complete(DocumentSpec);

        impl Generator for Complete {
            fn spec(&self) -> DocumentSpec {
                self.0
            }

            fn generate(&self, _inputs: &Inputs) -> Result<Generated, SourceFailure> {
                Ok(Generated {
                    body: format!("# {}", self.0.id),
                    document_version: None,
                    gaps: Vec::new(),
                })
            }
        }

        let expected = [
            "DOS-SRS",
            "DOS-ARCH",
            "DOS-TRACE",
            "DOS-RMM",
            "DOS-VR",
            "DOS-SOUP",
            "DOS-ANOM",
            "DOS-RISK",
            "DOS-RMP",
            "DOS-REL",
        ];
        let generators = builtin_generators(crate::testing::sources())
            .into_iter()
            .take(expected.len())
            .map(|generator| Box::new(Complete(generator.spec())) as Box<dyn Generator>)
            .collect();
        let registry = registry_of(generators).unwrap();
        let out = tempfile::tempdir().unwrap();
        let inputs = Inputs {
            scope: out.path().to_owned(),
            modules: Vec::new(),
            clause_bindings: Vec::new(),
            pack: None,
            soup_binding: None,
            soup_records: None,
            soup_support: None,
            snapshot: None,
            release_verify: None,
            as_of: Some("2026-11-26T00:00:00Z".to_owned()),
            version: "1.0".to_owned(),
            producer_version: "test-quoin-version".to_owned(),
        };
        let report = generate(&registry, &[], out.path(), &inputs).unwrap();
        let ids = report
            .documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, expected);
        for id in expected {
            assert!(out.path().join(format!("{id}.md")).is_file(), "{id}");
        }

        let reverse = registry
            .select(&[
                "release-record".to_owned(),
                "risk-management-plan".to_owned(),
                "risk-management".to_owned(),
                "anomalies".to_owned(),
                "soup".to_owned(),
                "verification-report".to_owned(),
                "rmm".to_owned(),
                "trace-matrix".to_owned(),
                "architecture".to_owned(),
                "srs".to_owned(),
            ])
            .unwrap();
        let rendered = reverse
            .iter()
            .map(|generator| Rendered {
                spec: generator.spec(),
                path: PathBuf::from(out.path()).join(format!("{}.md", generator.spec().id)),
                text: String::new(),
                gaps: Vec::new(),
            })
            .collect::<Vec<_>>();
        let options = ExportOptions {
            to: "pdf".to_owned(),
            output: out.path().join("bundle.pdf"),
            release: None,
            approvals: Vec::new(),
            modules: Vec::new(),
            template: None,
            quire: PathBuf::from("quire"),
        };
        let args = quire_args(
            &rendered,
            &options,
            "0123456789012345678901234567890123456789",
        );
        assert_eq!(
            args[1..1 + expected.len()],
            expected.map(|id| out.path().join(format!("{id}.md")).display().to_string())
        );
        assert!(args.windows(2).any(|pair| pair == ["--bundle-id", "DOS"]));
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
            22
        );
    }
}
