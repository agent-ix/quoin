// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Detecting installed default modules that lag the CLI's pins (FR-212).
//!
//! Lazy reconcile skips a module that is already installed, so a module
//! installed by an older CLI keeps its old release. This compares the release
//! each installed module was requested at with the release the manifest pins.

use semver::Version;

use crate::ids::ModuleName;
use crate::manifest::MarketplaceManifest;
use crate::registry::ModuleRegistry;

/// How an installed module fails to be at its pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LagKind {
    /// The installed ref is a semantic version lower than the pin.
    Older,
    /// The installed ref is absent or not a semantic version, and differs from
    /// the pin, so the two cannot be ordered.
    Incomparable,
}

/// One installed module that is not at its pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLag {
    /// The module's name.
    pub name: ModuleName,
    /// The ref the install registry records, if any.
    pub installed_ref: Option<String>,
    /// The ref the manifest pins.
    pub pinned_ref: String,
    /// Whether the installed ref is older or cannot be compared.
    pub kind: LagKind,
}

/// Parse a release tag such as `v0.27.0`; `None` for a branch or other ref.
fn release(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()
}

/// Every enabled manifest entry that is installed at a release other than, and
/// not newer than, its pin.
///
/// A module that is not installed, or whose pin is not a semantic version, is
/// not reported.
#[must_use]
pub fn lagging_modules(
    manifest: &MarketplaceManifest,
    registry: &ModuleRegistry,
) -> Vec<ModuleLag> {
    manifest
        .enabled_entries()
        .filter_map(|entry| {
            let pinned_ref = entry.source.requested_ref()?;
            let pinned = release(pinned_ref)?;
            let installed_ref = registry.find(&entry.name)?.r#ref.clone();
            let kind = match installed_ref.as_deref().and_then(release) {
                Some(installed) if installed >= pinned => return None,
                Some(_) => LagKind::Older,
                None => LagKind::Incomparable,
            };
            Some(ModuleLag {
                name: entry.name.clone(),
                installed_ref,
                pinned_ref: pinned_ref.to_owned(),
                kind,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::panic, reason = "a panic is the failing-test signal")]
mod tests {
    use ix_trace_rs::trace;

    use super::{LagKind, lagging_modules};
    use crate::manifest::MarketplaceManifest;
    use crate::registry::ModuleRegistry;

    fn manifest(pin: &str) -> MarketplaceManifest {
        MarketplaceManifest::from_yaml(&format!(
            "schemaVersion: 1\nentries:\n  - name: iso\n    source:\n      type: git-subdir\n      url: agent-ix/spec-artifacts-iso\n      path: spec_artifacts_iso\n      ref: {pin}\n"
        ))
        .unwrap_or_else(|e| panic!("fixture manifest is valid: {e}"))
    }

    fn registry(installed_ref: Option<&str>) -> ModuleRegistry {
        let reference = installed_ref.map_or(String::new(), |r| format!(", \"ref\": \"{r}\""));
        serde_json::from_str(&format!(
            "{{\"schemaVersion\":1,\"plugins\":[{{\"name\":\"iso\",\"source\":{{\"type\":\"git-subdir\",\"url\":\"agent-ix/spec-artifacts-iso\",\"path\":\"spec_artifacts_iso\"}},\"resolvedPath\":\"/c\",\"targetPath\":\"/t\",\"installedAt\":\"2026-01-01T00:00:00Z\"{reference}}}]}}"
        ))
        .unwrap_or_else(|e| panic!("fixture registry is valid: {e}"))
    }

    #[test]
    #[trace("FR-212-AC-1")]
    fn an_older_installed_release_is_reported_with_both_refs() {
        let lags = lagging_modules(&manifest("v0.27.0"), &registry(Some("v0.26.1")));
        let [lag] = lags.as_slice() else {
            panic!("expected exactly one lag, got {lags:?}");
        };
        assert_eq!(lag.name.as_str(), "iso");
        assert_eq!(lag.installed_ref.as_deref(), Some("v0.26.1"));
        assert_eq!(lag.pinned_ref, "v0.27.0");
        assert_eq!(lag.kind, LagKind::Older);
    }

    #[test]
    #[trace("FR-212-AC-2")]
    fn an_installed_release_at_or_above_the_pin_is_not_reported() {
        let manifest = manifest("v0.27.0");
        assert_eq!(
            lagging_modules(&manifest, &registry(Some("v0.27.0"))),
            [] as [crate::lag::ModuleLag; 0]
        );
        assert_eq!(
            lagging_modules(&manifest, &registry(Some("v0.28.0"))),
            [] as [crate::lag::ModuleLag; 0]
        );
    }

    #[test]
    #[trace("FR-212-AC-3")]
    fn a_branch_or_missing_installed_ref_is_incomparable_and_reported() {
        let manifest = manifest("v0.27.0");
        for installed in [Some("main"), None] {
            let lags = lagging_modules(&manifest, &registry(installed));
            let [lag] = lags.as_slice() else {
                panic!("expected exactly one lag for {installed:?}, got {lags:?}");
            };
            assert_eq!(lag.kind, LagKind::Incomparable);
            assert_eq!(lag.installed_ref.as_deref(), installed);
            assert_eq!(lag.pinned_ref, "v0.27.0");
        }
    }

    #[test]
    #[trace("FR-212-AC-4")]
    fn a_missing_module_or_a_non_version_pin_is_not_reported() {
        assert_eq!(
            lagging_modules(&manifest("v0.27.0"), &ModuleRegistry::new()),
            [] as [crate::lag::ModuleLag; 0]
        );
        assert_eq!(
            lagging_modules(&manifest("main"), &registry(Some("v0.1.0"))),
            [] as [crate::lag::ModuleLag; 0]
        );
        assert_eq!(
            lagging_modules(&manifest("main"), &registry(None)),
            [] as [crate::lag::ModuleLag; 0]
        );
    }
}
