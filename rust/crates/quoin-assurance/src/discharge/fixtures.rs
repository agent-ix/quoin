// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The discharge fixture set, shared by the unit tests of the modules that
//! own the code each one exercises.
//!
//! One clause-binding report, one attestation and one direct fact, exactly as
//! they stood in `discharge.rs` before the split.

use quoin_evidence::source::{EvidenceSource, MemoryEvidence};
use quoin_quire_types::SetPolicy;

use super::{
    ApprovalRefusal, ApprovalRequest, ApprovalVerdict, ApprovalVerifier, BuildDischargeRequest,
    DischargeChecks,
};

/// A reference to the run record [`store`] holds.
pub(super) const RUN_REF: &str = "evidence://runs/SUITE-1/one";

/// An evidence store holding the one run record [`RUN_REF`] names.
pub(super) fn store() -> MemoryEvidence {
    let mut store = MemoryEvidence::new();
    store
        .write("runs/SUITE-1/one.json", b"{}")
        .expect("a memory store accepts a write");
    store
}

/// Checks over a clause set whose clauses declare nothing.
pub(super) fn plain(policy: &SetPolicy) -> DischargeChecks<'_> {
    DischargeChecks {
        policy,
        verifier: None,
        offline: false,
    }
}

/// A verifier that answers every request with one canned result and records
/// what it was asked.
pub(super) struct StubVerifier {
    pub(super) answer: Result<ApprovalVerdict, ApprovalRefusal>,
    pub(super) asked: std::cell::RefCell<Vec<(String, String, String)>>,
}

impl StubVerifier {
    pub(super) fn approving(approver: &str, meaning: &[&str]) -> Self {
        Self::answering(Ok(ApprovalVerdict {
            status: "approved".to_owned(),
            meaning: meaning.iter().map(|m| (*m).to_owned()).collect(),
            approver: Some(approver.to_owned()),
        }))
    }

    pub(super) fn answering(answer: Result<ApprovalVerdict, ApprovalRefusal>) -> Self {
        Self {
            answer,
            asked: std::cell::RefCell::default(),
        }
    }
}

impl ApprovalVerifier for StubVerifier {
    fn verify(&self, request: &ApprovalRequest<'_>) -> Result<ApprovalVerdict, ApprovalRefusal> {
        self.asked.borrow_mut().push((
            request.tag.to_owned(),
            request.record_id.to_owned(),
            request.record_digest.to_owned(),
        ));
        self.answer.clone()
    }
}

pub(super) fn binding() -> quoin_quire_types::ClauseBindingReport {
    serde_json::from_value(serde_json::json!({
        "schemaVersion": "clause-binding-v1",
        "clauseSet": {
            "authority": "example.invalid",
            "id": "synthetic-widget-rules",
            "version": "1.0.0"
        },
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
        "expiresAt": "2026-09-01T00:00:00.000Z"
    })
}

pub(super) fn direct() -> serde_json::Value {
    serde_json::json!({
        "kind": "direct",
        "clauseId": "SYN-001",
        "evidenceRefs": [RUN_REF],
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
