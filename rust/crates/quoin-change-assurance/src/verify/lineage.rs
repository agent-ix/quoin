// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Turning a lineage refusal into the one reason a receipt records.
//!
//! The oracle does this by matching three regexes against the thrown message,
//! in order (`verify.ts:44-49`):
//!
//! ```text
//! /missing|no parent/i   -> parent_missing
//! /revision gap/i        -> revision_gap
//! /mismatch|cross-record/i -> parent_mismatch
//! otherwise              -> parent_invalid
//! ```
//!
//! This is that mapping written over the refusal's structure instead of over
//! its prose, one arm per message the oracle can throw. The correspondence is
//! not left to inspection: `tests/tc_455_classification.rs` drives the same
//! inputs through both trees and compares the reason.

use crate::error::{ChangeAssuranceError, LineageFailure};
use crate::model::outcome::Reason;

/// The receipt reason a lineage refusal records.
#[must_use]
pub fn lineage_reason(error: &ChangeAssuranceError) -> Reason {
    match error {
        // A parent that would not verify: the oracle sees that parent's own
        // message, so the same mapping applies one level down.
        ChangeAssuranceError::LineageParent { source, .. } => lineage_reason(source),

        ChangeAssuranceError::Lineage { failure } => match failure {
            // "revision 1 must have a null parent and no parent chain" —
            // caught by `/no parent/i`, not by `/mismatch/i`.
            LineageFailure::GenesisHasAParent => Reason::ParentMissing,
            LineageFailure::RevisionGap => Reason::RevisionGap,
            LineageFailure::CrossRecordParent
            | LineageFailure::ParentDigestMismatch
            | LineageFailure::ImmediateParentMismatch => Reason::ParentMismatch,
        },

        // A parent whose own digest did not verify.
        ChangeAssuranceError::DigestMismatch { .. } => Reason::ParentMismatch,

        // A parent missing a required member. Note that an *undeclared* member
        // is `extra field x`, which matches none of the three regexes and so
        // lands on `parent_invalid` — as does a malformed digest, whose
        // message says "must be exactly 64 lowercase hexadecimal characters".
        ChangeAssuranceError::Shape { failure, .. } if failure.reads_as_absence() => {
            Reason::ParentMissing
        }

        _ => Reason::ParentInvalid,
    }
}
