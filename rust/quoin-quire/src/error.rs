// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The crate's one error boundary (quoin#379, EPIC #373 FR-096).
//!
//! One `thiserror` enum, one stable code per distinguishable condition. The
//! **variant set is the API**: every state a caller must tell apart — "quire
//! has no traceability model" from "this document names no archetype" from
//! "the payload outgrew its ceiling" — is its own variant with typed fields,
//! so a caller branches on a discriminant instead of matching on prose.
//!
//! This replaces three separate TypeScript failure vocabularies that the
//! subprocess boundary forced apart:
//!
//! - `src/quire/exec.ts`'s three-way termination taxonomy (non-zero exit /
//!   signal / never-spawned), which existed only because a child process can
//!   die in ways a function call cannot;
//! - `src/quire/contract.ts`'s `VersionPremiseFailure`;
//! - `src/quire/validate.ts`'s `ContractViolation`.
//!
//! The first of those three is **gone, not ported**: an in-process call has no
//! exit status, no signal, and no `ENOBUFS`. What survives from it is the
//! resource bound it carried — see [`Error::PayloadTooLarge`] and
//! [`crate::payload::PayloadLimit`].

use std::path::PathBuf;

/// Stable machine code for one [`Error`] condition.
///
/// Codes are **never renamed and never reused**. A consumer (a CLI surface, a
/// stored diagnostic, a future `quoin-core` IPC envelope) keys on these; the
/// `Display` message is prose and may be reworded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// The scope argument does not resolve to a directory.
    ScopeNotADirectory,
    /// `<scope>/spec` is missing, so there is no document root.
    DocumentRootMissing,
    /// A supplied path escaped the root it was resolved against.
    PathEscapesRoot,
    /// A filesystem read or metadata call failed.
    Io,
    /// A module root did not load.
    ModuleLoad,
    /// A module set resolved to zero modules.
    ModuleSetEmpty,
    /// No module in scope declares a `traceability:` model.
    TraceabilityModelUndeclared,
    /// A document's frontmatter carries no `type`.
    ArchetypeUndeclared,
    /// A document names an archetype no loaded module registers.
    ArchetypeUnknown,
    /// The named exact clause-set version is not loaded.
    ClauseSetNotLoaded,
    /// Two clause-set versions could not be compared.
    ClauseSetNotComparable,
    /// A `KEY=VALUE` context entry is malformed.
    ContextEntryMalformed,
    /// A context dimension was supplied more than once.
    ContextKeyDuplicated,
    /// A payload exceeded its byte ceiling.
    PayloadTooLarge,
    /// A payload is not valid JSON.
    PayloadNotJson,
    /// A payload is valid JSON but not the shape the engine's type declares.
    PayloadShape,
    /// The assurance export could not be built or read.
    Assurance,
    /// A repository identity was empty.
    RepositoryEmpty,
    /// A revision was not 40 lowercase hexadecimal digits.
    RevisionMalformed,
    /// A stored payload names an engine older than this build requires.
    EnginePremise,
    /// A stored payload names a different engine object id.
    EngineRevisionMismatch,
}

impl ErrorCode {
    /// The stable wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScopeNotADirectory => "QQ-1001",
            Self::DocumentRootMissing => "QQ-1002",
            Self::PathEscapesRoot => "QQ-1003",
            Self::Io => "QQ-1004",
            Self::ModuleLoad => "QQ-1010",
            Self::ModuleSetEmpty => "QQ-1011",
            Self::TraceabilityModelUndeclared => "QQ-1020",
            Self::ArchetypeUndeclared => "QQ-1030",
            Self::ArchetypeUnknown => "QQ-1031",
            Self::ClauseSetNotLoaded => "QQ-1040",
            Self::ClauseSetNotComparable => "QQ-1041",
            Self::ContextEntryMalformed => "QQ-1042",
            Self::ContextKeyDuplicated => "QQ-1043",
            Self::PayloadTooLarge => "QQ-1050",
            Self::PayloadNotJson => "QQ-1051",
            Self::PayloadShape => "QQ-1052",
            Self::Assurance => "QQ-1060",
            Self::RepositoryEmpty => "QQ-1061",
            Self::RevisionMalformed => "QQ-1062",
            Self::EnginePremise => "QQ-1080",
            Self::EngineRevisionMismatch => "QQ-1081",
        }
    }

    /// Every code, in declaration order.
    ///
    /// Written out rather than derived so the catalog is reviewable, and
    /// covered by a test that every variant appears exactly once — the check a
    /// hand-written list otherwise lacks.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::ScopeNotADirectory,
            Self::DocumentRootMissing,
            Self::PathEscapesRoot,
            Self::Io,
            Self::ModuleLoad,
            Self::ModuleSetEmpty,
            Self::TraceabilityModelUndeclared,
            Self::ArchetypeUndeclared,
            Self::ArchetypeUnknown,
            Self::ClauseSetNotLoaded,
            Self::ClauseSetNotComparable,
            Self::ContextEntryMalformed,
            Self::ContextKeyDuplicated,
            Self::PayloadTooLarge,
            Self::PayloadNotJson,
            Self::PayloadShape,
            Self::Assurance,
            Self::RepositoryEmpty,
            Self::RevisionMalformed,
            Self::EnginePremise,
            Self::EngineRevisionMismatch,
        ]
    }

    /// Resolve a wire token back to its code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Everything this crate can refuse to do.
