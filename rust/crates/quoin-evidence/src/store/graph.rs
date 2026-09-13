// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The binding graph, the ratchet baseline, and the two operations over them.

use crate::error::EvidenceError;
use crate::ids::{Commit, ObligationId, StatementHash, SuiteId};
use crate::paths::{baseline_path, bindings_path};
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::types::{Affirmation, BaselineFile, Binding, BindingsFile, STORE_SCHEMA_VERSION};

/// The binding graph. An absent file reads as an empty graph, not an error.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`] when the file is present and unparseable. The
/// graph is the store's spine: silently reading an empty one because
/// `bindings.json` carries a merge conflict would report every obligation as
/// undischarged.
pub fn read_bindings<S: EvidenceSource + ?Sized>(
    source: &S,
) -> Result<BindingsFile, EvidenceError> {
    let path = bindings_path();
    match source.read(&path)? {
        None => Ok(BindingsFile {
            schema_version: STORE_SCHEMA_VERSION,
            bindings: Vec::new(),
        }),
        Some(text) => decode(&path, &text),
    }
}

/// Write the binding graph, ordered by `(obligation, suite)`.
///
/// Ordered so the checked-in diff *is* the per-PR delta, and compared with
/// Rust's own string order rather than a locale collation for the same reason
/// the retained source avoids `localeCompare`: two machines with different ICU
/// data would otherwise serialize one binding set in two orders and produce a
/// diff nobody made.
///
/// The caller supplies only the bindings. The version written is always
/// [`STORE_SCHEMA_VERSION`] and the retained `file.schemaVersion` was never
/// read.
///
/// # Errors
///
/// [`EvidenceError::Canonicalization`] or [`EvidenceError::StoreIo`].
pub fn write_bindings<S: EvidenceSource + ?Sized>(
    source: &mut S,
    bindings: &[Binding],
) -> Result<(), EvidenceError> {
    let mut sorted = bindings.to_vec();
    sorted.sort_by(|a, b| {
        a.obligation
            .as_str()
            .cmp(b.obligation.as_str())
            .then_with(|| a.suite.as_str().cmp(b.suite.as_str()))
    });
    let file = BindingsFile {
        schema_version: STORE_SCHEMA_VERSION,
        bindings: sorted,
    };
    let bytes = canonical_bytes_of("binding graph", &file)?;
    source.write(&bindings_path(), &bytes)
}

/// The ratchet baseline, or `None` when none has been accepted.
///
/// # Errors
///
/// As [`read_bindings`].
pub fn read_baseline<S: EvidenceSource + ?Sized>(
    source: &S,
) -> Result<Option<BaselineFile>, EvidenceError> {
    let path = baseline_path();
    match source.read(&path)? {
        None => Ok(None),
        Some(text) => decode(&path, &text).map(Some),
    }
}

/// Write the ratchet baseline with its accepted set sorted.
///
/// # Errors
///
/// [`EvidenceError::Canonicalization`] or [`EvidenceError::StoreIo`].
pub fn write_baseline<S: EvidenceSource + ?Sized>(
    source: &mut S,
    commit: &Commit,
    accepted: &[String],
) -> Result<(), EvidenceError> {
    let mut sorted = accepted.to_vec();
    sorted.sort();
    let file = BaselineFile {
        schema_version: STORE_SCHEMA_VERSION,
        commit: commit.clone(),
        accepted: sorted,
    };
    let bytes = canonical_bytes_of("ratchet baseline", &file)?;
    source.write(&baseline_path(), &bytes)
}

/// What [`bind`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct BindOutcome {
    /// The graph after the binding.
    pub bindings: Vec<Binding>,
    /// Whether this `(obligation, suite)` had no binding before.
    pub created: bool,
    /// Whether the stamped hash disagrees with the one just supplied.
    pub suspect: bool,
}

/// Bind an obligation to the run that discharged it.
///
/// **Keyed on `(obligation, suite)`, not on the obligation alone.** The graph
/// is cross-suite by design — one obligation discharged by a unit suite and a
/// mutation suite is the case `bindings.json` exists to hold in one file — and
/// keying on the obligation made the second suite overwrite the first. Two
/// silent consequences: the cross-suite relationship was destroyed on write,
/// and `insufficient-multiplicity` counted distinct suites across a set that
/// could never hold more than one, so the finding could not be cleared by any
/// amount of evidence (agent-ix/quoin#102).
///
/// **Auto-bind, explicit affirmation.** First discharge binds without asking;
/// what stays explicit is re-affirmation after a statement changes.
#[must_use]
pub fn bind(existing: &[Binding], next: &Binding) -> BindOutcome {
    let found = existing
        .iter()
        .position(|b| b.obligation == next.obligation && b.suite == next.suite);
    let mut bindings = existing.to_vec();
    let Some(index) = found else {
        let mut created = next.clone();
        created.affirmations = None;
        bindings.push(created);
        return BindOutcome {
            bindings,
            created: true,
            suspect: false,
        };
    };
    let Some(prior) = bindings.get_mut(index) else {
        unreachable!("the index came from this same slice")
    };
    // The hash is NOT overwritten on re-discharge. Re-running a test does not
    // re-affirm a reworded requirement — if it did, the suspect state would
    // clear itself on the next CI run and the detector would never fire.
    let suspect = prior.statement_hash_at_binding != next.statement_hash_at_binding;
    prior.commit = next.commit.clone();
    prior.symbols.clone_from(&next.symbols);
    // Lineage describes the CURRENT relationship. Omission on a new record
    // clears it rather than silently carrying an old independence claim into
    // evidence that did not state one.
    prior.lineage.clone_from(&next.lineage);
    BindOutcome {
        bindings,
        created: false,
        suspect,
    }
}

/// What [`affirm`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct AffirmOutcome {
    /// The graph after the affirmation.
    pub bindings: Vec<Binding>,
    /// Whether any binding matched.
    pub found: bool,
}

/// Record a re-affirmation, clearing suspicion.
///
/// Affirms **every** binding for the obligation, not the first one found. The
/// judgement being recorded is "I have read the new statement and the evidence
/// still discharges it", and that is a judgement about the requirement rather
/// than about one suite — affirming only the first would leave a sibling suite
/// suspect for a reason nobody could act on. `suite` narrows it when the
/// reviewer means only one.
#[must_use]
pub fn affirm(
    existing: &[Binding],
    obligation: &ObligationId,
    suite: Option<&SuiteId>,
    current_hash: &StatementHash,
    affirmation: &Affirmation,
) -> AffirmOutcome {
    let mut bindings = existing.to_vec();
    let mut found = false;
    for binding in &mut bindings {
        if &binding.obligation != obligation {
            continue;
        }
        if suite.is_some_and(|wanted| &binding.suite != wanted) {
            continue;
        }
        found = true;
        // Affirming moves the hash forward: the reviewer has read the new
        // statement and says the evidence still discharges it.
        binding.statement_hash_at_binding = current_hash.clone();
        binding
            .affirmations
            .get_or_insert_with(Vec::new)
            .push(affirmation.clone());
    }
    if found {
        AffirmOutcome {
            bindings,
            found: true,
        }
    } else {
        AffirmOutcome {
            bindings: existing.to_vec(),
            found: false,
        }
    }
}
