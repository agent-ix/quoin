// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Per-criterion property-shape classification (quoin#379; quire-rs FR-052).
//!
//! Replaces `runQuireAllowFailure(["properties", "spec/**/*.md", …])` plus
//! `parseProperties` in `src/commands/advise.ts`.
//!
//! ## The partial-result problem, as data
//!
//! `quire properties` exits **1** when any input document fails to resolve — an
//! asset with no `type:`, say — while still writing a complete, valid payload
//! for every document that did. quoin needed a second runner
//! (`runQuireAllowFailure`) purely to stop an exception being thrown over that,
//! because "two untyped files must not cost the whole shape axis"
//! (agent-ix/quoin#103).
//!
//! In process there is no exit code to reinterpret. [`Outcome`] carries the
//! classified documents **and** the list of documents that could not be
//! resolved, each with a typed reason. The caller sees both facts; neither can
//! be lost by a runner choosing what an exit status means.
//!
//! ## Why this payload has hand-written types
//!
//! `quire_rs::AcClassification` is deliberately not `Serialize` — its own doc
//! says "the JSON shape is built by hand". `quire-cli` builds it with
//! `serde_json::json!`, which is exactly the untyped-emitted-format shape
//! rust-review §10 refuses: no field list to review, no compiler check that a
//! branch populated a required field. Here the payload is a `Serialize` struct,
//! and [`tests::tc_379_063_local_labels_are_the_engines_labels`] pins every
//! enum label to the engine's own `as_str`, so the two declarations cannot
//! drift into different vocabularies.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::Provenance;
use crate::error::{Error, Result};
use crate::ids::ScopeRoot;
use crate::modules::{ModuleSelection, Notice};

/// One classification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The repository root documents are resolved and reported relative to.
    pub scope: ScopeRoot,
    /// Which modules supply the archetypes and the `property_idioms` registry.
    pub modules: ModuleSelection,
    /// Documents to classify. Relative paths resolve under the scope.
    pub documents: Vec<PathBuf>,
    /// Override the archetype instead of reading frontmatter `type`.
    ///
    /// When set, an unknown name is a hard [`Error::ArchetypeUnknown`] rather
    /// than a per-document [`Unresolved`]: the caller named it, so it is the
    /// caller's mistake and not the corpus's.
    pub archetype: Option<String>,
}

/// Why one document produced no records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum UnresolvedReason {
    /// Frontmatter carries no `type`.
    ArchetypeUndeclared,
    /// Frontmatter names an archetype no loaded module registers.
    ArchetypeUnknown {
        /// The name that resolved to nothing.
        archetype: String,
    },
    /// The document could not be read.
    Unreadable {
        /// The operating system's reason.
        detail: String,
    },
}

/// One document that could not be classified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unresolved {
    /// The document, scope-relative.
    pub document: String,
    /// Why.
    #[serde(flatten)]
    pub reason: UnresolvedReason,
}

/// What a classification run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The classified documents.
    pub report: Report,
    /// Documents that resolved to no archetype. **Not an error**: see the
    /// module header.
    pub unresolved: Vec<Unresolved>,
    /// Advisory notices from the module walk.
    pub notices: Vec<Notice>,
}

/// The `quire properties --json` payload (v1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// One entry per classified document, in the order supplied.
    pub documents: Vec<Document>,
    /// Which instrument produced it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Provenance>,
}

/// One document's classified criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// The document, scope-relative.
    pub document: String,
    /// The archetype that classified it.
    pub archetype: String,
    /// Its criteria, in document order.
    pub criteria: Vec<Criterion>,
}

/// A byte-offset span carrying its own text (quire-rs FR-052).
///
/// Offsets so a consumer can assert the decomposition partitions the
/// statement; text because handing a UTF-16 consumer raw UTF-8 byte offsets is
/// a defect generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Byte offset of the first byte, into `Criterion::statement`.
    pub start: usize,
    /// Byte offset one past the last.
    pub end: usize,
    /// The text those offsets name.
    pub text: String,
}

impl From<&quire_rs::PropertySpan> for Span {
    fn from(span: &quire_rs::PropertySpan) -> Self {
        Self {
            start: span.start,
            end: span.end,
            text: span.text.clone(),
        }
    }
}

/// The FR-047 shape axis. Closed in the engine, so closed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AcShape {
    /// Asserts an outcome directly.
    Assertion,
    /// States an obligation rather than an observation.
    Obligation,
    /// Given/When/Then.
    GivenWhenThen,
    /// Nothing to test with.
    Unstructured,
}

