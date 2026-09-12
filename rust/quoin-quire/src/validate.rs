// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Engine-driven document validation (quoin#379; quire-rs FR-032/FR-035).
//!
//! Replaces `runQuireBatch(["validate", "--scope", …, "--diagnostics-format",
//! "json", …])`, the third and last runner in `src/quire/exec.ts` and the one
//! `src/measurement/engine-run.ts` calls.
//!
//! ## What dissolves here
//!
//! `runQuireBatch` existed because the measurement runner needed "a working
//! directory, an added environment variable and its own buffer ceiling, none
//! of which `runQuireAllowFailure` carries". All three are artefacts of the
//! subprocess:
//!
//! - the **working directory** was needed because "`--scope` bounds relative
//!   globs, but the engine still resolves them against the process working
//!   directory: run this from anywhere else and it validates that directory's
//!   documents instead, reporting a clean batch for documents it never
//!   opened". Here every document is an absolute path resolved against a
//!   [`ScopeRoot`], so there is no ambient cwd to get wrong.
//! - the **environment variable** was `IX_FILAMENT_MODULES_PATH`, i.e. module
//!   selection. It is [`ModuleSelection::Closed`] here — an argument, not an
//!   ambient global.
//! - the **buffer ceiling** was `1 << 28` on a pipe. There is no pipe.
//!
//! And the consumer's own parsing goes with it: `engine-run.ts` splits stderr
//! on newlines, skips anything not starting with `{`, `JSON.parse`s each line,
//! and then regexes the document path out of the message, because "the engine
//! does not always carry `path` as a field... Matching on the field alone
//! associates zero diagnostics with every document and reports a clean batch —
//! a runner that cannot fail is not measuring anything." Here the document is
//! the loop variable.
//!
//! ## What is preserved exactly
//!
//! The three-way outcome taxonomy, and its reason for existing: *"`could-not-run`
//! is deliberately not a `fail`. 'The engine rejected this document' and 'the
//! engine never reached this document' are different facts, and merging them
//! inflates a failure rate with runs that never happened."*

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ids::ScopeRoot;
use crate::modules::{ModuleSelection, Notice};

/// One validation batch.
///
/// A batch is a repository, so an abnormal outcome is attributable to a
/// bounded set of documents rather than to the whole corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The repository root documents are resolved and reported relative to.
    pub scope: ScopeRoot,
    /// Which modules supply the archetypes.
    pub modules: ModuleSelection,
    /// The documents in this batch.
    pub documents: Vec<PathBuf>,
}

/// How one document came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentOutcome {
    /// The engine validated it and raised no error.
    Pass,
    /// The engine validated it and raised at least one error.
    Fail,
    /// The engine never reached it. **Never folded into `Fail`.**
    CouldNotRun,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Fails the document.
    Error,
    /// Advisory; never fails the document.
    Warning,
}

/// Why a diagnostic was raised. The engine's FR-032 reason vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(
    missing_docs,
    reason = "each variant's meaning is quire-rs's own doc on \
    `ValidationReason`; restating it here would be a second description to drift"
)]
pub enum Reason {
    Missing,
    Empty,
    Placeholder,
    Assert,
    Frontmatter,
    DuplicateHeading,
    UnresolvedField,
    UnknownObjectType,
    DisallowedEdgeType,
    Grammar,
    Semantic,
}

impl From<quire_rs::ValidationReason> for Reason {
    fn from(reason: quire_rs::ValidationReason) -> Self {
        // Exhaustive, no catch-all: a reason added upstream fails this build
        // rather than being exported as some other reason.
        match reason {
            quire_rs::ValidationReason::Missing => Self::Missing,
            quire_rs::ValidationReason::Empty => Self::Empty,
            quire_rs::ValidationReason::Placeholder => Self::Placeholder,
            quire_rs::ValidationReason::Assert => Self::Assert,
            quire_rs::ValidationReason::Frontmatter => Self::Frontmatter,
            quire_rs::ValidationReason::DuplicateHeading => Self::DuplicateHeading,
            quire_rs::ValidationReason::UnresolvedField => Self::UnresolvedField,
            quire_rs::ValidationReason::UnknownObjectType => Self::UnknownObjectType,
            quire_rs::ValidationReason::DisallowedEdgeType => Self::DisallowedEdgeType,
            quire_rs::ValidationReason::Grammar => Self::Grammar,
            quire_rs::ValidationReason::Semantic => Self::Semantic,
        }
    }
}

/// One validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Which document. A field, never a regular expression over the message.
    pub document: String,
    /// Whether it fails the document.
    pub severity: Severity,
    /// The machine-readable reason.
    pub reason: Reason,
    /// The engine's rendering.
    pub message: String,
    /// 1-based document line, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// Why a document could not be reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "failure", rename_all = "kebab-case")]
pub enum NotRunReason {
    /// The document could not be read.
    Unreadable {
        /// The operating system's reason.
        detail: String,
    },
    /// Frontmatter carries no `type`.
    ArchetypeUndeclared,
    /// Frontmatter names an archetype no loaded module registers.
    ArchetypeUnknown {
        /// The name that resolved to nothing.
        archetype: String,
    },
}

