// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The retained evidence bindings, read at a stricter boundary than the store's
//! own reader uses.
//!
//! `load.ts:380-383` says why in its own comment: `readBindings()` maps a JSON
//! `null` onto the legacy absent-file default, and FR-062 must tell an
//! existing, malformed store apart from an absent one. So the bytes are read
//! here and the three states — available, absent, unreadable — are a type
//! rather than a fallback.
//!
//! # Undeclared members are accepted and then dropped
//!
//! The retained schemas are `.passthrough()` (`load.ts:324`, `:335`, `:342`),
//! so an undeclared member must not be a refusal. It is also never written
//! back out: no binding reaches either report, only values derived
//! from the fields below. Carrying the rest would be carrying it nowhere.
//! That is the opposite call from `quoin-finding-types`, and the difference is
//! whether the producer's bytes are re-emitted.

use serde_json::Value;

use crate::ids::{Commit, ObligationId, SuiteId};
use crate::json as reader;

/// One obligation bound to one suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The obligation this binding is for.
    pub obligation: ObligationId,
    /// The suite that carries it.
    pub suite: SuiteId,
    /// The commit it was bound at.
    pub commit: Commit,
    /// The symbols it names.
    pub symbols: Vec<String>,
}

/// Whether the bindings store could be read, and what it said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingInput {
    /// The store was read and parsed.
    Available(Vec<Binding>),
    /// The store is not there.
    Absent {
        /// What to say in the gap.
        reason: String,
    },
    /// The store is there and could not be read as the retained schema.
    Unreadable {
        /// What to say in the gap.
        reason: String,
    },
}

impl BindingInput {
    /// The bindings, when there are any to have.
    #[must_use]
    pub fn available(&self) -> Option<&[Binding]> {
        match self {
            Self::Available(bindings) => Some(bindings),
            Self::Absent { .. } | Self::Unreadable { .. } => None,
        }
    }
}

/// Read the retained bindings file (`load.ts:337`).
///
/// # Errors
///
/// One line naming the member that failed and why. The caller turns that into
/// an `unreadable` availability rather than a refusal, because that is what
/// the retained implementation does with it.
pub fn parse_bindings_file(value: &Value) -> std::result::Result<Vec<Binding>, String> {
    let root = reader::object(value, "<root>")?;
    reader::literal_number(
        reader::member(root, "", "schemaVersion")?,
        "schemaVersion",
        u64::from(quoin_store::STORE_SCHEMA_VERSION),
    )?;
    let bindings = reader::array(reader::member(root, "", "bindings")?, "bindings")?;
    let mut parsed = Vec::with_capacity(bindings.len());
    for (index, binding) in bindings.iter().enumerate() {
        parsed.push(parse_binding(binding, &format!("bindings.{index}"))?);
    }
    Ok(canonicalize(parsed))
}

fn parse_binding(value: &Value, at: &str) -> std::result::Result<Binding, String> {
    let object = reader::object(value, at)?;
    let prefix = format!("{at}.");
    let obligation = non_empty(
        &prefix,
        reader::member(object, &prefix, "obligation")?,
        "obligation",
    )?;
    let suite = non_empty(&prefix, reader::member(object, &prefix, "suite")?, "suite")?;
    let commit = non_empty(
        &prefix,
        reader::member(object, &prefix, "commit")?,
        "commit",
    )?;
    let mut symbols = Vec::new();
    for (index, symbol) in reader::array(
        reader::member(object, &prefix, "symbols")?,
        &format!("{prefix}symbols"),
    )?
    .iter()
    .enumerate()
    {
        symbols.push(reader::text(symbol, &format!("{prefix}symbols.{index}"))?.to_owned());
    }
    Ok(Binding {
        obligation: ObligationId::new(obligation),
        suite: SuiteId::new(suite),
        commit: Commit::new(commit),
        symbols,
    })
}

fn non_empty(prefix: &str, value: &Value, name: &str) -> std::result::Result<String, String> {
    let at = format!("{prefix}{name}");
    let text = reader::text(value, &at)?;
    if text.is_empty() {
        return Err(format!("{at}: expected a non-empty string"));
    }
    Ok(text.to_owned())
}

/// `canonicalizeBindings` (`load.ts:417`), minus one tie-break.
///
/// # The tie-break that is not reproduced, and the test that says it cannot
/// matter
///
/// The retained sort has more levels than the fields kept here; the rest compared
/// `JSON.stringify` of values that no report reads.
///
/// It decides nothing. Binding order is not observable in either report:
/// fan-out accumulates into sets, change-impact takes the first binding per
/// `(obligation, suite)` and reads only `suite` from it. `tc_385_binding_order_is_not_observable.rs` holds that
/// property over the whole golden corpus by permuting every case's bindings,
/// which is a stronger statement than reproducing the tie-break would be.
#[must_use]
fn canonicalize(mut bindings: Vec<Binding>) -> Vec<Binding> {
    for binding in &mut bindings {
        binding
            .symbols
            .sort_by(|left, right| crate::ids::compare_text(left, right));
    }
    bindings.sort_by(|left, right| {
        (&left.obligation, &left.suite, &left.commit).cmp(&(
            &right.obligation,
            &right.suite,
            &right.commit,
        ))
    });
    bindings
}
