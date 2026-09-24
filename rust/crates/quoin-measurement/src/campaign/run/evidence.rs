// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Provisional runner assessment from retained attempt receipts.

use std::path::Path;

use engineering_assurance::campaign::CampaignAttempt;
use serde_json::Value;

use super::super::checker::{DomainOutcome, DomainVerdictReceipt};
use super::{AttemptEvidence, read_digest_bytes};

pub(super) fn evidence_of_attempt(repo: &Path, attempt: &CampaignAttempt) -> AttemptEvidence {
    let (Some(domain_digest), Some(verdict_digest)) = (
        attempt.domain_verdict_digest.as_deref(),
        attempt.verdict_digest.as_deref(),
    ) else {
        return AttemptEvidence::Inconclusive;
    };
    let Ok(domain_bytes) = read_digest_bytes(repo, "domain-verdicts", domain_digest, "json") else {
        return AttemptEvidence::Reject;
    };
    let Ok(domain) = serde_json::from_slice::<DomainVerdictReceipt>(&domain_bytes) else {
        return AttemptEvidence::Reject;
    };
    let Ok(verdict_bytes) = read_digest_bytes(repo, "verdicts", verdict_digest, "json") else {
        return AttemptEvidence::Reject;
    };
    let Ok(verdict) = serde_json::from_slice::<Value>(&verdict_bytes) else {
        return AttemptEvidence::Reject;
    };
    match (
        domain.verdict,
        verdict.get("verdict").and_then(Value::as_str),
    ) {
        (DomainOutcome::Accept, Some("accept")) => AttemptEvidence::Accept,
        (DomainOutcome::Reject, _) | (_, Some("reject")) => AttemptEvidence::Reject,
        _ => AttemptEvidence::Inconclusive,
    }
}
