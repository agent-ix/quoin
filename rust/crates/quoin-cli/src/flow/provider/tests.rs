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
use super::{
    AnalysisTools, CatalogReadError, ClaimSpan, Context, ContractPackageReadResult, FormalizeInput,
    admit_analysis_package, analysis_consistent_with_approved, analysis_consistent_with_tools,
    bind_package_claims, formal_rule_approved_with_binary, formalization_check_with_tools,
    validate_flow_report,
};
#[cfg(unix)]
use filament_contract_model::{
    CheckedNodeId, CheckedNodeTag, CheckedPackageEvidence, CheckedPackageReadLimits,
    CheckedPackageV2, CheckedPackageV2ReadResult, CompleteLoweringProfileV2,
    CompleteLoweringRecordV2, LoweringProfileRef,
};
use ix_trace_rs::trace;
#[cfg(unix)]
use quoin_quire::{ModuleRoot, ModuleSelection};
use serde_json::{Value, json};
#[cfg(unix)]
use std::collections::BTreeSet;
#[cfg(unix)]
use std::fmt::Write as _;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::{Path, PathBuf};

#[cfg(unix)]
type ExpectedClaimSpan = (String, String, u64, u64);

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
fn flow_report(denials: Value) -> Value {
    json!({
        "schema":"quire.flow-report/v1",
        "rules":[],
        "vocabularies":{},
        "facts":{
            "data_classification":[{
                "id":"CLASS-1",
                "fields":{"regulated_categories":["health"]},
                "edges":[]
            }],
            "trust_boundary":[{"id":"BOUNDARY-1","fields":{"trust_level":"untrusted"},"edges":[]}]
        },
        "denials":denials,
        "remediations":[]
    })
}

#[cfg(unix)]
fn flow_denial() -> Value {
    json!({
        "rule":"health-across-untrusted-boundary",
        "check":"health-across-untrusted-boundary",
        "subject":{"id":"CONN-1","archetype":"connection"},
        "matched":{"data_classification":[{"id":"CLASS-1"}],"trust_boundary":[{"id":"BOUNDARY-1"}]},
        "remediations":[{"name":"encrypt-in-transit","description":"Protect the connection.","change":{"adds":{"edge":"protects","direction":"incoming","from":"control","field":"safeguards","value":"encrypt-in-transit"}}}],
        "rationale":"Health data crossing an untrusted boundary needs protection.",
        "severity":"warning"
    })
}

#[cfg(unix)]
fn design_gate_instance(directory: &Path, quire: &Path, _report: Value, attempt: u64) -> Value {
    let mut history = Vec::new();
    for previous in 1..attempt {
        history.push(json!({"attempt":previous,"report":flow_report(json!([flow_denial()]))}));
    }
    json!({
        "protocol":"quoin.workflow-invariants/v1",
        "evaluated_at":"2026-11-17T00:00:00Z",
        "invariants":["dataflow_rules_pass"],
        "instance":{
            "defName":"design-gate",
            "items":{
                "quoin_context":[{"id":"test","repo_root":directory,"config_root":directory,"targets":[]}],
                "operation_request":[{"design_root":"design","module_roots":[directory.join("module")],"quire_executable":quire}],
                "design_gate_check":history
            },
            "gateConfig":{
                "passed->accepted":"hitl","passed->rejected":"hitl",
                "escalated->accepted":"hitl","escalated->rejected":"hitl"
            }
        }
    })
}

#[cfg(unix)]
fn design_gate_fixture(report: &Value, exit: i32) -> (tempfile::TempDir, Value, PathBuf, PathBuf) {
    let directory = tempfile::tempdir().expect("temporary design-gate repository");
    fs::create_dir_all(directory.path().join("design/spec")).expect("design scope");
    fs::create_dir_all(directory.path().join("module")).expect("module root");
    let output = serde_json::to_string(report).expect("report JSON");
    let capture = directory.path().join("args");
    let quire = executable(
        directory.path(),
        "quire-capture",
        &format!(
            "printf '%s\\n' \"$@\" > '{}'\nprintf '%s\\n' '{}'\nexit {exit}",
            capture.display(),
            output
        ),
    );
    let instance = design_gate_instance(directory.path(), &quire, report.clone(), 1);
    (directory, instance, quire, capture)
}

#[cfg(unix)]
#[trace("FR-209-AC-1", "FR-209-AC-3", "FR-209-AC-5")]
#[test]
fn design_gate_binds_to_explicit_quire_flow_json_and_denials_override_exit_status() {
    let report = flow_report(json!([flow_denial()]));
    let (directory, request, _, capture) = design_gate_fixture(&report, 17);
    let input = serde_json::to_vec(&request).expect("request");
    let mut output = Vec::new();
    let mut errors = Vec::new();
    assert_eq!(super::serve(input.as_slice(), &mut output, &mut errors), 0);
    assert!(errors.is_empty());
    let result: Value = serde_json::from_slice(&output).expect("provider result");
    assert_eq!(result["outcomes"][0]["status"], "failed");
    assert_eq!(result["outcomes"][0]["code"], "dataflow_denied");
    assert_eq!(
        result["outcomes"][0]["details"]["denials"][0]["rule"],
        "health-across-untrusted-boundary"
    );
    assert_eq!(
        result["outcomes"][0]["details"]["denials"][0]["data_categories"],
        json!(["health"])
    );
    assert_eq!(
        result["outcomes"][0]["details"]["denials"][0]["remediations"][0]["name"],
        "encrypt-in-transit"
    );
    let args = fs::read_to_string(capture).expect("captured quire arguments");
    assert!(args.contains("relations\n--scope\n"));
    assert!(args.contains("--flow\n--json\n--module\n"));
    assert!(
        args.contains(
            directory
                .path()
                .join("module")
                .to_str()
                .expect("module path")
        )
    );
}

#[cfg(unix)]
#[trace("FR-209-AC-1", "FR-209-AC-3")]
#[test]
fn design_gate_passes_only_for_valid_report_with_no_denials() {
    let report = flow_report(json!([]));
    let (_directory, request, _, _) = design_gate_fixture(&report, 4);
    let input = serde_json::to_vec(&request).expect("request");
    let mut output = Vec::new();
    let mut errors = Vec::new();
    assert_eq!(super::serve(input.as_slice(), &mut output, &mut errors), 0);
    let result: Value = serde_json::from_slice(&output).expect("provider result");
    assert_eq!(result["outcomes"][0]["status"], "passed");
    assert_eq!(
        result["outcomes"][0]["details"]["report"]["denials"],
        json!([])
    );
}

