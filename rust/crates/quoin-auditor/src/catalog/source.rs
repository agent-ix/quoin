// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The crate's one filesystem seam.
//!
//! Everything else in `quoin-auditor` is pure. This module is where the
//! `manifest.yaml` of an installed spec module is found and read, and
//! [`ModuleCatalogSource`] is the only way through — see [`crate::inert`] for
//! why that placement is what makes ADR-0011 invariant 1 a type-level fact
//! rather than a convention.
//!
//! The retained path is three hops:
//! `src/method-catalog.ts:795 loadMethodCatalog` →
//! `src/catalog.ts:58 defaultModuleRoots` → `:152 locateModuleRoot`, both
//! re-exported from `src/module-roots.ts`. All three are behind this one
//! trait.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::AuditorError;

/// A candidate or resolved module directory.
///
/// A newtype over the path *as a string*, because that is what ends up in
/// [`UnreadableModule::module_root`](super::UnreadableModule) and what the
/// report is ordered by. Parsing, not validating: [`ModuleRoot::resolved`] is
/// the constructor a source uses once it has decided a directory really holds
/// a `manifest.yaml`, and it is the only one that says so in its name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleRoot(String);

impl ModuleRoot {
    /// A path a caller offered, not yet known to be a module.
    #[must_use]
    pub fn candidate(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// A directory a source has established holds a `manifest.yaml`.
    ///
    /// Distinct from [`ModuleRoot::candidate`] in name only — the types are
    /// the same because the retained code passes the same strings around — but
    /// the name is the documentation of which side of `locateModuleRoot` a
    /// value is on, and a caller that mixes them reads wrong out loud.
    #[must_use]
    pub fn resolved(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// The path, verbatim.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path, as a [`Path`].
    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl std::fmt::Display for ModuleRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Where module manifests come from.
///
/// One trait with three methods rather than three traits: the three questions
/// are always asked together and always of the same world, and splitting them
/// would let a caller assemble a source that lists roots from disk and reads
/// manifests from memory — a configuration nothing wants and every test would
/// have to rule out.
pub trait ModuleCatalogSource {
    /// The candidate roots to search when the caller names none.
    ///
    /// `defaultModuleRoots()`: `QUOIN_MODULE_PATHS` split on `:`, then every
    /// entry of the installed modules directory.
    fn default_roots(&self) -> Vec<ModuleRoot>;

    /// The module root for a candidate: the path itself when it holds a
    /// `manifest.yaml`, otherwise its first child that does.
    ///
    /// [`None`] when the candidate does not exist, is not a directory, and
    /// holds no manifest.
    fn locate(&self, candidate: &ModuleRoot) -> Option<ModuleRoot>;

    /// The text of `<root>/manifest.yaml`.
    ///
    /// # Errors
    ///
    /// [`AuditorErrorCode::ManifestUnreadable`](crate::AuditorErrorCode::ManifestUnreadable)
    /// when the file cannot be read. The error is data to the caller, which
    /// records it and carries on.
    fn read_manifest(&self, root: &ModuleRoot) -> Result<String, AuditorError>;
}

/// The real one: `QUOIN_MODULE_PATHS`, `~/.ix/filament/modules`, and `std::fs`.
///
/// The environment is read **once, at construction**, and held. A source whose
/// answers changed between two calls within one audit would make the report
/// depend on when each question was asked.
#[derive(Debug, Clone)]
pub struct DiskModuleCatalogSource {
    home: PathBuf,
    module_paths: Option<String>,
}

impl DiskModuleCatalogSource {
    /// Read `IX_HOME` and `QUOIN_MODULE_PATHS` from the process environment.
    ///
    /// `ixHome()`: `IX_HOME` when it is set and non-empty, else `~/.ix`.
    #[must_use]
    pub fn from_env() -> Self {
        let home = std::env::var("IX_HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .map_or_else(
                || {
                    // `os.homedir()`. `HOME` is what it reads on every platform
                    // this ships to; an unset one yields a relative `.ix`,
                    // which resolves against the working directory exactly as
                    // `join(undefined, ".ix")` would have thrown — so it is
                    // named here rather than left to a panic.
                    let home = std::env::var("HOME").unwrap_or_default();
                    PathBuf::from(home).join(".ix")
                },
                PathBuf::from,
            );
        Self {
            home,
            module_paths: std::env::var("QUOIN_MODULE_PATHS")
                .ok()
                .filter(|value| !value.is_empty()),
        }
    }

    /// A source rooted at an explicit `~/.ix`, with an explicit search path.
    #[must_use]
    pub fn new(home: impl Into<PathBuf>, module_paths: Option<String>) -> Self {
        Self {
            home: home.into(),
            module_paths: module_paths.filter(|value| !value.is_empty()),
        }
    }

    /// The single directory that holds installed Filament modules.
    #[must_use]
    pub fn modules_dir(&self) -> PathBuf {
        self.home.join("filament").join("modules")
    }
}

impl ModuleCatalogSource for DiskModuleCatalogSource {
    fn default_roots(&self) -> Vec<ModuleRoot> {
        let mut roots = Vec::new();
        if let Some(paths) = &self.module_paths {
            // `env.split(":").filter(Boolean)` — an empty segment is dropped,
            // which is what makes a trailing `:` harmless.
            for segment in paths.split(':').filter(|part| !part.is_empty()) {
                roots.push(ModuleRoot::candidate(segment));
            }
        }
        let installed = self.modules_dir();
        if let Ok(entries) = std::fs::read_dir(&installed) {
            let mut names: Vec<std::ffi::OsString> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name())
                .collect();
            // `readdirSync` returns filesystem order in Node and an unspecified
            // order here, so this sorts: two machines must not merge the same
            // installed set in two orders, because the merge is first-wins.
            names.sort();
            for name in names {
                roots.push(ModuleRoot::candidate(
                    installed.join(name).to_string_lossy().into_owned(),
                ));
            }
        }
        roots
    }

    fn locate(&self, candidate: &ModuleRoot) -> Option<ModuleRoot> {
        let raw = candidate.as_path();
        let root = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(raw)
        };
        if !root.exists() {
            return None;
        }
        if root.join("manifest.yaml").exists() {
            return Some(ModuleRoot::resolved(root.to_string_lossy().into_owned()));
        }
        if !root.is_dir() {
            return None;
        }
        let mut names: Vec<std::ffi::OsString> = std::fs::read_dir(&root)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|name| root.join(name))
            .find(|child| child.join("manifest.yaml").exists())
            .map(|child| ModuleRoot::resolved(child.to_string_lossy().into_owned()))
    }

