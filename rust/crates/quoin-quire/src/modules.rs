// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Module-set resolution and the notices a run produces (quoin#379).
//!
//! Every capability in this crate needs a [`quire_rs::Registry`], and all of
//! them resolve one the same way `quire validate` does — which is why the rule
//! lives here once rather than in each module.
//!
//! ## Notices are returned, not printed
//!
//! quire writes load and extraction diagnostics to **stderr**, and
//! `src/quire/exec.ts` originally ran the child with `stdio: ["ignore",
//! "pipe", "ignore"]`, throwing away exactly the sentence the operator needed
//! (agent-ix/quoin#106). The fix there was to capture stderr and append it to
//! an exception message. That is still lossy: `src/measurement/engine-run.ts`
//! has to split stderr on newlines, skip anything not starting with `{`,
//! `JSON.parse` each line, and then **regex the document path out of the
//! message** because "the engine does not always carry `path` as a field".
//!
//! In process there is no stream to parse. A notice is a value with a typed
//! kind and an optional path, returned alongside the report, and the caller
//! decides what to show.

use quire_rs::Registry;

use crate::error::{Error, Result};
use crate::ids::{ModuleRoot, ScopeRoot};

/// What produced a notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum NoticeKind {
    /// A module loaded with something advisory to say (a duplicate archetype,
    /// a manifest without a name, a search path that is not a directory).
    ModuleLoad,
    /// The corpus walk had something to say about a document.
    Corpus,
    /// The source-symbol walk could not read a file, or refused a declared
    /// `source_exclude` list.
    SymbolExtraction,
}

impl NoticeKind {
    /// The stable token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModuleLoad => "module-load",
            Self::Corpus => "corpus",
            Self::SymbolExtraction => "symbol-extraction",
        }
    }
}

/// One advisory thing a run noticed.
///
/// Advisory by construction: anything that stops the run is an [`Error`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// Which walk produced it.
    pub kind: NoticeKind,
    /// The engine's own rendering.
    pub message: String,
    /// The file it is about, when it is about one. A field, never something to
    /// be recovered from the message with a regular expression.
    pub path: Option<String>,
}

/// How a run chooses its modules.
///
/// The three arms are `quire validate`'s own resolution order, made explicit:
/// an ambient default is a decision, and one a caller should have to write
/// down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleSelection {
    /// An exact, **closed** set of roots, used in the order given.
    ///
    /// Replaces ambient discovery rather than adding to it (quire-rs#405):
    /// neither `IX_FILAMENT_MODULES_PATH` nor `~/.ix/filament/modules` is
    /// consulted. A module materialized at a pinned revision *and* installed
    /// ambiently was otherwise loaded twice, resolved first-wins, and the
    /// report could not say which copy answered.
    Closed(Vec<ModuleRoot>),
    /// The module rooted at the scope itself, when `<scope>/manifest.yaml`
    /// exists; otherwise ambient discovery.
    ScopeOrAmbient,
    /// Ambient discovery only (`IX_FILAMENT_MODULES_PATH`, then the default
    /// install root).
    Ambient,
}

impl ModuleSelection {
    /// Load the registry this selection names, collecting its notices.
    ///
    /// # Errors
    /// [`Error::ModuleLoad`] when a named root fails to load, and
    /// [`Error::ModuleSetEmpty`] when a closed set produced no modules — the
    /// state the tolerant engine load reports as a failure while still
    /// returning an **empty** registry, which a caller that ignored
    /// `failures()` then met later as a misleading `UnknownArchetype`.
    pub fn resolve(&self, scope: &ScopeRoot, notices: &mut Vec<Notice>) -> Result<Registry> {
        let registry = match self {
            Self::Closed(roots) => {
                let paths: Vec<&std::path::Path> = roots.iter().map(ModuleRoot::as_path).collect();
                let registry =
                    Registry::load_module_set(&paths).map_err(|error| Error::ModuleLoad {
                        path: paths.first().map_or_else(
                            || scope.as_path().to_path_buf(),
                            |path| (*path).to_path_buf(),
                        ),
                        reason: error.to_string(),
                    })?;
                if registry.module_names().count() == 0 {
                    if let Some(failure) = registry.failures().first() {
                        return Err(Error::ModuleLoad {
                            path: failure.path.clone(),
                            reason: failure.reason.clone(),
                        });
                    }
                    return Err(Error::ModuleSetEmpty {
                        requested: roots.len(),
                    });
                }
                registry
            }
            Self::ScopeOrAmbient if scope.as_path().join("manifest.yaml").is_file() => {
                let root = scope.as_path();
                let registry = Registry::load_module(root).map_err(|error| Error::ModuleLoad {
                    path: root.to_path_buf(),
                    reason: error.to_string(),
                })?;
                if registry.module_names().count() == 0 {
                    if let Some(failure) = registry.failures().first() {
                        return Err(Error::ModuleLoad {
                            path: failure.path.clone(),
                            reason: failure.reason.clone(),
                        });
                    }
                    return Err(Error::ModuleSetEmpty { requested: 1 });
                }
                registry
            }
            Self::ScopeOrAmbient | Self::Ambient => {
                Registry::from_env().map_err(|error| Error::ModuleLoad {
                    path: scope.as_path().to_path_buf(),
                    reason: error.to_string(),
                })?
            }
        };
        collect(NoticeKind::ModuleLoad, registry.diagnostics(), notices);
        for failure in registry.failures() {
            notices.push(Notice {
                kind: NoticeKind::ModuleLoad,
                message: format!(
                    "archetype '{}' in module '{}' did not load: {}",
                    failure.archetype, failure.module, failure.reason
                ),
                path: Some(failure.path.display().to_string()),
            });
        }
        Ok(registry)
    }
}

/// Fold engine diagnostics into notices.
pub(crate) fn collect(
    kind: NoticeKind,
    diagnostics: &[quire_rs::Diagnostic],
    notices: &mut Vec<Notice>,
) {
    for diagnostic in diagnostics {
        notices.push(Notice {
            kind,
            message: diagnostic.to_string(),
            path: None,
        });
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// Trace: FR-096
    #[test]
    fn tc_379_040_notice_kinds_have_distinct_stable_tokens() {
        let kinds = [
            NoticeKind::ModuleLoad,
            NoticeKind::Corpus,
            NoticeKind::SymbolExtraction,
        ];
        let mut tokens: Vec<&str> = kinds.iter().map(|k| k.as_str()).collect();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(tokens.len(), kinds.len());
    }

    /// Trace: FR-096
    #[test]
    fn tc_379_041_a_closed_set_naming_nothing_loadable_is_refused_by_name() {
        let scope = tempdir();
        let empty = scope.join("empty-module");
        std::fs::create_dir_all(&empty).expect("fixture dir");
        let selection =
            ModuleSelection::Closed(vec![ModuleRoot::open(&empty).expect("directory exists")]);
        let scope = ScopeRoot::open(&scope).expect("directory exists");
        let mut notices = Vec::new();
        let error = selection
            .resolve(&scope, &mut notices)
            .expect_err("an empty module directory declares nothing");
        assert!(
            matches!(
                error.code(),
                crate::ErrorCode::ModuleSetEmpty | crate::ErrorCode::ModuleLoad
            ),
            "got {error:?}"
        );
    }

    /// A scratch directory removed by the process that made it. Deliberately
    /// local rather than a `tempfile` dependency: two tests need it.
    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "quoin-quire-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("scratch dir");
        base
    }
}
