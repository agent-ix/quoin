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
use crate::ids::{ClauseSetRef, ModuleRoot};
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

    /// Trace: FR-096
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

    /// Trace: FR-096
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
