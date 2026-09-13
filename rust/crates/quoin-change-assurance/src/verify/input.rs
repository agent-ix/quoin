// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What a verification is given.
//!
//! Everything a verification reads is **retained evidence**, and none of it is
//! produced here: the record and its parents as they were filed, the
//! attestations and their outputs as they were retained, the decision history
//! as ix-flow wrote it, and the audit reports as FR-032 emitted them. A
//! verification runs no producer and re-audits nothing.
//!
//! The record, its parents, the attestations and the decision history arrive
//! as [`JsonValue`] rather than as typed values, because a verification's job
//! includes deciding that one of them is not valid. A signature that could
//! only accept a valid record could not produce `schema_invalid`.

use quoin_store::JsonValue;

/// One selection: which attestation is offered for which proof obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    /// The obligation the attestation is offered against.
    pub proof_id: String,
    /// The digest of the attestation offered.
    pub attestation_digest: String,
}

/// One retained attestation and the output filed beside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedAttestation {
    /// The attestation as retained.
    pub attestation: JsonValue,
    /// The output bytes as retained, or `None` when they are not present.
    pub output: Option<Vec<u8>>,
}

/// One finding from a retained FR-032 audit report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportFinding {
    /// The obligation the finding is about.
    pub obligation: String,
    /// The finding's kind, as the auditor spelled it.
    pub kind: String,
}

/// One obligation the auditor did not evaluate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportUnevaluated {
    /// The obligation left unevaluated.
    pub obligation: String,
}

/// A retained FR-032 audit report.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuditReport {
    /// What the auditor found wrong.
    pub findings: Vec<ReportFinding>,
    /// Which obligations it found discharged.
    pub healthy: Vec<String>,
    /// Which obligations it did not evaluate.
    pub unevaluated: Vec<ReportUnevaluated>,
}

/// A retained audit, with the digest of the report it came from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedAudit {
    /// The obligation the report covers.
    pub proof_id: String,
    /// The report's digest, as the auditor filed it.
    pub report_digest: String,
    /// The report itself.
    pub report: AuditReport,
}

/// Everything one verification reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationInput {
    /// The record under verification, as retained.
    pub record: JsonValue,
    /// Its parent chain, as retained.
    pub parents: Vec<JsonValue>,
    /// The candidate revision the verification is about.
    pub candidate_revision: String,
    /// Which attestation is offered for which obligation.
    pub selections: Vec<Selection>,
    /// The attestations retained, and their outputs.
    pub attestations: Vec<RetainedAttestation>,
    /// The retained ix-flow decision history, or `None` when none was
    /// retained at all.
    pub decision_history: Option<JsonValue>,
    /// The retained audit reports.
    pub audits: Vec<RetainedAudit>,
}
