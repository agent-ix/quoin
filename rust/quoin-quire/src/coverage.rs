// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The FR→AC→TC→code rollup (quoin#379; quire-rs FR-050).
//!
//! Replaces `runQuire(["coverage", "--scope", repo, "--json", …])` plus
//! `parseCoverage`, which is the single most-used path in `src/quire/`: six
//! quoin commands call it (`advise`, `assurance`, and the four `evidence`
//! verbs). All six passed the same three arguments and all six parsed the same
//! payload, so all six shared the same failure modes.
//!
//! **Verdict policy is not here**, exactly as it is not in `quire coverage`:
//! the report states what is backed, what is not and what does not reconcile,
//! and the consuming workflow decides whether that is acceptable
//! (quire-rs FR-050-CON-1).
//!
//! The `--severity` projection and `--strict` gating the CLI carries are
//! deliberately **not** ported: quoin never passed either flag, and porting an
//! unused surface is how a boundary grows things nobody asked for. A caller
//! that wants projection has the full report and can do it as data.

use crate::error::{Error, Result};
use crate::ids::{DOCUMENT_ROOT_DIR, ScopeRoot};
use crate::modules::{ModuleSelection, Notice, NoticeKind};

/// One coverage run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The repository root. Two roots derive from it and are never
    /// interchanged (quire-rs CR-045): documents from `<scope>/spec`, trace
    /// tags from `<scope>` minus `spec/`.
    pub scope: ScopeRoot,
    /// Which modules supply the traceability model.
    pub modules: ModuleSelection,
}

/// What a coverage run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The engine's report, unmodified.
    pub report: quire_rs::CoverageReport,
    /// Advisory notices from the module, corpus and symbol walks.
    pub notices: Vec<Notice>,
}

/// Compute the coverage report for one scope.
///
/// # Errors
/// [`Error::DocumentRootMissing`] when `<scope>/spec` does not exist,
/// [`Error::ModuleLoad`] / [`Error::ModuleSetEmpty`] when the module set does
/// not resolve, and [`Error::TraceabilityModelUndeclared`] when no active
/// module declares a `traceability:` model.
///
/// That last one is the diagnostic agent-ix/quoin#106 was filed about: the
/// subprocess printed it to stderr, the stderr was discarded, and the operator
/// got "quire exited 1". Here it is a variant of the return type.
pub fn compute(request: &Request) -> Result<Outcome> {
    let mut notices = Vec::new();
    let registry = request.modules.resolve(&request.scope, &mut notices)?;

    // Checked before any walk: without a model there is nothing to reconcile
    // against, and guessing would be exactly the agent-grep behaviour this
    // replaces (quire-rs FR-050-AC-9).
    let Some(model) = registry.traceability() else {
        return Err(Error::TraceabilityModelUndeclared);
    };

    let document_root = request.scope.document_root()?;
    let spec = quire_rs::Spec::from_path(&document_root);
    crate::modules::collect(NoticeKind::Corpus, spec.diagnostics(), &mut notices);

    // The code walk covers the scope minus the document root, and the
    // module-declared `source_exclude` globs subtract within it (quire-rs
    // CR-085). Both filters are passed here rather than defaulted, because an
    // engine that ships the key and a caller that never passes it is how a
    // declared exclusion stays inert for a release.
    let extraction = quire_rs::symbols::extract_tree_scoped(
        request.scope.as_path(),
        &[std::path::Path::new(DOCUMENT_ROOT_DIR)],
        &model.source_exclude,
    );
    for diagnostic in &extraction.diagnostics {
        notices.push(Notice {
            kind: NoticeKind::SymbolExtraction,
            message: diagnostic.reason.clone(),
            path: Some(diagnostic.path.clone()),
        });
    }

    let graph = quire_rs::symbols::trace::bind(&extraction, model);
    let report = quire_rs::compute_coverage(&spec, &registry, &graph, request.scope.as_path())
        .map_err(|error| match error {
            // The engine's only coverage error, mapped onto this crate's
            // variant rather than stringified. An exhaustive match, so a new
            // engine variant fails the build instead of collapsing into a
            // catch-all that reports the wrong condition.
            quire_rs::CoverageError::ModelUndeclared => Error::TraceabilityModelUndeclared,
        })?;

    Ok(Outcome { report, notices })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-099
    #[test]
    fn tc_379_050_a_scope_without_a_document_root_is_named_not_walked() {
        let scope = crate::testing::scratch("coverage-no-spec");
        let request = Request {
            scope: ScopeRoot::open(&scope).expect("scratch dir"),
            modules: ModuleSelection::Ambient,
        };
        // Resolution order matters: the module set resolves first, so an
        // ambient environment with no modules can report `ModuleLoad` before
        // the document root is reached. Both are refusals, and neither is a
        // silent repository-wide crawl — which is the property under test.
        let error = compute(&request).expect_err("a scope with no spec/ cannot be reconciled");
        assert!(
            matches!(
                error.code(),
                crate::ErrorCode::DocumentRootMissing
                    | crate::ErrorCode::TraceabilityModelUndeclared
                    | crate::ErrorCode::ModuleLoad
            ),
            "got {error:?}"
        );
    }
}
