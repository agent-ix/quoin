// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `modules` domain's wire shapes, and the ceilings they are read under.
//!
//! Request and payload types only: what a caller may say, and what it gets
//! back. No decision is taken here. The operations that take them live in
//! [`super`], and the exit-taxonomy mapping in [`super::taxonomy`].
//!
//! The size ceilings sit with the shapes rather than with the operations
//! because each is a property of a field and not of the work: they bound what
//! may be read off stdin at all, and `super::check_bound` applies them before
//! any host is consulted. stdin is untrusted (rust-style §11).

use serde::{Deserialize, Serialize};

use quoin_modules::{InstalledModule, ReconcileMode};

/// The largest `default-modules.yaml` this domain will decide over, in bytes.
///
/// The same number as `quoin_modules::manifest::MAX_MANIFEST_BYTES`, restated
/// as an in-memory byte count and pinned equal by
/// `the_manifest_bound_is_the_crates_own_ceiling`.
pub const MAX_MANIFEST_BYTES: usize = 1 << 20;

/// The largest module name or home path this domain will accept, in bytes.
///
/// A module name is a directory name and a home is a path; 4 KiB is past every
/// platform's `PATH_MAX` and still refuses a stream. stdin is untrusted.
pub const MAX_SCALAR_BYTES: usize = 4 * 1024;

/// The largest install-source argument this domain will accept, in bytes.
///
/// `github:owner/repo//very/deep/subdir@ref` is the longest real shape; the
/// ceiling refuses an accumulator rather than truncating a url into a
/// different, resolvable one.
pub const MAX_SOURCE_ARG_BYTES: usize = 4 * 1024;

/// The `home` a request may name, with its bound already checked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ListRequest {
    /// The `~/.ix` home to read, or absent for the one the host resolves.
    #[serde(default)]
    pub home: Option<String>,
}

/// The payload `modules.list` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ListPayload {
    /// Every installed module, in registry order.
    ///
    /// Registry order and not sorted here: `src/commands/module/list.ts`
    /// printed what the registry held, in the order it held it, and a sort
    /// introduced at the boundary would be a user-visible change smuggled in
    /// under a port.
    pub modules: Vec<InstalledModule>,
}

/// The request accepted by `modules.install`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InstallRequest {
    /// The CLI source argument, in the `path:` / `github:` / `package:`
    /// spellings `src/plugins.ts`'s `parseSourceArg` accepted.
    pub source: String,
    /// The `~/.ix` home to install into.
    #[serde(default)]
    pub home: Option<String>,
}

/// The payload `modules.install` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InstallPayload {
    /// The registry record that was written.
    pub module: InstalledModule,
    /// Whether a previous version of the same module was replaced.
    pub replaced_previous: bool,
}

/// The request accepted by `modules.remove`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RemoveRequest {
    /// The installed module's name.
    pub name: String,
    /// The `~/.ix` home to remove from.
    #[serde(default)]
    pub home: Option<String>,
}

/// The payload `modules.remove` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RemovePayload {
    /// The module that was removed.
    pub removed: String,
}

/// The request accepted by `modules.ensure_defaults`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EnsureDefaultsRequest {
    /// The text of `default-modules.yaml`.
    ///
    /// Carried in the request rather than located here: the file ships inside
    /// the npm package, so the caller already knows where its own package root
    /// is and the boundary does not need to guess at one.
    pub manifest: String,
    /// The `~/.ix` home to reconcile.
    #[serde(default)]
    pub home: Option<String>,
    /// `lazy` (the default) installs only what is missing or re-pinned; `sync`
    /// re-resolves every entry.
    #[serde(default)]
    pub mode: Mode,
}

/// How hard a reconcile should look, in the wire spelling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Mode {
    /// Install only what is missing or re-pinned.
    ///
    /// The default, and deliberately so: `ensureDefaultModules` ran on every
    /// `quoin write`, and a default of `sync` would put a git fetch on the
    /// authoring path.
    #[default]
    Lazy,
    /// Re-resolve every entry.
    Sync,
}

impl From<Mode> for ReconcileMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Lazy => Self::Lazy,
            Mode::Sync => Self::Sync,
        }
    }
}

/// The payload `modules.ensure_defaults` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EnsureDefaultsPayload {
    /// Entries installed for the first time.
    pub installed: Vec<String>,
    /// Entries already present and correctly pinned; no network was used.
    pub unchanged: Vec<String>,
    /// Entries re-resolved to a different commit.
    pub updated: Vec<String>,
    /// Entries skipped because `defaultEnabled` is `false`.
    pub skipped: Vec<String>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::MAX_MANIFEST_BYTES;

    /// The boundary's manifest ceiling IS the crate's.
    #[test]
    fn the_manifest_bound_is_the_crates_own_ceiling() {
        assert_eq!(
            u64::try_from(MAX_MANIFEST_BYTES).unwrap(),
            quoin_modules::manifest::MAX_MANIFEST_BYTES
        );
    }
}
