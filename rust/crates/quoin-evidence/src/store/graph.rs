// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The binding graph and the two operations over it.

use crate::error::EvidenceError;
use crate::paths::bindings_path;
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::types::{Binding, BindingsFile, STORE_SCHEMA_VERSION};

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

/// What [`bind`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct BindOutcome {
    /// The graph after the binding.
    pub bindings: Vec<Binding>,
    /// Whether this `(obligation, suite)` had no binding before.
    pub created: bool,
}

/// Bind an obligation to the suite that discharged it.
///
/// **Keyed on `(obligation, suite)`, not on the obligation alone.** The graph
/// is cross-suite by design — one obligation discharged by a unit suite and a
/// mutation suite is the case `bindings.json` exists to hold in one file — and
/// keying on the obligation made the second suite overwrite the first. Two
/// silent consequences: the cross-suite relationship was destroyed on write,
/// and `insufficient-multiplicity` counted distinct suites across a set that
/// could never hold more than one, so the finding could not be cleared by any
/// amount of evidence (agent-ix/quoin#102).
#[must_use]
pub fn bind(existing: &[Binding], next: &Binding) -> BindOutcome {
    let found = existing
        .iter()
        .position(|b| b.obligation == next.obligation && b.suite == next.suite);
    let mut bindings = existing.to_vec();
    let Some(index) = found else {
        bindings.push(next.clone());
        return BindOutcome {
            bindings,
            created: true,
        };
    };
    let Some(prior) = bindings.get_mut(index) else {
        unreachable!("the index came from this same slice")
    };
    prior.symbols.clone_from(&next.symbols);
    // Lineage describes the CURRENT relationship. Omission on a new record
    // clears it rather than silently carrying an old independence claim into
    // evidence that did not state one.
    prior.lineage.clone_from(&next.lineage);
    BindOutcome {
        bindings,
        created: false,
    }
}
