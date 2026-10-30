// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A hand-built scenario in which everything agrees.
//!
//! One record, one attestation and its output, one approving decision and one
//! audit report - all addressed by identity - so that a verification of them
//! is `valid` with no reasons. Tests vary one thing at a time.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_change_assurance::error::Subject;
use quoin_change_assurance::ids::{RecordId, Revision};
use quoin_change_assurance::verify::input::{
    AuditReport, RetainedAttestation, RetainedAudit, Selection, VerificationInput,
};
use quoin_store::{JsonValue, parse_strict_json_str};

/// The bytes the attestation's retained output holds.
pub(crate) const OUTPUT: &[u8] = b"all tests passed\n";

/// Parse strict JSON.
pub(crate) fn json(text: &str) -> JsonValue {
    parse_strict_json_str(text).expect("the scenario is strict JSON")
}

/// The record body, as a producer would hand it in.
pub(crate) fn record_text() -> String {
    r#"{
      "schema_version": 1,
      "record_type": "change_assurance",
      "record_id": "change-1",
      "revision": 1,
      "subject": {"repository": "agent-ix/quoin", "base_revision": "abc123", "scope": ["src/evidence"]},
      "source_connections": [{"source_id": "FR-063", "kind": "requirement", "revision": "1"}],
      "impact_snapshot": {"identity": "impact-1", "revision": "1", "completeness": "complete", "truncated": false, "gaps": []},
      "definition": {
        "requirements": [{"id": "FR-063", "statement": "record it", "source_ids": ["FR-063"]}],
        "preservation_constraints": [],
        "proof_obligations": [{
          "proof_id": "proof-1",
          "statement": "tests pass",
          "obligation_ids": ["FR-063-AC-1"],
          "evidence_kind": "Unit",
          "command": {"argv": ["cargo", "test"], "working_directory": "."},
          "tool_identity": "cargo",
          "configuration_ref": "config/test.toml"
        }],
        "unknowns": []
      },
      "review_workflow": {"run_id": "run-1", "decision_event_kind": "change_assurance.review_decided"}
    }"#
    .to_owned()
}

/// The attestation body.
pub(crate) fn attestation_text() -> String {
    format!(
        r#"{{
      "schema_version": 1,
      "record_type": "proof_attestation",
      "attestation_id": "att-1",
      "record_id": "change-1",
      "record_revision": 1,
      "candidate_revision": "candidate-1",
      "proof_id": "proof-1",
      "command": {{"argv": ["cargo", "test"], "working_directory": "."}},
      "tool": {{"identity": "cargo", "version": "1.90.0", "configuration_ref": "config/test.toml"}},
      "environment": {{"os": "test", "workers": 4}},
      "observed_at": "2026-08-31T00:00:00Z",
      "result": "passed",
      "strength": "tested",
      "retained_output": {{"media_type": "text/plain", "size_bytes": {}}}
    }}"#,
        OUTPUT.len()
    )
}

/// One ix-flow event of the decision history.
pub(crate) fn event_text(id: &str, sequence: u64, kind: &str) -> String {
    format!(
        r#"{{
      "id": "{id}", "ts": "2026-08-31T00:00:00.000Z", "sequence": {sequence},
      "actor": {{"id": "reviewer-1", "kind": "human"}},
      "kind": "{kind}",
      "payload": {{"schema_version": 1, "record_id": "change-1", "revision": 1, "decision": "approved"}}
    }}"#
    )
}

/// A decision history of the given events.
pub(crate) fn history(events: &[String]) -> JsonValue {
    json(&format!(
        r#"{{"run_id": "run-1", "events": [{}]}}"#,
        events.join(",")
    ))
}

/// The scenario in which everything agrees.
pub(crate) fn base_input() -> VerificationInput {
    VerificationInput {
        record_id: RecordId::parse("change-1", Subject::Record, "record_id").unwrap(),
        record_revision: Revision::new(1, Subject::Record, "revision").unwrap(),
        record: json(&record_text()),
        parents: Vec::new(),
        candidate_revision: "candidate-1".to_owned(),
        selections: vec![Selection {
            proof_id: "proof-1".to_owned(),
            attestation_id: "att-1".to_owned(),
        }],
        attestations: vec![RetainedAttestation {
            filed_as: "att-1".to_owned(),
            attestation: json(&attestation_text()),
            output: Some(OUTPUT.to_vec()),
        }],
        decision_history: Some(history(&[event_text(
            "event-1",
            1,
            "change_assurance.review_decided",
        )])),
        audits: vec![RetainedAudit {
            proof_id: "proof-1".to_owned(),
            report: AuditReport {
                findings: Vec::new(),
                healthy: vec!["FR-063-AC-1".to_owned()],
                unevaluated: Vec::new(),
            },
        }],
        diff_paths: None,
        governing_plan: None,
    }
}

/// `value` with the member `name` set to `replacement`.
pub(crate) fn replacing(value: &JsonValue, name: &str, replacement: JsonValue) -> JsonValue {
    let mut members = value.as_object().unwrap().clone();
    members.set(name, replacement);
    JsonValue::Object(members)
}

/// `value` without the member `name`.
pub(crate) fn without(value: &JsonValue, name: &str) -> JsonValue {
    let mut members = value.as_object().unwrap().clone();
    members.remove(name);
    JsonValue::Object(members)
}

/// The member `name` of an object.
pub(crate) fn member<'a>(value: &'a JsonValue, name: &str) -> &'a JsonValue {
    value
        .as_object()
        .unwrap_or_else(|_| panic!("expected an object to read {name} from"))
        .get(name)
        .unwrap_or_else(|| panic!("no member {name}"))
}

/// The names of an object's members.
pub(crate) fn names(value: &JsonValue) -> Vec<String> {
    value
        .as_object()
        .unwrap()
        .names()
        .map(str::to_owned)
        .collect()
}
