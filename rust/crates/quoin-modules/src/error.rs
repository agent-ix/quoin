// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The single error boundary for `quoin-modules` (quoin#381, FR-096).
//!
//! One `thiserror` enum, one stable code per condition a caller must be able to
//! distinguish. Codes are wire-visible at the subprocess boundary and are never
//! renamed or reused.

use std::path::PathBuf;

use crate::ids::ModuleName;

/// Stable, machine-readable discriminants for [`ModulesError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ModulesErrorCode {
    /// A source descriptor was structurally invalid.
    InvalidSource,
    /// A source type is valid but its resolution is not implemented.
    UnsupportedSource,
    /// A CLI source argument could not be parsed.
    UnparsableSourceArg,
    /// A marketplace manifest was structurally invalid.
    InvalidManifest,
    /// A module name was empty or contained a path separator.
    InvalidModuleName,
    /// A `path:` source did not exist.
    PathSourceNotFound,
    /// No `manifest.yaml` was found under a resolved module root.
    ManifestNotFound,
    /// A `manifest.yaml` declared no usable `name`.
    ManifestHasNoName,
    /// A `manifest.yaml` was unreadable or unparsable.
    ManifestUnreadable,
    /// The install registry file could not be read or parsed.
    RegistryUnreadable,
    /// The install registry file could not be written.
    RegistryUnwritable,
    /// A git remote could not be reached, or the protocol failed.
    GitTransport,
    /// A git revision (tag, branch or sha) could not be resolved.
    GitRevisionNotFound,
    /// A git object store operation failed.
    GitObjectStore,
    /// A git fetch exceeded its wall-clock budget.
    GitTimeout,
    /// The wall-clock guard for a git fetch could not be started.
    FetchDeadlineUnavailable,
    /// A resolved tree exceeded a declared resource bound.
    ResourceBoundExceeded,
    /// A tree entry named a path that escapes its destination.
    UnsafeTreePath,
    /// Materializing a module into its target directory failed.
    MaterializeFailed,
    /// A named module is not installed.
    ModuleNotInstalled,
    /// An installed module violated the semantic contract.
    SemanticContractViolation,
}

impl ModulesErrorCode {
    /// The stable wire string for this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidSource => "QM001_INVALID_SOURCE",
            Self::UnsupportedSource => "QM002_UNSUPPORTED_SOURCE",
            Self::UnparsableSourceArg => "QM003_UNPARSABLE_SOURCE_ARG",
            Self::InvalidManifest => "QM004_INVALID_MANIFEST",
            Self::InvalidModuleName => "QM005_INVALID_MODULE_NAME",
            Self::PathSourceNotFound => "QM006_PATH_SOURCE_NOT_FOUND",
            Self::ManifestNotFound => "QM007_MANIFEST_NOT_FOUND",
            Self::ManifestHasNoName => "QM008_MANIFEST_HAS_NO_NAME",
            Self::ManifestUnreadable => "QM009_MANIFEST_UNREADABLE",
            Self::RegistryUnreadable => "QM010_REGISTRY_UNREADABLE",
            Self::RegistryUnwritable => "QM011_REGISTRY_UNWRITABLE",
            Self::GitTransport => "QM012_GIT_TRANSPORT",
            Self::GitRevisionNotFound => "QM013_GIT_REVISION_NOT_FOUND",
            Self::GitObjectStore => "QM014_GIT_OBJECT_STORE",
            Self::GitTimeout => "QM015_GIT_TIMEOUT",
            Self::ResourceBoundExceeded => "QM016_RESOURCE_BOUND_EXCEEDED",
            Self::UnsafeTreePath => "QM017_UNSAFE_TREE_PATH",
            Self::MaterializeFailed => "QM018_MATERIALIZE_FAILED",
            Self::ModuleNotInstalled => "QM019_MODULE_NOT_INSTALLED",
            Self::SemanticContractViolation => "QM020_SEMANTIC_CONTRACT_VIOLATION",
            Self::FetchDeadlineUnavailable => "QM021_FETCH_DEADLINE_UNAVAILABLE",
        }
    }

    /// Every code, for catalogue round-trip tests.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::InvalidSource,
            Self::UnsupportedSource,
            Self::UnparsableSourceArg,
            Self::InvalidManifest,
            Self::InvalidModuleName,
            Self::PathSourceNotFound,
            Self::ManifestNotFound,
            Self::ManifestHasNoName,
            Self::ManifestUnreadable,
            Self::RegistryUnreadable,
            Self::RegistryUnwritable,
            Self::GitTransport,
            Self::GitRevisionNotFound,
            Self::GitObjectStore,
            Self::GitTimeout,
            Self::ResourceBoundExceeded,
            Self::UnsafeTreePath,
            Self::MaterializeFailed,
            Self::ModuleNotInstalled,
            Self::SemanticContractViolation,
            Self::FetchDeadlineUnavailable,
        ]
    }

    /// Parse a wire code back to its variant.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for ModulesErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which structural rule a source descriptor broke.