#[cfg(unix)]
#[trace("FR-209-AC-2")]
#[test]
fn scripted_remediation_turns_a_denial_into_a_clean_report() {
    let denied = flow_report(json!([flow_denial()]));
    let clean = flow_report(json!([]));
    let (directory, value, _quire, _) = design_gate_fixture(&denied, 0);
    let design_root = directory.path().join("design");
    let denied_report = directory.path().join("denied.json");
    let clean_report = directory.path().join("clean.json");
    fs::write(
        &denied_report,
        serde_json::to_vec(&denied).expect("denied JSON"),
    )
    .expect("report");
    fs::write(
        &clean_report,
        serde_json::to_vec(&clean).expect("clean JSON"),
    )
    .expect("report");
    let body = format!(
        "if [ -f '{}/controls/encrypt-in-transit.md' ]; then cat '{}'; else cat '{}'; fi",
        design_root.display(),
        clean_report.display(),
        denied_report.display()
    );
    let quire = executable(directory.path(), "quire-dynamic", &body);
    let mut value = value;
    value["instance"]["items"]["operation_request"][0]["quire_executable"] = json!(quire);

    let first =
        serde_json::from_value::<super::Instance>(value["instance"].clone()).expect("instance");
    let failure = super::check_with_details(&first, "dataflow_rules_pass")
        .expect_err("unprotected health flow is denied");
    assert_eq!(failure.code.to_string(), "dataflow_denied");
    assert_eq!(
        failure.details["denials"][0]["data_categories"],
        json!(["health"])
    );
    assert_eq!(failure.details["denials"][0]["subject"]["id"], "CONN-1");
    assert_eq!(
        failure.details["denials"][0]["matched"]["data_classification"][0]["id"],
        "CLASS-1"
    );
    assert_eq!(
        failure.details["denials"][0]["remediations"][0]["name"],
        "encrypt-in-transit"
    );
    assert_eq!(
        failure.details["denials"][0]["remediations"][0]["description"],
        "Protect the connection."
    );
    assert_eq!(
        failure.details["denials"][0]["remediations"][0]["change"]["adds"]["value"],
        "encrypt-in-transit"
    );

    value["instance"]["items"]["design_gate_check"] = json!([{
        "attempt":1,
        "report":failure.details["report"]
    }]);
    value["instance"]["items"]["design_edit"] = json!([{
        "edits":[{"path":"controls/encrypt-in-transit.md","content":"control: encrypt-in-transit"}]
    }]);
    let proposing =
        serde_json::from_value::<super::Instance>(value["instance"].clone()).expect("instance");
    super::check_with_details(&proposing, "design_edit_targets_safe")
        .expect("trusted editor applies scoped edit");
    assert_eq!(
        fs::read_to_string(design_root.join("controls/encrypt-in-transit.md")).expect("edit"),
        "control: encrypt-in-transit"
    );

    let revised =
        serde_json::from_value::<super::Instance>(value["instance"].clone()).expect("instance");
    let passed = super::check_with_details(&revised, "dataflow_rules_pass")
        .expect("the control clears the rule denial")
        .expect("report details");
    assert_eq!(passed["report"]["denials"], json!([]));
}

#[cfg(unix)]
#[trace("FR-209-AC-4")]
#[test]
fn design_gate_rejects_rule_module_edit_before_writing_any_batch_item() {
    let report = flow_report(json!([flow_denial()]));
    let (directory, mut value, _, _) = design_gate_fixture(&report, 0);
    let design_file = directory.path().join("design/spec/draft.md");
    fs::write(&design_file, "original").expect("design file");
    let module_file = directory.path().join("module/rules.md");
    fs::write(&module_file, "protected").expect("rule file");
    value["instance"]["items"]["design_edit"] = json!([{
        "edits":[
            {"path":"spec/draft.md","content":"changed"},
            {"path":"../module/rules.md","content":"tampered"}
        ]
    }]);
    let instance =
        serde_json::from_value::<super::Instance>(value["instance"].clone()).expect("instance");
    let failure = super::check_with_details(&instance, "design_edit_targets_safe")
        .expect_err("protected edit denied");
    assert_eq!(failure.code.to_string(), "rules_modified");
    assert_eq!(
        fs::read_to_string(design_file).expect("design file"),
        "original"
    );
    assert_eq!(
        fs::read_to_string(module_file).expect("rule file"),
        "protected"
    );
}

#[cfg(unix)]
#[trace("FR-209-AC-10")]
#[test]
fn unavailable_design_agent_stops_before_applying_or_counting_a_check() {
    let report = flow_report(json!([flow_denial()]));
    let (_directory, value, _, _) = design_gate_fixture(&report, 0);
    let instance =
        serde_json::from_value::<super::Instance>(value["instance"].clone()).expect("instance");
    let failure = super::check_with_details(&instance, "design_edit_targets_safe")
        .expect_err("missing proposer output stops this phase");
    assert_eq!(failure.code.to_string(), "design_agent_unavailable");
    assert!(instance.items["design_gate_check"].is_empty());

    let mut no_output = instance;
    no_output
        .items
        .insert("design_edit".to_owned(), vec![json!({"edits": []})]);
    let failure = super::check_with_details(&no_output, "design_edit_targets_safe")
        .expect_err("empty proposer output is unusable");
    assert_eq!(failure.code.to_string(), "design_agent_unavailable");
    assert!(no_output.items["design_gate_check"].is_empty());
}

#[cfg(unix)]
#[trace("FR-209-AC-3")]
#[test]
fn design_gate_rejects_malformed_or_wrong_schema_reports() {
    let directory = tempfile::tempdir().expect("temp");
    fs::create_dir_all(directory.path().join("design/spec")).expect("design scope");
    fs::create_dir_all(directory.path().join("module")).expect("module root");
    let report = flow_report(json!([]));
    for (name, output) in [
        ("malformed", "{bad-json".to_owned()),
        ("wrong-schema", json!({"schema":"other"}).to_string()),
        ("missing-denials", json!({"schema":"quire.flow-report/v1","rules":[],"vocabularies":{},"facts":{},"remediations":[]}).to_string()),
        ("missing-matched", {
            let mut invalid = flow_report(json!([flow_denial()]));
            invalid["denials"][0].as_object_mut().expect("denial").remove("matched");
            invalid.to_string()
        }),
    ] {
        let quire = executable(
            directory.path(),
            &format!("quire-{name}"),
            &format!("printf '%s\\n' '{}'", output.replace('\'', "'\\''")),
        );
        let input = design_gate_instance(directory.path(), &quire, report.clone(), 1);
        let input = serde_json::to_vec(&input).expect("request");
        let mut payload = Vec::new();
        let mut errors = Vec::new();
        assert_eq!(super::serve(input.as_slice(), &mut payload, &mut errors), 0);
        let result: Value = serde_json::from_slice(&payload).expect("provider result");
        assert_eq!(result["outcomes"][0]["status"], "failed", "{name}");
        assert_eq!(result["outcomes"][0]["code"], "dataflow_check_unavailable", "{name}");
    }
    let missing = directory.path().join("missing-quire");
    let input = design_gate_instance(directory.path(), &missing, report, 1);
    let failure = super::check_with_details(
        &serde_json::from_value::<super::Instance>(input["instance"].clone()).expect("instance"),
        "dataflow_rules_pass",
    )
    .expect_err("missing evaluator cannot pass");
    assert_eq!(failure.code.to_string(), "dataflow_check_unavailable");
    let failing = executable(directory.path(), "quire-fails-without-report", "exit 9");
    let input = design_gate_instance(directory.path(), &failing, flow_report(json!([])), 1);
    let failure = super::check_with_details(
        &serde_json::from_value::<super::Instance>(input["instance"].clone()).expect("instance"),
        "dataflow_rules_pass",
    )
    .expect_err("nonzero evaluator without JSON report is unavailable");
    assert_eq!(failure.code.to_string(), "dataflow_check_unavailable");
    assert!(validate_flow_report(&json!({"schema":"other"})).is_err());
}

