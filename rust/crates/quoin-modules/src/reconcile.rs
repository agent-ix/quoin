// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reconciling the default module set (quoin#381, FR-019).
//!
//! Ports `reconcile` from `@agent-ix/ts-plugin-kit` plus `ensureDefaultModules`
//! from `src/modules.ts`. The property that matters is the **lazy** path: an
//! entry already installed, by name, performs **no
//! network I/O at all**. `ensure_defaults` runs before every catalog read, so a
//! settled default set must cost nothing but a registry read and a few
//! `stat` calls.

use crate::error::ModulesError;
use crate::ids::ModuleName;
use crate::install::ModuleInstaller;
use crate::manifest::MarketplaceManifest;
use crate::registry::ModuleRegistry;

/// How hard reconcile should look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReconcileMode {
    /// Install only what is missing; zero git when settled.
    #[default]
    Lazy,
    /// Re-resolve every entry, so the default branch picks up new commits.
    Sync,
}

/// What one reconcile did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Entries installed for the first time.
    pub installed: Vec<ModuleName>,
    /// Entries already installed; no network was used.
    pub unchanged: Vec<ModuleName>,
    /// Entries re-installed by a sync.
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

        if mode == ReconcileMode::Lazy && materialized {
            report.unchanged.push(entry.name.clone());
            continue;
        }

        let outcome = installer.install(entry)?;
        if outcome.replaced_previous {
            report.updated.push(entry.name.clone());
        } else {
            report.installed.push(entry.name.clone());
        }
    }
    Ok(report)
}

/// Lazily install the committed default module set.
///
/// Idempotent and, once installed, does no git — safe to call before
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