///
/// No `anyhow`, no `Box<dyn Error>`: a consumer of `quoin-quire` is the
/// TypeScript-replacing engine path, and it decides what to report from the
/// discriminant.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The scope argument does not resolve to a directory.
    #[error("scope '{}' is not a directory", path.display())]
    ScopeNotADirectory {
        /// The path as resolved.
        path: PathBuf,
    },

    /// `<scope>/spec` is missing.
    ///
    /// Named rather than folded into an empty report: silently walking the
    /// whole repository instead is how a repository-wide crawl once survived
    /// unnoticed (quire-rs#113).
    #[error(
        "no document root at '{}': spec documents live in `spec/` under the scope; \
         point the scope at the repository root, or create spec/",
        path.display()
    )]
    DocumentRootMissing {
        /// The document root that was looked for.
        path: PathBuf,
    },

    /// A supplied path resolved outside the root it was taken relative to.
    #[error("path '{}' resolves outside root '{}'", path.display(), root.display())]
    PathEscapesRoot {
        /// The offending path.
        path: PathBuf,
        /// The root it had to stay within.
        root: PathBuf,
    },

    /// A filesystem operation failed.
    ///
    /// `reason` is `std::io::Error`'s rendering: `io::Error` is neither `Clone`
    /// nor `PartialEq`, and the path is the field a caller acts on.
    #[error("could not read '{}': {reason}", path.display())]
    Io {
        /// The path that could not be read.
        path: PathBuf,
        /// The operating system's reason.
        reason: String,
    },

    /// A module root did not load.
    #[error("module load failed at '{}': {reason}", path.display())]
    ModuleLoad {
        /// The module root.
        path: PathBuf,
        /// The engine's load failure reason.
        reason: String,
    },

    /// A module set resolved to zero modules.
    #[error("no module loaded from the {requested} requested module root(s)")]
    ModuleSetEmpty {
        /// How many roots were asked for.
        requested: usize,
    },

    /// No active module declares a `traceability:` model.
    ///
    /// The exact condition whose diagnostic `stdio: ["ignore", "pipe",
    /// "ignore"]` used to discard (agent-ix/quoin#106). In process it is a
    /// variant, so it cannot be thrown away by a stream configuration.
    #[error(
        "no module in scope declares a `traceability:` model, so there is nothing to \
         reconcile; install a module that declares one (e.g. spec-artifacts-process) \
         or name one explicitly"
    )]
    TraceabilityModelUndeclared,

    /// A document's frontmatter carries no `type`.
    #[error("{document}: required 'type' is missing from frontmatter")]
    ArchetypeUndeclared {
        /// The document, as the caller named it.
        document: String,
    },

    /// A document names an archetype no loaded module registers.
    #[error("{document}: unknown type '{archetype}' (no archetype registered for it)")]
    ArchetypeUnknown {
        /// The document, as the caller named it.
        document: String,
        /// The archetype name that resolved to nothing.
        archetype: String,
    },

    /// The named exact clause-set version is not loaded.
    #[error("clause set {authority}/{id}/{version} is not loaded; available: {available}")]
    ClauseSetNotLoaded {
        /// Requested authority.
        authority: String,
        /// Requested set id.
        id: String,
        /// Requested exact version.
        version: String,
        /// What the registry does hold, comma separated, or `none`.
        available: String,
    },

    /// Two clause-set versions could not be compared.
    #[error("clause sets are not comparable: {reason}")]
    ClauseSetNotComparable {
        /// The engine's reason.
        reason: String,
    },

    /// A `KEY=VALUE` context entry is malformed.
    #[error("context entry '{entry}' must be a non-empty KEY=VALUE")]
    ContextEntryMalformed {
        /// The entry, verbatim.
        entry: String,
    },

    /// A context dimension was supplied more than once.
    #[error("context declares '{key}' more than once")]
    ContextKeyDuplicated {
        /// The duplicated dimension.
        key: String,
    },

    /// A payload exceeded its byte ceiling.
    ///
    /// The in-process successor to the `ENOBUFS` message in
    /// `src/quire/exec.ts`. A library call cannot be killed by Node for
    /// outgrowing a pipe buffer, but a payload read off disk or encoded into
    /// memory is still unbounded input, so the ceiling stays (rust-review §11).
    #[error(
        "{subject} is {observed} bytes, over the {limit}-byte ceiling; raise the \
         PayloadLimit deliberately if the corpus has genuinely grown"
    )]
    PayloadTooLarge {
        /// What was being read or written.
        subject: String,
        /// Bytes observed, or the ceiling plus one when the size was only
        /// known to exceed it.
        observed: u64,
        /// The ceiling in force.
        limit: u64,
    },

    /// A payload is not valid JSON.
    #[error("{subject} is not valid JSON: {reason}")]
    PayloadNotJson {
        /// What was being read.
        subject: String,
        /// `serde_json`'s reason, which names line and column.
        reason: String,
    },

    /// A payload is valid JSON but not the shape the engine's own type states.
    ///
    /// This is the whole of what `validate.ts`'s `ContractViolation` reported,
    /// minus the vendored schema: the type the payload is read into is the
    /// engine's, so "the shape quoin expects" and "the shape quire emits" are
    /// one declaration and cannot drift apart.
    #[error("{subject} does not match the {shape} the linked engine declares: {reason}")]
    PayloadShape {
        /// What was being read.
        subject: String,
        /// The engine type it was read into.
        shape: &'static str,
        /// `serde_json`'s reason, which names the failing field and position.
        reason: String,
    },

    /// The assurance export could not be built or read.
    ///
    /// Wraps the engine's own fail-closed reader verbatim
    /// (`quire_rs::AssuranceError`). quoin does not restate its refusals.
    #[error("assurance: {0}")]
    Assurance(#[from] quire_rs::AssuranceError),

    /// A repository identity was empty.
    #[error("repository identity must not be empty")]
    RepositoryEmpty,

    /// A revision was not 40 lowercase hexadecimal digits.
    #[error("revision '{revision}' is not a full lowercase Git object id")]
    RevisionMalformed {
        /// The value supplied.
        revision: String,
    },

    /// A stored payload names an engine this build will not read.
    ///
    /// The successor to `checkVersionPremise`, and narrower by construction:
    /// for a payload this process just computed the premise is a **compile-time
    /// fact** (see [`crate::engine`]). It survives only for payloads read back
    /// from disk, which were produced by some other build.
    #[error(
        "{subject} was produced by engine {found}, older than the {required} this build \
         reads; re-run the measurement rather than reinterpreting the artifact"
    )]
    EnginePremise {
        /// What was being read.
        subject: String,
        /// The engine version the artifact names, or `unknown`.
        found: String,
        /// The minimum this build accepts.
        required: String,
    },

    /// A stored payload was produced by a **different** engine object id.
    ///
    /// Separate from [`Self::EnginePremise`] because a revision is not
    /// ordered: the artifact's engine may be newer, older, or on a side
    /// branch, and only equality is knowable. Reporting that as "older" would
    /// be a claim the data does not support.
    #[error(
        "{subject} was produced by engine revision {found}; this build links {required}. \
         Revisions are comparable only by equality, so re-run the measurement rather \
         than assuming which is ahead"
    )]
    EngineRevisionMismatch {
        /// The object id the artifact names.
        found: String,
        /// The object id this build links.
        required: String,
        /// What was being read.
        subject: String,
    },
}

