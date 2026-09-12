// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The source-grounded assurance export (quoin#379; quire-rs FR-062/FR-067).
//!
//! Replaces `validateAssurance` / `parseAssurance` — used by
//! `src/graph-analysis/load.ts` and `src/measurement/graph-adapters.ts` — and
//! adds the producing half quoin never had: it could only ever *read* an
//! export somebody else's `quire assurance` had written.
//!
//! ## The clearest case that `contract.ts` dissolves
//!
//! quoin vendored `assurance-v1.schema.json`, hashed it, and compiled it with
//! ajv to check an export before reading it. The engine ships
//! [`quire_rs::read_assurance_export`]: a **fail-closed reader** that checks
//! the format tag, the format version, the value shape *and* the caller's
//! accepted module and schema-digest premises, and returns typed
//! `AssuranceError` variants for each. It is stricter than the schema check it
//! replaces — the schema cannot know which modules the caller accepts — and it
//! is maintained by the same project that emits the payload.
//!
//! quoin does not restate any of that here. [`read`] adds exactly one thing the
//! engine's reader does not do, because it takes `&[u8]` and not a path: a
//! byte ceiling on the file (see [`crate::payload::PayloadLimit`]).

use crate::error::Result;
use crate::ids::{DOCUMENT_ROOT_DIR, RepositoryId, RevisionId, ScopeRoot};
use crate::modules::{ModuleSelection, Notice, NoticeKind};
use crate::payload::PayloadLimit;

/// One assurance export build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The repository root.
    pub scope: ScopeRoot,
    /// Which modules supply the archetypes and relation vocabulary.
    pub modules: ModuleSelection,
    /// The repository identity copied into the source premise.
    pub repository: RepositoryId,
    /// The caller-selected immutable revision.
    pub revision: RevisionId,
}

/// What an export build produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The export.
    pub export: quire_rs::AssuranceExport,
    /// Advisory notices from the module, corpus and symbol walks.
    pub notices: Vec<Notice>,
}

/// Build a source-grounded assurance export for one scope.
///
/// # Errors
/// [`crate::Error::DocumentRootMissing`], the module-set errors, and
/// [`crate::Error::Assurance`] carrying the engine's own fail-closed refusal.
pub fn build(request: &Request) -> Result<Outcome> {
    let mut notices = Vec::new();
    let registry = request.modules.resolve(&request.scope, &mut notices)?;

    let document_root = request.scope.document_root()?;
    let spec = quire_rs::Spec::from_path(&document_root);
    crate::modules::collect(NoticeKind::Corpus, spec.diagnostics(), &mut notices);

    let model = registry.traceability();
    let no_excludes = Vec::new();
    let extraction = quire_rs::symbols::extract_tree_scoped(
        request.scope.as_path(),
        &[std::path::Path::new(DOCUMENT_ROOT_DIR)],
        model.map_or(&no_excludes, |traceability| &traceability.source_exclude),
    );
    for diagnostic in &extraction.diagnostics {
        notices.push(Notice {
            kind: NoticeKind::SymbolExtraction,
            message: diagnostic.reason.clone(),
            path: Some(diagnostic.path.clone()),
        });
    }
    // A module set with no traceability model still exports artifacts,
    // obligations and symbols; only the bound `verifies`/`implements`
    // relations need the model. An empty graph is the modelled answer, not a
    // refusal — which is why this is `map`, not `ok_or`.
    let graph = model.map_or_else(quire_rs::symbols::trace::SymbolGraph::default, |model| {
        quire_rs::symbols::trace::bind(&extraction, model)
    });

    let export = quire_rs::build_assurance_export(quire_rs::AssuranceInput {
        spec: &spec,
        registry: &registry,
        corpus_root: &document_root,
        symbols: &extraction,
        symbol_graph: &graph,
        source: quire_rs::AssuranceSource {
            repository: request.repository.as_str().to_string(),
            revision: request.revision.as_str().to_string(),
        },
    })?;

    Ok(Outcome { export, notices })
}

/// Read an export somebody else produced, under a byte ceiling.
///
/// `accepted` is the caller's premise set: which module at which version, and
/// which archetype schema digests. There is no "accept whatever it says"
/// overload, deliberately — an export read without a premise is an export
/// whose provenance nothing checked.
///
/// # Errors
/// [`crate::Error::PayloadTooLarge`] at the ceiling, and
/// [`crate::Error::Assurance`] for every refusal the engine's reader makes.
pub fn read(
    subject: &str,
    bytes: &[u8],
    accepted: &quire_rs::AcceptedAssurancePremises,
    limit: PayloadLimit,
) -> Result<quire_rs::AssuranceExport> {
    limit.admit_slice(subject, bytes)?;
    Ok(quire_rs::read_assurance_export(bytes, accepted)?)
}

/// Read an export from a file, under a byte ceiling.
///
/// # Errors
/// [`crate::Error::Io`] plus everything [`read`] reports.
pub fn read_file(
    path: impl AsRef<std::path::Path>,
    accepted: &quire_rs::AcceptedAssurancePremises,
    limit: PayloadLimit,
) -> Result<quire_rs::AssuranceExport> {
    let path = path.as_ref();
    let bytes = crate::payload::read_bounded(path, limit)?;
    read(&path.display().to_string(), &bytes, accepted, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-100
    #[test]
    fn tc_379_090_an_oversized_export_is_refused_before_the_engine_parses_it() {
        let accepted = quire_rs::AcceptedAssurancePremises {
            format_version: 1,
            modules: Vec::new(),
        };
        let bytes = vec![b'x'; 64];
        let error = read(
            "export",
            &bytes,
            &accepted,
            PayloadLimit::bytes(8).expect("non-zero"),
        )
        .expect_err("must refuse");
        assert_eq!(error.code(), crate::ErrorCode::PayloadTooLarge);
    }

    /// Trace: FR-100
    #[test]
    fn tc_379_091_the_engines_own_refusals_are_carried_not_restated() {
        let accepted = quire_rs::AcceptedAssurancePremises {
            format_version: 1,
            modules: Vec::new(),
        };
        let error = read(
            "export",
            br#"{"format":"not-quire","format_version":1}"#,
            &accepted,
            PayloadLimit::DEFAULT,
        )
        .expect_err("must refuse");
        assert_eq!(error.code(), crate::ErrorCode::Assurance);
        assert!(
            matches!(
                error,
                crate::Error::Assurance(quire_rs::AssuranceError::UnsupportedFormat { .. })
            ),
            "the engine's variant must survive the wrap: {error:?}"
        );
    }
}
