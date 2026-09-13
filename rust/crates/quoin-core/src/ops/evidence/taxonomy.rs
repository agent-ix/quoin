// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `EvidenceError`, one exit status.
//!
//! The single place a `quoin-evidence` failure becomes a number on the
//! boundary's exit taxonomy. Split from the operations in [`super`] for the
//! same reason `ops::semantic::taxonomy` is: the operations decide *what to
//! do*, this decides *how a failure is reported*, and keeping the mapping alone
//! in a file is what lets it be asserted against the error catalogue itself
//! rather than against whichever variants a test double happens to construct.

use quoin_evidence::{EvidenceError, EvidenceErrorCode};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-evidence` failure onto the boundary's exit taxonomy.
///
/// The mapping is deliberate and total, and it is the ONE place an
/// `EvidenceError` becomes an exit status. Three groups:
///
/// - **`BadRequest` (3)** — the caller's own document is wrong, and the fix is
///   in what they sent. An adapter that cannot read the producer file it was
///   handed, an `--adapter` naming nothing, an id that is not `ETD-<digits>`, a
///   record that fails its schema, a policy naming an obligation nothing
///   derives, and a value with no canonical spelling are all this: quoin read
///   the request correctly and the request is malformed.
/// - **`Refused` (2)** — the request was understood and the WORLD declined it:
///   a checked-in store file left holding a merge conflict, or a path that
///   cannot be read, written, listed or removed. Fixing it means changing the
///   repository, not the request.
/// - **`Refused` (2) for `RecordIntegrity`** — a content-addressed record whose
///   stored identity disagrees with its content. The store on disk is wrong,
///   not the request; publishing is append-only and refuses rather than
///   overwrites.
///
/// There is deliberately **no `Internal` group**. Nothing `quoin-evidence`
/// reports is quoin's own shipped data being broken — that is what
/// `ops::semantic`'s `Io` group is for, and this domain ships no data.
///
/// `EvidenceErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm
/// and the compiler will NOT flag a code added upstream.
/// `the_error_mapping_covers_every_evidence_code` pins
/// `EvidenceErrorCode::ALL.len()`, which a loop over `ALL` could not do — such
/// a loop re-runs this same match and agrees with itself (the quoin#443
/// failure mode).
pub(super) fn map_error(error: &EvidenceError, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = core_code(code);
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("evidence_code", code.as_str());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one `EvidenceErrorCode` maps to, or `None` for a code
/// this build does not know.
const fn core_code(code: EvidenceErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        EvidenceErrorCode::AdapterInput
        | EvidenceErrorCode::AdapterUnknown
        | EvidenceErrorCode::InvalidIdentity
        | EvidenceErrorCode::InvalidRecord
        | EvidenceErrorCode::PolicyUnknownObligation
        | EvidenceErrorCode::Canonicalization => CoreErrorCode::BadRequest,

        EvidenceErrorCode::StoreRead
        | EvidenceErrorCode::StoreIo
        | EvidenceErrorCode::RecordIntegrity => CoreErrorCode::Refused,

        // Required by `#[non_exhaustive]`. A code this build has never seen is
        // reported as unmapped and exits Internal — the safest answer, since an
        // unknown rule is not one the caller can act on. The count pin in
        // `the_error_mapping_covers_every_evidence_code` is what makes reaching
        // this arm a test failure rather than a silent widening.
        _ => return None,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use quoin_evidence::{EvidenceError, EvidenceErrorCode};

    use super::{CoreErrorCode, core_code, map_error};

    /// `EvidenceErrorCode` is `#[non_exhaustive]`, so `map_error`'s `_` arm
    /// means the compiler cannot catch a code added upstream. This pins the
    /// COUNT instead, and lists every group by name.
    ///
    /// When this fails: a code was added to `quoin-evidence`. Decide which
    /// group it belongs to, add it to that arm, and raise the count.
    ///
    /// Trace: FR-101-AC-3
    #[test]
    fn the_error_mapping_covers_every_evidence_code() {
        assert_eq!(
            EvidenceErrorCode::ALL.len(),
            9,
            "quoin-evidence gained or lost an error code; map it in map_error deliberately"
        );

        let bad_request = [
            EvidenceErrorCode::AdapterInput,
            EvidenceErrorCode::AdapterUnknown,
            EvidenceErrorCode::InvalidIdentity,
            EvidenceErrorCode::InvalidRecord,
            EvidenceErrorCode::PolicyUnknownObligation,
            EvidenceErrorCode::Canonicalization,
        ];
        let refused = [
            EvidenceErrorCode::StoreRead,
            EvidenceErrorCode::StoreIo,
            EvidenceErrorCode::RecordIntegrity,
        ];
        assert_eq!(
            bad_request.len() + refused.len(),
            EvidenceErrorCode::ALL.len(),
            "a code is in `ALL` but in neither of the two groups"
        );

        for code in bad_request {
            assert_eq!(core_code(code), Some(CoreErrorCode::BadRequest), "{code}");
        }
        for code in refused {
            assert_eq!(core_code(code), Some(CoreErrorCode::Refused), "{code}");
        }

        // And the envelope carries the evidence code through, so an operator
        // can tell which rule declined without parsing the sentence.
        let envelope = map_error(
            &EvidenceError::UnknownAdapter {
                name: "junti".to_owned(),
                available: "entries, junit".to_owned(),
            },
            "evidence.record",
        );
        assert_eq!(envelope.context["evidence_code"], "QE-E002");
        assert_eq!(envelope.code, CoreErrorCode::BadRequest);
        assert_eq!(envelope.outcome().code(), 3);
    }
}
