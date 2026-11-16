// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The document registry (FR-191).
//!
//! A generator is registered once per document kind. Registry order is the
//! fixed bundle order and the order of every report, so the SRS,
//! architecture, trace-matrix and RMM generators of later tickets register in
//! that order and nothing else decides it.

use std::path::PathBuf;

use crate::error::{DossierError, DossierErrorCode, SourceFailure};
use crate::gap::Gap;

/// What identifies one dossier document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentSpec {
    /// The `--doc` selector and the `document_kind` frontmatter value.
    pub kind: &'static str,
    /// The document id, for example `DOS-SRS`; also the file stem.
    pub id: &'static str,
    /// The document title.
    pub title: &'static str,
    /// Whether the document needs `--as-of` (and records it as `as_of`).
    pub needs_as_of: bool,
}

/// Everything a generator may read. A run reads no wall clock: time arrives
/// only as `as_of`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inputs {
    /// The repository scope the records are read from.
    pub scope: PathBuf,
    /// Module roots to resolve record types in.
    pub modules: Vec<PathBuf>,
    /// Clause-binding reports.
    pub clause_bindings: Vec<PathBuf>,
    /// The regulatory pack, where a document needs one.
    pub pack: Option<String>,
    /// The explicit evaluation instant, an RFC 3339 string.
    pub as_of: Option<String>,
    /// The document version written into every generated file.
    pub version: String,
}

/// What a generator produces: the document body and the gaps it met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated {
    /// Markdown between the frontmatter and the `Gaps` section.
    pub body: String,
    /// The missing inputs, in any order; the writer sorts them.
    pub gaps: Vec<Gap>,
}

/// Produces one document from the repository's records.
pub trait Generator: Send + Sync {
    /// Which document this generator writes.
    fn spec(&self) -> DocumentSpec;

    /// Whether this run needs an explicit evaluation instant. A source-backed
    /// document may have no source selected and report that absence as a gap.
    fn requires_as_of(&self, _inputs: &Inputs) -> bool {
        self.spec().needs_as_of
    }

    /// Compute the document in memory.
    ///
    /// # Errors
    ///
    /// Returns the failed source when a record reader fails (exit 3).
    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure>;
}

/// The registered generators, in fixed order.
#[derive(Default)]
pub struct Registry {
    generators: Vec<Box<dyn Generator>>,
}

impl Registry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a generator after those already registered.
    ///
    /// # Errors
    ///
    /// Returns [`DossierErrorCode::RegistryConflict`] when a registered
    /// generator already claims the kind or the id.
    pub fn register(&mut self, generator: Box<dyn Generator>) -> Result<(), DossierError> {
        let spec = generator.spec();
        if let Some(existing) = self
            .specs()
            .find(|other| other.kind == spec.kind || other.id == spec.id)
        {
            return Err(DossierError::new(
                DossierErrorCode::RegistryConflict,
                format!(
                    "document {} ({}) conflicts with registered {} ({})",
                    spec.kind, spec.id, existing.kind, existing.id
                ),
            ));
        }
        self.generators.push(generator);
        Ok(())
    }

    /// Every registered spec, in registry order.
    pub(crate) fn specs(&self) -> impl Iterator<Item = DocumentSpec> + '_ {
        self.generators.iter().map(|generator| generator.spec())
    }

    /// Resolve `--doc` values to generators, in registry order. An empty
    /// selection selects every registered kind. Repeats collapse.
    ///
    /// # Errors
    ///
    /// Returns [`DossierErrorCode::UnknownKind`] naming every unknown value
    /// and the kinds that exist.
    pub(crate) fn select(&self, kinds: &[String]) -> Result<Vec<&dyn Generator>, DossierError> {
        if self.generators.is_empty() {
            return Err(DossierError::new(
                DossierErrorCode::NoDocuments,
                "no dossier documents are registered in this build, so there is nothing to select; \
                 the srs, architecture, trace-matrix and rmm generators register in PLAT-1437 and PLAT-1438",
            ));
        }
        if kinds.is_empty() {
            return Ok(self.generators.iter().map(AsRef::as_ref).collect());
        }
        let unknown: Vec<&str> = kinds
            .iter()
            .map(String::as_str)
            .filter(|kind| self.specs().all(|spec| spec.kind != *kind))
            .collect();
        if !unknown.is_empty() {
            let known: Vec<&str> = self.specs().map(|spec| spec.kind).collect();
            return Err(DossierError::new(
                DossierErrorCode::UnknownKind,
                format!(
                    "unknown document kind {}; the kinds are {}",
                    unknown.join(", "),
                    known.join(", ")
                ),
            )
            .with_context("kind", unknown.join(",")));
        }
        Ok(self
            .generators
            .iter()
            .map(AsRef::as_ref)
            .filter(|generator| kinds.iter().any(|kind| kind == generator.spec().kind))
            .collect())
    }
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
    use crate::testing::{doc, registry};
    use ix_trace_rs::trace;

    fn three() -> Registry {
        registry(vec![
            doc("srs", "DOS-SRS", false),
            doc("architecture", "DOS-ARCH", false),
            doc("rmm", "DOS-RMM", false),
        ])
    }

    fn ids(selected: &[&dyn Generator]) -> Vec<&'static str> {
        selected
            .iter()
            .map(|generator| generator.spec().id)
            .collect()
    }

    #[trace("FR-191-AC-8")]
    #[test]
    fn no_selector_selects_every_kind_in_registry_order() {
        let registry = three();
        assert_eq!(
            ids(&registry.select(&[]).unwrap()),
            ["DOS-SRS", "DOS-ARCH", "DOS-RMM"]
        );
    }

    #[trace("FR-191-AC-8")]
    #[test]
    fn selection_is_in_registry_order_whatever_the_flag_order() {
        let registry = three();
        let kinds = ["rmm", "srs", "rmm"].map(str::to_owned);
        assert_eq!(
            ids(&registry.select(&kinds).unwrap()),
            ["DOS-SRS", "DOS-RMM"]
        );
    }

    #[trace("FR-191-AC-3")]
    #[test]
    fn an_unknown_kind_is_named_with_the_known_ones() {
        let registry = three();
        let error = registry
            .select(&["srs".to_owned(), "nope".to_owned()])
            .err()
            .unwrap();
        assert_eq!(error.code(), DossierErrorCode::UnknownKind);
        assert!(error.message().contains("nope"));
        assert!(error.message().contains("architecture"));
    }

    #[trace("FR-191")]
    #[test]
    fn a_second_generator_for_one_kind_or_id_is_refused() {
        let mut registry = three();
        let kind = registry
            .register(Box::new(doc("srs", "DOS-OTHER", false)))
            .unwrap_err();
        let id = registry
            .register(Box::new(doc("other", "DOS-SRS", false)))
            .unwrap_err();
        assert_eq!(kind.code(), DossierErrorCode::RegistryConflict);
        assert_eq!(id.code(), DossierErrorCode::RegistryConflict);
        assert_eq!(registry.specs().count(), 3);
    }

    #[trace("FR-191")]
    #[test]
    fn an_empty_registry_refuses_every_selection_with_exit_2() {
        let registry = Registry::new();
        for kinds in [&[][..], &["srs".to_owned()][..]] {
            let error = registry.select(kinds).err().unwrap();
            assert_eq!(error.code(), DossierErrorCode::NoDocuments);
            assert_eq!(error.outcome().code(), 2);
        }
    }
}
