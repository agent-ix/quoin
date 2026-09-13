// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Why an intervention record was refused.
//!
//! A port of `intervention-types.ts:104-122`. The refusal set is the API: a
//! caller that learned `raw_evidence_mismatch` keeps it forever, which is the
//! same contract `quoin_core::error::CoreErrorCode` carries and the reason the
//! spellings are generated from one table rather than written at each use.
//!
//! Intake itself — `readInterventionRecords`, `recordIntervention` and the
//! checks that raise these — is `intervention.ts`, and lands with quoin#471.
//! This module is the refusal vocabulary those checks will raise into.

use crate::common::wire_enum::wire_enum;

wire_enum! {
    /// The reason intake refused a record. `intervention-types.ts:104-110`.
    pub enum InterventionRefusalCode {
        /// The record did not validate against the intervention schema.
        InvalidRecord => "invalid_record",
        /// A referenced raw evidence file does not match its recorded digest.
        RawEvidenceMismatch => "raw_evidence_mismatch",
        /// No governing measurement plan admits this record.
        GoverningPlanAbsent => "governing_plan_absent",
        /// The record does not match the producer definition it names.
        DefinitionMismatch => "definition_mismatch",
        /// Another intake holds the store.
        IntakeBusy => "intake_busy",
        /// A record already exists under this identity.
        RecordIdCollision => "record_id_collision",
    }
}

/// An intake refusal, with the findings that justify it.
///
/// `intervention-types.ts:112-122`. The rendered message is the TypeScript
/// one — `` `${code}: ${findings.join("; ")}` `` — because it is what a caller
/// reading stderr today already sees.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {}", .code.as_str(), .findings.join("; "))]
pub struct InterventionIntakeError {
    /// The refusal reason. Codes are contractual; messages are not.
    code: InterventionRefusalCode,
    /// What was found, in the order it was found.
    findings: Vec<String>,
}

impl InterventionIntakeError {
    /// Builds a refusal.
    #[must_use]
    pub fn new(code: InterventionRefusalCode, findings: Vec<String>) -> Self {
        Self { code, findings }
    }

    /// The refusal reason.
    #[must_use]
    pub fn code(&self) -> InterventionRefusalCode {
        self.code
    }

    /// What was found.
    #[must_use]
    pub fn findings(&self) -> &[String] {
        &self.findings
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{InterventionIntakeError, InterventionRefusalCode};

    /// The literal the TypeScript constructor writes, not a re-derivation of it.
    #[test]
    fn the_message_is_the_typescript_message() {
        let error = InterventionIntakeError::new(
            InterventionRefusalCode::RawEvidenceMismatch,
            vec![
                "a/one.json differs".to_owned(),
                "b/two.json absent".to_owned(),
            ],
        );
        assert_eq!(
            error.to_string(),
            "raw_evidence_mismatch: a/one.json differs; b/two.json absent"
        );
    }

    #[test]
    fn a_refusal_with_no_findings_still_names_its_code() {
        let error = InterventionIntakeError::new(InterventionRefusalCode::IntakeBusy, Vec::new());
        assert_eq!(error.to_string(), "intake_busy: ");
    }
}