impl From<quire_rs::grammar::ac::AcShape> for AcShape {
    fn from(shape: quire_rs::grammar::ac::AcShape) -> Self {
        // Exhaustive, with no catch-all: a variant added upstream must fail
        // this build rather than be exported as the wrong label.
        match shape {
            quire_rs::grammar::ac::AcShape::Assertion => Self::Assertion,
            quire_rs::grammar::ac::AcShape::Obligation => Self::Obligation,
            quire_rs::grammar::ac::AcShape::GivenWhenThen => Self::GivenWhenThen,
            quire_rs::grammar::ac::AcShape::Unstructured => Self::Unstructured,
        }
    }
}

/// The FR-052 property-shape axis. Closed by FR-052-CON-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PropertyShape {
    /// Two operations composed, with an identity back-reference.
    RoundTrip,
    /// Applying an operation a second time yields the first result.
    Idempotence,
    /// An ordering or stable-sort property.
    Ordering,
    /// Asserted to hold unconditionally.
    Invariant,
    /// A named failure over a class of bad inputs.
    ErrorCase,
    /// A quantification over state transitions.
    Lifecycle,
    /// A quantification over interleavings.
    Concurrency,
    /// Universally quantified — the catch-all, and not a specific shape.
    Universal,
    /// One specific scenario. A first-class outcome, never a defect.
    Example,
    /// No signal fired. Also a first-class outcome.
    Unclassified,
}

impl From<quire_rs::PropertyShape> for PropertyShape {
    fn from(shape: quire_rs::PropertyShape) -> Self {
        match shape {
            quire_rs::PropertyShape::RoundTrip => Self::RoundTrip,
            quire_rs::PropertyShape::Idempotence => Self::Idempotence,
            quire_rs::PropertyShape::Ordering => Self::Ordering,
            quire_rs::PropertyShape::Invariant => Self::Invariant,
            quire_rs::PropertyShape::ErrorCase => Self::ErrorCase,
            quire_rs::PropertyShape::Lifecycle => Self::Lifecycle,
            quire_rs::PropertyShape::Concurrency => Self::Concurrency,
            quire_rs::PropertyShape::Universal => Self::Universal,
            quire_rs::PropertyShape::Example => Self::Example,
            quire_rs::PropertyShape::Unclassified => Self::Unclassified,
        }
    }
}

/// What a generator may do with a criterion (quire-rs CR-033).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Extraction {
    /// A generator emits a test unattended.
    Extractable,
    /// A generator may emit a test and must mark it as requiring review.
    Candidate,
    /// Neither, and not a defect.
    NotExtractable,
}

impl From<quire_rs::Extraction> for Extraction {
    fn from(extraction: quire_rs::Extraction) -> Self {
        match extraction {
            quire_rs::Extraction::Extractable => Self::Extractable,
            quire_rs::Extraction::Candidate => Self::Candidate,
            quire_rs::Extraction::NotExtractable => Self::NotExtractable,
        }
    }
}

/// The obligation as nested on a criterion record (quire-rs FR-053).
///
/// Carries no `id`, `statement` or `document`: the criterion and its enclosing
/// document object already have all three.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionObligation {
    /// The declared obligation source.
    pub source: String,
    /// The hash suspect detection is `current != at_binding` over.
    pub statement_hash: String,
    /// The declared verification method, when the cell names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Declared parameters. Omitted rather than emitted empty: a declared key
    /// with no cell is absent, and an empty map reads as "declared and empty"
    /// (quire-rs FR-053-AC-6).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
    /// The declared criticality, when the cell names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criticality: Option<String>,
}

impl From<&quire_rs::obligation::CriterionObligation> for CriterionObligation {
    fn from(obligation: &quire_rs::obligation::CriterionObligation) -> Self {
        Self {
            source: obligation.source.clone(),
            statement_hash: obligation.statement_hash.clone(),
            method: obligation.method.clone(),
            parameters: obligation.parameters.clone(),
            criticality: obligation.criticality.clone(),
        }
    }
}

