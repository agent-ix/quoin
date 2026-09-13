// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The discharge fixture set, shared by the unit tests of the modules that
//! own the code each one exercises.
//!
//! One clause-binding report, one attestation and one direct fact, exactly as
//! they stood in `discharge.rs` before the split — `digest()` is read by both
//! [`super::parse`]'s and [`super::build`]'s tests, so it has one definition.

use super::BuildDischargeRequest;

pub(super) fn digest() -> String {
    format!("sha256:{}", "a".repeat(64))
}

pub(super) fn binding() -> quoin_quire_types::ClauseBindingReport {
    serde_json::from_value(serde_json::json!({
        "schemaVersion": "clause-binding-v1",
        "clauseSet": {
            "authority": "example.invalid",
            "id": "synthetic-widget-rules",
            "version": "1.0.0"
        },
        "clauseSetDigest": digest(),
        "context": { "product": "widget", "deployment": "test" },
        "clauses": [
            {"clauseId": "SYN-001", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": ["test-result"]},
            {"clauseId": "SYN-005", "force": "permitted", "outcome": "not_binding",
             "reasons": [], "expectedOutputs": []}
        ]
    }))
    .expect("the fixture matches the pinned wire shape")
}

pub(super) fn attestation() -> serde_json::Value {
    serde_json::json!({
        "attestedBy": "reviewer-1",
        "authority": "quality-lead",
        "attestedAt": "2026-08-01T00:00:00.000Z",
        "expiresAt": "2026-09-01T00:00:00.000Z",
        "sourceRevision": "0123456789abcdef",
        "evidenceDigest": digest()
    })
}

pub(super) fn direct() -> serde_json::Value {
    serde_json::json!({
        "kind": "direct",
        "clauseId": "SYN-001",
        "evidenceRefs": ["evidence://run/one"],
        "attestation": attestation()
    })
}

pub(super) fn request(facts: Vec<serde_json::Value>) -> BuildDischargeRequest {
    BuildDischargeRequest {
        binding: binding(),
        facts,
        as_of: "2026-08-15T00:00:00.000Z".to_owned(),
    }
}
