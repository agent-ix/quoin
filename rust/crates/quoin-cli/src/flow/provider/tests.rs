// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Provider protocol tests.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions may panic"
)]
use super::serve;
use serde_json::{Value, json};

fn request() -> Value {
    json!({"protocol":"quoin.workflow-invariants/v1","evaluated_at":"2026-10-24T00:00:00Z","invariants":["unknown"],"instance":{"defName":"review","items":{},"gateConfig":{}}})
}

/// Trace: FR-021-AC-5
#[test]
fn protocol_errors_never_emit_a_success_payload() {
    for input in [
        "null".to_owned(),
        "{broken".to_owned(),
        " ".repeat(1024 * 1024 + 1),
        json!({"protocol":"obsolete"}).to_string(),
    ] {
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        assert_ne!(serve(input.as_bytes(), &mut output, &mut errors), 0);
        assert_eq!(output.len(), 0);
        let diagnostics: Value = serde_json::from_slice(&errors).expect("diagnostics");
        assert!(diagnostics[0]["code"].is_string());
    }
    let mut extra = request();
    extra["unexpected"] = json!(true);
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    assert_ne!(
        serve(extra.to_string().as_bytes(), &mut output, &mut errors),
        0
    );
    assert_eq!(output.len(), 0);
}

/// Trace: FR-021-AC-5
#[test]
fn failed_checks_are_ordered_provider_outcomes_and_not_infrastructure_success() {
    let mut input = request();
    input["invariants"] = json!(["selected_analyses_covered", "human_acceptance_required"]);
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    assert_eq!(
        serve(input.to_string().as_bytes(), &mut output, &mut errors),
        0
    );
    assert_eq!(errors.len(), 0);
    let result: Value = serde_json::from_slice(&output).expect("provider payload");
    assert_eq!(result["protocol"], "quoin.workflow-invariants-result/v1");
    assert_eq!(
        result["outcomes"][0]["invariant"],
        "selected_analyses_covered"
    );
    assert_eq!(
        result["outcomes"][1]["invariant"],
        "human_acceptance_required"
    );
    assert_eq!(result["outcomes"][0]["status"], "failed");
    assert_eq!(result["outcomes"][0]["code"], "WORKFLOW_ITEM_REQUIRED");
}
