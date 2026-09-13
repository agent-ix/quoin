// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The single error boundary for `quoin-config` (quoin#381, FR-096).
//!
//! One `thiserror` enum, one stable code per distinguishable condition. The
//! code strings are wire-visible — they are what the subprocess boundary
//! reports as the non-success state — so they are **never renamed or reused**.

use std::path::PathBuf;

/// Stable, machine-readable discriminants for [`ConfigError`].
///
/// Codes are a closed catalogue: add, never rename, never reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ConfigErrorCode {
    /// A plugin id did not match `^[a-z][a-z0-9-]*$` or exceeded 64 bytes.
    InvalidPluginId,
    /// A package name was empty.
    InvalidPackageName,
    /// A package name could not be reduced to a legal plugin id.
    UnderivablePluginId,
    /// A second registration was attempted for a package already registered.
    DuplicateRegistration,
    /// A config operation named a plugin id nothing has registered.
    UnknownPlugin,
    /// A config file could not be read (permissions, or any non-ENOENT error).
    ConfigIo,
    /// A config file was not parsable as YAML.
    ConfigParse,
    /// A config file parsed but its top-level value was not a mapping.
    ConfigNotAMapping,
    /// A merged config failed schema validation.
    ConfigSchema,
    /// A config write refused because the target path is a symlink.
    ConfigSymlinkRefused,
    /// A config file could not be written.
    ConfigWrite,
    /// A `config set` value could not be parsed as the target key's type.
    ConfigSetParse,
    /// A `config set`/`config get` named a key the schema does not declare.
    UnknownConfigKey,
    /// An organization name was empty or whitespace-only.
    InvalidOrgName,
    /// The advisory write lock on a config file could not be acquired in time.
    ConfigLockTimeout,
    /// A plugin schema offered for registration was not strict.
    SchemaNotStrict,
}

impl ConfigErrorCode {
    /// The stable wire string for this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidPluginId => "QC001_INVALID_PLUGIN_ID",
            Self::InvalidPackageName => "QC002_INVALID_PACKAGE_NAME",
            Self::UnderivablePluginId => "QC003_UNDERIVABLE_PLUGIN_ID",
            Self::DuplicateRegistration => "QC004_DUPLICATE_REGISTRATION",
            Self::UnknownPlugin => "QC005_UNKNOWN_PLUGIN",
            Self::ConfigIo => "QC006_CONFIG_IO",
            Self::ConfigParse => "QC007_CONFIG_PARSE",
            Self::ConfigNotAMapping => "QC008_CONFIG_NOT_A_MAPPING",
            Self::ConfigSchema => "QC009_CONFIG_SCHEMA",
            Self::ConfigSymlinkRefused => "QC010_CONFIG_SYMLINK_REFUSED",
            Self::ConfigWrite => "QC011_CONFIG_WRITE",
            Self::ConfigSetParse => "QC012_CONFIG_SET_PARSE",
            Self::UnknownConfigKey => "QC013_UNKNOWN_CONFIG_KEY",
            Self::InvalidOrgName => "QC014_INVALID_ORG_NAME",
            Self::ConfigLockTimeout => "QC015_CONFIG_LOCK_TIMEOUT",
            Self::SchemaNotStrict => "QC016_SCHEMA_NOT_STRICT",
        }
    }

    /// Every code, for catalogue round-trip tests.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::InvalidPluginId,
            Self::InvalidPackageName,
            Self::UnderivablePluginId,
            Self::DuplicateRegistration,
            Self::UnknownPlugin,
            Self::ConfigIo,
            Self::ConfigParse,
            Self::ConfigNotAMapping,
            Self::ConfigSchema,
            Self::ConfigSymlinkRefused,
            Self::ConfigWrite,
            Self::ConfigSetParse,
            Self::UnknownConfigKey,
            Self::InvalidOrgName,
            Self::ConfigLockTimeout,
            Self::SchemaNotStrict,
        ]
    }

    /// Parse a wire code back to its variant.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for ConfigErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One schema-validation issue, mirroring ix-cli-core's `ConfigIssue`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConfigIssue {
    /// Dotted path of the offending key; empty for a whole-document issue.
    pub key_path: String,
    /// What the schema required there.
    pub expected: String,
    /// Human-readable detail.
    pub message: String,
}

