// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `ChangeAssuranceError`, one exit status.
//!
//! The single place a `quoin-change-assurance` failure becomes a number on the
//! boundary's exit taxonomy. Split from the operations in [`super`] because it
//! is a different job: the operations decide *what to do*, this decides *how a
//! failure is reported*, and keeping the mapping alone in a file is what lets
//! it be asserted against the error catalogue itself rather than against
//! whichever variants a test double happens to be able to construct.
//!
//! A **verification verdict is not an error here and never reaches this file.**
//! An `invalid` or `incomplete` receipt is a successful answer: the evidence
//! was read and it came to that. Only a failure to READ an input, or a store
//! that would not take one, arrives here.

use quoin_change_assurance::{ChangeAssuranceError, ChangeAssuranceErrorCode};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-change-assurance` failure onto the boundary's exit taxonomy.
///
/// The mapping is deliberate and total, and it is the ONE place a
/// `ChangeAssuranceError` becomes an exit status. Two groups:
///
/// - **`BadRequest` → Invalid (3)** — the document the caller supplied does
///   not satisfy the contract it claims: a record, attestation, receipt or
///   decision history that fails its v1 schema, a malformed digest, a sealed
///   value whose digest disagrees with its own bytes, or a parent chain that
///   is not strict N-1. Every one of these is fixed by changing what was sent.
/// - **`Refused` → Refused (2)** — the request was understood and the retained
///   world declined it: output bytes that do not reproduce what the
///   attestation declares, a digest-named slot that already holds different
///   content, evidence filed under a digest it does not carry, a filesystem
///   operation that failed, and a `quoin-store` refusal passed through.
///   Fixing one means changing the store or the bytes on disk, not the
///   request.
/// - There is deliberately **no Internal group**. Nothing in this domain is
///   quoin's own shipped data: every input is the caller's and every store is
///   the caller's repository, so an unexpected failure is still a statement
///   about their world rather than about this build. The `_` arm below is the
///   sole exception and it says so.
///
/// `ChangeAssuranceErrorCode` is `#[non_exhaustive]`, so this match needs a `_`
/// arm and the compiler will NOT flag a code added upstream.
/// `the_error_mapping_covers_every_change_assurance_code` pins
/// `ChangeAssuranceErrorCode::ALL.len()`, which a loop over `ALL` could not do
/// — such a loop re-runs this same match and agrees with itself (the quoin#443
/// failure mode).
pub(super) fn map_error(error: &ChangeAssuranceError, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = core_code(code);
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("change_assurance_code", code.as_str());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one `ChangeAssuranceErrorCode` maps to, or `None` for
/// a code this build does not know.
///
/// `Option` rather than a total function, because `ChangeAssuranceErrorCode` is
/// `#[non_exhaustive]`: "unknown upstream code" and "deliberately Internal" are
/// different facts, and collapsing them would make the wildcard arm
/// indistinguishable from a considered decision — to a reader and to the
/// linter both.
const fn core_code(code: ChangeAssuranceErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        ChangeAssuranceErrorCode::RecordInvalid
        | ChangeAssuranceErrorCode::AttestationInvalid
        | ChangeAssuranceErrorCode::ReceiptInvalid
        | ChangeAssuranceErrorCode::DecisionHistoryInvalid
        | ChangeAssuranceErrorCode::DigestMalformed
        | ChangeAssuranceErrorCode::DigestMismatch
        | ChangeAssuranceErrorCode::LineageInvalid => CoreErrorCode::BadRequest,

        ChangeAssuranceErrorCode::OutputIntegrityMismatch
        | ChangeAssuranceErrorCode::ContentCollision
        | ChangeAssuranceErrorCode::PathDigestMismatch
        | ChangeAssuranceErrorCode::Io
        | ChangeAssuranceErrorCode::Store => CoreErrorCode::Refused,

        // Required by `#[non_exhaustive]`. A code this build has never seen is
        // reported as unmapped and exits Internal — the safest answer, since an
        // unknown rule is not one the caller can act on. The count pin in
        // `the_error_mapping_covers_every_change_assurance_code` is what makes
        // reaching this arm a test failure rather than a silent widening.
        _ => return None,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use quoin_change_assurance::{ChangeAssuranceError, ChangeAssuranceErrorCode, Subject};

    use super::{CoreErrorCode, core_code, map_error};

    /// `ChangeAssuranceErrorCode` is `#[non_exhaustive]`, so `map_error`'s `_`
    /// arm means the compiler cannot catch a code added upstream. This pins the
    /// COUNT instead. A loop over `ALL` would only re-run the same match and
    /// agree with itself — the quoin#443 failure mode — so the number is
    /// written out here, and every group is listed by name.
    ///
    /// When this fails: a code was added to `quoin-change-assurance`. Decide
    /// which of the two groups it belongs to, add it to that arm, and raise the
    /// count.
    #[test]
    fn the_error_mapping_covers_every_change_assurance_code() {
        assert_eq!(
            ChangeAssuranceErrorCode::ALL.len(),
            12,
            "quoin-change-assurance gained or lost an error code; map it in \
             map_error deliberately"
        );

        let invalid = [
            ChangeAssuranceErrorCode::RecordInvalid,
            ChangeAssuranceErrorCode::AttestationInvalid,
            ChangeAssuranceErrorCode::ReceiptInvalid,
            ChangeAssuranceErrorCode::DecisionHistoryInvalid,
            ChangeAssuranceErrorCode::DigestMalformed,
            ChangeAssuranceErrorCode::DigestMismatch,
            ChangeAssuranceErrorCode::LineageInvalid,
        ];
        let refused = [
            ChangeAssuranceErrorCode::OutputIntegrityMismatch,
            ChangeAssuranceErrorCode::ContentCollision,
            ChangeAssuranceErrorCode::PathDigestMismatch,
            ChangeAssuranceErrorCode::Io,
            ChangeAssuranceErrorCode::Store,
        ];
        assert_eq!(
            invalid.len() + refused.len(),
            ChangeAssuranceErrorCode::ALL.len(),
            "a code is in `ALL` but in neither of the two groups"
        );

        for code in invalid {
            assert_eq!(core_code(code), Some(CoreErrorCode::BadRequest), "{code}");
        }
        for code in refused {
            assert_eq!(core_code(code), Some(CoreErrorCode::Refused), "{code}");
        }
    }

    /// And the envelope carries the domain code through, so an operator can
    /// tell which rule declined without parsing the sentence.
    #[test]
    fn the_envelope_names_the_change_assurance_code_that_declined() {
        let envelope = map_error(
            &ChangeAssuranceError::DigestMismatch {
                subject: Subject::Record,
                stored: "a".repeat(64),
                recomputed: "b".repeat(64),
            },
            "change_assurance.seal_record",
        );
        assert_eq!(
            envelope.context["change_assurance_code"],
            "QCA-DIGEST-MISMATCH"
        );
        assert_eq!(envelope.code, CoreErrorCode::BadRequest);
        assert_eq!(envelope.outcome().code(), 3);
        assert_eq!(envelope.context["op"], "change_assurance.seal_record");
    }
}
