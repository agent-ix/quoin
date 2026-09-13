// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The authored-argument fixture set, shared by the unit tests of the modules
//! that own the code each one exercises.
//!
//! The fixture `tests/authored-argument.test.ts` carried, exactly as it stood
//! in `authored.rs` before the split. `build()` and `json()` are read by the
//! tests of [`super::build`], [`super::render`], [`super::parse`] and
//! [`super::view`] alike, so there is one definition of each.

use super::{AuthoredArgumentView, BuildAuthoredArgumentRequest, build_authored_argument_view};

pub(super) const DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const CRITERION: &str = "Every binding synthetic clause has a current disposition.";
pub(super) const AS_OF: &str = "2026-08-15T00:00:00.000Z";

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
            "subject": "widget revision 0123456789abcdef"
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

pub(super) fn request(
    argument: serde_json::Value,
    decisions: Vec<serde_json::Value>,
    as_of: &str,
) -> BuildAuthoredArgumentRequest {
    BuildAuthoredArgumentRequest {
        argument,
        decisions,
        as_of: as_of.to_owned(),
        discharge: None,
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
