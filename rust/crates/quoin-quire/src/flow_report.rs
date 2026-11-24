// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The generic flow findings from quire-rs FR-096, read as a linked library operation.

use crate::coverage::Request;
use crate::error::Result;
use crate::modules::Notice;

/// Compute the relation report's generic flow section for one selected corpus.
///
/// The returned report is the engine's flow report unchanged. `None` means
/// that no loaded module declares flow rules.
///
/// # Errors
/// Returns scope or module selection errors, or a relation-report error from
/// the linked engine.
pub fn compute(request: &Request) -> Result<(Option<quire_rs::corpus::FlowReport>, Vec<Notice>)> {
    let mut notices = Vec::new();
    let registry = request.modules.resolve(&request.scope, &mut notices)?;
    crate::modules::require_complete_model(&registry)?;
    let document_root = request.scope.document_root()?;
    let corpus = quire_rs::Spec::from_path(&document_root);
    crate::modules::collect(
        crate::modules::NoticeKind::Corpus,
        corpus.diagnostics(),
        &mut notices,
    );
    let report = quire_rs::relation_report(
        &corpus,
        &registry,
        &[quire_rs::RelationReportMember {
            root: request.scope.as_path(),
            matrix: &[],
        }],
        &[],
    )
    .map_err(crate::design_join::relation_error)?;
    Ok((report.flow, notices))
}