#[cfg(unix)]
#[trace("FR-209-AC-5")]
#[test]
fn design_gate_retry_routes_allow_three_retries_then_escalate() {
    let report = flow_report(json!([flow_denial()]));
    let (_directory, instance, _quire, _) = design_gate_fixture(&report, 0);
    let mut instance: super::Instance =
        serde_json::from_value(instance["instance"].clone()).expect("instance");
    for (attempt, expected_retry) in [(1, true), (2, true), (3, true), (4, false)] {
        instance.items.insert(
            "design_gate_check".to_owned(),
            (1..=attempt)
                .map(|i| json!({"attempt":i,"report":report}))
                .collect(),
        );
        let retry = super::design_gate_retry_allowed(&instance);
        let exhausted = super::check_with_details(&instance, "design_gate_retries_exhausted");
        if attempt == 4 {
            assert!(!expected_retry);
            assert_eq!(
                retry
                    .expect_err("fourth denial cannot retry")
                    .code
                    .to_string(),
                "design_gate_retries_exhausted"
            );
            assert!(exhausted.is_ok(), "fourth denial reaches escalation");
        } else {
            assert!(expected_retry);
            assert!(retry.is_ok(), "valid denial before retry bound can revise");
            assert!(
                exhausted.is_err(),
                "escalation before fourth denial is refused"
            );
        }
    }
}

#[cfg(unix)]
#[trace("FR-209-AC-8")]
#[test]
fn escalated_human_acceptance_retains_the_denied_evaluator_result() {
    let report = flow_report(json!([flow_denial()]));
    let (_directory, value, _, _) = design_gate_fixture(&report, 0);
    let mut instance: super::Instance =
        serde_json::from_value(value["instance"].clone()).expect("instance");
    instance.items.insert(
        "design_gate_check".to_owned(),
        (1..=4)
            .map(|attempt| json!({"attempt":attempt,"report":report}))
            .collect(),
    );
    let details = super::check_with_details(&instance, "escalated_acceptance_preserves_denial")
        .expect("human decision remains distinct from the evaluator verdict")
        .expect("acceptance details");
    assert_eq!(details["decision"], "accepted");
    assert_eq!(details["flow_result"], "dataflow_denied");
    assert_eq!(
        details["unresolved_denials"][0]["rule"],
        "health-across-untrusted-boundary"
    );
}

#[cfg(unix)]
fn module_selection(directory: &Path) -> ModuleSelection {
    ModuleSelection::Closed(vec![ModuleRoot::open(directory).expect("module root")])
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
            analysis_package: None,
            engineering_assurance: None,
            allowed_signers: None,
            gitsign: None,
            drafted_symbols: Vec::new(),
        },
        Context {
            id: "test-context".to_owned(),
            repo_root: root.clone(),
            config_root: root,
            targets: Vec::new(),
        },
    )
}

#[cfg(unix)]
fn approval_fixture() -> (tempfile::TempDir, FormalizeInput, Context) {
    let (directory, mut request, context) = formalization_fixture();
    request.engineering_assurance = Some("/tmp/engineering-assurance".to_owned());
    request.allowed_signers = Some("/tmp/allowed-signers".to_owned());
    request.gitsign = Some(json!({
        "certificate_identity": "identity",
        "certificate_oidc_issuer": "issuer",
        "principal": "alice"
    }));
    request.drafted_symbols = vec!["qsl:qsl/rule.native::Rule".to_owned()];
    (directory, request, context)
}

#[cfg(unix)]
#[trace("FR-189-AC-17")]
#[test]
fn analysis_invariant_requires_an_explicit_package_before_tool_invocation() {
    let (_directory, mut request, context) = formalization_fixture();
    request.drafted_symbols = vec!["qsl:qsl/rule.native::Rule".to_owned()];
    request.analysis_package = None;
    let failure = analysis_consistent_with_approved(&request, &context)
        .expect_err("missing explicit package must fail closed");
    assert_eq!(failure.code.to_string(), "analysis_package_invalid");

    for path in ["relative/package.json", ""] {
        request.analysis_package = Some(path.to_owned());
        let failure = analysis_consistent_with_approved(&request, &context)
            .expect_err("invalid package path must fail before tool invocation");
        assert_eq!(failure.code.to_string(), "analysis_package_invalid");
    }
}

#[cfg(unix)]
#[trace("FR-189-AC-18")]
#[test]
fn analysis_claim_spans_bind_only_to_checked_draft_and_approved_rules() {
    let (_directory, mut request, _context) = formalization_fixture();
    let draft = "qsl:qsl/rule.native::Rule".to_owned();
    request.drafted_symbols = vec![draft.clone()];
    let expected = vec![ClaimSpan {
        authority: "file".to_owned(),
        identity: "qsl/rule.native".to_owned(),
        start: 10,
        end: 30,
        symbol: draft.clone(),
    }];
    let package = vec![ClaimSpan {
        authority: "repo".to_owned(),
        identity: "qsl/rule.native".to_owned(),
        start: 12,
        end: 28,
        symbol: String::new(),
    }];
    let bound = bind_package_claims(&expected, &package, &request)
        .expect("claim region inside checked declaration is accepted");
    let bound_claim = bound.first().expect("one source claim");
    assert_eq!(bound_claim.symbol, draft);
    assert_eq!(
        bound_claim.authority, "repo",
        "selection preserves package authority"
    );

    let extra = vec![ClaimSpan {
        authority: "repo".to_owned(),
        identity: "qsl/other.native".to_owned(),
        start: 12,
        end: 28,
        symbol: String::new(),
    }];
    let failure = bind_package_claims(&expected, &extra, &request)
        .expect_err("a claim from another source cannot map to the checked draft");
    assert_eq!(failure.code.to_string(), "analysis_package_invalid");

    let failure = bind_package_claims(&expected, &[], &request)
        .expect_err("the package cannot omit the draft claim");
    assert_eq!(failure.code.to_string(), "analysis_package_invalid");
}

#[cfg(unix)]
#[trace("VER-109-AC-1", "FR-189-AC-24", "FR-189-AC-26")]
#[test]
fn analysis_invariant_uses_valid_package_and_draft_only_selection() {
    let (directory, request, context) = formalization_fixture();
    let claims = [
        ("draft-symbol", "qsl/rule.native"),
        ("approved-symbol", "qsl/approved.native"),
    ];
    let (package, spans) = native_analysis_package(directory.path(), &claims);
    let draft = spans.first().expect("draft span");
    let request = analysis_request(request, &package, std::slice::from_ref(&draft.0));
    let status = status_for_rules(
        directory.path(),
        &[
            status_rule(draft, "draft"),
            status_rule(span_at(&spans, 1), "approved"),
        ],
    );
    let (analyzer, log, selection_path) = analyzer_stub(
        directory.path(),
        &[
            ("consistency", "satisfied"),
            ("contradiction", "refuted"),
            ("redundancy", "satisfied"),
        ],
    );
    let z3 = executable(directory.path(), "z3", "exit 0");
    let cvc5 = executable(directory.path(), "cvc5", "exit 0");
    let result = analysis_consistent_with_tools(
        &request,
        &context,
        &analysis_tools(&status, &analyzer, &z3, &cvc5),
    )
    .expect("valid package passes when no contradiction is found");
    assert_eq!(
        result.get("classification").and_then(Value::as_str),
        Some("redundant")
    );
    assert_eq!(
        fs::read_to_string(log)
            .expect("mode log")
            .lines()
            .collect::<Vec<_>>(),
        ["consistency", "contradiction", "redundancy"]
    );
    let selection: Value =
        serde_json::from_slice(&fs::read(selection_path).expect("selection capture"))
            .expect("selection json");
    assert_eq!(
        selection.get("protocol").and_then(Value::as_str),
        Some("filament-analyze.claim-selection/v1")
    );
    let selected_spans = selection
        .get("spans")
        .and_then(Value::as_array)
        .expect("spans");
    assert_eq!(selected_spans.len(), 1);
    assert_eq!(
        selected_spans
            .first()
            .and_then(|span| span.get("identity"))
            .and_then(Value::as_str),
        Some("qsl/rule.native")
    );
    let report_paths = result
        .get("reports")
        .and_then(Value::as_array)
        .expect("report paths");
    let report_path = report_paths
        .first()
        .and_then(Value::as_str)
        .expect("consistency report path");
    let report: Value =
        serde_json::from_slice(&fs::read(report_path).expect("differential report"))
            .expect("differential report JSON");
    assert_eq!(
        report
            .pointer("/selection/selected")
            .and_then(Value::as_array),
        Some(&vec![json!("claim-0")])
    );
    assert_eq!(
        report
            .pointer("/selection/excluded")
            .and_then(Value::as_array),
        Some(&vec![json!("claim-1")])
    );
    let report_dir = result
        .get("report_directory")
        .and_then(Value::as_str)
        .expect("report directory");
    let converted: Value = serde_json::from_slice(
        &fs::read(Path::new(report_dir).join("analysis-package.json"))
            .expect("lowered native package"),
    )
    .expect("lowered package JSON");
    assert_eq!(
        converted.get("contract_version").and_then(Value::as_str),
        Some("filament.contract-package/v1")
    );
}

