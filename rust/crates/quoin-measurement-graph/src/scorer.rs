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
//! record declares `raw_scorer_output.byte_length`, and the collection's
//! `rawEvidence.scorer` carries that length again and the bytes, base64.
//! Three things can be wrong, and they are not the same kind of wrong:
//!
//! * something is **not there** to check — `unknown`, because the evidence is
//!   silent rather than contradicted;
//! * the producer and the attachment **declare different lengths** —
//!   `unreadable`, because two statements about one artifact disagree;
//! * the retained bytes **are not that many bytes** — `unreadable`, because
//!   the artifact is not the one that was measured.
//!
//! No digest is computed. The decode is [`crate::base64::decode`], the crate's
//! one decoder.

use quoin_store::JsonObject;

use crate::availability::HistoryAvailability;
use crate::base64;
use crate::fields::{member, record, string_field};

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
    /// The sentence stated when a declared length or the retained bytes are
    /// missing.
    pub const NOTHING_TO_CHECK: &'static str =
        "raw scorer byte length or retained bytes are unavailable";

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
/// [`None`] is the only accepting answer.
#[must_use]
pub fn integrity_failure(
    producer_record: Option<&JsonObject>,
    scorer: Option<&JsonObject>,
) -> Option<ScorerIntegrityFailure> {
    let length_of = |holder: Option<&JsonObject>| {
        member(holder, "byteLength")
            .or_else(|| member(holder, "byte_length"))
            .and_then(|value| match value {
                quoin_store::JsonValue::Number(number) => number.to_string().parse::<u64>().ok(),
                _ => None,
            })
    };
    let producer_length = length_of(record(
        producer_record.and_then(|held| held.get("raw_scorer_output")),
    ));
    let scorer_length = length_of(scorer);
    let bytes_base64 = string_field(scorer, "bytesBase64");
    let (Some(producer_length), Some(scorer_length), Some(bytes_base64)) =
        (producer_length, scorer_length, bytes_base64)
    else {
        return Some(ScorerIntegrityFailure::Unknown {
            reason: ScorerIntegrityFailure::NOTHING_TO_CHECK.to_owned(),
        });
    };
    if producer_length != scorer_length {
        return Some(ScorerIntegrityFailure::Unreadable {
            reason: format!(
                "producer scorer length {producer_length} does not match retained {scorer_length}"
            ),
        });
    }
    // A refusal here is `unreadable`: the retained bytes are not the artifact
    // that was measured.
    let bytes = match base64::decode(&bytes_base64) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Some(ScorerIntegrityFailure::Unreadable {
                reason: format!("retained scorer bytes are not base64: {}", error.message()),
            });
        }
    };
    let observed = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if observed == scorer_length {
        None
    } else {
        Some(ScorerIntegrityFailure::Unreadable {
            reason: format!("retained scorer bytes are {observed} bytes, expected {scorer_length}"),
        })
    }
}