///
/// A separate enum rather than a message so a caller can branch on the rule
/// without parsing prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFieldRule {
    /// The field must be a non-empty string.
    NonEmpty,
    /// The field must not begin with `-`, which a git invocation would read as
    /// an option rather than a value.
    NotOptionLike,
}

impl std::fmt::Display for SourceFieldRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonEmpty => f.write_str("must be a non-empty string"),
            Self::NotOptionLike => f.write_str("must not begin with \"-\""),
        }
    }
}

/// The error boundary for this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModulesError {
    /// A source descriptor field broke a structural rule.
    #[error("{code}: source field {field:?} {rule}", code = ModulesErrorCode::InvalidSource)]
    InvalidSourceField {
        /// The offending field.
        field: &'static str,
        /// The rule it broke.
        rule: SourceFieldRule,
    },

    /// A source descriptor named an unknown type.
    #[error("{code}: unknown source type {found:?}", code = ModulesErrorCode::InvalidSource)]
    UnknownSourceType {
        /// The type string that was supplied.
        found: String,
    },

    /// A source type is valid but not resolvable yet.
    #[error("{code}: source type {source_type:?} is not yet supported", code = ModulesErrorCode::UnsupportedSource)]
    UnsupportedSource {
        /// The source type.
        source_type: &'static str,
    },

    /// A CLI `--source` argument could not be parsed.
    #[error("{code}: cannot parse install source {arg:?}: {detail}", code = ModulesErrorCode::UnparsableSourceArg)]
    UnparsableSourceArg {
        /// The raw argument.
        arg: String,
        /// Why.
        detail: &'static str,
    },

    /// A marketplace manifest was structurally invalid.
    #[error("{code}: {detail}", code = ModulesErrorCode::InvalidManifest)]
    InvalidManifest {
        /// What was wrong.
        detail: String,
    },

    /// A module name was not usable as a directory name.
    #[error("{code}: module name {name:?} is empty or contains a path separator", code = ModulesErrorCode::InvalidModuleName)]
    InvalidModuleName {
        /// The rejected name.
        name: String,
    },

    /// A `path:` source did not exist.
    #[error("{code}: path source not found: {path}", code = ModulesErrorCode::PathSourceNotFound, path = path.display())]
    PathSourceNotFound {
        /// The path that was looked for.
        path: PathBuf,
    },

    /// No `manifest.yaml` was found under a module root.
    #[error("{code}: no manifest.yaml found under {root}", code = ModulesErrorCode::ManifestNotFound, root = root.display())]
    ManifestNotFound {
        /// The root that was searched.
        root: PathBuf,
    },

    /// A `manifest.yaml` declared no usable `name`.
    #[error("{code}: manifest {path} has no name", code = ModulesErrorCode::ManifestHasNoName, path = path.display())]
    ManifestHasNoName {
        /// The manifest.
        path: PathBuf,
    },

    /// A `manifest.yaml` was unreadable or unparsable.
    #[error("{code}: cannot read manifest {path}: {detail}", code = ModulesErrorCode::ManifestUnreadable, path = path.display())]
    ManifestUnreadable {
        /// The manifest.
        path: PathBuf,
        /// Why.
        detail: String,
    },

    /// The install registry could not be read.
    #[error("{code}: cannot read module registry {path}: {detail}", code = ModulesErrorCode::RegistryUnreadable, path = path.display())]
    RegistryUnreadable {
        /// The registry file.
        path: PathBuf,
        /// Why.
        detail: String,
    },

    /// The install registry could not be written.
    #[error("{code}: cannot write module registry {path}: {source}", code = ModulesErrorCode::RegistryUnwritable, path = path.display())]
    RegistryUnwritable {
        /// The registry file.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// A git remote could not be reached, or the protocol failed.
    #[error("{code}: git transport failed for {url}: {detail}", code = ModulesErrorCode::GitTransport)]
    GitTransport {
        /// The remote url.
        url: String,
        /// Why.
        detail: String,
    },

    /// A git revision could not be resolved in the fetched repository.
    #[error("{code}: revision {revision:?} not found in {url} (a rebuilt history can orphan a pinned sha)", code = ModulesErrorCode::GitRevisionNotFound)]
    GitRevisionNotFound {
        /// The remote url.
        url: String,
        /// The tag, branch or sha that was requested.
        revision: String,
    },

    /// A git object store operation failed.
    #[error("{code}: git object store failure in {path}: {detail}", code = ModulesErrorCode::GitObjectStore, path = path.display())]
    GitObjectStore {
        /// The repository directory.
        path: PathBuf,
        /// Why.
        detail: String,
    },

    /// A git fetch exceeded its wall-clock budget.
    #[error("{code}: git fetch of {url} exceeded its {budget_secs}s budget: {detail}", code = ModulesErrorCode::GitTimeout)]
    GitTimeout {
        /// The remote url.
        url: String,
        /// The budget that was exceeded, in seconds.
        budget_secs: u64,
        /// The transport failure the interrupt produced. Carried rather than
        /// discarded: the budget says why the fetch stopped, this says what the
        /// transport reported when it did.
        detail: String,
    },

    /// The wall-clock guard for a git fetch could not be started.
    ///
    /// Reported rather than ignored: with no watchdog the blocking fetch has no
    /// wall-clock bound, so continuing would silently drop the only bound this
    /// module promises.
    #[error("{code}: cannot start the {budget_secs}s fetch deadline watchdog: {source}", code = ModulesErrorCode::FetchDeadlineUnavailable)]
    FetchDeadlineUnavailable {
        /// The budget the watchdog would have enforced, in seconds.
        budget_secs: u64,
        /// Why the thread could not be spawned.
        source: std::io::Error,
    },

    /// A resolved tree exceeded a declared resource bound.
    #[error("{code}: {what} exceeded its bound ({observed} > {limit})", code = ModulesErrorCode::ResourceBoundExceeded)]
    ResourceBoundExceeded {
        /// Which bound.
        what: &'static str,
        /// What was observed.
        observed: u64,
        /// The ceiling.
        limit: u64,
    },

    /// A tree entry named a path that escapes its destination.
    #[error("{code}: refusing tree entry {entry:?}: it escapes the destination directory", code = ModulesErrorCode::UnsafeTreePath)]
    UnsafeTreePath {
        /// The offending entry path.
        entry: String,
    },

    /// Materializing a module into its target directory failed.
    #[error("{code}: cannot materialize module into {target}: {source}", code = ModulesErrorCode::MaterializeFailed, target = target.display())]
    MaterializeFailed {
        /// The target directory.
        target: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// A named module is not installed.
    #[error("{code}: module {name} is not installed", code = ModulesErrorCode::ModuleNotInstalled)]
    ModuleNotInstalled {
        /// The module that was asked for.
        name: ModuleName,
    },

    /// An installed module violated the semantic contract.
    #[error("{code}: module {name} rejected: semantic contract violations\n{report}", code = ModulesErrorCode::SemanticContractViolation)]
    SemanticContractViolation {
        /// The offending module.
        name: ModuleName,
        /// The gate's rendered diagnostics.
        report: String,
        /// Whether the previous version was restored, and any failure doing so.
        rollback: RollbackOutcome,
    },
}

/// What happened to the previous version when an install was rejected.
///
/// Carried as typed data rather than appended to the message, so a caller can
/// tell "your module is unchanged" from "your module is now absent" without
/// reading prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackOutcome {
    /// There was no previous version; the freshly installed copy was removed.
    RemovedFreshInstall,
    /// The previous version was restored.
    Restored,
    /// Restoring the previous version also failed.
    Failed {
        /// Why it failed. The rejection is still the primary error.
        detail: String,
    },
}