#[cfg(unix)]
#[trace("FR-189-AC-16")]
#[test]
fn analysis_invariant_keeps_contract_package_v1_input_supported() {
    let (directory, request, context) = formalization_fixture();
    let claims = [("draft-symbol", "qsl/rule.native")];
    let (package, spans) = contract_analysis_package(directory.path(), &claims);
    let draft = spans.first().expect("draft span");
    let request = analysis_request(request, &package, std::slice::from_ref(&draft.0));
    let status = status_for_rules(directory.path(), &[status_rule(draft, "draft")]);
    let (analyzer, log, _) = analyzer_stub(directory.path(), &[("consistency", "satisfied")]);
    let z3 = executable(directory.path(), "z3", "exit 0");
    let cvc5 = executable(directory.path(), "cvc5", "exit 0");
    let result = analysis_consistent_with_tools(
        &request,
        &context,
        &analysis_tools(&status, &analyzer, &z3, &cvc5),
    )
    .expect("contract package v1 remains accepted");
    assert_eq!(
        fs::read_to_string(log)
            .expect("mode log")
            .lines()
            .collect::<Vec<_>>(),
        ["consistency"]
    );
    let report_dir = result
        .get("report_directory")
        .and_then(Value::as_str)
        .expect("report directory");
    assert!(
        !Path::new(report_dir).join("analysis-package.json").exists(),
        "existing v1 input remains the analyzer package"
    );
}

#[cfg(unix)]
#[trace("FR-189-AC-17", "FR-189-AC-27")]
#[test]
fn analysis_invariant_rejects_unreadable_malformed_and_wrong_protocol_packages() {
    let (directory, mut request, context) = formalization_fixture();
    let marker = directory.path().join("analyzer-called");
    let analyzer = executable(
        directory.path(),
        "analyzer-must-not-run",
        &format!("touch '{}'\\nexit 0", marker.display()),
    );
    let status = status_for_rules(directory.path(), &[]);
    let z3 = executable(directory.path(), "z3", "exit 0");
    let cvc5 = executable(directory.path(), "cvc5", "exit 0");
    let unreadable = directory.path().join("missing-package.json");
    let malformed = directory.path().join("malformed-package.json");
    let wrong_protocol = directory.path().join("wrong-protocol.json");
    let refused_checked = directory.path().join("refused-checked-package.json");
    fs::write(&malformed, "{").expect("malformed package");
    fs::write(&wrong_protocol, r#"{"contract_version":"other/v1"}"#)
        .expect("wrong protocol package");
    fs::write(
        &refused_checked,
        include_bytes!("../../../tests/fixtures/formal-rules/checked-package-v2-invalid.json"),
    )
    .expect("refused checked package");
    for package in [&unreadable, &malformed, &wrong_protocol, &refused_checked] {
        request.analysis_package = Some(package.to_string_lossy().into_owned());
        let failure = analysis_consistent_with_tools(
            &request,
            &context,
            &analysis_tools(&status, &analyzer, &z3, &cvc5),
        )
        .expect_err("invalid package fails closed before analysis");
        assert_eq!(failure.code.to_string(), "analysis_package_invalid");
        if package == &refused_checked {
            assert!(failure.message.contains("checked package was refused"));
        }
    }
    assert!(!marker.exists());
    assert!(
        fs::read_dir(&context.config_root)
            .expect("config root")
            .filter_map(Result::ok)
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .starts_with("quoin-analysis-")),
        "refused input must not create an analyzer report directory"
    );
}

#[cfg(unix)]
#[test]
fn analysis_package_reader_catalog_failure_is_not_a_package_refusal() {
    let result = ContractPackageReadResult::CatalogUnavailable(CatalogReadError::new(
        "operations[3].kind",
        "unsupported operation catalog entry",
    ));
    let failure = admit_analysis_package(result)
        .expect_err("reader catalog failure is an infrastructure failure");
    assert_eq!(failure.code.to_string(), "WORKFLOW_CATALOG_UNAVAILABLE");
}

#[cfg(unix)]
#[trace("FR-189-AC-18", "FR-189-AC-25")]
#[test]
fn analysis_invariant_rejects_claims_outside_the_checked_draft_and_approved_set() {
    let (directory, request, context) = formalization_fixture();
    let claims = [
        ("draft-symbol", "qsl/rule.native"),
        ("foreign-symbol", "qsl/foreign.native"),
    ];
    let (package, spans) = native_analysis_package(directory.path(), &claims);
    let first = span_at(&spans, 0);
    let second = span_at(&spans, 1);
    let request = analysis_request(request, &package, std::slice::from_ref(&first.0));
    let mut unrelated_approved = status_rule(second, "approved");
    *unrelated_approved
        .as_object_mut()
        .and_then(|rule| rule.get_mut("formalizes"))
        .expect("formalizes status field") = json!(["FR-002"]);
    let status = status_for_rules(
        directory.path(),
        &[status_rule(first, "draft"), unrelated_approved],
    );
    let marker = directory.path().join("analyzer-called");
    let analyzer = executable(
        directory.path(),
        "analyzer-must-not-run",
        &format!("touch '{}'\\nexit 0", marker.display()),
    );
    let z3 = executable(directory.path(), "z3", "exit 0");
    let cvc5 = executable(directory.path(), "cvc5", "exit 0");
    let failure = analysis_consistent_with_tools(
        &request,
        &context,
        &analysis_tools(&status, &analyzer, &z3, &cvc5),
    )
    .expect_err("foreign source claim is rejected before analyzer invocation");
    assert_eq!(failure.code.to_string(), "analysis_package_invalid");
    assert!(!marker.exists());
}

