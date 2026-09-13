// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The three input contracts, as the loader and any other caller see them
//! (`input.ts`).
//!
//! Each function here is one retained entry point with its refusal attached to
//! the input it came from, so a caller can say *which* of the three documents
//! was wrong without matching on prose. The contracts themselves live with the
//! types they produce — [`crate::model::premises`], [`crate::model::audit`] —
//! because a contract and the type it admits are one thing.
//!
//! # The diagnostic text is not the contract
//!
//! The retained refusals are zod's prose, one line per failing path. These
//! refusals are this crate's own sentences naming the same member. The
//! *verdict* — accepted or refused, and which input — is asserted against the
//! oracle; the wording is not, for the reason quoin#403 states and
//! `quoin-jsonschema`'s parity test already relies on.

use quoin_quire::model::AssuranceExport;

use crate::error::{GraphError, GraphInput, Result};
use crate::json as reader;
use crate::model::audit::{self, AuditEnvelope};
use crate::model::premises::AcceptedPremises;

/// Read an assurance export (`src/quire/validate.ts:136`, via `load.ts:194`).
///
/// The document is checked against the vendored `assurance-v1` contract by
/// [`quoin_quire::validate_assurance`] — the owner of that schema — and only
/// then typed.
///
/// # Errors
///
/// [`GraphError::InputInvalid`] for [`GraphInput::Export`].
pub fn parse_assurance_export(text: &str) -> Result<AssuranceExport> {
    let document =
        reader::document(text).map_err(|reason| GraphError::invalid(GraphInput::Export, reason))?;
    let valid = quoin_quire::validate_assurance(document).map_err(|error| match error {
        quoin_quire::Error::AssuranceContract { violations } => GraphError::InputInvalid {
            input: GraphInput::Export,
            violations,
        },
        other => GraphError::invalid(GraphInput::Export, other.to_string()),
    })?;
    serde_json::from_value(valid.into_value())
        .map_err(|error| GraphError::invalid(GraphInput::Export, error.to_string()))
}

/// Read the accepted premises (`input.ts:102`).
///
/// # Errors
///
/// [`GraphError::InputInvalid`] for [`GraphInput::Premises`].
pub fn parse_accepted_premises(text: &str) -> Result<AcceptedPremises> {
    let document = reader::document(text)
        .map_err(|reason| GraphError::invalid(GraphInput::Premises, reason))?;
    AcceptedPremises::parse(&document)
        .map_err(|reason| GraphError::invalid(GraphInput::Premises, reason))
}

/// Read the audit envelope (`input.ts:111`).
///
/// # Errors
///
/// [`GraphError::InputInvalid`] for [`GraphInput::Audit`].
pub fn parse_audit_envelope(text: &str) -> Result<AuditEnvelope> {
    let document =
        reader::document(text).map_err(|reason| GraphError::invalid(GraphInput::Audit, reason))?;
    let envelope = AuditEnvelope::parse(&document)
        .map_err(|reason| GraphError::invalid(GraphInput::Audit, reason))?;
    Ok(AuditEnvelope {
        report: audit::canonicalize_report(envelope.report)?,
        ..envelope
    })
}

/// The premises the caller accepted must be the export's own
/// (`input.ts:131`).
///
/// # Errors
///
/// [`GraphError::InputInvalid`] for [`GraphInput::Premises`].
pub fn check_accepted_premises(
    export: &AssuranceExport,
    accepted: &AcceptedPremises,
) -> Result<()> {
    audit::validate_accepted_premises(export, accepted)?
        .map_err(|reason| GraphError::invalid(GraphInput::Premises, reason))
}

/// The audit must have been taken at the export's identity (`input.ts:149`).
///
/// # Errors
///
/// [`GraphError::InputInvalid`] for [`GraphInput::Audit`].
pub fn check_audit_identity(audit: &AuditEnvelope, export: &AssuranceExport) -> Result<()> {
    audit::validate_audit_identity(audit, export)?
        .map_err(|reason| GraphError::invalid(GraphInput::Audit, reason))
}
