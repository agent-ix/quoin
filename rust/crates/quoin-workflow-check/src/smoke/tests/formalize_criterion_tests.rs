// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Workflow smoke scenarios for the FR-189 formalization gate and retry reset.

use super::super::{
    Harness, analyzer_for_status, executable, formalize_analysis_fixture,
    formalize_analysis_scenario, passed, require, text,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

#[cfg(feature = "workflow-test")]
#[trace("FR-189-AC-1")]
#[test]
fn a_scripted_valid_draft_reaches_awaiting_approval() {
    let binary = std::env::var("QUOIN_BINARY").expect("QUOIN_BINARY from workflow-test");
    let runner = std::env::var("IX_FLOW_RUNNER").expect("IX_FLOW_RUNNER from workflow-test");
    let plugin = std::env::var("QUOIN_PLUGIN_ROOT").expect("QUOIN_PLUGIN_ROOT from workflow-test");
    let harness = Harness::new_for_formalize(&binary, &runner, &plugin, false)
        .expect("formalize smoke harness");

    formalize_analysis_scenario(
        &harness,
        "smoke-formalize-valid-approval",
        "satisfied",
        "awaiting_approval",
    )
    .expect("passing check and analysis stop at the human approval gate");
}

#[cfg(feature = "workflow-test")]
#[trace("FR-189-AC-12")]
#[test]
fn two_failures_then_a_valid_draft_reset_the_next_failure_count() {
    let binary = std::env::var("QUOIN_BINARY").expect("QUOIN_BINARY from workflow-test");
    let runner = std::env::var("IX_FLOW_RUNNER").expect("IX_FLOW_RUNNER from workflow-test");
    let plugin = std::env::var("QUOIN_PLUGIN_ROOT").expect("QUOIN_PLUGIN_ROOT from workflow-test");
    let harness = Harness::new_for_formalize(&binary, &runner, &plugin, false)
        .expect("formalize smoke harness");
    let (package, engineering_assurance, allowed_signers, tools) =
        formalize_analysis_fixture(&harness).expect("formalize analysis fixture");
    let fsl_bad = executable(
        &tools,
        "fsl-bad",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"refusal\",\"diagnostics\":[{\"locus\":{\"kind\":\"region\",\"start\":1},\"message\":\"expected Boolean\"}]}' ; exit 20",
        "echo {\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"refusal\",\"diagnostics\":[{\"locus\":{\"kind\":\"region\",\"start\":1},\"message\":\"expected Boolean\"}]}\r\nexit /b 20",
    )
    .expect("failing checker is available");
    let fsl_ok = executable(
        &tools,
        "fsl-ok",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"success\",\"diagnostics\":[]}'",
        "echo {\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"success\",\"diagnostics\":[]}\r\nexit /b 0",
    )
    .expect("passing checker is available");
    let quire = executable(
        &tools,
        "quire-trace",
        "printf '%s\\n' '{\"query\":{\"kind\":\"file\",\"path\":\"qsl/rule.native\"},\"claims\":{\"formalizes\":[{\"trace_id\":\"FR-001\",\"path\":\"qsl/rule.native\",\"symbol\":\"P1\"}]}}'",
        "echo {\"query\":{\"kind\":\"file\",\"path\":\"qsl/rule.native\"},\"claims\":{\"formalizes\":[{\"trace_id\":\"FR-001\",\"path\":\"qsl/rule.native\",\"symbol\":\"P1\"}]}}\r\nexit /b 0",
    )
    .expect("Quire trace driver is available");

    let id = "smoke-formalize-retry-reset";
    harness.start("formalize", id).expect("workflow starts");
    harness
        .answers(
            id,
            "request",
            &json!({
                "requirement_id":"FR-001",
                "draft_path":"qsl/rule.native",
                "analysis_package":package,
                "engineering_assurance":engineering_assurance,
                "allowed_signers":allowed_signers
            }),
        )
        .expect("formalization request is complete");

    for attempt in 0..2 {
        let failed = harness
            .flow_with_tools(&["advance", id, "checked"], Some((&fsl_bad, &quire)))
            .expect("failed check is returned as a workflow outcome");
        let failure: Value = serde_json::from_slice(&failed.stdout).expect("failure JSON");
        require(
            failure.to_string().contains("qsl_check_failed"),
            format!("draft attempt {attempt} must report its QSL diagnostic: {failure}"),
        )
        .expect("failed draft is diagnosed");
        let diagnostic = failure.to_string();
        let before = passed(
            &harness
                .flow(&["status", id])
                .expect("failed run status is available"),
        )
        .expect("status is valid JSON");
        require(
            text(&before, "/current_phase").expect("phase") == "drafting"
                && before
                    .pointer("/data/items/formalization_failure")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len)
                    == attempt,
            format!("failed draft {attempt} returns to drafting before recording: {before}"),
        )
        .expect("failed draft is ready for its retry record");
        passed(
            &harness
                .flow(&[
                    "add-item",
                    id,
                    "formalization_failure",
                    "--item",
                    &json!({"id":format!("fsl-failure-{attempt}"),"diagnostic":diagnostic})
                        .to_string(),
                ])
                .expect("failure item is added"),
        )
        .expect("failure item is accepted");
    }

    let checked = passed(
        &harness
            .flow_with_tools(&["advance", id, "checked"], Some((&fsl_ok, &quire)))
            .expect("passing draft advances"),
    )
    .expect("passing draft result is valid");
    require(
        text(&checked, "/current_phase").expect("phase") == "checked",
        "the clean draft reaches checked",
    )
    .expect("draft passes QSL check");

    let status = passed(
        &harness
            .flow(&["status", id])
            .expect("successful run status is available"),
    )
    .expect("status is valid JSON");
    let failures = status
        .pointer("/data/items/formalization_failure")
        .and_then(Value::as_array)
        .expect("prior failure items remain recorded");
    require(
        failures.len() == 2,
        format!("both earlier diagnostics remain recorded: {status}"),
    )
    .expect("two failed attempts are retained");
    for item in failures {
        let item_id = item
            .get("id")
            .and_then(Value::as_str)
            .expect("failure item has an id");
        passed(
            &harness
                .flow(&[
                    "update-item",
                    id,
                    "formalization_failure",
                    item_id,
                    "--patch",
                    "{\"cleared\":true}",
                ])
                .expect("successful check clears the old failure streak"),
        )
        .expect("old failure is marked cleared");
    }

    harness
        .advance(id, "analysed")
        .expect("cleanly checked draft advances to analysis");
    let request = passed(
        &harness
            .flow(&["status", id])
            .expect("checked request is available"),
    )
    .expect("status is valid JSON");
    let request_item = request
        .pointer("/data/items/formalization_request/0")
        .expect("formalization request is retained");
    let request_id = request_item
        .get("id")
        .and_then(Value::as_str)
        .expect("formalization request has an id");
    passed(
        &harness
            .flow(&[
                "update-item",
                id,
                "formalization_request",
                request_id,
                "--patch",
                "{\"drafted_symbols\":[\"qsl:qsl/rule.native::P1\"]}",
            ])
            .expect("checked draft symbols are recorded"),
    )
    .expect("draft symbol is accepted");

    let analyzer = analyzer_for_status(&tools, "refuted").expect("refuted analyzer is available");
    let z3 = executable(&tools, "z3", "exit 0", "exit /b 0").expect("z3 stand-in is available");
    let cvc5 =
        executable(&tools, "cvc5", "exit 0", "exit /b 0").expect("cvc5 stand-in is available");
    let outcome = harness
        .flow_with_analysis_tools(
            &["advance", id, "awaiting_approval"],
            None,
            Some((&analyzer, &z3, &cvc5)),
        )
        .expect("refuted analysis returns its failure outcome");
    let failure: Value = serde_json::from_slice(&outcome.stdout).expect("analysis failure JSON");
    require(
        failure.to_string().contains("draft_claims_inconsistent"),
        format!("refuted analysis retains its diagnostic: {failure}"),
    )
    .expect("refuted analysis is diagnosed");
    passed(
        &harness
            .flow(&[
                "add-item",
                id,
                "formalization_failure",
                "--item",
                &json!({"id":"analysis-failure-1","diagnostic":failure.to_string()}).to_string(),
            ])
            .expect("analysis failure is recorded"),
    )
    .expect("analysis failure item is accepted");
    harness
        .advance(id, "drafting")
        .expect("failed analysis returns to drafting");

    let after = passed(
        &harness
            .flow(&["status", id])
            .expect("retry status is available"),
    )
    .expect("status is valid JSON");
    let items = after
        .pointer("/data/items/formalization_failure")
        .and_then(Value::as_array)
        .expect("failure history is retained");
    let active_failures = items
        .iter()
        .filter(|item| item.get("cleared").and_then(Value::as_bool) != Some(true))
        .count();
    require(
        text(&after, "/current_phase").expect("phase") == "drafting"
            && items.len() == 3
            && active_failures == 1,
        format!("the next consecutive failure count starts at one: {after}"),
    )
    .expect("successful draft reset the next failure count");
}