#[cfg(unix)]
#[trace("FR-189-AC-19")]
#[test]
fn analysis_invariant_fails_with_contradiction_classification() {
    let (directory, request, context) = formalization_fixture();
    let claims = [
        ("draft-symbol", "qsl/rule.native"),
        ("approved-symbol", "qsl/approved.native"),
    ];
    let (package, spans) = native_analysis_package(directory.path(), &claims);
    let draft = spans.first().expect("draft span");
    let request = analysis_request(request, &package, std::slice::from_ref(&draft.0));
    let status = status_for_rules(
        directory.path(),
        &[
            status_rule(draft, "draft"),
            status_rule(span_at(&spans, 1), "approved"),
        ],
    );
    let (analyzer, log, _) = analyzer_stub(
        directory.path(),
        &[("consistency", "satisfied"), ("contradiction", "satisfied")],
    );
    let z3 = executable(directory.path(), "z3", "exit 0");
    let cvc5 = executable(directory.path(), "cvc5", "exit 0");
    let failure = analysis_consistent_with_tools(
        &request,
        &context,
        &analysis_tools(&status, &analyzer, &z3, &cvc5),
    )
    .expect_err("an agreed contradiction fails the transition");
    assert_eq!(failure.code.to_string(), "draft_claims_contradict_approved");
    assert_eq!(
        failure
            .details
            .get("classification")
            .and_then(Value::as_str),
        Some("contradicts")
    );
    assert_eq!(
        fs::read_to_string(log)
            .expect("mode log")
            .lines()
            .collect::<Vec<_>>(),
        ["consistency", "contradiction"]
    );
}

#[cfg(unix)]
#[trace("FR-189-AC-20")]
#[test]
fn analysis_invariant_classifies_redundancy_or_consistency_for_one_draft() {
    for (redundancy, expected) in [("satisfied", "redundant"), ("refuted", "consistent")] {
        let (directory, request, context) = formalization_fixture();
        let claims = [
            ("draft-symbol", "qsl/rule.native"),
            ("approved-symbol", "qsl/approved.native"),
        ];
        let (package, spans) = native_analysis_package(directory.path(), &claims);
        let draft = spans.first().expect("draft span");
        let request = analysis_request(request, &package, std::slice::from_ref(&draft.0));
        let status = status_for_rules(
            directory.path(),
            &[
                status_rule(draft, "draft"),
                status_rule(span_at(&spans, 1), "approved"),
            ],
        );
        let (analyzer, _, _) = analyzer_stub(
            directory.path(),
            &[
                ("consistency", "satisfied"),
                ("contradiction", "refuted"),
                ("redundancy", redundancy),
            ],
        );
        let z3 = executable(directory.path(), "z3", "exit 0");
        let cvc5 = executable(directory.path(), "cvc5", "exit 0");
        let result = analysis_consistent_with_tools(
            &request,
            &context,
            &analysis_tools(&status, &analyzer, &z3, &cvc5),
        )
        .expect("no contradiction passes");
        assert_eq!(
            result.get("classification").and_then(Value::as_str),
            Some(expected)
        );
        assert_eq!(
            result
                .get("reports")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(3)
        );
    }
}

#[cfg(unix)]
#[trace("FR-189-AC-21")]
#[test]
fn analysis_invariant_compares_multi_draft_without_redundancy_mode() {
    let (directory, request, context) = formalization_fixture();
    let claims = [
        ("draft-one", "qsl/rule.native"),
        ("draft-two", "qsl/rule.native"),
        ("approved-peer", "qsl/approved.native"),
    ];
    let (package, spans) = native_analysis_package(directory.path(), &claims);
    let request = analysis_request(
        request,
        &package,
        &[span_at(&spans, 0).0.clone(), span_at(&spans, 1).0.clone()],
    );
    let status = status_for_rules(
        directory.path(),
        &[
            status_rule(span_at(&spans, 0), "draft"),
            status_rule(span_at(&spans, 1), "draft"),
            status_rule(span_at(&spans, 2), "approved"),
        ],
    );
    let (analyzer, log, selection_path) = analyzer_stub(
        directory.path(),
        &[("consistency", "satisfied"), ("contradiction", "refuted")],
    );
    let z3 = executable(directory.path(), "z3", "exit 0");
    let cvc5 = executable(directory.path(), "cvc5", "exit 0");
    let result = analysis_consistent_with_tools(
        &request,
        &context,
        &analysis_tools(&status, &analyzer, &z3, &cvc5),
    )
    .expect("multi-draft set is consistent");
    assert_eq!(
        result.get("classification").and_then(Value::as_str),
        Some("consistent")
    );
    assert_eq!(
        fs::read_to_string(log)
            .expect("mode log")
            .lines()
            .collect::<Vec<_>>(),
        ["consistency", "contradiction"]
    );
    let selection: Value = serde_json::from_slice(&fs::read(selection_path).expect("selection"))
        .expect("selection json");
    assert_eq!(
        selection
            .get("spans")
            .and_then(Value::as_array)
            .expect("spans")
            .len(),
        2
    );
}

#[cfg(unix)]
#[trace("FR-189-AC-22", "FR-189-AC-28")]
#[test]
fn analysis_invariant_fails_closed_for_disagreement_and_unavailable_reports() {
    for nonconclusive in ["disagreement", "unavailable"] {
        let (directory, request, context) = formalization_fixture();
        let claims = [
            ("draft-symbol", "qsl/rule.native"),
            ("approved-symbol", "qsl/approved.native"),
        ];
        let (package, spans) = native_analysis_package(directory.path(), &claims);
        let draft = spans.first().expect("draft span");
        let request = analysis_request(request, &package, std::slice::from_ref(&draft.0));
        let status = status_for_rules(
            directory.path(),
            &[
                status_rule(draft, "draft"),
                status_rule(span_at(&spans, 1), "approved"),
            ],
        );
        let (analyzer, _, _) = analyzer_stub(
            directory.path(),
            &[
                ("consistency", "satisfied"),
                ("contradiction", nonconclusive),
            ],
        );
        let z3 = executable(directory.path(), "z3", "exit 0");
        let cvc5 = executable(directory.path(), "cvc5", "exit 0");
        let failure = analysis_consistent_with_tools(
            &request,
            &context,
            &analysis_tools(&status, &analyzer, &z3, &cvc5),
        )
        .expect_err("non-conclusive required analysis fails closed");
        assert_eq!(failure.code.to_string(), "analysis_inconclusive");
    }
}

#[cfg(unix)]
#[trace("FR-189-AC-23")]
#[test]
fn analysis_invariant_stops_after_a_self_inconsistent_draft() {
    let (directory, request, context) = formalization_fixture();
    let claims = [
        ("draft-symbol", "qsl/rule.native"),
        ("approved-symbol", "qsl/approved.native"),
    ];
    let (package, spans) = native_analysis_package(directory.path(), &claims);
    let draft = spans.first().expect("draft span");
    let request = analysis_request(request, &package, std::slice::from_ref(&draft.0));
    let status = status_for_rules(
        directory.path(),
        &[
            status_rule(draft, "draft"),
            status_rule(span_at(&spans, 1), "approved"),
        ],
    );
    let (analyzer, log, _) = analyzer_stub(
        directory.path(),
        &[("consistency", "refuted"), ("contradiction", "satisfied")],
    );
    let z3 = executable(directory.path(), "z3", "exit 0");
    let cvc5 = executable(directory.path(), "cvc5", "exit 0");
    let failure = analysis_consistent_with_tools(
        &request,
        &context,
        &analysis_tools(&status, &analyzer, &z3, &cvc5),
    )
    .expect_err("self inconsistency fails before peer analysis");
    assert_eq!(failure.code.to_string(), "draft_claims_inconsistent");
    assert_eq!(
        fs::read_to_string(log)
            .expect("mode log")
            .lines()
            .collect::<Vec<_>>(),
        ["consistency"]
    );
}

#[cfg(unix)]
fn status_for_rule(directory: &Path, state: &str, reason: Option<&str>) -> PathBuf {
    let mut rule = json!({
        "symbol": "qsl:qsl/rule.native::Rule",
        "state": state
    });
    if let Some(reason) = reason {
        rule["reason"] = json!(reason);
    }
    let report = json!({"protocol": "quoin.formal-rule-status/v1", "rules": [rule]});
    let body = format!("printf '%s\\n' '{report}'");
    executable(directory, "quoin-status", &body)
}

