// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The FR-086 design relation join over the same repository and module set as coverage.

use crate::coverage::{Request, coverage_error, walk};
use crate::error::{Error, Result};

/// Join requirement rows to design relations declared by the loaded modules.
///
/// # Errors
/// Returns the same scope and module errors as coverage, or `DesignJoin` when
/// a module names a design relation the linked engine cannot resolve.
pub fn compute(request: &Request) -> Result<quire_rs::RelationReport> {
    let walked = walk(request)?;
    let coverage = quire_rs::compute_coverage(
        &walked.spec,
        &walked.registry,
        &walked.graph,
        request.scope.as_path(),
    )
    .map_err(coverage_error)?;
    quire_rs::design_join(
        &walked.spec,
        &walked.registry,
        &[quire_rs::RelationReportMember {
            root: request.scope.as_path(),
            matrix: &coverage.coverage_matrix,
        }],
    )
    .map_err(|error| Error::DesignJoin {
        reason: error.to_string(),
    })
}