    fn read_manifest(&self, root: &ModuleRoot) -> Result<String, AuditorError> {
        let path = root.as_path().join("manifest.yaml");
        std::fs::read_to_string(&path).map_err(|cause| {
            AuditorError::manifest_unreadable(format!("{}: {cause}", path.display()))
        })
    }
}

/// A source with no filesystem behind it.
///
/// Shipped rather than confined to `#[cfg(test)]` on purpose: the seam exists
/// so that a caller can audit a catalog it assembled itself, and a test double
/// that only tests can reach is a seam nobody outside the crate can use.
#[derive(Debug, Clone, Default)]
pub struct MemoryModuleCatalogSource {
    roots: Vec<ModuleRoot>,
    manifests: BTreeMap<String, String>,
    unreadable: BTreeMap<String, String>,
}

impl MemoryModuleCatalogSource {
    /// An empty source: no roots, no manifests.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a module root whose manifest reads as `text`.
    #[must_use]
    pub fn with_module(mut self, root: impl Into<String>, text: impl Into<String>) -> Self {
        let root = root.into();
        self.roots.push(ModuleRoot::candidate(root.clone()));
        self.manifests.insert(root, text.into());
        self
    }

    /// Add a module root whose manifest cannot be read, and why.
    #[must_use]
    pub fn with_unreadable(mut self, root: impl Into<String>, reason: impl Into<String>) -> Self {
        let root = root.into();
        self.roots.push(ModuleRoot::candidate(root.clone()));
        self.unreadable.insert(root, reason.into());
        self
    }

    /// Add a candidate that resolves to nothing, as a missing directory does.
    #[must_use]
    pub fn with_missing(mut self, root: impl Into<String>) -> Self {
        self.roots.push(ModuleRoot::candidate(root.into()));
        self
    }
}

impl ModuleCatalogSource for MemoryModuleCatalogSource {
    fn default_roots(&self) -> Vec<ModuleRoot> {
        self.roots.clone()
    }