/// What the engine said about one document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evaluation {
    /// The document, scope-relative.
    pub document: String,
    /// Pass, fail, or never reached.
    pub outcome: DocumentOutcome,
    /// Its diagnostics, errors before warnings, in engine order.
    pub diagnostics: Vec<Diagnostic>,
    /// Set when the outcome is [`DocumentOutcome::CouldNotRun`].
    /// `None` flattens to nothing, so a passing evaluation carries no
    /// `failure` key at all — which is what makes the two states tellable
    /// apart on the wire.
    #[serde(default, flatten)]
    pub not_run: Option<NotRunReason>,
}

/// What a batch produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// One entry per requested document, in the order supplied.
    pub evaluations: Vec<Evaluation>,
    /// Advisory notices from the module walk.
    pub notices: Vec<Notice>,
}

/// Validate one batch.
///
/// # Errors
/// [`crate::Error::ModuleLoad`] / [`crate::Error::ModuleSetEmpty`] when the
/// module set does not resolve, and [`crate::Error::PathEscapesRoot`] when a
/// document resolves outside the scope. **A document the engine rejects is not
/// an error** — it is a [`DocumentOutcome::Fail`] evaluation.
pub fn run(request: &Request) -> Result<Outcome> {
    let mut notices = Vec::new();
    let registry = request.modules.resolve(&request.scope, &mut notices)?;
    let mut evaluations = Vec::with_capacity(request.documents.len());

    for path in &request.documents {
        let relative = request.scope.relativize(path)?;
        let document = relative.as_str().to_string();
        let absolute = request.scope.as_path().join(relative.as_str());

        let text = match std::fs::read_to_string(&absolute) {
            Ok(text) => text,
            Err(error) => {
                evaluations.push(not_run(
                    document,
                    NotRunReason::Unreadable {
                        detail: error.to_string(),
                    },
                ));
                continue;
            }
        };
        let Some(name) = archetype_of(&text) else {
            evaluations.push(not_run(document, NotRunReason::ArchetypeUndeclared));
            continue;
        };
        let Some(archetype) = registry.archetype(&name) else {
            evaluations.push(not_run(
                document,
                NotRunReason::ArchetypeUnknown { archetype: name },
            ));
            continue;
        };

        let result = quire_rs::validate_document_in_registry(&registry, archetype, &text);
        let mut diagnostics: Vec<Diagnostic> = result
            .errors
            .iter()
            .map(|error| Diagnostic {
                document: document.clone(),
                severity: Severity::Error,
                reason: error.reason.into(),
                message: error.message.clone(),
                line: error.line,
            })
            .collect();
        diagnostics.extend(result.warnings.iter().map(|warning| Diagnostic {
            document: document.clone(),
            severity: Severity::Warning,
            reason: warning.reason.into(),
            message: warning.message.clone(),
            line: warning.line,
        }));
        evaluations.push(Evaluation {
            document,
            // `is_valid` is the engine's own definition — warnings never fail
            // validation — rather than a second rule derived from the counts.
            outcome: if result.is_valid {
                DocumentOutcome::Pass
            } else {
                DocumentOutcome::Fail
            },
            diagnostics,
            not_run: None,
        });
    }

    Ok(Outcome {
        evaluations,
        notices,
    })
}

fn not_run(document: String, reason: NotRunReason) -> Evaluation {
    Evaluation {
        document,
        outcome: DocumentOutcome::CouldNotRun,
        diagnostics: Vec::new(),
        not_run: Some(reason),
    }
}

fn archetype_of(text: &str) -> Option<String> {
    let document = quire_rs::parse_document(text);
    quire_rs::concept_type(&document).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-097
    #[test]
    fn tc_379_070_reason_labels_are_the_engines_labels() {
        let reasons = [
            quire_rs::ValidationReason::Missing,
            quire_rs::ValidationReason::Empty,
            quire_rs::ValidationReason::Placeholder,
            quire_rs::ValidationReason::Assert,
            quire_rs::ValidationReason::Frontmatter,
            quire_rs::ValidationReason::DuplicateHeading,
            quire_rs::ValidationReason::UnresolvedField,
            quire_rs::ValidationReason::UnknownObjectType,
            quire_rs::ValidationReason::DisallowedEdgeType,
            quire_rs::ValidationReason::Grammar,
            quire_rs::ValidationReason::Semantic,
        ];
        for reason in reasons {
            assert_eq!(
                serde_json::to_value(Reason::from(reason)).expect("serializes"),
                serde_json::Value::String(reason.as_str().to_string()),
            );
        }
    }

    /// Trace: FR-096
    #[test]
    fn tc_379_071_could_not_run_is_distinguishable_from_fail_on_the_wire() {
        let never = serde_json::to_value(not_run(
            "spec/a.md".into(),
            NotRunReason::ArchetypeUndeclared,
        ))
        .expect("serializes");
        assert_eq!(never["outcome"], "could-not-run");
        assert_eq!(never["failure"], "archetype-undeclared");

        let rejected = serde_json::to_value(Evaluation {
            document: "spec/b.md".into(),
            outcome: DocumentOutcome::Fail,
            diagnostics: vec![Diagnostic {
                document: "spec/b.md".into(),
                severity: Severity::Error,
                reason: Reason::Missing,
                message: "required section absent".into(),
                line: Some(7),
            }],
            not_run: None,
        })
        .expect("serializes");
        assert_eq!(rejected["outcome"], "fail");
        assert!(rejected.get("failure").is_none());
        assert_eq!(rejected["diagnostics"][0]["document"], "spec/b.md");
        assert_eq!(rejected["diagnostics"][0]["line"], 7);
    }
}