/// One classified acceptance criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterion {
    /// The criterion's own id, from the `ID` column or the supplement heading.
    pub row_id: Option<String>,
    /// The untruncated statement every span offset indexes.
    pub statement: String,
    /// 1-based, file-relative line.
    pub line: Option<usize>,
    /// The FR-047 shape axis.
    pub shape: AcShape,
    /// The FR-052 property-shape axis.
    pub property: PropertyShape,
    /// Whether a generator can extract a property.
    pub extractable: bool,
    /// What a generator may do with it.
    pub extraction: Extraction,
    /// The quantified noun phrase.
    pub domain: Option<Span>,
    /// The restrictive filter clause.
    pub precondition: Option<Span>,
    /// The predicate and outcome.
    pub oracle: Option<Span>,
    /// Stable ids of the signals that fired, in evaluation order.
    pub signals: Vec<String>,
    /// The obligation this criterion states, when its module declares a source.
    pub obligation: Option<CriterionObligation>,
}

impl From<&quire_rs::AcClassification> for Criterion {
    fn from(record: &quire_rs::AcClassification) -> Self {
        Self {
            row_id: record.row_id.clone(),
            statement: record.statement.clone(),
            line: record.line,
            shape: record.shape.into(),
            property: record.property.into(),
            extractable: record.extractable,
            extraction: record.extraction.into(),
            domain: record.domain.as_ref().map(Span::from),
            precondition: record.precondition.as_ref().map(Span::from),
            oracle: record.oracle.as_ref().map(Span::from),
            signals: record.signals.iter().map(|s| (*s).to_string()).collect(),
            obligation: record.obligation.as_ref().map(CriterionObligation::from),
        }
    }
}