impl std::fmt::Display for RollbackOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RemovedFreshInstall => f.write_str("the partial install was removed"),
            Self::Restored => f.write_str("the previous version was restored"),
            Self::Failed { detail } => {
                write!(f, "restoring the previous version also failed: {detail}")
            }
        }
    }
}

impl ModulesError {
    /// The stable code for this error.
    #[must_use]
    pub const fn code(&self) -> ModulesErrorCode {
        match self {
            Self::InvalidSourceField { .. } | Self::UnknownSourceType { .. } => {
                ModulesErrorCode::InvalidSource
            }
            Self::UnsupportedSource { .. } => ModulesErrorCode::UnsupportedSource,
            Self::UnparsableSourceArg { .. } => ModulesErrorCode::UnparsableSourceArg,
            Self::InvalidManifest { .. } => ModulesErrorCode::InvalidManifest,
            Self::InvalidModuleName { .. } => ModulesErrorCode::InvalidModuleName,
            Self::PathSourceNotFound { .. } => ModulesErrorCode::PathSourceNotFound,
            Self::ManifestNotFound { .. } => ModulesErrorCode::ManifestNotFound,
            Self::ManifestHasNoName { .. } => ModulesErrorCode::ManifestHasNoName,
            Self::ManifestUnreadable { .. } => ModulesErrorCode::ManifestUnreadable,
            Self::RegistryUnreadable { .. } => ModulesErrorCode::RegistryUnreadable,
            Self::RegistryUnwritable { .. } => ModulesErrorCode::RegistryUnwritable,
            Self::GitTransport { .. } => ModulesErrorCode::GitTransport,
            Self::GitRevisionNotFound { .. } => ModulesErrorCode::GitRevisionNotFound,
            Self::GitObjectStore { .. } => ModulesErrorCode::GitObjectStore,
            Self::GitTimeout { .. } => ModulesErrorCode::GitTimeout,
            Self::FetchDeadlineUnavailable { .. } => ModulesErrorCode::FetchDeadlineUnavailable,
            Self::ResourceBoundExceeded { .. } => ModulesErrorCode::ResourceBoundExceeded,
            Self::UnsafeTreePath { .. } => ModulesErrorCode::UnsafeTreePath,
            Self::MaterializeFailed { .. } => ModulesErrorCode::MaterializeFailed,
            Self::ModuleNotInstalled { .. } => ModulesErrorCode::ModuleNotInstalled,
            Self::SemanticContractViolation { .. } => ModulesErrorCode::SemanticContractViolation,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]

    use super::*;

    /// Trace: FR-096
    #[test]
    fn tc_381_202_the_error_code_catalogue_round_trips_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for code in ModulesErrorCode::all() {
            assert!(
                seen.insert(code.as_str()),
                "duplicate wire string {}",
                code.as_str()
            );
            assert_eq!(ModulesErrorCode::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(ModulesErrorCode::from_code("QM999_NOPE"), None);
        let doc = include_str!("../ERROR_CODES.md");
        for code in ModulesErrorCode::all() {
            assert!(
                doc.contains(code.as_str()),
                "{} is undocumented in ERROR_CODES.md",
                code.as_str()
            );
        }
    }
}
