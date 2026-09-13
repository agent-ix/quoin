// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Adapting a retained FR-032 audit report to one proof obligation.
//!
//! The port of `adaptAudit` and `mapAuditFinding` (`verify.ts:236` and
//! `:469`). **The auditor is never re-run**: this reads what was filed and
//! narrows it to the obligations one proof discharges. An obligation the
//! report does not mention at all is neither healthy nor a finding, and the
//! proof's `audit_finding` for it comes from the healthy list not naming it.

use crate::error::{ChangeAssuranceError, Subject};
use crate::ids::{ArtifactDigest, NonEmptyText};
use crate::model::outcome::Reason;
use crate::verify::input::RetainedAudit;

/// Whether the auditor reached a verdict on the owning obligations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AuditState {
    /// It evaluated every owning obligation.
    Evaluated,
    /// It left at least one owning obligation unevaluated.
    NotEvaluated,
}

/// A retained audit narrowed to one proof obligation's evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptedAudit {
    /// The report's digest.
    pub report_digest: ArtifactDigest,
    /// Whether the owning obligations were evaluated at all.
    pub state: AuditState,
    /// The findings against the owning obligations.
    pub findings: Vec<(String, NonEmptyText)>,
    /// The owning obligations the report called healthy.
    pub healthy_obligation_ids: Vec<String>,
}

/// Narrow a retained report to the obligations `owning` names.
///
/// # Errors
///
/// Refuses a report digest that is not 64 lowercase hexadecimal characters,
/// and a finding or healthy entry that is empty.
pub fn adapt_audit(
    retained: &RetainedAudit,
    owning: &[String],
) -> Result<AdaptedAudit, ChangeAssuranceError> {
    const SUBJECT: Subject = Subject::Receipt;
    let report_digest =
        ArtifactDigest::parse(&retained.report_digest, SUBJECT, "FR-032 report digest")?;
    let mut findings = Vec::new();
    for finding in &retained.report.findings {
        NonEmptyText::parse(&finding.obligation, SUBJECT, "FR-032 finding obligation")?;
        let kind = NonEmptyText::parse(&finding.kind, SUBJECT, "FR-032 finding kind")?;
        if owning.iter().any(|id| id == &finding.obligation) {
            findings.push((finding.obligation.clone(), kind));
        }
    }
    let mut healthy_obligation_ids = Vec::new();
    for entry in &retained.report.healthy {
        NonEmptyText::parse(entry, SUBJECT, "FR-032 healthy obligation")?;
        if owning.iter().any(|id| id == entry) {
            healthy_obligation_ids.push(entry.clone());
        }
    }
    let state = if retained
        .report
        .unevaluated
        .iter()
        .any(|entry| owning.iter().any(|id| id == &entry.obligation))
    {
        AuditState::NotEvaluated
    } else {
        AuditState::Evaluated
    };
    Ok(AdaptedAudit {
        report_digest,
        state,
        findings,
        healthy_obligation_ids,
    })
}

/// The receipt reason one FR-032 finding kind maps to.
///
/// Four kinds have their own reason; everything else is `audit_finding`, so a
/// new auditor finding kind lands in the receipt rather than being dropped.
#[must_use]
pub fn map_audit_finding(kind: &str) -> Reason {
    match kind {
        "stale-evidence" => Reason::EvidenceStale,
        "suspect-link" => Reason::EvidenceSuspect,
        "vacuous-evidence" => Reason::EvidenceVacuous,
        "undischarged" => Reason::EvidenceUnrelated,
        _ => Reason::AuditFinding,
    }
}
