// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Verification outcomes and the closed reason vocabulary.
//!
//! A faithful port of `Outcome`, `Reason`, `ALL_REASONS`, `INCOMPLETE`,
//! `outcomeForReasons` and `uniqueReasons` from `src/change-assurance/`.
//!
//! The precedence rule (FR-065-AC-2) is one sentence and it is the reason this
//! is a module rather than two `&str` constants: **any reason outside the
//! incomplete set makes the outcome invalid; a non-empty reason set that lies
//! entirely inside it makes the outcome incomplete; only an empty set is
//! valid.** Nothing may express a fourth state, so [`Outcome`] has three
//! members and no `Other`.

use std::fmt;

/// The three verification states.
///
/// `invalid` dominates `incomplete`, which dominates `valid`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Outcome {
    /// Every premise held.
    Valid,
    /// A premise was contradicted.
    Invalid,
    /// A premise was absent or unevaluated, and none was contradicted.
    Incomplete,
}

impl Outcome {
    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Invalid => "invalid",
            Self::Incomplete => "incomplete",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "valid" => Some(Self::Valid),
            "invalid" => Some(Self::Invalid),
            "incomplete" => Some(Self::Incomplete),
            _ => None,
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Declare the closed reason vocabulary once, in the oracle's own order.
macro_rules! reasons {
    ($( $(#[$meta:meta])* $variant:ident => $spelling:literal , $incomplete:literal );* $(;)?) => {
        /// Every premise a verification can name.
        ///
        /// Closed: `verifyReceipt` refuses a receipt carrying a reason outside
        /// this list, so a new member is a schema change and not an addition.
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub enum Reason {
            $( $(#[$meta])* $variant, )*
        }

        impl Reason {
            /// Every reason, in the oracle's declaration order.
            pub const ALL: &'static [Self] = &[ $( Self::$variant, )* ];

            /// The stored spelling.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $( Self::$variant => $spelling, )* }
            }

            /// Read a stored spelling.
            #[must_use]
            pub fn parse(value: &str) -> Option<Self> {
                match value { $( $spelling => Some(Self::$variant), )* _ => None }
            }

            /// Whether this reason describes an absence rather than a
            /// contradiction, and so leaves the outcome `incomplete`.
            #[must_use]
            pub const fn is_incomplete(self) -> bool {
                match self { $( Self::$variant => $incomplete, )* }
            }
        }
    };
}

reasons! {
    /// The record did not satisfy its v1 schema.
    SchemaInvalid => "schema_invalid", false;
    /// The record's stored digest disagreed with its bytes.
    RecordDigestMismatch => "record_digest_mismatch", false;
    /// A required parent was absent from the chain.
    ParentMissing => "parent_missing", true;
    /// A parent was present and unreadable.
    ParentInvalid => "parent_invalid", false;
    /// A parent did not link where it claimed to.
    ParentMismatch => "parent_mismatch", false;
    /// The chain skipped a revision.
    RevisionGap => "revision_gap", false;
    /// The impact snapshot declared itself incomplete.
    ImpactIncomplete => "impact_incomplete", true;
    /// The impact snapshot declared itself truncated.
    ImpactTruncated => "impact_truncated", true;
    /// An unknown was not resolved.
    UnresolvedUnknown => "unresolved_unknown", true;
    /// No review decision event was found.
    DecisionMissing => "decision_missing", true;
    /// No decision history was retained.
    EventChainMissing => "event_chain_missing", true;
    /// The retained decision history did not verify.
    EventChainInvalid => "event_chain_invalid", false;
    /// The decision event did not identify this record and revision.
    DecisionMismatch => "decision_mismatch", false;
    /// The review decision was a rejection.
    ReviewRejected => "review_rejected", false;
    /// The review decision requested a revision.
    ReviewRevisionRequested => "review_revision_requested", false;
    /// A selected attestation was not retained.
    AttestationMissing => "attestation_missing", true;
    /// A retained attestation did not satisfy its v1 schema.
    AttestationSchemaInvalid => "attestation_schema_invalid", false;
    /// A retained attestation's digest did not verify.
    AttestationDigestMismatch => "attestation_digest_mismatch", false;
    /// The attestation's retained output was absent.
    OutputMissing => "output_missing", true;
    /// The retained output did not reproduce its declared digest or size.
    OutputDigestMismatch => "output_digest_mismatch", false;
    /// The attestation was bound to a different record.
    RecordBindingMismatch => "record_binding_mismatch", false;
    /// The attestation was taken at a different candidate revision.
    CandidateRevisionMismatch => "candidate_revision_mismatch", false;
    /// The attestation named a different proof.
    ProofIdMismatch => "proof_id_mismatch", false;
    /// The attestation's command did not match the proof obligation's.
    CommandMismatch => "command_mismatch", false;
    /// The attestation's tool identity did not match the obligation's.
    ToolIdentityMismatch => "tool_identity_mismatch", false;
    /// The attestation's tool configuration did not match the obligation's.
    ConfigurationMismatch => "configuration_mismatch", false;
    /// The producer reported a failure.
    ResultFailed => "result_failed", false;
    /// The producer could not run.
    ResultUnavailable => "result_unavailable", true;
    /// The producer did not compute a result.
    ResultNotComputed => "result_not_computed", true;
    /// The auditor found the evidence stale.
    EvidenceStale => "evidence_stale", false;
    /// The auditor found the evidence link suspect.
    EvidenceSuspect => "evidence_suspect", false;
    /// The auditor found the evidence vacuous.
    EvidenceVacuous => "evidence_vacuous", false;
    /// The auditor found the evidence unrelated to its obligation.
    EvidenceUnrelated => "evidence_unrelated", false;
    /// The auditor reported a finding with no more specific mapping.
    AuditFinding => "audit_finding", false;
    /// The auditor did not evaluate the owning obligation.
    AuditNotEvaluated => "audit_not_evaluated", true;
    /// The change's diff touched the linked plan's protected apparatus.
    ApparatusTouched => "apparatus_touched", false;
    /// The linked plan declared a negative control this verification has no
    /// rule to evaluate.
    NegativeControlUncaught => "negative_control_uncaught", true;
}

impl fmt::Display for Reason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Deduplicate and order a reason list the way `uniqueReasons` does.
///
/// The oracle's `[...new Set(reasons)].sort()` orders by the reasons' own
/// spelling, not by their declaration order — `attestation_missing` precedes
/// `audit_finding` because `t` precedes `u`, not because it was declared
/// first. Every spelling is ASCII, so a byte sort and a UTF-16 sort agree
/// here, and this is the one place in the crate where that is true by
/// inspection rather than by construction.
#[must_use]
pub fn normalize_reasons(reasons: &[Reason]) -> Vec<Reason> {
    let mut unique: Vec<Reason> = Vec::new();
    for reason in reasons {
        if !unique.contains(reason) {
            unique.push(*reason);
        }
    }
    unique.sort_unstable_by_key(|reason| reason.as_str());
    unique
}

/// The outcome a normalized reason list implies.
///
/// Empty is valid; wholly-incomplete is incomplete; anything else is invalid.
#[must_use]
pub fn outcome_for_reasons(reasons: &[Reason]) -> Outcome {
    if reasons.is_empty() {
        Outcome::Valid
    } else if reasons.iter().all(|reason| reason.is_incomplete()) {
        Outcome::Incomplete
    } else {
        Outcome::Invalid
    }
}

/// A named check and the premises it refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Check {
    /// The check's verdict.
    pub outcome: Outcome,
    /// The premises it named, deduplicated and ordered.
    pub reasons: Vec<Reason>,
}

impl Check {
    /// Build a check from its reasons, taking `non_valid` when there is at
    /// least one.
    ///
    /// Mirrors `check()` (`verify.ts:484`), whose default non-valid outcome is
    /// `valid` — meaning a check with reasons but no stated non-valid outcome
    /// cannot arise, because every caller passes one.
    #[must_use]
    pub fn from_reasons(reasons: &[Reason], non_valid: Outcome) -> Self {
        let reasons = normalize_reasons(reasons);
        Self {
            outcome: if reasons.is_empty() {
                Outcome::Valid
            } else {
                non_valid
            },
            reasons,
        }
    }

    /// A check that refused nothing.
    #[must_use]
    pub const fn valid() -> Self {
        Self {
            outcome: Outcome::Valid,
            reasons: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use super::{Outcome, Reason, normalize_reasons, outcome_for_reasons};

    #[test]
    fn the_reason_vocabulary_is_closed_and_round_trips() {
        assert_eq!(Reason::ALL.len(), 37);
        for reason in Reason::ALL {
            assert_eq!(Reason::parse(reason.as_str()), Some(*reason));
        }
        assert_eq!(Reason::parse("not_a_reason"), None);
    }

    #[test]
    fn exactly_twelve_reasons_leave_the_outcome_incomplete() {
        let incomplete: Vec<&str> = Reason::ALL
            .iter()
            .filter(|reason| reason.is_incomplete())
            .map(|reason| reason.as_str())
            .collect();
        assert_eq!(
            incomplete,
            [
                "parent_missing",
                "impact_incomplete",
                "impact_truncated",
                "unresolved_unknown",
                "decision_missing",
                "event_chain_missing",
                "attestation_missing",
                "output_missing",
                "result_unavailable",
                "result_not_computed",
                "audit_not_evaluated",
                "negative_control_uncaught",
            ]
        );
    }

    #[test]
    fn precedence_puts_invalid_above_incomplete_above_valid() {
        assert_eq!(outcome_for_reasons(&[]), Outcome::Valid);
        assert_eq!(
            outcome_for_reasons(&[Reason::ParentMissing]),
            Outcome::Incomplete
        );
        assert_eq!(
            outcome_for_reasons(&[Reason::ParentMissing, Reason::ReviewRejected]),
            Outcome::Invalid
        );
    }

    #[test]
    fn normalization_deduplicates_and_sorts_by_spelling() {
        let normalized = normalize_reasons(&[
            Reason::ReviewRejected,
            Reason::AttestationMissing,
            Reason::ReviewRejected,
        ]);
        assert_eq!(
            normalized
                .iter()
                .map(|reason| reason.as_str())
                .collect::<Vec<_>>(),
            ["attestation_missing", "review_rejected"]
        );
    }
}
