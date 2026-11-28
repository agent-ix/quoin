// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The design-bearing coverage matrix, delegated to quire-rs FR-092.

use crate::coverage::{self, Request};
use crate::error::{Error, Result};
use ix_trace_rs::implements;

pub use quire_rs::corpus::{DesignMatrix, DesignMatrixEntry, DroppedRelation};

/// One design-matrix run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The engine's coverage report, unmodified.
    pub report: quire_rs::CoverageReport,
    /// The engine's design matrix, including errors for dropped relations.
    pub design_matrix: DesignMatrix,
    /// Advisory notices from the module, corpus and symbol walks.
    pub notices: Vec<crate::Notice>,
}

/// Compute the coverage and design matrices from one corpus walk.
///
/// # Errors
/// The same scope, module, and extraction failures as [`coverage::compute`],
/// or [`Error::DesignMatrixUnknownRelation`] when a declared design relation
/// names no required relation.
#[implements("FR-193-AC-10")]
#[implements("FR-193-AC-11")]
#[implements("NFR-011-M-2")]
pub fn compute(request: &Request) -> Result<Outcome> {
    compute_with_spec_loader(request, |root| quire_rs::Spec::from_path(root))
}

fn compute_with_spec_loader(
    request: &Request,
    load_spec: impl FnOnce(&std::path::Path) -> quire_rs::Spec,
) -> Result<Outcome> {
    let walked =
        coverage::walk_with_dropped_relation_policy_and_spec_loader(request, true, load_spec)?;
    let report = quire_rs::coverage::compute_coverage_for_design_matrix(
        &walked.spec,
        &walked.registry,
        &walked.graph,
        request.scope.as_path(),
    )
    .map_err(coverage::coverage_error)?;
    let design_matrix = for_coverage(
        request.scope.as_path(),
        &walked.spec,
        &walked.registry,
        &report,
    )?;
    Ok(Outcome {
        report,
        design_matrix,
        notices: walked.notices,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions report failures"
)]
mod tests {
    use super::*;
    use crate::coverage::Request;
    use crate::ids::{ModuleRoot, ScopeRoot};
    use crate::modules::ModuleSelection;
    use ix_trace_rs::trace;

    #[trace("NFR-011-M-2")]
    #[test]
    fn matrix_loads_its_selected_documents_in_one_spec_pass() {
        let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("repo");
        let canonical_repository = repository.canonicalize().expect("canonical repository");
        let request = Request {
            scope: ScopeRoot::open(&repository).expect("fixture scope"),
            modules: ModuleSelection::Closed(vec![
                ModuleRoot::open(repository).expect("fixture module"),
            ]),
        };
        let mut passes = 0;
        let mut documents = Vec::new();
        let _outcome = compute_with_spec_loader(&request, |root| {
            passes += 1;
            let spec = quire_rs::Spec::from_path(root);
            documents.extend(spec.documents().iter().map(|document| {
                document
                    .path
                    .canonicalize()
                    .expect("canonical selected document")
                    .strip_prefix(&canonical_repository)
                    .expect("selected document is under repository root")
                    .to_path_buf()
            }));
            spec
        })
        .expect("matrix computation");

        assert_eq!(passes, 1, "matrix has one selected-document load pass");
        documents.sort();
        assert_eq!(
            documents,
            [
                std::path::PathBuf::from("spec/FR-001-example.md"),
                std::path::PathBuf::from("spec/TM-001-matrix.md"),
            ],
            "the one pass covers Quire's exact selected document set"
        );
    }
}

#[implements("FR-193-AC-10")]
#[implements("FR-193-AC-11")]
pub(crate) fn for_coverage(
    root: &std::path::Path,
    spec: &quire_rs::Spec,
    registry: &quire_rs::Registry,
    report: &quire_rs::CoverageReport,
) -> Result<DesignMatrix> {
    quire_rs::design_matrix(
        spec,
        registry,
        &[quire_rs::RelationReportMember {
            root,
            matrix: &report.coverage_matrix,
        }],
    )
    .map_err(|error| match error {
        quire_rs::RelationReportError::UnknownDesignRelation { name } => {
            Error::DesignMatrixUnknownRelation { name }
        }
        other => crate::design_join::relation_error(other),
    })
}