/// The error boundary for this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// A plugin id was not a legal id.
    #[error("{code}: plugin id {id:?} must match ^[a-z][a-z0-9-]*$ and be at most 64 bytes", code = ConfigErrorCode::InvalidPluginId)]
    InvalidPluginId {
        /// The rejected id.
        id: String,
    },

    /// A package name was empty.
    #[error("{code}: package name must be a non-empty string", code = ConfigErrorCode::InvalidPackageName)]
    InvalidPackageName,

    /// A package name reduced to something that is not a legal plugin id.
    #[error("{code}: package {package:?} derives plugin id {derived:?}, which is not legal", code = ConfigErrorCode::UnderivablePluginId)]
    UnderivablePluginId {
        /// The package name that was supplied.
        package: String,
        /// What the derivation produced.
        derived: String,
    },

    /// A package (or its derived plugin id) is already registered.
    #[error("{code}: plugin schema for {package:?} is already registered (first registration preserved)", code = ConfigErrorCode::DuplicateRegistration)]
    DuplicateRegistration {
        /// The package name whose second registration was refused.
        package: String,
    },

    /// A config operation named an unregistered plugin id.
    #[error("{code}: unknown plugin id {id:?} — registered ids: {}", if registered.is_empty() { "<none>".to_owned() } else { registered.join(", ") }, code = ConfigErrorCode::UnknownPlugin)]
    UnknownPlugin {
        /// The id that was asked for.
        id: String,
        /// Every registered id, sorted.
        registered: Vec<String>,
    },

    /// A config file could not be read.
    #[error("{code}: cannot read config file {path}: {source}", code = ConfigErrorCode::ConfigIo, path = path.display())]
    ConfigIo {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// A config file was not parsable YAML.
    #[error("{code}: cannot parse config file {path}: {detail}", code = ConfigErrorCode::ConfigParse, path = path.display())]
    ConfigParse {
        /// The file that could not be parsed.
        path: PathBuf,
        /// The parser's message.
        detail: String,
    },

    /// A config file's top-level value was not a mapping.
    #[error("{code}: top-level value of {path} is not an object (found {found})", code = ConfigErrorCode::ConfigNotAMapping, path = path.display())]
    ConfigNotAMapping {
        /// The offending file.
        path: PathBuf,
        /// What was found instead (`null`, `array`, `string`, …).
        found: &'static str,
    },

    /// A merged config failed schema validation.
    #[error("{code}: config for {plugin_id} at {path} failed validation ({n} issue(s))", code = ConfigErrorCode::ConfigSchema, path = path.display(), n = issues.len())]
    ConfigSchema {
        /// The plugin whose config failed.
        plugin_id: String,
        /// The user-level file the failure is attributed to.
        path: PathBuf,
        /// Every issue found.
        issues: Vec<ConfigIssue>,
    },

    /// A config write refused a symlinked target.
    #[error("{code}: refusing to write config through a symlink at {path}", code = ConfigErrorCode::ConfigSymlinkRefused, path = path.display())]
    ConfigSymlinkRefused {
        /// The symlinked path.
        path: PathBuf,
    },

    /// A config file could not be written.
    #[error("{code}: cannot write config file {path}: {source}", code = ConfigErrorCode::ConfigWrite, path = path.display())]
    ConfigWrite {
        /// The file that could not be written.
        path: PathBuf,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// A `config set` value could not be parsed as the key's declared type.
    #[error("{code}: failed to parse value for {key_path}: expected {expected} as JSON ({detail}). Wrap non-scalar values in single quotes, e.g. '[\"a\",\"b\"]'.", code = ConfigErrorCode::ConfigSetParse)]
    ConfigSetParse {
        /// The dotted key being set.
        key_path: String,
        /// The type the schema declares there.
        expected: String,
        /// The parser's message.
        detail: String,
    },

    /// A `config get`/`config set` named a key the schema does not declare.
    #[error("{code}: {plugin_id} declares no config key {key_path:?}", code = ConfigErrorCode::UnknownConfigKey)]
    UnknownConfigKey {
        /// The plugin whose schema was consulted.
        plugin_id: String,
        /// The dotted key that is not declared.
        key_path: String,
    },

    /// An organization name was empty or whitespace-only.
    #[error("{code}: organization name must not be empty or whitespace-only", code = ConfigErrorCode::InvalidOrgName)]
    InvalidOrgName,

    /// The advisory write lock on a config file could not be acquired in time.
    ///
    /// Mirrors ix-cli-core's `ConfigLockTimeoutError`: the two implementations
    /// contend for the same `<path>.lock` sentinel while both are installed.
    #[error("{code}: timed out waiting for advisory lock at {lock_path} after {timeout_ms}ms — another process is writing the same plugin's config", code = ConfigErrorCode::ConfigLockTimeout, lock_path = lock_path.display())]
    ConfigLockTimeout {
        /// The lock sentinel that could not be claimed.
        lock_path: PathBuf,
        /// How long acquisition was attempted, in milliseconds.
        timeout_ms: u64,
    },

    /// A plugin schema offered for registration was not strict.
    ///
    /// Its own code rather than a message packed into an id field: the id here
    /// is a legal id, and the defect is the schema's strictness.
    #[error("{code}: plugin schema {id:?} is not strict; a non-strict schema would let `config set` write an unknown key", code = ConfigErrorCode::SchemaNotStrict)]
    SchemaNotStrict {
        /// The plugin id whose schema was offered.
        id: String,
    },
}

impl ConfigError {
    /// The stable code for this error.
    #[must_use]
    pub const fn code(&self) -> ConfigErrorCode {
        match self {
            Self::InvalidPluginId { .. } => ConfigErrorCode::InvalidPluginId,
            Self::InvalidPackageName => ConfigErrorCode::InvalidPackageName,
            Self::UnderivablePluginId { .. } => ConfigErrorCode::UnderivablePluginId,
            Self::DuplicateRegistration { .. } => ConfigErrorCode::DuplicateRegistration,
            Self::UnknownPlugin { .. } => ConfigErrorCode::UnknownPlugin,
            Self::ConfigIo { .. } => ConfigErrorCode::ConfigIo,
            Self::ConfigParse { .. } => ConfigErrorCode::ConfigParse,
            Self::ConfigNotAMapping { .. } => ConfigErrorCode::ConfigNotAMapping,
            Self::ConfigSchema { .. } => ConfigErrorCode::ConfigSchema,
            Self::ConfigSymlinkRefused { .. } => ConfigErrorCode::ConfigSymlinkRefused,
            Self::ConfigWrite { .. } => ConfigErrorCode::ConfigWrite,
            Self::ConfigSetParse { .. } => ConfigErrorCode::ConfigSetParse,
            Self::UnknownConfigKey { .. } => ConfigErrorCode::UnknownConfigKey,
            Self::InvalidOrgName => ConfigErrorCode::InvalidOrgName,
            Self::ConfigLockTimeout { .. } => ConfigErrorCode::ConfigLockTimeout,
            Self::SchemaNotStrict { .. } => ConfigErrorCode::SchemaNotStrict,
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
    fn tc_381_004_the_error_code_catalogue_round_trips_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for code in ConfigErrorCode::all() {
            assert!(
                seen.insert(code.as_str()),
                "duplicate wire string {}",
                code.as_str()
            );
            assert_eq!(ConfigErrorCode::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(ConfigErrorCode::from_code("QC999_NOPE"), None);
        // `ERROR_CODES.md` is the catalogue a consumer reads; a code added
        // without documenting it is the failure this guards.
        let doc = include_str!("../ERROR_CODES.md");
        for code in ConfigErrorCode::all() {
            assert!(
                doc.contains(code.as_str()),
                "{} is undocumented in ERROR_CODES.md",
                code.as_str()
            );
        }
    }
}