    fn locate(&self, candidate: &ModuleRoot) -> Option<ModuleRoot> {
        let key = candidate.as_str();
        (self.manifests.contains_key(key) || self.unreadable.contains_key(key))
            .then(|| ModuleRoot::resolved(key))
    }

    fn read_manifest(&self, root: &ModuleRoot) -> Result<String, AuditorError> {
        if let Some(text) = self.manifests.get(root.as_str()) {
            return Ok(text.clone());
        }
        Err(AuditorError::manifest_unreadable(
            self.unreadable
                .get(root.as_str())
                .cloned()
                .unwrap_or_else(|| format!("no manifest at {root}")),
        ))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        DiskModuleCatalogSource, MemoryModuleCatalogSource, ModuleCatalogSource, ModuleRoot,
    };

    #[test]
    fn a_candidate_directory_holding_a_manifest_resolves_to_itself() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("manifest.yaml"), "name: m\n").unwrap();
        let source = DiskModuleCatalogSource::new(temp.path(), None);
        let candidate = ModuleRoot::candidate(temp.path().to_string_lossy().into_owned());
        assert_eq!(
            source
                .locate(&candidate)
                .map(|root| root.as_str().to_owned()),
            Some(temp.path().to_string_lossy().into_owned())
        );
    }

    #[test]
    fn a_candidate_directory_resolves_one_level_into_its_children() {
        let temp = tempfile::tempdir().unwrap();
        let child = temp.path().join("module-b");
        std::fs::create_dir_all(temp.path().join("module-a")).unwrap();
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(child.join("manifest.yaml"), "name: m\n").unwrap();
        let source = DiskModuleCatalogSource::new(temp.path(), None);
        let candidate = ModuleRoot::candidate(temp.path().to_string_lossy().into_owned());
        assert_eq!(
            source
                .locate(&candidate)
                .map(|root| root.as_str().to_owned()),
            Some(child.to_string_lossy().into_owned()),
            "a sibling without a manifest must not win the search"
        );
    }

    #[test]
    fn the_first_sorted_child_with_a_manifest_wins() {
        // `readdirSync` returns filesystem order, so the retained side can
        // answer either way here. Sorting makes the merge — which is
        // first-wins by method id — the same on every machine. See
        // `DIVERGENCE.md` §3.
        let temp = tempfile::tempdir().unwrap();
        // Created in reverse order on purpose: a source that walked creation
        // order rather than sorted order would answer `zeta` here.
        for name in ["zeta", "alpha"] {
            let child = temp.path().join(name);
            std::fs::create_dir_all(&child).unwrap();
            std::fs::write(child.join("manifest.yaml"), "name: m\n").unwrap();
        }
        let source = DiskModuleCatalogSource::new(temp.path(), None);
        let candidate = ModuleRoot::candidate(temp.path().to_string_lossy().into_owned());
        assert_eq!(
            source
                .locate(&candidate)
                .map(|root| root.as_str().to_owned()),
            Some(temp.path().join("alpha").to_string_lossy().into_owned()),
            "the search must be deterministic across machines, not in readdir order"
        );
    }

    #[test]
    fn a_missing_candidate_resolves_to_nothing() {
        let source = DiskModuleCatalogSource::new("/definitely/not/here", None);
        assert_eq!(
            source.locate(&ModuleRoot::candidate("/definitely/not/here/either")),
            None
        );
    }

    #[test]
    fn an_empty_search_path_segment_is_dropped() {
        let source = DiskModuleCatalogSource::new("/nowhere", Some("/a::/b:".to_owned()));
        let roots: Vec<String> = source
            .default_roots()
            .into_iter()
            .map(|root| root.as_str().to_owned())
            .collect();
        assert_eq!(roots, ["/a", "/b"]);
    }

    #[test]
    fn the_memory_source_answers_the_same_three_questions() {
        let source = MemoryModuleCatalogSource::new()
            .with_module("/m", "name: m\n")
            .with_unreadable("/broken", "EISDIR")
            .with_missing("/gone");
        assert_eq!(source.default_roots().len(), 3);
        assert!(source.locate(&ModuleRoot::candidate("/gone")).is_none());
        assert_eq!(
            source
                .read_manifest(&ModuleRoot::resolved("/m"))
                .unwrap()
                .trim(),
            "name: m"
        );
        assert!(
            source
                .read_manifest(&ModuleRoot::resolved("/broken"))
                .is_err()
        );
    }
}
