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
//! back out: no binding reaches any of the three reports, only values derived
//! from the six fields below. Carrying the rest would be carrying it nowhere.
//! That is the opposite call from `quoin-finding-types`, and the difference is
//! whether the producer's bytes are re-emitted.

use serde_json::Value;

use crate::ids::{Author, Commit, ObligationId, StatementHash, SuiteId};
use crate::json as reader;

/// One recorded re-affirmation of a binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affirmation {
    /// Who affirmed it.
    pub who: Author,
    /// At which commit.
    pub commit: Commit,
    /// Their note, when they left one.
    pub note: Option<String>,
}

/// One obligation bound to one suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The obligation this binding is for.
    pub obligation: ObligationId,
    /// The statement hash the binding was made against.
    pub statement_hash_at_binding: StatementHash,
    /// The suite that carries it.
    pub suite: SuiteId,
    /// The commit it was bound at.
    pub commit: Commit,
    /// The symbols it names.
    pub symbols: Vec<String>,
    /// The re-affirmation history, when the store recorded one.
    ///
    /// `Option` and not an empty `Vec`: `churn` distinguishes "no affirmations
    /// recorded" from "an affirmations array that is empty" only in that both
    /// contribute no events, but `canonicalizeBindings` emits the member only
    /// when it was present (`load.ts:422`) and this type says which it was.
    pub affirmations: Option<Vec<Affirmation>>,
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
    let statement_hash = non_empty(
        &prefix,
        reader::member(object, &prefix, "statementHashAtBinding")?,
        "statementHashAtBinding",
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
    let affirmations = match object.get("affirmations") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let mut entries = Vec::new();
            for (index, entry) in reader::array(value, &format!("{prefix}affirmations"))?
                .iter()
                .enumerate()
            {
                entries.push(parse_affirmation(
                    entry,
                    &format!("{prefix}affirmations.{index}"),
                )?);
            }
            Some(entries)
        }
    };
    Ok(Binding {
        obligation: ObligationId::new(obligation),
        statement_hash_at_binding: StatementHash::new(statement_hash),
        suite: SuiteId::new(suite),
        commit: Commit::new(commit),
        symbols,
        affirmations,
    })
}

fn parse_affirmation(value: &Value, at: &str) -> std::result::Result<Affirmation, String> {
    let object = reader::object(value, at)?;
    let prefix = format!("{at}.");
    let who = reader::text(
        reader::member(object, &prefix, "who")?,
        &format!("{prefix}who"),
    )?;
    let commit = reader::text(
        reader::member(object, &prefix, "commit")?,
        &format!("{prefix}commit"),
    )?;
    let note = match object.get("note") {
        None | Some(Value::Null) => None,
        Some(value) => Some(reader::text(value, &format!("{prefix}note"))?.to_owned()),
    };
    Ok(Affirmation {
        who: Author::new(who),
        commit: Commit::new(commit),
        note,
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
/// The retained sort has six levels. Four are plain field comparisons and are
/// here. The last two compare `JSON.stringify(symbols)` and
/// `JSON.stringify(affirmations ?? [])` — and `JSON.stringify` on an object
/// emits members in **JavaScript insertion order**, which for a
/// `.passthrough()` schema is the order the bytes on disk happened to use. A
/// Rust type that reproduced that would have to carry the undeclared members
/// and their file order, to decide a tie between two bindings.
///
/// It decides nothing. Binding order is not observable in any of the three
/// reports: fan-out accumulates into sets, churn keys events by
/// `(obligation, who, commit, note)` and sorts the suites it unions,
/// change-impact takes the first binding per `(obligation, suite)` and reads
/// only `suite` from it. `tc_385_binding_order_is_not_observable.rs` holds that
/// property over the whole golden corpus by permuting every case's bindings,
/// which is a stronger statement than reproducing the tie-break would be.
#[must_use]
fn canonicalize(mut bindings: Vec<Binding>) -> Vec<Binding> {
    for binding in &mut bindings {
        binding
            .symbols
            .sort_by(|left, right| quoin_store::json::order::cmp_utf16(left, right));
        if let Some(affirmations) = binding.affirmations.as_mut() {
            affirmations.sort_by(|left, right| {
                (&left.who, &left.commit)
                    .cmp(&(&right.who, &right.commit))
                    .then_with(|| {
                        quoin_store::json::order::cmp_utf16(
                            left.note.as_deref().unwrap_or(""),
                            right.note.as_deref().unwrap_or(""),
                        )
                    })
            });
        }
    }
    bindings.sort_by(|left, right| {
        (
            &left.obligation,
            &left.suite,
            &left.commit,
            &left.statement_hash_at_binding,
        )
            .cmp(&(
                &right.obligation,
                &right.suite,
                &right.commit,
                &right.statement_hash_at_binding,
            ))
    });
    bindings
}