/// Every markdown document under `<scope>/spec`, sorted.
///
/// The one glob quoin actually passes is `spec/**/*.md`, so this is that glob
/// as a function rather than a pattern language. Sorted, because
/// `documents[]` order is the payload's order and a walk order is not a
/// contract (quire-rs NFR-006).
///
/// # Errors
/// [`Error::DocumentRootMissing`] when `<scope>/spec` is absent, and
/// [`Error::Io`] when a directory under it cannot be read.
pub fn documents_under_spec(scope: &ScopeRoot) -> Result<Vec<PathBuf>> {
    let root = scope.document_root()?;
    let mut found = Vec::new();
    let mut pending = vec![root];
    // An explicit stack rather than recursion: the walk follows directory
    // entries from disk, and unbounded recursion over attacker-shaped input is
    // a panic surface (rust-review §6).
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory).map_err(|error| Error::Io {
            path: directory.clone(),
            reason: error.to_string(),
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| Error::Io {
                path: directory.clone(),
                reason: error.to_string(),
            })?;
            let path = entry.path();
            // `symlink_metadata`, so a symlinked directory cannot loop the
            // walk or reach outside the scope.
            let metadata = std::fs::symlink_metadata(&path).map_err(|error| Error::Io {
                path: path.clone(),
                reason: error.to_string(),
            })?;
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file()
                && path.extension().is_some_and(|extension| extension == "md")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Classify the requested documents.
///
/// # Errors
/// [`Error::ModuleLoad`] / [`Error::ModuleSetEmpty`] when the module set does
/// not resolve, [`Error::ArchetypeUnknown`] when an explicit
/// [`Request::archetype`] names nothing, and [`Error::PathEscapesRoot`] when a
/// document resolves outside the scope.
///
/// A document whose *own* frontmatter does not resolve is **not** an error; it
/// is an [`Unresolved`] entry on the outcome.
pub fn classify(request: &Request) -> Result<Outcome> {
    let mut notices = Vec::new();
    let registry = request.modules.resolve(&request.scope, &mut notices)?;

    let mut documents = Vec::new();
    let mut unresolved = Vec::new();

    for path in &request.documents {
        let relative = request.scope.relativize(path)?;
        let absolute = request.scope.as_path().join(relative.as_str());
        let text = match std::fs::read_to_string(&absolute) {
            Ok(text) => text,
            Err(error) => {
                unresolved.push(Unresolved {
                    document: relative.as_str().to_string(),
                    reason: UnresolvedReason::Unreadable {
                        detail: error.to_string(),
                    },
                });
                continue;
            }
        };

        let declared = match &request.archetype {
            Some(name) => Some(name.clone()),
            None => archetype_of(&text),
        };
        let Some(name) = declared else {
            unresolved.push(Unresolved {
                document: relative.as_str().to_string(),
                reason: UnresolvedReason::ArchetypeUndeclared,
            });
            continue;
        };
        let Some(archetype) = registry.archetype(&name) else {
            if request.archetype.is_some() {
                return Err(Error::ArchetypeUnknown {
                    document: relative.as_str().to_string(),
                    archetype: name,
                });
            }
            unresolved.push(Unresolved {
                document: relative.as_str().to_string(),
                reason: UnresolvedReason::ArchetypeUnknown { archetype: name },
            });
            continue;
        };

        // The scope-relative path is passed so an obligation source's
        // `exclude:` binds this surface as well as the rollup (quire-rs
        // FR-053-AC-14). Without it a criterion in an excluded fixture states
        // no obligation in `coverage --json` and states one here — and this
        // payload is what a generator reads.
        let records = quire_rs::classify_document_criteria(
            &registry,
            archetype,
            &text,
            Some(Path::new(relative.as_str())),
        );
        documents.push(Document {
            document: relative.as_str().to_string(),
            archetype: name,
            criteria: records.iter().map(Criterion::from).collect(),
        });
    }

    Ok(Outcome {
        report: Report {
            documents,
            engine: Some(Provenance::current()),
        },
        unresolved,
        notices,
    })
}

/// The archetype a document's frontmatter names.
fn archetype_of(text: &str) -> Option<String> {
    let document = quire_rs::parse_document(text);
    quire_rs::concept_type(&document).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-097
    #[test]
    fn tc_379_063_local_labels_are_the_engines_labels() {
        // The one place this payload restates an engine vocabulary. Asserted
        // against `as_str()` for every variant, so a rename upstream fails
        // here instead of shipping a payload with a label nothing recognises.
        let shapes = [
            (
                quire_rs::grammar::ac::AcShape::Assertion,
                AcShape::from(quire_rs::grammar::ac::AcShape::Assertion),
            ),
            (
                quire_rs::grammar::ac::AcShape::Obligation,
                AcShape::from(quire_rs::grammar::ac::AcShape::Obligation),
            ),
            (
                quire_rs::grammar::ac::AcShape::GivenWhenThen,
                AcShape::from(quire_rs::grammar::ac::AcShape::GivenWhenThen),
            ),
            (
                quire_rs::grammar::ac::AcShape::Unstructured,
                AcShape::from(quire_rs::grammar::ac::AcShape::Unstructured),
            ),
        ];
        for (engine, local) in shapes {
            assert_eq!(
                serde_json::to_value(local).expect("serializes"),
                serde_json::Value::String(engine.as_str().to_string()),
            );
        }

        let properties = [
            quire_rs::PropertyShape::RoundTrip,
            quire_rs::PropertyShape::Idempotence,
            quire_rs::PropertyShape::Ordering,
            quire_rs::PropertyShape::Invariant,
            quire_rs::PropertyShape::ErrorCase,
            quire_rs::PropertyShape::Lifecycle,
            quire_rs::PropertyShape::Concurrency,
            quire_rs::PropertyShape::Universal,
            quire_rs::PropertyShape::Example,
            quire_rs::PropertyShape::Unclassified,
        ];
        for engine in properties {
            assert_eq!(
                serde_json::to_value(PropertyShape::from(engine)).expect("serializes"),
                serde_json::Value::String(engine.as_str().to_string()),
            );
        }

        let extractions = [
            quire_rs::Extraction::Extractable,
            quire_rs::Extraction::Candidate,
            quire_rs::Extraction::NotExtractable,
        ];
        for engine in extractions {
            assert_eq!(
                serde_json::to_value(Extraction::from(engine)).expect("serializes"),
                serde_json::Value::String(engine.as_str().to_string()),
            );
        }
    }

    /// Trace: FR-096
    #[test]
    fn tc_379_064_an_unresolved_document_names_which_premise_failed() {
        let undeclared = serde_json::to_value(Unresolved {
            document: "spec/a.md".into(),
            reason: UnresolvedReason::ArchetypeUndeclared,
        })
        .expect("serializes");
        assert_eq!(undeclared["reason"], "archetype-undeclared");
        assert_eq!(undeclared["document"], "spec/a.md");

        let unknown = serde_json::to_value(Unresolved {
            document: "spec/b.md".into(),
            reason: UnresolvedReason::ArchetypeUnknown {
                archetype: "Nope".into(),
            },
        })
        .expect("serializes");
        assert_eq!(unknown["reason"], "archetype-unknown");
        assert_eq!(unknown["archetype"], "Nope");
    }
}
