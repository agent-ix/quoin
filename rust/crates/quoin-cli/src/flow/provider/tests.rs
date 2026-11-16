// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Provider protocol tests.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions may panic"
)]
use super::serve;
#[cfg(unix)]
use super::{Context, FormalizeInput, formalization_check_with_tools};
use ix_trace_rs::trace;
use serde_json::{Value, json};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::{Path, PathBuf};

fn request() -> Value {
    json!({"protocol":"quoin.workflow-invariants/v1","evaluated_at":"2026-10-24T00:00:00Z","invariants":["unknown"],"instance":{"defName":"review","items":{},"gateConfig":{}}})
}

#[trace("FR-021-AC-5")]
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

#[trace("FR-021-AC-5")]
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

#[cfg(unix)]
fn executable(directory: &Path, name: &str, body: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write executable");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("executable permissions");
    path
}

#[cfg(unix)]
fn formalization_fixture() -> (tempfile::TempDir, FormalizeInput, Context) {
    let directory = tempfile::tempdir().expect("temporary repository");
    fs::create_dir_all(directory.path().join("qsl")).expect("qsl dir");
    fs::write(
        directory.path().join("qsl/rule.native"),
        "first line\nsecond line\n",
    )
    .expect("draft");
    fs::create_dir_all(directory.path().join("spec")).expect("spec dir");
    fs::write(
        directory.path().join("spec/FR-001.md"),
        "---\nid: FR-001\ntype: FR\n---\n\n## Description\n\nRequirement.\n",
    )
    .expect("requirement");
    let root = directory.path().canonicalize().expect("canonical root");
    (
        directory,
        FormalizeInput {
            id: None,
            interview_id: None,
            answered_at: None,
            requirement_id: "FR-001".to_owned(),
            draft_path: PathBuf::from("qsl/rule.native"),
        },
        Context {
            id: "test-context".to_owned(),
            repo_root: root.clone(),
            config_root: root,
            targets: Vec::new(),
        },
    )
}

#[trace("FR-189-AC-1", "FR-189-AC-7")]
#[cfg(unix)]
#[test]
fn qsl_check_passes_only_with_clean_fsl_outcome_and_bound_formalizes_relation() {
    let (directory, request, context) = formalization_fixture();
    let requirement_path = context.repo_root.join("spec/FR-001.md");
    let requirement_before = fs::read(&requirement_path).expect("read requirement before");
    let fsl = executable(
        directory.path(),
        "fsl",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"success\",\"diagnostics\":[]}'; exit 0",
    );
    let quire = executable(
        directory.path(),
        "quire",
        "printf '%s\\n' '{\"query\":{\"kind\":\"file\",\"path\":\"qsl/rule.native\"},\"claims\":{\"formalizes\":[{\"trace_id\":\"FR-001\",\"path\":\"qsl/rule.native\",\"symbol\":\"Rule\"}]}}'; exit 0",
    );

    assert!(formalization_check_with_tools(&request, &context, &fsl, &quire).is_ok());
    assert_eq!(
        fs::read(&requirement_path).expect("read requirement after"),
        requirement_before
    );
}

#[trace("FR-189-AC-2")]
#[cfg(unix)]
#[test]
fn qsl_check_failure_returns_the_diagnostic_file_line_and_message() {
    let (directory, request, context) = formalization_fixture();
    let fsl = executable(
        directory.path(),
        "fsl",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"refusal\",\"diagnostics\":[{\"locus\":{\"kind\":\"region\",\"start\":11},\"message\":\"expected Boolean\"}]}' ; exit 20",
    );
    let quire = executable(directory.path(), "quire", "exit 99");

    let failure = formalization_check_with_tools(&request, &context, &fsl, &quire)
        .expect_err("type error fails check");
    assert_eq!(failure.code.to_string(), "qsl_check_failed");
    assert_eq!(failure.details["file"], "qsl/rule.native");
    assert_eq!(failure.details["diagnostics"][0]["line"], 2);
    assert_eq!(
        failure.details["diagnostics"][0]["message"],
        "expected Boolean"
    );
}

#[trace("FR-189-AC-3")]
#[cfg(unix)]
#[test]
fn a_checked_draft_without_a_bound_formalizes_id_fails_closed() {
    let (directory, request, context) = formalization_fixture();
    let fsl = executable(
        directory.path(),
        "fsl",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"success\",\"diagnostics\":[]}'; exit 0",
    );
    let quire = executable(
        directory.path(),
        "quire",
        "printf '%s\\n' '{\"query\":{\"kind\":\"file\",\"path\":\"qsl/rule.native\"},\"claims\":{\"formalizes\":[]}}'; exit 0",
    );

    let failure = formalization_check_with_tools(&request, &context, &fsl, &quire)
        .expect_err("unbound marker fails");
    assert_eq!(failure.code.to_string(), "formalizes_marker_missing");
}

#[cfg(unix)]
#[trace("FR-189-AC-6")]
#[test]
fn a_missing_fsl_executable_never_passes_the_invariant() {
    let (directory, request, context) = formalization_fixture();
    let missing = directory.path().join("missing-fsl");
    let quire = executable(directory.path(), "quire", "exit 99");

    let failure = formalization_check_with_tools(&request, &context, &missing, &quire)
        .expect_err("missing executable fails closed");
    assert_eq!(failure.code.to_string(), "qsl_check_unavailable");
}

#[trace("FR-189-AC-3")]
#[test]
fn the_retry_guard_requires_four_recorded_consecutive_failures() {
    let failures = |count: usize, cleared: usize| {
        (0..count)
            .map(|index| {
                json!({
                    "diagnostic": format!("failure {index}"),
                    "id": format!("failure-{index}"),
                    "cleared": index < cleared
                })
            })
            .collect::<Vec<_>>()
    };
    let make_instance = |count, cleared| {
        serde_json::from_value(json!({
            "defName": "formalize",
            "items": {"formalization_failure": failures(count, cleared)},
            "gateConfig": {}
        }))
        .expect("instance")
    };

    assert!(super::retries_exhausted(&make_instance(3, 0)).is_err());
    assert!(super::retries_exhausted(&make_instance(4, 0)).is_ok());
    assert!(super::retries_exhausted(&make_instance(4, 1)).is_err());
}

#[trace("FR-189-AC-5")]
#[test]
fn formalize_workflow_gates_approval_and_has_no_terminal_outgoing_edges() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../skills/formalize-requirement/workflows/formalize/def.yaml");
    let yaml = std::fs::read_to_string(path).expect("read formalize workflow");
    let definition = quoin_yaml::from_str(&yaml).expect("parse formalize workflow");
    let phases: Vec<_> = definition["phases"]
        .as_array()
        .expect("phases")
        .iter()
        .filter_map(|phase| phase["name"].as_str())
        .collect();
    assert_eq!(
        phases,
        [
            "drafting",
            "checked",
            "analysed",
            "awaiting_approval",
            "approved",
            "rejected",
            "stopped"
        ]
    );
    let transitions = definition["transitions"].as_array().expect("transitions");
    let approved: Vec<_> = transitions
        .iter()
        .filter(|transition| transition["to"] == "approved")
        .collect();
    assert_eq!(approved.len(), 1);
    assert_eq!(approved[0]["from"], "awaiting_approval");
    assert_eq!(approved[0]["defaultGate"], "hitl");
    for terminal in ["approved", "rejected", "stopped"] {
        assert!(
            transitions
                .iter()
                .all(|transition| transition["from"] != terminal),
            "terminal {terminal} has an outgoing transition"
        );
    }
}