impl Error {
    /// The stable code for this condition.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::ScopeNotADirectory { .. } => ErrorCode::ScopeNotADirectory,
            Self::DocumentRootMissing { .. } => ErrorCode::DocumentRootMissing,
            Self::PathEscapesRoot { .. } => ErrorCode::PathEscapesRoot,
            Self::Io { .. } => ErrorCode::Io,
            Self::ModuleLoad { .. } => ErrorCode::ModuleLoad,
            Self::ModuleSetEmpty { .. } => ErrorCode::ModuleSetEmpty,
            Self::TraceabilityModelUndeclared => ErrorCode::TraceabilityModelUndeclared,
            Self::ArchetypeUndeclared { .. } => ErrorCode::ArchetypeUndeclared,
            Self::ArchetypeUnknown { .. } => ErrorCode::ArchetypeUnknown,
            Self::ClauseSetNotLoaded { .. } => ErrorCode::ClauseSetNotLoaded,
            Self::ClauseSetNotComparable { .. } => ErrorCode::ClauseSetNotComparable,
            Self::ContextEntryMalformed { .. } => ErrorCode::ContextEntryMalformed,
            Self::ContextKeyDuplicated { .. } => ErrorCode::ContextKeyDuplicated,
            Self::PayloadTooLarge { .. } => ErrorCode::PayloadTooLarge,
            Self::PayloadNotJson { .. } => ErrorCode::PayloadNotJson,
            Self::PayloadShape { .. } => ErrorCode::PayloadShape,
            Self::Assurance(_) => ErrorCode::Assurance,
            Self::RepositoryEmpty => ErrorCode::RepositoryEmpty,
            Self::RevisionMalformed { .. } => ErrorCode::RevisionMalformed,
            Self::EnginePremise { .. } => ErrorCode::EnginePremise,
            Self::EngineRevisionMismatch { .. } => ErrorCode::EngineRevisionMismatch,
        }
    }
}