#[cfg(unix)]
fn checked_analysis_graph(claims: &[(&str, &str)]) -> (Value, Vec<ExpectedClaimSpan>) {
    let package_name = "test/flow";
    let draft_identity = "qsl/rule.native";
    let id = |label: &str| json!({"package":package_name,"path":[label],"ordinal":0});
    let source = |identity: &str| json!({"authority":"repo","identity":identity});
    let mut nodes = Vec::new();
    let mut node_sources = Vec::new();
    let mut add_node = |label: &str,
                        tag: &str,
                        role: &str,
                        form: &str,
                        semantic_type: &str,
                        dependencies: &[&str],
                        body: Value,
                        source_identity: &str| {
        let position = u64::try_from(nodes.len()).expect("node index");
        nodes.push(json!({
            "node_id":id(label),
            "schema_version":"filament.checked-semantic-graph/v2",
            "node_tag":tag,
            "semantic_form":form,
            "semantic_type":id(semantic_type),
            "dependencies":dependencies.iter().map(|dependency| id(dependency)).collect::<Vec<_>>(),
            "occurrences":[{"role":role,"ordinal":0}],
            "body":body,
        }));
        node_sources.push(json!({
            "node_id":id(label),"role":role,"ordinal":0,
            "regions":[{"source":source(source_identity),"start":position * 10 + 3,"end":position * 10 + 7}],
        }));
        (position * 10 + 3, position * 10 + 7)
    };
    for scalar in ["boolean", "integer", "text"] {
        let label = format!("scalar_type/{scalar}");
        add_node(
            &label,
            "scalar_type",
            "type",
            scalar,
            &label,
            &[],
            json!({"term":"aggregate","members":[]}),
            draft_identity,
        );
    }
    add_node(
        "value/literal",
        "value",
        "expression",
        "literal",
        "scalar_type/boolean",
        &[],
        json!({"term":"literal","type":id("scalar_type/boolean"),"value_kind":"boolean","value":true}),
        draft_identity,
    );
    let mut expected = Vec::new();
    for (index, (symbol, identity)) in claims.iter().enumerate() {
        let label = format!("claim/{index}");
        let (start, end) = add_node(
            &label,
            "claim",
            "claim",
            "analysis_claim",
            "scalar_type/boolean",
            &["value/literal"],
            json!({"term":"reference","target":id("value/literal")}),
            identity,
        );
        expected.push(((*symbol).to_owned(), (*identity).to_owned(), start, end));
    }
    let mut sources = claims
        .iter()
        .map(|(_, identity)| *identity)
        .collect::<BTreeSet<_>>();
    sources.insert(draft_identity);
    let definitions =
        |identities: BTreeSet<&str>| identities.into_iter().map(source).collect::<Vec<_>>();
    let checked = json!({
        "contract_version":"filament.checked-package/v2",
        "name":package_name,
        "lock":{
            "sources":definitions(sources),
            "edition":{"role":"edition","definition":{"authority":"repo","identity":"test/edition"}},
            "profile_selections":[],"definition_selections":[],
            "required_features":["filament.value.complete/v1"],"dependency_selections":[],
        },
        "semantic_graph":{"graph_version":"filament.checked-semantic-graph/v2","operand_rows":nodes.iter().map(|_| json!([])).collect::<Vec<_>>(),"nodes":nodes},
        "source_map":node_sources,
        "capability_report":[{"feature":"filament.value.complete/v1","disposition":"available"}],
        "diagnostics":{"catalog":{"authority":"repo","identity":"test/diagnostics"},"entries":[]},
    });
    (checked, expected)
}

#[cfg(unix)]
fn checked_analysis_package(claims: &[(&str, &str)]) -> (Vec<u8>, Vec<ExpectedClaimSpan>) {
    let (checked, expected) = checked_analysis_graph(claims);
    let checked_bytes = quire_canonical::to_vec(
        &checked,
        quire_canonical::Limits::new(CheckedPackageReadLimits::bounded().bytes),
    )
    .expect("checked package canonical JSON");
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    let checked = match CheckedPackageV2::read(
        &checked_bytes,
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => Ok(*package),
        other => Err(format!("checked package fixture rejected: {other:?}")),
    }
    .expect("checked package fixture admitted");
    drop(checked);
    (checked_bytes, expected)
}

#[cfg(unix)]
fn native_analysis_package(
    directory: &Path,
    claims: &[(&str, &str)],
) -> (PathBuf, Vec<ExpectedClaimSpan>) {
    let (package, expected) = checked_analysis_package(claims);
    let path = directory.join("analysis-package.json");
    fs::write(&path, package).expect("write analysis package");
    (path, expected)
}

#[cfg(unix)]
fn contract_analysis_package(
    directory: &Path,
    claims: &[(&str, &str)],
) -> (PathBuf, Vec<ExpectedClaimSpan>) {
    let (checked_bytes, expected) = checked_analysis_package(claims);
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    let checked = match CheckedPackageV2::read(
        &checked_bytes,
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => Ok(*package),
        other => Err(format!("checked package fixture rejected: {other:?}")),
    };
    let checked = checked.expect("checked package fixture admitted");
    let requested = claims
        .iter()
        .enumerate()
        .map(|(index, _)| {
            serde_json::from_value::<CheckedNodeId>(json!({
                "package": "test/flow",
                "path": [format!("claim/{index}")],
                "ordinal": 0
            }))
            .expect("claim node id")
        })
        .collect::<Vec<_>>();
    let profile = CompleteLoweringProfileV2 {
        reference: LoweringProfileRef {
            authority: "filament-analyze".into(),
            identity: "quoin-provider-test".into(),
        },
        required_source_selections: Vec::new(),
        supported_tags: CheckedNodeTag::ALL.iter().copied().collect(),
        require_bounds: true,
        work_limit: u64::MAX,
    };
    let lowered = checked.lower(&requested, &profile);
    assert!(
        lowered
            .records
            .iter()
            .all(|record| matches!(record, CompleteLoweringRecordV2::Lowered { .. }))
    );
    let package = lowered
        .package
        .canonical_bytes()
        .expect("canonical contract package")
        .to_vec();
    let path = directory.join("contract-analysis-package.json");
    fs::write(&path, package).expect("write contract package");
    (path, expected)
}

#[cfg(unix)]
fn status_for_rules(directory: &Path, rules: &[Value]) -> PathBuf {
    let report = json!({"protocol":"quoin.formal-rule-status/v1","rules":rules});
    let body = format!("printf '%s\\n' '{report}'");
    executable(directory, "quoin-status-analysis", &body)
}

