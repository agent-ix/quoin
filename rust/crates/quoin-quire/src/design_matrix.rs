// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The design-bearing coverage matrix, delegated to quire-rs FR-092.

use crate::coverage::{self, Request};
use crate::error::{Error, Result};

/// One design-matrix run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The engine's coverage report, unmodified.
    pub report: quire_rs::CoverageReport,
    /// The engine's design matrix, unmodified and in engine order.
    pub design_matrix: Vec<quire_rs::DesignMatrixEntry>,
    /// Advisory notices from the module, corpus and symbol walks.
    pub notices: Vec<crate::Notice>,
}

/// Compute the coverage and design matrices from one corpus walk.
///
/// # Errors
/// The same scope, module, and extraction failures as [`coverage::compute`],
/// or [`Error::DesignMatrixUnknownRelation`] when a declared design relation
/// names no required relation.
pub fn compute(request: &Request) -> Result<Outcome> {
    let walked = coverage::walk(request)?;
    let report = quire_rs::compute_coverage(
        &walked.spec,
        &walked.registry,
        &walked.graph,
        request.scope.as_path(),
    )
    .map_err(coverage::coverage_error)?;
    let matrix = quire_rs::design_matrix(
        &walked.spec,
        &walked.registry,
        &[quire_rs::RelationReportMember {
            root: request.scope.as_path(),
            matrix: &report.coverage_matrix,
        }],
    )
    .map_err(|error| match error {
        quire_rs::RelationReportError::UnknownDesignRelation { name } => {
            Error::DesignMatrixUnknownRelation { name }
        }
        other => Error::DesignJoin {
            reason: other.to_string(),
        },
    })?;
    Ok(Outcome {
        report,
        design_matrix: matrix,
        notices: walked.notices,
    })
}
