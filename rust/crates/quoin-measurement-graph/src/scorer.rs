// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Whether the retained scorer attachment is still the bytes the producer
//! declared.
//!
//! Ports `scorerIntegrityFailure` (`graph-portfolio.ts:795-836`).
//!
//! # What is being checked
//!
//! A graph-quality collection retains its scorer output *inline*: the producer
//! record declares `raw_scorer_output.digest`, and the collection's
//! `rawEvidence.scorer` carries both that digest again and the bytes, base64.
//! Three things can be wrong, and they are not the same kind of wrong:
//!
//! * something is **not there** to check — `unknown`, because the evidence is
//!   silent rather than contradicted;
//! * the producer and the attachment **declare different digests** —
//!   `unreadable`, because two statements about one artifact disagree;
//! * the retained bytes **do not hash to the declared digest** — `unreadable`,
//!   because the artifact is not the one that was measured.
//!
//! The hash is [`quoin_store::digest_bytes_sha256`]; there is no sha256 in this
//! crate and `tests/tc_476_boundary.rs` asserts there is not. The decode is
//! [`crate::base64::decode`], the crate's one decoder — quoin#475 landed it
//! for the encode side of the same attachment, and the retained *decode* call
//! site is this one. Its strictness is the quoin#465 divergence that module
//! declares; this call site widens what that entry covers, and
//! `DIVERGENCE.md` says so rather than declaring a second.

use quoin_store::{JsonObject, digest_bytes_sha256};

use crate::availability::HistoryAvailability;
use crate::base64;
use crate::fields::{record, string_field};

/// Why the scorer attachment could not be accepted.
/// `graph-portfolio.ts:797`.
///
/// The retained return type is `{ availability: "unreadable" | "unknown";
/// reason: string } | null`, and the two availabilities mean different things,
/// so they are variants rather than a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScorerIntegrityFailure {
    /// Something needed to check is absent.
    Unknown {
        /// The sentence to report.
        reason: String,
    },
    /// What is there contradicts itself.
    Unreadable {
        /// The sentence to report.
        reason: String,
    },
}

impl ScorerIntegrityFailure {
    /// The sentence `graph-portfolio.ts:809-810` states when a digest or the
    /// retained bytes are missing.
    pub const NOTHING_TO_CHECK: &'static str =
        "raw scorer digest or retained bytes are unavailable";

    /// How this failure is reported on a history row.
    #[must_use]
    pub const fn availability(&self) -> HistoryAvailability {
        match *self {
            Self::Unknown { .. } => HistoryAvailability::Unknown,
            Self::Unreadable { .. } => HistoryAvailability::Unreadable,
        }
    }

    /// The sentence to report.
    #[must_use]
    pub fn reason(&self) -> &str {
        match *self {
            Self::Unknown { ref reason } | Self::Unreadable { ref reason } => reason,
        }
    }
}

/// Check the retained scorer attachment against what the producer declared.
///
/// `scorerIntegrityFailure` (`graph-portfolio.ts:795-836`): [`None`] is the
/// retained `null`, which is the only accepting answer.
#[must_use]
pub fn integrity_failure(
    producer_record: Option<&JsonObject>,
    scorer: Option<&JsonObject>,
) -> Option<ScorerIntegrityFailure> {
    let producer_digest = string_field(
        record(producer_record.and_then(|held| held.get("raw_scorer_output"))),
        "digest",
    );
    let scorer_digest = string_field(scorer, "digest");
    let bytes_base64 = string_field(scorer, "bytesBase64");
    let (Some(producer_digest), Some(scorer_digest), Some(bytes_base64)) =
        (producer_digest, scorer_digest, bytes_base64)
    else {
        return Some(ScorerIntegrityFailure::Unknown {
            reason: ScorerIntegrityFailure::NOTHING_TO_CHECK.to_owned(),
        });
    };
    if producer_digest != scorer_digest {
        return Some(ScorerIntegrityFailure::Unreadable {
            reason: format!(
                "producer scorer digest {producer_digest} does not match retained {scorer_digest}"
            ),
        });
    }
    // A refusal here is `unreadable` for the same reason a digest mismatch is:
    // the retained bytes are not the artifact that was measured. The retained
    // decoder would have invented a decoding and then reported the mismatch;
    // this reports what is actually wrong. Declared, quoin#465.
    let bytes = match base64::decode(&bytes_base64) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Some(ScorerIntegrityFailure::Unreadable {
                reason: format!("retained scorer bytes are not base64: {}", error.message()),
            });
        }
    };
    let observed = digest_bytes_sha256(&bytes).to_stored();
    if observed == scorer_digest {
        None
    } else {
        Some(ScorerIntegrityFailure::Unreadable {
            reason: format!("retained scorer bytes hash to {observed}, expected {scorer_digest}"),
        })
    }
}