#[cfg(unix)]
fn analyzer_stub(directory: &Path, outcomes: &[(&str, &str)]) -> (PathBuf, PathBuf, PathBuf) {
    let log = directory.join("analysis-modes.log");
    let selection = directory.join("captured-selection.json");
    let mut cases = String::new();
    for (mode, status) in outcomes {
        let branch = match *status {
            "satisfied" => {
                "printf '{\"analysisKind\":\"%s\",\"differential\":{\"disposition\":\"agreement\",\"agreedStatus\":\"satisfied\"},\"selection\":{\"selected\":[\"claim-0\"],\"excluded\":[\"claim-1\"]}}' \"$mode\" > \"$output\"; exit 0"
            }
            "refuted" => {
                "printf '{\"analysisKind\":\"%s\",\"differential\":{\"disposition\":\"agreement\",\"agreedStatus\":\"refuted\"},\"selection\":{\"selected\":[\"claim-0\"],\"excluded\":[\"claim-1\"]}}' \"$mode\" > \"$output\"; exit 1"
            }
            "disagreement" => {
                "printf '{\"analysisKind\":\"%s\",\"differential\":{\"disposition\":\"disagreement\",\"agreedStatus\":null},\"selection\":{\"selected\":[\"claim-0\"],\"excluded\":[\"claim-1\"]}}' \"$mode\" > \"$output\"; exit 3"
            }
            _ => "exit 4",
        };
        writeln!(&mut cases, "{mode}) {branch} ;;").expect("append analyzer case");
    }
    let body = format!(
        "while [ \"$#\" -gt 0 ]; do case \"$1\" in --output) output=\"$2\"; shift 2 ;; --request) mode=\"$2\"; shift 2 ;; --select) cp \"$2\" '{}' ; shift 2 ;; *) shift ;; esac; done\nprintf '%s\\n' \"$mode\" >> '{}'\ncase \"$mode\" in\n{}*) exit 4 ;;\nesac",
        selection.display(),
        log.display(),
        cases
    );
    (
        executable(directory, "filament-analyze-stub", &body),
        log,
        selection,
    )
}

#[cfg(unix)]
fn analysis_request(
    mut request: FormalizeInput,
    package: &Path,
    drafted_symbols: &[String],
) -> FormalizeInput {
    request.engineering_assurance = Some("/tmp/engineering-assurance".to_owned());
    request.allowed_signers = Some("/tmp/allowed-signers".to_owned());
    request.analysis_package = Some(package.to_string_lossy().into_owned());
    request.drafted_symbols = drafted_symbols.to_vec();
    request
}

#[cfg(unix)]
fn status_rule(expected: &(String, String, u64, u64), state: &str) -> Value {
    json!({
        "symbol":expected.0,
        "path":expected.1,
        "declaration_range":{"start":expected.2,"end":expected.3},
        "formalizes":["FR-001"],
        "state":state,
    })
}

#[cfg(unix)]
fn analysis_tools<'a>(
    status: &'a Path,
    analyzer: &'a Path,
    z3: &'a Path,
    cvc5: &'a Path,
) -> AnalysisTools<'a> {
    AnalysisTools {
        status,
        analyzer,
        z3,
        cvc5,
    }
}

#[cfg(unix)]
fn span_at(spans: &[(String, String, u64, u64)], index: usize) -> &(String, String, u64, u64) {
    spans.get(index).expect("fixture span")
}

#[cfg(unix)]
#[trace("FR-189-AC-15")]
#[test]
fn formal_rule_approval_rejects_invalid_trust_before_status_or_verifier_invocation() {
    let (directory, mut request, context) = formalization_fixture();
    request.allowed_signers = Some("relative-allowed-signers".to_owned());
    request.drafted_symbols = vec!["qsl:qsl/rule.native::Rule".to_owned()];
    let marker = directory.path().join("status-called");
    let body = format!(
        "printf '%s\\n' called > '{}'\nprintf '%s\\n' '{{\"protocol\":\"quoin.formal-rule-status/v1\",\"rules\":[]}}'",
        marker.display()
    );
    let status = executable(directory.path(), "quoin-status", &body);

    let failure = formal_rule_approved_with_binary(&request, &context, &status)
        .expect_err("relative trust path is refused");
    assert_eq!(failure.code.to_string(), "formal_rule_trust_inputs_invalid");
    assert!(
        !marker.exists(),
        "invalid trust must not start quoin status"
    );

    request.allowed_signers = Some("/tmp/allowed-signers".to_owned());
    request.engineering_assurance = Some("/tmp/ea\0invalid".to_owned());
    let failure = formal_rule_approved_with_binary(&request, &context, &status)
        .expect_err("NUL-containing trust path is refused");
    assert_eq!(failure.code.to_string(), "formal_rule_trust_inputs_invalid");
    assert!(
        !marker.exists(),
        "invalid trust must not start quoin status"
    );

    request.engineering_assurance = Some("/tmp/engineering-assurance".to_owned());
    request.engineering_assurance = Some(String::new());
    let failure = formal_rule_approved_with_binary(&request, &context, &status)
        .expect_err("empty trust path is refused");
    assert_eq!(failure.code.to_string(), "formal_rule_trust_inputs_invalid");
    assert!(
        !marker.exists(),
        "invalid trust must not start quoin status"
    );

    request.engineering_assurance = Some("/tmp/engineering-assurance".to_owned());
    for gitsign in [
        json!({
            "certificate_identity": "identity",
            "certificate_oidc_issuer": "issuer",
            "principal": "alice",
            "unexpected": true
        }),
        json!({
            "certificate_identity": "identity",
            "certificate_oidc_issuer": "issuer"
        }),
        json!({
            "certificate_identity": "-identity",
            "certificate_oidc_issuer": "issuer",
            "principal": "alice"
        }),
        json!({
            "certificate_identity": "identity",
            "certificate_oidc_issuer": "issuer",
            "principal": "bad\nprincipal"
        }),
    ] {
        request.gitsign = Some(gitsign);
        let failure = formal_rule_approved_with_binary(&request, &context, &status)
            .expect_err("invalid gitsign object is refused");
        assert_eq!(failure.code.to_string(), "formal_rule_trust_inputs_invalid");
        assert!(
            !marker.exists(),
            "invalid trust must not start quoin status"
        );
    }
}

#[cfg(unix)]
#[trace("FR-189-AC-10")]
#[test]
fn formal_rule_approval_passes_only_for_a_nonempty_fully_approved_drafted_set() {
    let (directory, mut request, context) = formalization_fixture();
    request.engineering_assurance = Some("/tmp/engineering-assurance".to_owned());
    request.allowed_signers = Some("/tmp/allowed-signers".to_owned());
    request.gitsign = Some(json!({
        "certificate_identity": "identity",
        "certificate_oidc_issuer": "issuer",
        "principal": "alice"
    }));
    request.drafted_symbols = vec!["qsl:qsl/rule.native::Rule".to_owned()];
    let marker = directory.path().join("status-args");
    let body = format!(
        "printf '%s\\n' \"$@\" > '{}'\nprintf '%s\\n' '{{\"protocol\":\"quoin.formal-rule-status/v1\",\"rules\":[{{\"symbol\":\"qsl:qsl/rule.native::Rule\",\"state\":\"approved\"}}]}}'",
        marker.display()
    );
    let status = executable(directory.path(), "quoin-status", &body);
    assert!(formal_rule_approved_with_binary(&request, &context, &status).is_ok());
    let arguments = fs::read_to_string(&marker).expect("captured status arguments");
    for argument in [
        "--engineering-assurance\n/tmp/engineering-assurance",
        "--allowed-signers\n/tmp/allowed-signers",
        "--gitsign-certificate-identity\nidentity",
        "--gitsign-certificate-oidc-issuer\nissuer",
        "--gitsign-principal\nalice",
        "--json",
    ] {
        assert!(
            arguments.contains(argument),
            "missing explicit argument {argument}"
        );
    }

    request.drafted_symbols.clear();
    let failure = formal_rule_approved_with_binary(&request, &context, &status)
        .expect_err("empty drafted set is never approved");
    assert_eq!(failure.code.to_string(), "approval_commit_missing");
}

