// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Module-supplied clause sets (quoin#379; quire-rs FR-067).
//!
//! Replaces `parseClauseBinding` in `src/commands/discharge.ts`, which read a
//! `quire clauses evaluate --format json` payload **off disk** — the one
//! consumption path in `src/quire/` that never shelled out at all. Both halves
//! are here: evaluating a set in process, and reading a payload somebody else
//! produced (see [`crate::payload::ClauseBindingPayload`]).
//!
//! The engine owns the model, validation, applicability semantics and diff.
//! This module owns exact-version resolution and the context premise.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::ids::{ClauseSetAuthority, ClauseSetId, ClauseSetRef, ClauseSetVersion, ModuleRoot};
use crate::modules::{Notice, NoticeKind};

/// The declared context a clause set is evaluated against.
///
/// A validated map rather than a `Vec<String>` of `KEY=VALUE`: a duplicate
/// dimension is a premise failure, and the only place to notice it is where
/// the entry is accepted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context(BTreeMap<String, String>);

impl Context {
    /// An empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one dimension.
    ///
    /// # Errors
    /// [`Error::ContextEntryMalformed`] for an empty key or value, and
    /// [`Error::ContextKeyDuplicated`] when the dimension is already declared.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let key = key.into();
        let value = value.into();
        if key.trim().is_empty() || value.trim().is_empty() {
            return Err(Error::ContextEntryMalformed {
                entry: format!("{key}={value}"),
            });
        }
        let (key, value) = (key.trim().to_string(), value.trim().to_string());
        // Checked before the insert, not after: `BTreeMap::insert` reports the
        // displaced value *and has already displaced it*, so a refusal built
        // on its return value leaves the second value in the map — a refused
        // call that half-applied.
        if self.0.contains_key(&key) {
            return Err(Error::ContextKeyDuplicated { key });
        }
        self.0.insert(key, value);
        Ok(())
    }

    /// Parse `KEY=VALUE` entries, as the CLI flag accepts them.
    ///
    /// # Errors
    /// As [`Self::insert`].
    pub fn from_entries<S: AsRef<str>>(entries: &[S]) -> Result<Self> {
        let mut context = Self::new();
        for entry in entries {
            let entry = entry.as_ref();
            let Some((key, value)) = entry.split_once('=') else {
                return Err(Error::ContextEntryMalformed {
                    entry: entry.to_string(),
                });
            };
            context.insert(key, value)?;
        }
        Ok(context)
    }

    /// The dimensions, as the engine takes them.
    #[must_use]
    pub fn as_map(&self) -> &BTreeMap<String, String> {
        &self.0
    }
}

/// Evaluate one exact clause-set version against a declared context.
///
/// # Errors
/// [`Error::ModuleLoad`] when the module does not load, and
/// [`Error::ClauseSetNotLoaded`] when the exact version is absent — with the
/// available exact sets named, because "not loaded" and "loaded at a different
/// version" are the two things a caller needs to tell apart.
pub fn evaluate(
    module: &ModuleRoot,
    set: &ClauseSetRef,
    context: &Context,
    notices: &mut Vec<Notice>,
) -> Result<quire_rs::ClauseBindingReport> {
    let registry = load_strict(module, notices)?;
    let loaded = exact(&registry, set)?;
    Ok(loaded.evaluate(context.as_map()))
}

/// Evaluate every clause set the module at `module` declares against
/// `context`, from one load of the module, in the engine's key order.
///
/// # Errors
/// [`Error::ModuleLoad`] when the module does not load strictly, and
/// [`Error::ContextEntryMalformed`] when a loaded set has an empty identity
/// field.
pub fn evaluate_all(
    module: &ModuleRoot,
    context: &Context,
    notices: &mut Vec<Notice>,
) -> Result<Vec<(ClauseSetRef, quire_rs::ClauseBindingReport)>> {
    let registry = load_strict(module, notices)?;
    registry
        .clause_sets()
        .map(|set| {
            let reference = ClauseSetRef {
                authority: ClauseSetAuthority::new(set.authority.as_str())?,
                id: ClauseSetId::new(set.id.as_str())?,
                version: ClauseSetVersion::new(set.version.as_str())?,
            };
            Ok((reference, set.evaluate(context.as_map())))
        })
        .collect()
}

/// The exact clause-set keys the module rooted at `module` declares, in the
/// engine's key order; empty for a module whose manifest lists no clause sets.
///
/// # Errors
/// [`Error::ModuleLoad`] when a module that lists clause sets does not load
/// strictly, and when the manifest cannot be read to tell.
pub fn declared_clause_sets(
    module: &ModuleRoot,
    notices: &mut Vec<Notice>,
) -> Result<Vec<ClauseSetRef>> {
    if !declares_clause_sets(module.as_path())? {
        return Ok(Vec::new());
    }
    let registry = load_strict(module, notices)?;
    registry
        .clause_sets()
        .map(|set| {
            Ok(ClauseSetRef {
                authority: ClauseSetAuthority::new(set.authority.as_str())?,
                id: ClauseSetId::new(set.id.as_str())?,
                version: ClauseSetVersion::new(set.version.as_str())?,
            })
        })
        .collect()
}

