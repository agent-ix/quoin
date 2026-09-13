// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reconciling the default module set (quoin#381, FR-019).
//!
//! Ports `reconcile` from `@agent-ix/ts-plugin-kit` plus `ensureDefaultModules`
//! from `src/modules.ts`. The property that matters is the **lazy** path: an
//! entry already present and pinned to the same ref or sha performs **no
//! network I/O at all**. `ensure_defaults` runs before every catalog read, so a
//! settled default set must cost nothing but a registry read and a few
//! `stat` calls.

use crate::error::ModulesError;
use crate::ids::ModuleName;
use crate::install::ModuleInstaller;
use crate::manifest::{MarketplaceEntry, MarketplaceManifest};
use crate::registry::{InstalledModule, ModuleRegistry};

/// How hard reconcile should look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReconcileMode {
    /// Install only what is missing or re-pinned; zero git when settled.
    #[default]
    Lazy,
    /// Re-resolve every entry, so a moving ref picks up new commits.
    Sync,
}

/// What one reconcile did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Entries installed for the first time.
    pub installed: Vec<ModuleName>,
    /// Entries already present and correctly pinned; no network was used.
    pub unchanged: Vec<ModuleName>,
    /// Entries re-resolved to a different commit.
    pub updated: Vec<ModuleName>,
    /// Entries skipped because `defaultEnabled` is `false`.
    pub skipped: Vec<ModuleName>,
}

impl ReconcileReport {
    /// Whether any entry required a fetch.
    #[must_use]
    pub fn touched_network(&self) -> bool {
        !self.installed.is_empty() || !self.updated.is_empty()
    }
}

/// Whether an installed record already satisfies an entry's pin.
///
/// A `sha`-pinned entry matches on the resolved commit; a `ref`-pinned one
/// matches on the requested ref. An entry with neither never matches, because
/// "whatever the default branch is now" cannot be satisfied from a record.
#[must_use]
fn pin_matches(installed: &InstalledModule, entry: &MarketplaceEntry) -> bool {
    match (entry.source.pinned_sha(), entry.source.requested_ref()) {
        (Some(sha), _) => installed.sha.as_ref().is_some_and(|s| s.as_str() == sha),
        (None, Some(r#ref)) => installed.r#ref.as_deref() == Some(r#ref),
        (None, None) => false,
    }
}

/// Reconcile `manifest` into the installer's home.
///
/// # Errors
/// The first entry's failure stops the reconcile and is returned; entries
/// already processed keep their effect, matching the oracle's fail-fast loop.
pub fn reconcile(
    manifest: &MarketplaceManifest,
    installer: &ModuleInstaller<'_>,
    mode: ReconcileMode,
) -> Result<ReconcileReport, ModulesError> {
    manifest.validate()?;
    let mut report = ReconcileReport::default();
    let registry = ModuleRegistry::read(&installer.paths().registry_path)?;

    for entry in &manifest.entries {
        if !entry.is_enabled() {
            report.skipped.push(entry.name.clone());
            continue;
        }
        let existing = registry.find(&entry.name);
        let materialized = existing.is_some_and(|installed| {
            entry
                .name
                .dir_under(&installer.paths().target_root)
                .is_dir()
                && !installed.target_path.is_empty()
        });

        if mode == ReconcileMode::Lazy
            && materialized
            && existing.is_some_and(|installed| pin_matches(installed, entry))
        {
            report.unchanged.push(entry.name.clone());
            continue;
        }

        let outcome = installer.install(entry)?;
        if outcome.replaced_previous {
            let same_commit = existing
                .and_then(|i| i.sha.as_ref())
                .zip(outcome.module.sha.as_ref())
                .is_some_and(|(before, after)| before == after);
            if same_commit {
                report.unchanged.push(entry.name.clone());
            } else {
                report.updated.push(entry.name.clone());
            }
        } else {
            report.installed.push(entry.name.clone());
        }
    }
    Ok(report)
}

/// Lazily install the committed default module set.
///
/// Idempotent and, once installed and pinned, does no git — safe to call before
/// every catalog read.
///
/// # Errors
/// As [`reconcile`].
pub fn ensure_defaults(
    manifest: &MarketplaceManifest,
    installer: &ModuleInstaller<'_>,
) -> Result<ReconcileReport, ModulesError> {
    reconcile(manifest, installer, ReconcileMode::Lazy)
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
    use crate::source::Source;

    fn entry(name: &str, source: Source) -> MarketplaceEntry {
        MarketplaceEntry {
            name: ModuleName::new(name).expect("test name is legal"),
            source,
            version: None,
            default_enabled: None,
            path: None,
        }
    }

    fn installed(name: &str, r#ref: Option<&str>, sha: Option<&str>) -> InstalledModule {
        InstalledModule {
            name: ModuleName::new(name).expect("test name is legal"),
            source: Source::Path {
                path: "/x".to_owned(),
            },
            r#ref: r#ref.map(ToOwned::to_owned),
            sha: sha.map(|s| crate::ids::CommitSha::new(s).expect("test sha is 40 hex")),
            resolved_path: "/x".to_owned(),
            target_path: "/t".to_owned(),
            installed_at: "2026-01-01T00:00:00Z".to_owned(),
            semantic: None,
        }
    }

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_270_a_sha_pin_matches_on_the_commit_not_the_ref() {
        let e = entry(
            "m",
            Source::Github {
                repo: "a/b".to_owned(),
                r#ref: Some("v1".to_owned()),
                sha: Some(SHA_A.to_owned()),
            },
        );
        assert!(pin_matches(&installed("m", None, Some(SHA_A)), &e));
        assert!(!pin_matches(&installed("m", Some("v1"), Some(SHA_B)), &e));
        assert!(!pin_matches(&installed("m", Some("v1"), None), &e));
    }

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_271_an_unpinned_entry_never_matches() {
        let e = entry(
            "m",
            Source::Github {
                repo: "a/b".to_owned(),
                r#ref: None,
                sha: None,
            },
        );
        assert!(
            !pin_matches(&installed("m", None, Some(SHA_A)), &e),
            "an entry tracking the default branch cannot be satisfied from a record"
        );
    }
}
