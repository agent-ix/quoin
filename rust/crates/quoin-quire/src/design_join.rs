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
    .map_err(relation_error)
}

/// The engine's relation-report errors onto this crate's variants. A dropped
/// declaration is `QQ-1022`, never the design-join code (FR-029-AC-19).
pub(crate) fn relation_error(error: quire_rs::RelationReportError) -> Error {
    match error {
        quire_rs::RelationReportError::DeclarationsDropped(dropped) => dropped.into(),
        other => Error::DesignJoin {
            reason: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::relation_error;
    use ix_trace_rs::trace;
    use quire_rs::traceability::{DeclarationKind, DroppedDeclarationRef, DroppedDeclarations};

    /// FR-029-AC-19: the engine's relation-report drop error is `QQ-1022` and
    /// carries its records, never the design-join code `QQ-1110`; every other
    /// relation-report error stays the design-join code.
    #[trace("FR-029-AC-19")]
    #[test]
    fn a_relation_report_drop_maps_to_qq_1022_never_qq_1110() {
        let dropped = DroppedDeclarations(vec![DroppedDeclarationRef {
            module: "m".to_owned(),
            kind: DeclarationKind::FlowRule,
            item: "r".to_owned(),
        }]);
        let error = relation_error(quire_rs::RelationReportError::DeclarationsDropped(dropped));
        assert_eq!(error.code().as_str(), "QQ-1022");
        assert!(matches!(
            &error,
            crate::Error::TraceabilityDeclarationsDropped { dropped }
                if dropped == &[crate::DroppedDeclaration {
                    module: "m".to_owned(),
                    kind: "flow_rule".to_owned(),
                    item: "r".to_owned(),
                }]
        ));

        let other = relation_error(quire_rs::RelationReportError::UnknownRelation {
            name: "x".to_owned(),
        });
        assert_eq!(other.code().as_str(), "QQ-1110");
    }
}
