// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The authored-argument fixture set, shared by the unit tests of the modules
//! that own the code each one exercises.
//!
//! The fixture `tests/authored-argument.test.ts` carried, exactly as it stood
//! in `authored.rs` before the split. `build()` and `json()` are read by the
//! tests of [`super::build`], [`super::render`], [`super::parse`] and
//! [`super::view`] alike, so there is one definition of each.

use super::{
    AuthoredArgumentView, BuildAuthoredArgumentRequest, EvidenceIndexEntry, EvidenceRefState,
    build_authored_argument_view,
};

pub(super) const DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const CRITERION: &str = "Every binding synthetic clause has a current disposition.";
pub(super) const AS_OF: &str = "2026-08-15T00:00:00.000Z";
/// The top claim's own citation in [`argument`] — deliberately the same
/// reference the fixture's sufficiency decision already cites, since one
/// evidence record legitimately backs both a criterion and the claim it
/// supports.
pub(super) const CLAIM_EVIDENCE_REF: &str = "evidence://discharge/widget-900";

/// The fixture `tests/authored-argument.test.ts` carried, field for field.
pub(super) fn argument() -> serde_json::Value {
    serde_json::json!({
        "id": "AA-900",
        "title": "Synthetic widget release decision",
        "type": "AssuranceArgument",
        "status": "active",
        "owner": "release-owner",
        "profile": "ix://example.invalid/widget/AP-900",
        "top_claim": {
            "id": "CLAIM-900",
            "statement": "The bounded synthetic widget change is acceptable.",
            "subject": "widget revision 0123456789abcdef",
            "evidence_refs": [CLAIM_EVIDENCE_REF]
        },
        "reasoning": [{
            "id": "ARG-900",
            "statement": "Argue from the explicitly reviewed clause disposition.",
            "supports": "CLAIM-900",
            "sufficiency_criteria": [CRITERION]
        }],
        "assumptions": [{
            "id": "ASM-900",
            "statement": "The test environment represents the bounded target.",
            "owner": "release-owner",
            "status": "accepted",
            "review_by": "2026-09-01T00:00:00.000Z"
        }],
        "participants": [{
            "id": "reviewer-900",
            "role": "decision reviewer",
            "authority": "may accept or reject this synthetic release",
            "independence": "did not produce the implementation evidence"
        }],
        "challenges": [{
            "id": "CH-900",
            "target": "CLAIM-900",
            "statement": "A bounded recovery case needed review.",
            "status": "resolved",
            "owner": "release-owner",
            "resolution_refs": ["evidence://experiment/recovery-900"]
        }],
        "relationships": [{
            "target": "ix://example.invalid/widget/AP-900",
            "type": "references"
        }]
    })
}

pub(super) fn decision() -> serde_json::Value {
    serde_json::json!({
        "reasoningId": "ARG-900",
        "criterion": CRITERION,
        "state": "satisfied",
        "evidenceRefs": ["evidence://discharge/widget-900"],
        "decidedBy": "reviewer-900",
        "authority": "may accept or reject this synthetic release",
        "decidedAt": "2026-08-01T00:00:00.000Z",
        "expiresAt": "2026-09-01T00:00:00.000Z",
        "sourceRevision": "0123456789abcdef",
        "evidenceDigest": DIGEST
    })
}

/// `{ ...base, [key]: value }`.
pub(super) fn with(
    base: &serde_json::Value,
    key: &str,
    value: serde_json::Value,
) -> serde_json::Value {
    let mut copy = base.clone();
    copy[key] = value;
    copy
}

/// The evidence index that resolves [`CLAIM_EVIDENCE_REF`] cleanly — the
/// default every existing test builds against, so a suite written before
/// PLAT-966 still reports a `supported` top claim without naming evidence at
/// all.
pub(super) fn resolved_evidence() -> Vec<EvidenceIndexEntry> {
    vec![EvidenceIndexEntry {
        reference: CLAIM_EVIDENCE_REF.to_owned(),
        state: EvidenceRefState::Resolved,
    }]
}

pub(super) fn request(
    argument: serde_json::Value,
    decisions: Vec<serde_json::Value>,
    as_of: &str,
) -> BuildAuthoredArgumentRequest {
    request_with_evidence(argument, decisions, as_of, resolved_evidence())
}

/// [`request`], with an evidence index the caller states explicitly — for the
/// PLAT-966 cases that need the claim's citation to read as unresolved,
/// stale, vacuous or suspect rather than resolved.
pub(super) fn request_with_evidence(
    argument: serde_json::Value,
    decisions: Vec<serde_json::Value>,
    as_of: &str,
    evidence: Vec<EvidenceIndexEntry>,
) -> BuildAuthoredArgumentRequest {
    BuildAuthoredArgumentRequest {
        argument,
        decisions,
        as_of: as_of.to_owned(),
        discharge: None,
        evidence,
    }
}

pub(super) fn build(
    argument: serde_json::Value,
    decisions: Vec<serde_json::Value>,
    as_of: &str,
) -> AuthoredArgumentView {
    build_authored_argument_view(&request(argument, decisions, as_of)).expect("the fixture builds")
}

pub(super) fn json(view: &AuthoredArgumentView) -> serde_json::Value {
    serde_json::to_value(view).expect("the view serialises")
}