/// The licence-restricted clause sets of the module rooted at `module`, as the
/// engine's `check_publishable` reports them; empty means no restricted set.
///
/// Quoin restates no rights rule: the verdict is the engine's alone. A module
/// that declares clause sets and does not load is an error here and never an
/// empty answer, because a module whose clause sets could not be read must not
/// be taken for one without licensed text. A module that fails to load for any
/// other reason and declares no `clause_sets` has none to restrict, so the
/// engine's other load verdicts (semantic-contract problems that `module
/// inspect` reports) do not decide pack status.
///
/// # Errors
/// [`Error::ModuleLoad`] when a module that declares clause sets does not load,
/// naming the failing file where the engine names one, and when the manifest
/// cannot be read to tell.
pub fn restricted_clause_sets(module: &ModuleRoot) -> Result<Vec<quire_rs::RestrictedClauseSet>> {
    let root = module.as_path();
    let load_error = |path: &std::path::Path, reason: String| Error::ModuleLoad {
        path: path.to_path_buf(),
        reason,
    };
    let outcome = quire_rs::loader::load_single_module(root);
    let failure = outcome
        .manifest_errors
        .first()
        .map(|failure| (failure.path.clone(), failure.message.clone()))
        .or_else(|| {
            outcome
                .failures
                .first()
                .map(|failure| (failure.path.clone(), failure.reason.clone()))
        });
    if let Some((path, reason)) = failure {
        return if declares_clause_sets(root)? {
            Err(load_error(&path, reason))
        } else {
            Ok(Vec::new())
        };
    }
    let mut modules = outcome.modules.iter();
    match (modules.next(), modules.next()) {
        (Some(loaded), None) => Ok(quire_rs::check_publishable(loaded)),
        _ => Err(load_error(
            root,
            "expected exactly one module at this root".to_owned(),
        )),
    }
}

/// Whether the module's `manifest.yaml` lists any `clause_sets` entry.
fn declares_clause_sets(root: &std::path::Path) -> Result<bool> {
    let manifest = root.join("manifest.yaml");
    let unreadable = |reason: String| Error::ModuleLoad {
        path: manifest.clone(),
        reason,
    };
    let bytes = std::fs::read_to_string(&manifest).map_err(|e| unreadable(e.to_string()))?;
    let value = quoin_yaml::from_str(&bytes).map_err(|e| unreadable(e.to_string()))?;
    Ok(value
        .get("clause_sets")
        .is_some_and(|sets| !sets.is_null() && sets.as_array().is_none_or(|list| !list.is_empty())))
}

/// Compare two exact versions of the same clause set.
///
/// # Errors
/// As [`evaluate`], plus [`Error::ClauseSetNotComparable`] when the engine
/// refuses the comparison.
pub fn diff(
    module: &ModuleRoot,
    before: &ClauseSetRef,
    after: &ClauseSetRef,
    notices: &mut Vec<Notice>,
) -> Result<quire_rs::ClauseSetDiff> {
    let registry = load_strict(module, notices)?;
    let before = exact(&registry, before)?;
    let after = exact(&registry, after)?;
    quire_rs::diff_clause_sets(before, after).map_err(|error| Error::ClauseSetNotComparable {
        reason: error.to_string(),
    })
}

/// Clause sets load **strictly**: a duplicate authority/id/version key
/// contributed with different content is a refusal, not a first-wins warning.
/// A rights-bearing clause set resolved by luck of load order is not a
/// contract anybody can rely on.
fn load_strict(module: &ModuleRoot, notices: &mut Vec<Notice>) -> Result<quire_rs::Registry> {
    let registry = quire_rs::Registry::load_module_strict(module.as_path()).map_err(|error| {
        Error::ModuleLoad {
            path: module.as_path().to_path_buf(),
            reason: error.to_string(),
        }
    })?;
    crate::modules::collect(NoticeKind::ModuleLoad, registry.diagnostics(), notices);
    if let Some(failure) = registry.failures().first() {
        return Err(Error::ModuleLoad {
            path: failure.path.clone(),
            reason: failure.reason.clone(),
        });
    }
    Ok(registry)
}

