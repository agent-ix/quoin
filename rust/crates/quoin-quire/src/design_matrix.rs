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
pub fn compute(request: &Request) -> Result<Outcome> {
    let walked = coverage::walk_for_design_matrix(request)?;
    let report = quire_rs::compute_coverage_for_design_matrix(
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
