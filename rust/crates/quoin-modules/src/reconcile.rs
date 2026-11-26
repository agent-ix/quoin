// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reconciling the default module set (quoin#381, FR-019).
//!
//! Ports `reconcile` from `@agent-ix/ts-plugin-kit` plus `ensureDefaultModules`
//! from `src/modules.ts`. The lazy path skips installed modules; the pinned
//! path also updates installed modules whose refs lag the current CLI pins.

use crate::error::ModulesError;
use crate::ids::ModuleName;
use crate::install::{MATERIALIZED_MARKER, ModuleInstaller};
use crate::lag::lagging_modules;
use crate::manifest::MarketplaceManifest;
use crate::registry::ModuleRegistry;
use ix_trace_rs::implements;

/// How hard reconcile should look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReconcileMode {
    /// Install only what is missing; zero git when settled.
    #[default]
    Lazy,
    /// Re-resolve every entry, so the default branch picks up new commits.
    Sync,
    /// Re-resolve installed entries older than or incomparable with their pins.
    Pinned,
}

/// What one reconcile did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Entries installed for the first time.
    pub installed: Vec<ModuleName>,
    /// Entries already installed; no network was used.
    pub unchanged: Vec<ModuleName>,
    /// Entries re-installed by a sync or because their installed refs lagged.
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
#[implements("FR-099-AC-11")]
#[implements("FR-212-AC-8")]
pub fn reconcile(
    manifest: &MarketplaceManifest,
    installer: &ModuleInstaller<'_>,
    mode: ReconcileMode,
) -> Result<ReconcileReport, ModulesError> {
    manifest.validate()?;
    let mut report = ReconcileReport::default();
    let registry = ModuleRegistry::read(&installer.paths().registry_path)?;
    let lagging = (mode == ReconcileMode::Pinned)
        .then(|| {
            lagging_modules(manifest, &registry)
                .into_iter()
                .map(|lag| lag.name)
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    for entry in &manifest.entries {
        if !entry.is_enabled() {
            report.skipped.push(entry.name.clone());
            continue;
        }
        let existing = registry.find(&entry.name);
        let target = entry.name.dir_under(&installer.paths().target_root);
        let materialized = existing.is_some_and(|installed| {
            let source_complete =
                installed.source.git_url().is_none() || target.join(MATERIALIZED_MARKER).is_file();
            target.is_dir()
                && target.join("manifest.yaml").is_file()
                && source_complete
                && !installed.target_path.is_empty()
        });

        if materialized
            && (mode == ReconcileMode::Lazy
                || (mode == ReconcileMode::Pinned && !lagging.contains(&entry.name)))
        {
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
/// every catalog read. Installed modules whose refs are older than or
/// incomparable with their semantic-version pins are updated.
///
/// # Errors
/// As [`reconcile`].
#[implements("FR-212-AC-8")]
pub fn ensure_defaults(
    manifest: &MarketplaceManifest,
    installer: &ModuleInstaller<'_>,
) -> Result<ReconcileReport, ModulesError> {
    reconcile(manifest, installer, ReconcileMode::Pinned)
}
