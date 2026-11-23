// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Detecting installed default modules that lag the CLI's pins (FR-211).
//!
//! Lazy reconcile skips a module that is already installed, so a module
//! installed by an older CLI keeps its old release. This compares the release
//! each installed module was requested at with the release the manifest pins.

use semver::Version;

use crate::ids::ModuleName;
use crate::manifest::MarketplaceManifest;
use crate::registry::ModuleRegistry;

/// One installed module older than its pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLag {
    /// The module's name.
    pub name: ModuleName,
    /// The release the install registry records.
    pub installed: Version,
    /// The release the manifest pins.
    pub pinned: Version,
}

/// Parse a release tag such as `v0.27.0`; `None` for a branch or other ref.
fn release(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()
}

/// Every enabled manifest entry whose installed release is older than its pin.
///
/// A module that is not installed, or whose installed or pinned ref is not a
/// semantic version, is not reported.
#[must_use]
pub fn lagging_modules(
    manifest: &MarketplaceManifest,
    registry: &ModuleRegistry,
) -> Vec<ModuleLag> {
    manifest
        .enabled_entries()
        .filter_map(|entry| {
            let pinned = release(entry.source.requested_ref()?)?;
            let installed = release(registry.find(&entry.name)?.r#ref.as_deref()?)?;
            (installed < pinned).then(|| ModuleLag {
                name: entry.name.clone(),
                installed,
                pinned,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test fixtures may panic")]
mod tests {
    use ix_trace_rs::trace;

    use super::lagging_modules;
    use crate::manifest::MarketplaceManifest;
    use crate::registry::ModuleRegistry;

    fn manifest() -> MarketplaceManifest {
        MarketplaceManifest::from_yaml(
            "schemaVersion: 1\nentries:\n  - name: iso\n    source:\n      type: git-subdir\n      url: agent-ix/spec-artifacts-iso\n      path: spec_artifacts_iso\n      ref: v0.27.0\n",
        )
        .unwrap()
    }

    fn registry(installed_ref: Option<&str>) -> ModuleRegistry {
        let reference = installed_ref.map_or(String::new(), |r| format!(", \"ref\": \"{r}\""));
        serde_json::from_str(&format!(
            "{{\"schemaVersion\":1,\"plugins\":[{{\"name\":\"iso\",\"source\":{{\"type\":\"git-subdir\",\"url\":\"agent-ix/spec-artifacts-iso\",\"path\":\"spec_artifacts_iso\"}},\"resolvedPath\":\"/c\",\"targetPath\":\"/t\",\"installedAt\":\"2026-01-01T00:00:00Z\"{reference}}}]}}"
        ))
        .unwrap()
    }

    #[test]
    #[trace("FR-211-AC-1")]
    fn an_older_installed_release_is_reported_with_both_versions() {
        let lags = lagging_modules(&manifest(), &registry(Some("v0.26.1")));
        assert_eq!(lags.len(), 1);
        assert_eq!(lags[0].name.as_str(), "iso");
        assert_eq!(lags[0].installed.to_string(), "0.26.1");
        assert_eq!(lags[0].pinned.to_string(), "0.27.0");
    }

    #[test]
    #[trace("FR-211-AC-2")]
    fn an_equal_newer_missing_or_unversioned_install_is_not_reported() {
        let manifest = manifest();
        assert!(lagging_modules(&manifest, &registry(Some("v0.27.0"))).is_empty());
        assert!(lagging_modules(&manifest, &registry(Some("v0.28.0"))).is_empty());
        assert!(lagging_modules(&manifest, &registry(Some("main"))).is_empty());
        assert!(lagging_modules(&manifest, &registry(None)).is_empty());
        assert!(lagging_modules(&manifest, &ModuleRegistry::new()).is_empty());
    }
}