#[cfg(unix)]
#[trace("FR-189-AC-4")]
#[test]
fn formal_rule_approval_fails_for_acknowledged_gate_without_approval_commit() {
    let (directory, request, context) = approval_fixture();
    let status = status_for_rule(directory.path(), "draft", Some("no_approval"));

    let failure = formal_rule_approved_with_binary(&request, &context, &status)
        .expect_err("an acknowledged gate does not replace a signed approval commit");
    assert_eq!(failure.code.to_string(), "approval_commit_missing");
}

#[cfg(unix)]
#[trace("FR-189-AC-5")]
#[test]
fn formal_rule_approval_passes_for_a_drafted_symbol_reported_approved() {
    let (directory, request, context) = approval_fixture();
    let status = status_for_rule(directory.path(), "approved", None);

    formal_rule_approved_with_binary(&request, &context, &status)
        .expect("an approved status passes for the drafted symbol");
}

#[cfg(unix)]
#[trace("FR-189-AC-8")]
#[test]
fn formal_rule_approval_fails_for_untrusted_signed_rule() {
    let (directory, request, context) = approval_fixture();
    let status = status_for_rule(directory.path(), "draft", Some("approval_untrusted"));

    let failure = formal_rule_approved_with_binary(&request, &context, &status)
        .expect_err("an untrusted signature does not approve the drafted symbol");
    assert_eq!(failure.code.to_string(), "approval_commit_missing");
}

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
    let modules = module_selection(directory.path());

    assert!(formalization_check_with_tools(&request, &context, &fsl, &quire, &modules).is_ok());
    assert_eq!(
        fs::read(&requirement_path).expect("read requirement after"),
        requirement_before
    );
}

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
    let modules = module_selection(directory.path());

    let failure = formalization_check_with_tools(&request, &context, &fsl, &quire, &modules)
        .expect_err("type error fails check");
    assert_eq!(failure.code.to_string(), "qsl_check_failed");
    assert_eq!(failure.details["file"], "qsl/rule.native");
    assert_eq!(failure.details["diagnostics"][0]["line"], 2);
    assert_eq!(
        failure.details["diagnostics"][0]["message"],
        "expected Boolean"
    );
}

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
    let modules = module_selection(directory.path());

    let failure = formalization_check_with_tools(&request, &context, &fsl, &quire, &modules)
        .expect_err("unbound marker fails");
    assert_eq!(failure.code.to_string(), "formalizes_marker_missing");
}

#[cfg(unix)]
#[test]
fn a_missing_fsl_executable_never_passes_the_invariant() {
    let (directory, request, context) = formalization_fixture();
    let missing = directory.path().join("missing-fsl");
    let quire = executable(directory.path(), "quire", "exit 99");
    let modules = module_selection(directory.path());

    let failure = formalization_check_with_tools(&request, &context, &missing, &quire, &modules)
        .expect_err("missing executable fails closed");
    assert_eq!(failure.code.to_string(), "qsl_check_unavailable");
}

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
    let expected_transitions = quoin_yaml::from_str(
        r#"
- {from: drafting, to: checked, invariants: ["interview.complete:request", qsl_check_passes], defaultGate: auto}
- {from: checked, to: drafting, defaultGate: auto}
- {from: checked, to: analysed, defaultGate: auto}
- {from: analysed, to: drafting, defaultGate: auto}
- {from: analysed, to: awaiting_approval, invariants: [analysis_consistent_with_approved], defaultGate: auto}
- {from: awaiting_approval, to: approved, invariants: [formal_rule_approved], defaultGate: hitl}
- {from: awaiting_approval, to: rejected, defaultGate: hitl}
- {from: drafting, to: stopped, invariants: [formalization_retries_exhausted], defaultGate: auto}
"#,
    )
    .expect("expected transition contract");
    assert_eq!(definition["transitions"], expected_transitions);
    let request_schema = &definition["itemSchemas"]["formalization_request"];
    assert_eq!(
        request_schema["required"],
        json!([
            "requirement_id",
            "draft_path",
            "analysis_package",
            "engineering_assurance",
            "allowed_signers"
        ])
    );
    assert_eq!(
        request_schema["properties"]["gitsign"]["properties"],
        json!({
            "certificate_identity":{"type":"string"},
            "certificate_oidc_issuer":{"type":"string"},
            "principal":{"type":"string"}
        })
    );
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

#[trace("FR-209-AC-5", "FR-209-AC-6", "FR-209-AC-7")]
#[test]
fn design_gate_workflow_routes_four_denials_and_gates_human_decisions() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../skills/design-gate/workflows/design-gate/def.yaml");
    let yaml = std::fs::read_to_string(path).expect("read design-gate workflow");
    let definition = quoin_yaml::from_str(&yaml).expect("parse design-gate workflow");
    let phases: Vec<_> = definition["phases"]
        .as_array()
        .expect("phases")
        .iter()
        .filter_map(|phase| phase["name"].as_str())
        .collect();
    assert_eq!(
        phases,
        [
            "proposing",
            "checking",
            "revising",
            "passed",
            "escalated",
            "accepted",
            "rejected"
        ]
    );
    let transitions = definition["transitions"].as_array().expect("transitions");
    let route = |from: &str, to: &str| {
        transitions
            .iter()
            .find(|transition| transition["from"] == from && transition["to"] == to)
            .unwrap_or_else(|| panic!("missing transition {from}->{to}"))
    };
    assert_eq!(
        route("proposing", "checking")["invariants"],
        json!(["interview.complete:request", "design_edit_targets_safe"])
    );
    assert_eq!(
        route("checking", "passed")["invariants"],
        json!(["dataflow_rules_pass"])
    );
    assert_eq!(
        route("checking", "escalated")["invariants"],
        json!(["design_gate_retries_exhausted"])
    );
    assert_eq!(
        route("checking", "revising")["invariants"],
        json!(["design_gate_retry_allowed"])
    );
    assert_eq!(
        route("revising", "checking")["invariants"],
        json!(["design_gate_retry_allowed", "design_edit_targets_safe"])
    );
    assert!(
        route("escalated", "accepted")["invariants"]
            .as_array()
            .is_some_and(|items| items.contains(&json!("escalated_acceptance_preserves_denial")))
    );
    for transition in transitions {
        if matches!(transition["to"].as_str(), Some("accepted" | "rejected")) {
            assert_eq!(transition["defaultGate"], "hitl");
            assert!(
                transition["invariants"]
                    .as_array()
                    .is_some_and(|items| items.contains(&json!("human_acceptance_required")))
            );
        }
        assert!(!matches!(
            transition["from"].as_str(),
            Some("accepted" | "rejected")
        ));
    }
    assert_eq!(
        definition["itemSchemas"]["design_edit"]["required"],
        json!(["edits"])
    );
    let skill_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../skills/design-gate/SKILL.md");
    let skill = std::fs::read_to_string(skill_path).expect("read design-gate skill");
    assert!(skill.contains("design_agent_unavailable"));
    assert!(skill.contains("does not enable signing"));
}

#[cfg(unix)]
#[test]
fn design_gate_acceptance_invariant_requires_every_hitl_edge() {
    let report = flow_report(json!([]));
    let (_directory, value, _, _) = design_gate_fixture(&report, 0);
    let mut instance: super::Instance =
        serde_json::from_value(value["instance"].clone()).expect("instance");
    instance.gates.remove("escalated->rejected");
    let failure = super::check_with_details(&instance, "human_acceptance_required")
        .expect_err("all accept and reject transitions require HITL");
    assert_eq!(failure.code.to_string(), "WORKFLOW_HUMAN_GATE_REQUIRED");
}