fn exact<'a>(
    registry: &'a quire_rs::Registry,
    wanted: &ClauseSetRef,
) -> Result<&'a quire_rs::ClauseSet> {
    registry
        .clause_set(
            wanted.authority.as_str(),
            wanted.id.as_str(),
            wanted.version.as_str(),
        )
        .ok_or_else(|| {
            let available: Vec<String> = registry
                .clause_sets()
                .map(|set| format!("{}/{}/{}", set.authority, set.id, set.version))
                .collect();
            Error::ClauseSetNotLoaded {
                authority: wanted.authority.to_string(),
                id: wanted.id.to_string(),
                version: wanted.version.to_string(),
                available: if available.is_empty() {
                    "none".to_string()
                } else {
                    available.join(", ")
                },
            }
        })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    use std::fmt::Write as _;

    use crate::testing::Scratch;

    fn clause_set(id: &str, text_rights: &str) -> String {
        let text = if text_rights == "none" {
            ""
        } else {
            "    text: sentinel-text\n"
        };
        format!(
            "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: {id}\n\
             title: Synthetic {id}\nversion: \"2\"\nrights:\n  structure: original\n  \
             text: {text_rights}\nclauses:\n  - id: C-1\n    force: mandatory\n{text}"
        )
    }

    fn write_module(scratch: &Scratch, sets: &[(&str, &str)]) -> ModuleRoot {
        let mut manifest = String::from("name: synthetic-pack\nversion: 0.1.0\nclause_sets:\n");
        for (id, rights) in sets {
            writeln!(manifest, "  - {id}.yaml").expect("write to a String");
            std::fs::write(
                scratch.path().join(format!("{id}.yaml")),
                clause_set(id, rights),
            )
            .expect("clause set written");
        }
        std::fs::write(scratch.path().join("manifest.yaml"), manifest).expect("manifest written");
        ModuleRoot::open(scratch.path()).expect("module root")
    }

    #[trace("FR-137-AC-12")]
    #[test]
    fn tc_1492_001_restricted_sets_are_exactly_what_check_publishable_reports() {
        let scratch = Scratch::new("tc-1492-001");
        let root = write_module(
            &scratch,
            &[
                ("zeta", "licensed"),
                ("alpha", "licensed"),
                ("open", "none"),
            ],
        );
        let reported = restricted_clause_sets(&root).expect("loads");

        let outcome = quire_rs::loader::load_single_module(root.as_path());
        let expected = quire_rs::check_publishable(&outcome.modules[0]);
        assert_eq!(reported, expected);
        let ids: Vec<&str> = reported.iter().map(|set| set.id.as_str()).collect();
        assert_eq!(ids, ["alpha", "zeta"]);
    }

    #[trace("FR-137-AC-3")]
    #[test]
    fn tc_1492_002_a_module_without_licensed_text_has_no_restricted_set() {
        let scratch = Scratch::new("tc-1492-002");
        let root = write_module(&scratch, &[("a", "none"), ("b", "original")]);
        assert!(restricted_clause_sets(&root).expect("loads").is_empty());
    }

    #[trace("FR-137-AC-8")]
    #[test]
    fn tc_1492_003_a_clause_set_without_rights_is_a_load_error_naming_the_file() {
        let scratch = Scratch::new("tc-1492-003");
        std::fs::write(
            scratch.path().join("manifest.yaml"),
            "name: synthetic-pack\nversion: 0.1.0\nclause_sets:\n  - bare.yaml\n",
        )
        .expect("manifest");
        std::fs::write(
            scratch.path().join("bare.yaml"),
            "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: bare\n\
             title: Bare\nversion: \"1\"\nclauses: []\n",
        )
        .expect("clause set");
        let root = ModuleRoot::open(scratch.path()).expect("root");
        let error =
            restricted_clause_sets(&root).expect_err("a missing rights object is no verdict");
        assert_eq!(error.code(), crate::ErrorCode::ModuleLoad);
        assert!(error.to_string().contains("bare.yaml"), "{error}");
    }

    #[trace("FR-137-AC-8")]
    #[test]
    fn tc_1492_004_a_module_that_fails_to_load_without_clause_sets_has_none_to_restrict() {
        let scratch = Scratch::new("tc-1492-004");
        std::fs::write(
            scratch.path().join("manifest.yaml"),
            "name: synthetic-plain\nversion: 0.1.0\nretired_field_that_fails_load: true\n",
        )
        .expect("manifest");
        let root = ModuleRoot::open(scratch.path()).expect("root");
        assert!(
            restricted_clause_sets(&root)
                .expect("no clause sets declared")
                .is_empty()
        );
    }

    #[trace("FR-096")]
    #[test]
    fn tc_379_080_a_duplicate_context_dimension_is_refused_not_overwritten() {
        let mut context = Context::new();
        context
            .insert("jurisdiction", "eu")
            .expect("first wins nothing");
        let error = context
            .insert("jurisdiction", "us")
            .expect_err("a second value for one dimension is a premise failure");
        assert_eq!(error.code(), crate::ErrorCode::ContextKeyDuplicated);
        // The first value is still the only one: a refused insert must not
        // have half-applied.
        assert_eq!(
            context.as_map().get("jurisdiction").map(String::as_str),
            Some("eu")
        );
    }

    #[trace("FR-096")]
    #[test]
    fn tc_379_081_context_entries_must_be_key_equals_value() {
        assert_eq!(
            Context::from_entries(&["jurisdiction"])
                .expect_err("no separator")
                .code(),
            crate::ErrorCode::ContextEntryMalformed
        );
        assert_eq!(
            Context::from_entries(&["jurisdiction="])
                .expect_err("empty value")
                .code(),
            crate::ErrorCode::ContextEntryMalformed
        );
        let parsed = Context::from_entries(&["jurisdiction = eu ", "tier=1"]).expect("parses");
        assert_eq!(
            parsed.as_map().get("jurisdiction").map(String::as_str),
            Some("eu")
        );
        assert_eq!(parsed.as_map().len(), 2);
    }
}