/// This crate's result alias.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Trace: FR-096
    #[test]
    fn tc_379_001_every_code_is_unique_and_round_trips() {
        let codes: BTreeSet<&str> = ErrorCode::all().iter().map(|c| c.as_str()).collect();
        assert_eq!(
            codes.len(),
            ErrorCode::all().len(),
            "two ErrorCode variants share a wire token"
        );
        for code in ErrorCode::all() {
            assert_eq!(ErrorCode::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(ErrorCode::from_code("QQ-9999"), None);
    }

    /// Trace: FR-096
    #[test]
    fn tc_379_002_each_variant_reports_its_own_code() {
        // One representative per code, so a copy-paste in `Error::code` that
        // maps two variants to one token fails here. The set built from the
        // samples must be as large as the sample list.
        let samples = [
            Error::ScopeNotADirectory {
                path: PathBuf::from("/x"),
            },
            Error::DocumentRootMissing {
                path: PathBuf::from("/x/spec"),
            },
            Error::PathEscapesRoot {
                path: PathBuf::from("/x/../y"),
                root: PathBuf::from("/x"),
            },
            Error::Io {
                path: PathBuf::from("/x"),
                reason: "denied".into(),
            },
            Error::ModuleLoad {
                path: PathBuf::from("/m"),
                reason: "bad manifest".into(),
            },
            Error::ModuleSetEmpty { requested: 2 },
            Error::TraceabilityModelUndeclared,
            Error::ArchetypeUndeclared {
                document: "a.md".into(),
            },
            Error::ArchetypeUnknown {
                document: "a.md".into(),
                archetype: "Nope".into(),
            },
            Error::ClauseSetNotLoaded {
                authority: "a".into(),
                id: "i".into(),
                version: "1".into(),
                available: "none".into(),
            },
            Error::ClauseSetNotComparable {
                reason: "differing ids".into(),
            },
            Error::ContextEntryMalformed {
                entry: "nope".into(),
            },
            Error::ContextKeyDuplicated { key: "k".into() },
            Error::PayloadTooLarge {
                subject: "coverage".into(),
                observed: 2,
                limit: 1,
            },
            Error::PayloadNotJson {
                subject: "coverage".into(),
                reason: "eof".into(),
            },
            Error::PayloadShape {
                subject: "coverage".into(),
                shape: "CoverageReport",
                reason: "missing field".into(),
            },
            Error::Assurance(quire_rs::AssuranceError::EmptyRepository),
            Error::RepositoryEmpty,
            Error::RevisionMalformed {
                revision: "abc".into(),
            },
            Error::EnginePremise {
                subject: "coverage".into(),
                found: "0.20.0".into(),
                required: "0.46.0".into(),
            },
            Error::EngineRevisionMismatch {
                subject: "coverage".into(),
                found: "a".repeat(40),
                required: "b".repeat(40),
            },
        ];
        let seen: BTreeSet<ErrorCode> = samples.iter().map(Error::code).collect();
        assert_eq!(
            seen.len(),
            samples.len(),
            "two Error variants report the same ErrorCode"
        );
        assert_eq!(
            seen.len(),
            ErrorCode::all().len(),
            "a code has no sampled variant, or a variant has no code"
        );
        // The message is prose, but it must never be empty: an empty
        // `Display` is how a typed error degrades into "something failed".
        for sample in &samples {
            assert!(!sample.to_string().is_empty());
        }
    }
}
