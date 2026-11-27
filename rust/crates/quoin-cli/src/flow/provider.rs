// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Quoin-owned workflow invariants (FR-020, FR-021, FR-095).
//! ix-flow owns transitions and persistence; this provider reads actual catalog
//! contracts and documents on every evaluation, including final acceptance.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::{Duration, Instant};

use filament_contract_model::{
    CatalogReadError, CheckedPackageReadLimits, ContractPackageReadResult, ContractPackageV1,
    ContractPackageWireV1,
};
use ix_trace_rs::implements;
use quoin_quire::{ModuleRoot, ModuleSelection, ScopeRoot, validate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{analysis_package, core_runtime, invocation};

const LENSES: [&str; 7] = [
    "failure-domain",
    "integrity",
    "dependency",
    "evidence",
    "risk-complexity",
    "scope-boundary",
    "ears-conformance",
];
const MAX_REQUEST: u64 = 1024 * 1024;
const MAX_DOCUMENT: u64 = 4 * 1024 * 1024;
const MAX_DOCUMENTS: usize = 256;
const MAX_TOOL_OUTPUT: u64 = 1024 * 1024;
const TOOL_DEADLINE: Duration = Duration::from_secs(4);
const MAX_GITSIGN_VALUE_BYTES: usize = 256;
const REQUEST_PROTOCOL: &str = "quoin.workflow-invariants/v1";
const RESULT_PROTOCOL: &str = "quoin.workflow-invariants-result/v1";

type Check = Result<(), Failure>;
type ToolResults = (
    Result<std::process::Output, String>,
    Result<std::process::Output, String>,
);

#[derive(Clone, Copy, Debug)]
enum Code {
    AnalysesMissing,
    AnalysisUnsupported,
    BadRequest,
    CatalogUnavailable,
    ContextInvalid,
    DocumentInvalid,
    DocumentLimit,
    DocumentUnavailable,
    FormalizesMarkerMissing,
    FormalizesMarkerUnavailable,
    FormalRuleApprovalMissing,
    FormalRuleStatusInvalid,
    FormalRuleStatusUnavailable,
    FormalRuleTrustInputsInvalid,
    DataflowCheckUnavailable,
    DataflowDenied,
    DataflowNotDenied,
    DesignAgentUnavailable,
    RulesModified,
    DesignGateRequestInvalid,
    DesignGateRetriesExhausted,
    DesignGateRetriesNotExhausted,
    DesignGateDecisionInvalid,
    AnalysisPackageInvalid,
    AnalysisInconclusive,
    DraftClaimsInconsistent,
    DraftContradictsApproved,
    HumanGateRequired,
    InputUnreadable,
    InvariantUnknown,
    ItemInvalid,
    ItemRequired,
    PathInvalid,
    PlanIncomplete,
    ProfileDispositionRequired,
    ProfileInvalid,
    ProtocolUnsupported,
    RequestOversized,
    FormalizationRetriesNotExhausted,
    QslCheckFailed,
    QslCheckUnavailable,
    ReviewMismatch,
    SelectionInvalid,
    Unknown,
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AnalysesMissing => "WORKFLOW_ANALYSES_MISSING",
            Self::AnalysisUnsupported => "WORKFLOW_ANALYSIS_UNSUPPORTED",
            Self::BadRequest => "WORKFLOW_BAD_REQUEST",
            Self::CatalogUnavailable => "WORKFLOW_CATALOG_UNAVAILABLE",
            Self::ContextInvalid => "WORKFLOW_CONTEXT_INVALID",
            Self::DocumentInvalid => "WORKFLOW_DOCUMENT_INVALID",
            Self::DocumentLimit => "WORKFLOW_DOCUMENT_LIMIT",
            Self::DocumentUnavailable => "WORKFLOW_DOCUMENT_UNAVAILABLE",
            Self::FormalizesMarkerMissing => "formalizes_marker_missing",
            Self::FormalizesMarkerUnavailable => "formalizes_marker_unavailable",
            Self::FormalRuleApprovalMissing => "approval_commit_missing",
            Self::FormalRuleStatusInvalid => "formal_rule_status_invalid",
            Self::FormalRuleStatusUnavailable => "formal_rule_status_unavailable",
            Self::FormalRuleTrustInputsInvalid => "formal_rule_trust_inputs_invalid",
            Self::DataflowCheckUnavailable => "dataflow_check_unavailable",
            Self::DataflowDenied => "dataflow_denied",
            Self::DataflowNotDenied => "dataflow_not_denied",
            Self::DesignAgentUnavailable => "design_agent_unavailable",
            Self::RulesModified => "rules_modified",
            Self::DesignGateRequestInvalid => "design_gate_request_invalid",
            Self::DesignGateRetriesExhausted => "design_gate_retries_exhausted",
            Self::DesignGateRetriesNotExhausted => "design_gate_retries_not_exhausted",
            Self::DesignGateDecisionInvalid => "design_gate_decision_invalid",
            Self::AnalysisPackageInvalid => "analysis_package_invalid",
            Self::AnalysisInconclusive => "analysis_inconclusive",
            Self::DraftClaimsInconsistent => "draft_claims_inconsistent",
            Self::DraftContradictsApproved => "draft_claims_contradict_approved",
            Self::HumanGateRequired => "WORKFLOW_HUMAN_GATE_REQUIRED",
            Self::InputUnreadable => "WORKFLOW_INPUT_UNREADABLE",
            Self::InvariantUnknown => "WORKFLOW_INVARIANT_UNKNOWN",
            Self::ItemInvalid => "WORKFLOW_ITEM_INVALID",
            Self::ItemRequired => "WORKFLOW_ITEM_REQUIRED",
            Self::PathInvalid => "WORKFLOW_PATH_INVALID",
            Self::PlanIncomplete => "WORKFLOW_PLAN_INCOMPLETE",
            Self::ProfileDispositionRequired => "WORKFLOW_PROFILE_DISPOSITION_REQUIRED",
            Self::ProfileInvalid => "WORKFLOW_PROFILE_INVALID",
            Self::ProtocolUnsupported => "WORKFLOW_PROTOCOL_UNSUPPORTED",
            Self::RequestOversized => "WORKFLOW_REQUEST_OVERSIZED",
            Self::FormalizationRetriesNotExhausted => "formalization_retries_not_exhausted",
            Self::QslCheckFailed => "qsl_check_failed",
            Self::QslCheckUnavailable => "qsl_check_unavailable",
            Self::ReviewMismatch => "WORKFLOW_REVIEW_MISMATCH",
            Self::SelectionInvalid => "WORKFLOW_SELECTION_INVALID",
            Self::Unknown => "WORKFLOW_UNKNOWN",
        })
    }
}

impl Serialize for Code {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
struct Failure {
    code: Code,
    message: String,
    details: BTreeMap<String, Value>,
}

fn fail(code: Code, message: impl Into<String>) -> Failure {
    let message = message.into().chars().take(4096).collect::<String>();
    Failure {
        code,
        details: BTreeMap::from([("message".to_owned(), Value::String(message.clone()))]),
        message,
    }
}

fn fail_with_details(
    code: Code,
    message: impl Into<String>,
    details: BTreeMap<String, Value>,
) -> Failure {
    Failure {
        code,
        message: message.into().chars().take(4096).collect(),
        details,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    invariants: Vec<String>,
    run: RunIdentity,
    instance: Instance,
    evaluated_at: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunIdentity {
    instance_id: String,
    state_version: u64,
    state: String,
    transition_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Instance {
    #[serde(rename = "defName")]
    name: String,
    items: BTreeMap<String, Vec<Value>>,
    #[serde(rename = "gateConfig")]
    gates: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    id: String,
    repo_root: PathBuf,
    config_root: PathBuf,
    targets: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewInput {
    #[serde(default)]
    id: Option<String>,
    #[serde(default, rename = "interviewId")]
    interview_id: Option<String>,
    #[serde(default, rename = "answeredAt")]
    answered_at: Option<String>,
    review_set: String,
    #[serde(default)]
    selected_analyses: Vec<String>,
    #[serde(default)]
    assurance_profiles: Vec<PathBuf>,
    profile_disposition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewDoc {
    id: String,
    analysis: String,
    path: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanInput {
    #[serde(default)]
    id: Option<String>,
    #[serde(default, rename = "interviewId")]
    interview_id: Option<String>,
    #[serde(default, rename = "answeredAt")]
    answered_at: Option<String>,
    plan_path: PathBuf,
    index_path: PathBuf,
    task_paths: Vec<PathBuf>,
    readiness_notes: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormalizeInput {
    #[serde(default)]
    id: Option<String>,
    #[serde(default, rename = "interviewId")]
    interview_id: Option<String>,
    #[serde(default, rename = "answeredAt")]
    answered_at: Option<String>,
    requirement_id: String,
    draft_path: PathBuf,
    analysis_package: Option<String>,
    #[serde(default)]
    engineering_assurance: Option<String>,
    #[serde(default)]
    allowed_signers: Option<String>,
    #[serde(default)]
    gitsign: Option<Value>,
    #[serde(default)]
    drafted_symbols: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormalizationFailure {
    diagnostic: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    cleared: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DesignGateRequest {
    #[serde(default, rename = "id")]
    item_id: Option<String>,
    #[serde(default, rename = "interviewId")]
    interview_id: Option<String>,
    #[serde(default, rename = "answeredAt")]
    answered_at: Option<String>,
    design_root: PathBuf,
    module_roots: Vec<PathBuf>,
    quire_executable: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DesignGateCheck {
    attempt: u64,
    state_version: u64,
    report: Value,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DesignGateRunState {
    run_id: String,
    checks: Vec<DesignGateCheck>,
    proposal_pending: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DesignEditBatch {
    edits: Vec<DesignEdit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DesignEdit {
    path: PathBuf,
    content: String,
}

#[derive(Serialize)]
struct ResultEnvelope {
    protocol: &'static str,
    outcomes: Vec<Outcome>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum Outcome {
    Passed {
        invariant: String,
    },
    Failed {
        invariant: String,
        code: Code,
        details: BTreeMap<String, Value>,
    },
}

pub(crate) fn serve(input: impl Read, mut output: impl Write, mut errors: impl Write) -> u8 {
    let result = read_request(input).map(|request| evaluate(&request));
    match result {
        Ok(result) => match serde_json::to_value(result)
            .map_err(|e| e.to_string())
            .and_then(|value| {
                quoin_core::protocol::canonical_json(&value).map_err(|e| e.to_string())
            }) {
            Ok(text) => {
                if writeln!(output, "{text}").is_ok() {
                    0
                } else {
                    ix_cli_kit::exit::Outcome::Internal.code()
                }
            }
            Err(error) => {
                let _ = writeln!(
                    errors,
                    "{}",
                    serde_json::json!([{"code":"WORKFLOW_OUTPUT_INVALID","message":error}])
                );
                ix_cli_kit::exit::Outcome::Internal.code()
            }
        },
        Err(error) => {
            let _ = writeln!(
                errors,
                "{}",
                serde_json::json!([{"code":error.code,"message":error.message}])
            );
            ix_cli_kit::exit::Outcome::Invalid.code()
        }
    }
}

fn read_request(input: impl Read) -> Result<Request, Failure> {
    let mut bytes = Vec::new();
    input
        .take(MAX_REQUEST + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| fail(Code::InputUnreadable, e.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_REQUEST {
        return Err(fail(
            Code::RequestOversized,
            "workflow request exceeds 1 MiB",
        ));
    }
    let request: Request =
        serde_json::from_slice(&bytes).map_err(|e| fail(Code::BadRequest, e.to_string()))?;
    if request.protocol != REQUEST_PROTOCOL || request.evaluated_at.is_empty() {
        return Err(fail(
            Code::ProtocolUnsupported,
            "expected quoin.workflow-invariants/v1 and evaluated_at",
        ));
    }
    if request.run.instance_id.is_empty()
        || request.run.state.is_empty()
        || request.run.transition_key.is_empty()
    {
        return Err(fail(
            Code::BadRequest,
            "run requires immutable instance_id, state_version, state, and transition_key",
        ));
    }
    if request.invariants.is_empty()
        || request.invariants.len() > 16
        || request
            .invariants
            .iter()
            .any(|name| name.is_empty() || name.len() > 128)
    {
        return Err(fail(Code::BadRequest, "expected 1..16 invariant names"));
    }
    Ok(request)
}

#[implements("FR-189-AC-1", "FR-209-AC-2")]
fn evaluate(request: &Request) -> ResultEnvelope {
    let outcomes = request
        .invariants
        .iter()
        .map(|name| {
            match check_with_details(
                &request.instance,
                name,
                &request.run.instance_id,
                request.run.state_version,
            ) {
                // ix-flow accepts details only on failed outcomes; a successful
                // provider result has the closed `{invariant, status}` shape.
                Ok(_) => Outcome::Passed {
                    invariant: name.clone(),
                },
                Err(error) => Outcome::Failed {
                    invariant: name.clone(),
                    code: error.code,
                    details: error.details,
                },
            }
        })
        .collect();
    ResultEnvelope {
        protocol: RESULT_PROTOCOL,
        outcomes,
    }
}

fn one<T: serde::de::DeserializeOwned>(instance: &Instance, kind: &str) -> Result<T, Failure> {
    let values = instance
        .items
        .get(kind)
        .filter(|values| values.len() == 1)
        .ok_or_else(|| {
            fail(
                Code::ItemRequired,
                format!("exactly one {kind} item is required"),
            )
        })?;
    let value = values
        .first()
        .ok_or_else(|| fail(Code::ItemRequired, kind))?;
    serde_json::from_value(value.clone())
        .map_err(|e| fail(Code::ItemInvalid, format!("{kind}: {e}")))
}

fn context(instance: &Instance) -> Result<Context, Failure> {
    let context: Context = one(instance, "quoin_context")?;
    if context.id.is_empty()
        || !context.repo_root.is_absolute()
        || !context.config_root.is_absolute()
        || !context.repo_root.is_dir()
        || context.targets.len() > MAX_DOCUMENTS
    {
        return Err(fail(
            Code::ContextInvalid,
            "context requires absolute repository/config roots and bounded targets",
        ));
    }
    for target in &context.targets {
        scoped_path(&context.repo_root, target)?;
    }
    Ok(context)
}

fn check_with_details(
    instance: &Instance,
    name: &str,
    run_id: &str,
    state_version: u64,
) -> Result<Option<BTreeMap<String, Value>>, Failure> {
    if !matches!(
        instance.name.as_str(),
        "review" | "to-plan" | "formalize" | "design-gate"
    ) {
        return Err(fail(Code::Unknown, &instance.name));
    }
    let context = context(instance)?;
    match (instance.name.as_str(), name) {
        ("review", "review_selection_consistent") => {
            let review: ReviewInput = one(instance, "operation_request")?;
            let catalog = Catalog::load(&context)?;
            selection(&review, &catalog)?;
            profiles(&review, &context, &catalog).map(|()| None)
        }
        ("review", "selected_analyses_covered") => {
            review_documents(instance, &context).map(|()| None)
        }
        ("to-plan", "plan_inputs_ready") => {
            let plan: PlanInput = one(instance, "plan_answers")?;
            plan_paths(&plan, &context)?;
            Ok(None)
        }
        ("to-plan", "plan_documents_valid") => plan_documents(instance, &context).map(|()| None),
        ("formalize", "qsl_check_passes") => {
            let request: FormalizeInput = one(instance, "formalization_request")?;
            formalization_check(&request, &context).map(|()| None)
        }
        ("formalize", "formal_rule_approved") => {
            let request: FormalizeInput = one(instance, "formalization_request")?;
            formal_rule_approved(&request, &context).map(|()| None)
        }
        ("formalize", "analysis_consistent_with_approved") => {
            let request: FormalizeInput = one(instance, "formalization_request")?;
            analysis_consistent_with_approved(&request, &context).map(Some)
        }
        ("formalize", "formalization_retries_exhausted") => {
            retries_exhausted(instance).map(|()| None)
        }
        ("design-gate", "dataflow_rules_pass") => {
            let request: DesignGateRequest = one(instance, "operation_request")?;
            let (attempt, report) = design_gate_report(&request, &context, run_id, state_version)?;
            if report_denials(&report).is_empty() {
                Ok(Some(BTreeMap::from([
                    ("attempt".to_owned(), Value::from(attempt)),
                    ("report".to_owned(), report),
                ])))
            } else {
                Err(dataflow_denied(attempt, report))
            }
        }
        ("design-gate", "design_edit_targets_safe") => {
            let request: DesignGateRequest = one(instance, "operation_request")?;
            apply_design_edits(instance, &request, &context, run_id).map(|()| None)
        }
        ("design-gate", "design_gate_retry_allowed") => {
            design_gate_retry_allowed(&context, run_id).map(|()| None)
        }
        ("design-gate", "design_gate_retries_exhausted") => {
            design_gate_retries_exhausted(&context, run_id).map(|()| None)
        }
        ("design-gate", "escalated_acceptance_preserves_denial") => {
            design_gate_escalated_acceptance(&context, run_id).map(Some)
        }
        (_, "human_acceptance_required") => {
            let required: &[&str] = if instance.name == "design-gate" {
                &[
                    "passed->accepted",
                    "passed->rejected",
                    "escalated->accepted",
                    "escalated->rejected",
                ]
            } else {
                &["validated->accepted"]
            };
            if required
                .iter()
                .any(|edge| instance.gates.get(*edge).map(String::as_str) != Some("hitl"))
            {
                return Err(fail(
                    Code::HumanGateRequired,
                    "every acceptance or rejection transition must use hitl",
                ));
            }
            Ok(None)
        }
        _ => Err(fail(Code::InvariantUnknown, name)),
    }
}

fn apply_design_edits(
    instance: &Instance,
    request: &DesignGateRequest,
    context: &Context,
    run_id: &str,
) -> Check {
    let Some(values) = instance.items.get("design_edit") else {
        return Err(fail(
            Code::DesignAgentUnavailable,
            "proposer supplied no design_edit output",
        ));
    };
    if values.len() != 1 {
        return Err(fail(
            Code::DesignAgentUnavailable,
            "proposer must supply exactly one design_edit batch",
        ));
    }
    let value = values
        .first()
        .cloned()
        .ok_or_else(|| fail(Code::DesignAgentUnavailable, "design_edit batch is missing"))?;
    let batch: DesignEditBatch = serde_json::from_value(value).map_err(|error| {
        fail(
            Code::DesignAgentUnavailable,
            format!("design_edit: {error}"),
        )
    })?;
    if batch.edits.is_empty() {
        return Err(fail(
            Code::DesignAgentUnavailable,
            "proposer supplied no usable edits",
        ));
    }
    if request.module_roots.is_empty() || request.module_roots.len() > 16 {
        return Err(fail(
            Code::DesignAgentUnavailable,
            "design edit requires the configured explicit rule modules",
        ));
    }
    let state_path = design_gate_state_path(context, run_id)?;
    let mut state = read_design_gate_state(&state_path, run_id)?;
    if state.proposal_pending || state.checks.len() >= 4 {
        return Err(fail(
            Code::DesignAgentUnavailable,
            "the previous proposal has not received an evaluator result or retries are exhausted",
        ));
    }
    if state
        .checks
        .last()
        .is_some_and(|last| report_denials(&last.report).is_empty())
    {
        return Err(fail(
            Code::DesignAgentUnavailable,
            "a passing design cannot be revised in this run",
        ));
    }
    let repo = context
        .repo_root
        .canonicalize()
        .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?;
    let design_root = scoped_path(&repo, &request.design_root).and_then(|path| {
        path.canonicalize()
            .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))
    })?;
    let modules = design_module_paths(&request.module_roots)?;
    let planned = plan_design_edits(batch.edits, &design_root, &modules)?;
    write_design_edits(planned)?;
    state.proposal_pending = true;
    write_design_gate_state(&state_path, &state)?;
    Ok(())
}

fn design_module_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, Failure> {
    paths
        .iter()
        .map(|path| {
            if !path.is_absolute() {
                return Err(fail(
                    Code::DesignAgentUnavailable,
                    "every rule module root must be an absolute path",
                ));
            }
            ModuleRoot::open(path)
                .map(|module| module.as_path().to_path_buf())
                .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))
        })
        .collect()
}

fn plan_design_edits(
    edits: Vec<DesignEdit>,
    design_root: &Path,
    modules: &[PathBuf],
) -> Result<Vec<(PathBuf, String)>, Failure> {
    let mut planned = Vec::with_capacity(edits.len());
    let mut unique = BTreeSet::new();
    for edit in edits {
        let target = design_edit_target(&edit.path, design_root, modules)?;
        if !unique.insert(target.clone()) {
            return Err(fail(
                Code::DesignAgentUnavailable,
                "design edit batch contains duplicate targets",
            ));
        }
        planned.push((target, edit.content));
    }
    Ok(planned)
}

fn design_edit_target(
    path: &Path,
    design_root: &Path,
    modules: &[PathBuf],
) -> Result<PathBuf, Failure> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(fail(
            Code::RulesModified,
            "design edits must use safe relative paths inside design_root",
        ));
    }
    let joined = design_root.join(path);
    let parent = joined
        .parent()
        .ok_or_else(|| fail(Code::RulesModified, "edit path has no parent"))?;
    let mut existing = parent;
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| fail(Code::RulesModified, "edit path escapes design_root"))?;
    }
    let canonical_existing = existing
        .canonicalize()
        .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?;
    let suffix = parent
        .strip_prefix(existing)
        .map_err(|_| fail(Code::RulesModified, "edit path escapes design_root"))?;
    let canonical_parent = canonical_existing.join(suffix);
    let filename = joined
        .file_name()
        .ok_or_else(|| fail(Code::RulesModified, "edit path has no filename"))?;
    let target = if joined.exists() {
        joined
            .canonicalize()
            .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?
    } else {
        canonical_parent.join(filename)
    };
    if !target.starts_with(design_root) || modules.iter().any(|module| target.starts_with(module)) {
        return Err(fail(
            Code::RulesModified,
            "design edit targets a rule module or leaves design_root",
        ));
    }
    Ok(target)
}

fn write_design_edits(planned: Vec<(PathBuf, String)>) -> Check {
    for (path, content) in planned {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?;
        }
        let parent = path.parent().ok_or_else(|| {
            fail(
                Code::DesignAgentUnavailable,
                "authorized edit target has no parent directory",
            )
        })?;
        let mut replacement = tempfile::NamedTempFile::new_in(parent)
            .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?;
        replacement
            .write_all(content.as_bytes())
            .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?;
        if let Ok(metadata) = std::fs::metadata(&path) {
            replacement
                .as_file()
                .set_permissions(metadata.permissions())
                .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?;
        }
        replacement
            .persist(&path)
            .map_err(|error| fail(Code::DesignAgentUnavailable, error.to_string()))?;
    }
    Ok(())
}

fn design_gate_state_path(context: &Context, run_id: &str) -> Result<PathBuf, Failure> {
    if run_id.is_empty()
        || run_id.len() > 128
        || run_id == "."
        || run_id == ".."
        || !run_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return Err(fail(
            Code::ContextInvalid,
            "immutable workflow run id is not a safe state key",
        ));
    }
    let repo = context
        .repo_root
        .canonicalize()
        .map_err(|error| fail(Code::ContextInvalid, error.to_string()))?;
    std::fs::create_dir_all(&context.config_root)
        .map_err(|error| fail(Code::ContextInvalid, error.to_string()))?;
    let config = context
        .config_root
        .canonicalize()
        .map_err(|error| fail(Code::ContextInvalid, error.to_string()))?;
    if config.starts_with(&repo) {
        return Err(fail(
            Code::ContextInvalid,
            "design-gate state must live outside the design repository",
        ));
    }
    let directory = config.join("design-gate");
    std::fs::create_dir_all(&directory)
        .map_err(|error| fail(Code::ContextInvalid, error.to_string()))?;
    let canonical_directory = directory
        .canonicalize()
        .map_err(|error| fail(Code::ContextInvalid, error.to_string()))?;
    if canonical_directory.starts_with(&repo) {
        return Err(fail(
            Code::ContextInvalid,
            "design-gate state directory resolves inside the design repository",
        ));
    }
    Ok(canonical_directory.join(format!("{run_id}.json")))
}

fn read_design_gate_state(path: &Path, run_id: &str) -> Result<DesignGateRunState, Failure> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DesignGateRunState {
                run_id: run_id.to_owned(),
                ..DesignGateRunState::default()
            });
        }
        Err(error) => return Err(fail(Code::DataflowCheckUnavailable, error.to_string())),
    };
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(fail(
            Code::DataflowCheckUnavailable,
            "trusted design-gate run state exceeds 5 MiB",
        ));
    }
    let state: DesignGateRunState = serde_json::from_slice(&bytes)
        .map_err(|error| fail(Code::DataflowCheckUnavailable, error.to_string()))?;
    if state.run_id != run_id
        || state.checks.len() > 4
        || state.checks.iter().enumerate().any(|(index, check)| {
            check.attempt != u64::try_from(index + 1).unwrap_or(u64::MAX)
                || validate_flow_report(&check.report).is_err()
        })
    {
        return Err(fail(
            Code::DataflowCheckUnavailable,
            "trusted design-gate state is malformed or out of sequence",
        ));
    }
    Ok(state)
}

fn write_design_gate_state(path: &Path, state: &DesignGateRunState) -> Check {
    let parent = path
        .parent()
        .ok_or_else(|| fail(Code::DataflowCheckUnavailable, "state path has no parent"))?;
    let bytes = serde_json::to_vec(state)
        .map_err(|error| fail(Code::DataflowCheckUnavailable, error.to_string()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| fail(Code::DataflowCheckUnavailable, error.to_string()))?;
    temporary
        .write_all(&bytes)
        .map_err(|error| fail(Code::DataflowCheckUnavailable, error.to_string()))?;
    temporary
        .persist(path)
        .map_err(|error| fail(Code::DataflowCheckUnavailable, error.to_string()))?;
    Ok(())
}

fn design_gate_report(
    request: &DesignGateRequest,
    context: &Context,
    run_id: &str,
    state_version: u64,
) -> Result<(u64, Value), Failure> {
    let (root, design_root, module_roots) = design_gate_paths(request, context)?;
    let state_path = design_gate_state_path(context, run_id)?;
    let mut state = read_design_gate_state(&state_path, run_id)?;
    if !state.proposal_pending {
        if let Some(last) = state.checks.last() {
            return Ok((last.attempt, last.report.clone()));
        }
        return Err(fail(
            Code::DataflowCheckUnavailable,
            "no trusted design proposal is pending evaluation",
        ));
    }
    if state.checks.len() >= 4 {
        return Err(fail(
            Code::DesignGateRetriesExhausted,
            "the fourth denial has already exhausted the retry allowance",
        ));
    }
    validate_prior_design_denials(&state.checks)?;
    let attempt = u64::try_from(state.checks.len() + 1).map_err(|_| {
        fail(
            Code::DataflowCheckUnavailable,
            "design gate check count is out of range",
        )
    })?;
    let report = run_design_evaluator(request, &root, &design_root, &module_roots)?;
    state.checks.push(DesignGateCheck {
        attempt,
        state_version,
        report: report.clone(),
    });
    state.proposal_pending = false;
    write_design_gate_state(&state_path, &state).map_err(|error| {
        fail(
            Code::DataflowCheckUnavailable,
            format!("could not retain evaluator result: {}", error.message),
        )
    })?;
    Ok((attempt, report))
}

fn design_gate_paths(
    request: &DesignGateRequest,
    context: &Context,
) -> Result<(PathBuf, PathBuf, Vec<PathBuf>), Failure> {
    if request.item_id.as_deref().is_some_and(|id| id != "request")
        || request
            .interview_id
            .as_deref()
            .is_some_and(|id| id != "request")
        || request.answered_at.as_deref().is_some_and(str::is_empty)
        || request.module_roots.is_empty()
        || request.module_roots.len() > 16
    {
        return Err(fail(
            Code::DesignGateRequestInvalid,
            "design_gate_request requires a quire executable and 1..16 explicit module roots",
        ));
    }
    if !request.quire_executable.is_absolute() || !request.quire_executable.is_file() {
        return Err(fail(
            Code::DataflowCheckUnavailable,
            "the configured quire executable is unavailable",
        ));
    }
    let root = context
        .repo_root
        .canonicalize()
        .map_err(|error| fail(Code::ContextInvalid, error.to_string()))?;
    let design_root = scoped_path(&root, &request.design_root)
        .map_err(|error| fail(Code::DataflowCheckUnavailable, error.message))?;
    if !design_root.is_dir() {
        return Err(fail(
            Code::DesignGateRequestInvalid,
            "design_root must be a directory within quoin_context.repo_root",
        ));
    }
    let mut module_roots = Vec::with_capacity(request.module_roots.len());
    let mut module_paths = BTreeSet::new();
    for path in &request.module_roots {
        if !path.is_absolute() {
            return Err(fail(
                Code::DesignGateRequestInvalid,
                "every module root must be an absolute path",
            ));
        }
        let module = ModuleRoot::open(path)
            .map_err(|error| fail(Code::DataflowCheckUnavailable, error.to_string()))?;
        if !module_paths.insert(module.as_path().to_path_buf()) {
            return Err(fail(
                Code::DesignGateRequestInvalid,
                "module roots must be unique",
            ));
        }
        module_roots.push(module.as_path().to_path_buf());
    }
    Ok((root, design_root, module_roots))
}

fn validate_prior_design_denials(checks: &[DesignGateCheck]) -> Check {
    for (index, check) in checks.iter().enumerate() {
        let expected = u64::try_from(index + 1).unwrap_or(u64::MAX);
        if check.attempt != expected || validate_flow_report(&check.report).is_err() {
            return Err(fail(
                Code::DataflowCheckUnavailable,
                "trusted evaluator reports must be valid and sequential from attempt one",
            ));
        }
        if report_denials(&check.report).is_empty() {
            return Err(fail(
                Code::DataflowCheckUnavailable,
                "every prior trusted evaluator result must contain a denial",
            ));
        }
    }
    Ok(())
}

fn run_design_evaluator(
    request: &DesignGateRequest,
    root: &Path,
    design_root: &Path,
    module_roots: &[PathBuf],
) -> Result<Value, Failure> {
    let root_text = design_root.to_str().ok_or_else(|| {
        fail(
            Code::DataflowCheckUnavailable,
            "design_root is not valid UTF-8",
        )
    })?;
    let mut args = vec!["relations", "--scope", root_text, "--flow", "--json"];
    let module_strings = module_roots
        .iter()
        .map(|path| {
            path.to_str().ok_or_else(|| {
                fail(
                    Code::DataflowCheckUnavailable,
                    "module path is not valid UTF-8",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    for module in &module_strings {
        args.push("--module");
        args.push(module);
    }
    let output = run_tool(
        &request.quire_executable,
        &args,
        root,
        Instant::now() + Duration::from_secs(30),
    )
    .map_err(|message| fail(Code::DataflowCheckUnavailable, message))?;
    let report: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        fail(
            Code::DataflowCheckUnavailable,
            format!("quire did not return a usable JSON report: {error}"),
        )
    })?;
    validate_flow_report(&report).map_err(|error| {
        fail(
            Code::DataflowCheckUnavailable,
            format!("quire.flow-report/v1 is incomplete: {error}"),
        )
    })?;
    Ok(report)
}

fn design_gate_checks(context: &Context, run_id: &str) -> Result<DesignGateRunState, Failure> {
    let state_path = design_gate_state_path(context, run_id)?;
    read_design_gate_state(&state_path, run_id)
}

fn design_gate_retry_allowed(context: &Context, run_id: &str) -> Check {
    let state = design_gate_checks(context, run_id)?;
    let Some(last) = state.checks.last() else {
        return Err(fail(
            Code::DataflowNotDenied,
            "a trusted evaluator denial is required before revision",
        ));
    };
    if state.proposal_pending {
        return Err(fail(
            Code::DataflowNotDenied,
            "the pending proposal must be evaluated before revision",
        ));
    }
    if last.attempt == 4 && !report_denials(&last.report).is_empty() {
        return Err(fail(
            Code::DesignGateRetriesExhausted,
            "revision is not allowed after the fourth failed evaluator check",
        ));
    }
    if last.attempt < 4 && !report_denials(&last.report).is_empty() {
        Ok(())
    } else {
        Err(fail(
            Code::DataflowNotDenied,
            "revision requires the latest trusted evaluator result to contain denials before attempt four",
        ))
    }
}

fn design_gate_retries_exhausted(context: &Context, run_id: &str) -> Check {
    let state = design_gate_checks(context, run_id)?;
    if !state.proposal_pending
        && state
            .checks
            .last()
            .is_some_and(|last| last.attempt == 4 && !report_denials(&last.report).is_empty())
    {
        Ok(())
    } else {
        Err(fail(
            Code::DesignGateRetriesNotExhausted,
            "escalation requires the fourth valid evaluator check with denials",
        ))
    }
}

fn design_gate_escalated_acceptance(
    context: &Context,
    run_id: &str,
) -> Result<BTreeMap<String, Value>, Failure> {
    let state = design_gate_checks(context, run_id)?;
    let Some(last) = state.checks.last() else {
        return Err(fail(
            Code::DesignGateDecisionInvalid,
            "escalated acceptance requires the final denied report",
        ));
    };
    let denials = report_denials(&last.report).to_vec();
    if state.proposal_pending || last.attempt != 4 || denials.is_empty() {
        return Err(fail(
            Code::DesignGateDecisionInvalid,
            "an escalated acceptance must preserve the fourth flow denial",
        ));
    }
    Ok(BTreeMap::from([
        ("decision".to_owned(), Value::String("accepted".to_owned())),
        ("report".to_owned(), last.report.clone()),
        (
            "flow_result".to_owned(),
            Value::String(Code::DataflowDenied.to_string()),
        ),
        ("unresolved_denials".to_owned(), Value::Array(denials)),
    ]))
}

fn validate_flow_report(report: &Value) -> Result<(), String> {
    let object = report
        .as_object()
        .ok_or_else(|| "report must be an object".to_owned())?;
    let expected = [
        "schema",
        "rules",
        "vocabularies",
        "facts",
        "denials",
        "remediations",
    ];
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("report must contain exactly schema, rules, vocabularies, facts, denials, and remediations".to_owned());
    }
    if report.get("schema").and_then(Value::as_str) != Some("quire.flow-report/v1")
        || report.get("rules").and_then(Value::as_array).is_none()
        || report
            .get("vocabularies")
            .and_then(Value::as_object)
            .is_none()
        || report.get("facts").and_then(Value::as_object).is_none()
        || report.get("denials").and_then(Value::as_array).is_none()
        || report
            .get("remediations")
            .and_then(Value::as_array)
            .is_none()
    {
        return Err("report fields do not match quire.flow-report/v1".to_owned());
    }
    validate_rules_and_vocabularies(report)?;
    validate_facts(report)?;
    validate_denials(report)?;
    validate_top_remediations(report)
}

fn validate_rules_and_vocabularies(report: &Value) -> Result<(), String> {
    for rule in report
        .get("rules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if rule.get("name").and_then(Value::as_str).is_none()
            || rule.get("subject").and_then(Value::as_array).is_none()
            || rule.get("match").and_then(Value::as_array).is_none()
            || rule.get("unless_any").and_then(Value::as_array).is_none()
            || rule.get("check").and_then(Value::as_str).is_none()
            || rule.get("remediations").and_then(Value::as_array).is_none()
            || rule.get("rationale").and_then(Value::as_str).is_none()
        {
            return Err("each rule must match the flow-rule declaration shape".to_owned());
        }
    }
    for (archetype, fields) in report
        .get("vocabularies")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if archetype.is_empty() || !fields.is_object() {
            return Err("vocabularies must map archetypes to field objects".to_owned());
        }
        for values in fields
            .as_object()
            .into_iter()
            .flatten()
            .map(|(_, values)| values)
        {
            if !values
                .as_array()
                .is_some_and(|values| values.iter().all(Value::is_string))
            {
                return Err("vocabulary values must be arrays of strings".to_owned());
            }
        }
    }
    Ok(())
}

fn validate_facts(report: &Value) -> Result<(), String> {
    for (archetype, facts) in report
        .get("facts")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if archetype.is_empty() || !facts.is_array() {
            return Err("facts must map archetypes to arrays".to_owned());
        }
        for fact in facts.as_array().into_iter().flatten() {
            if fact.get("id").and_then(Value::as_str).is_none()
                || fact.get("fields").and_then(Value::as_object).is_none()
                || fact.get("edges").and_then(Value::as_array).is_none()
                || fact.get("repo").is_some_and(|repo| !repo.is_string())
            {
                return Err(format!(
                    "{archetype} fact must contain id, fields and edges"
                ));
            }
        }
    }
    Ok(())
}

fn validate_denials(report: &Value) -> Result<(), String> {
    for denial in report
        .get("denials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if denial.get("rule").and_then(Value::as_str).is_none()
            || denial.get("check").and_then(Value::as_str).is_none()
            || denial.get("rationale").and_then(Value::as_str).is_none()
            || denial.get("severity").and_then(Value::as_str).is_none()
        {
            return Err("each denial must name its rule, check, rationale and severity".to_owned());
        }
        let subject = denial
            .get("subject")
            .ok_or_else(|| "denial subject is missing".to_owned())?;
        if subject.get("id").and_then(Value::as_str).is_none()
            || subject.get("archetype").and_then(Value::as_str).is_none()
            || subject.get("repo").is_some_and(|repo| !repo.is_string())
        {
            return Err("denial subject must contain id and archetype".to_owned());
        }
        let matched = denial
            .get("matched")
            .and_then(Value::as_object)
            .ok_or_else(|| "denial matched must be an object".to_owned())?;
        for (archetype, entries) in matched {
            if archetype.is_empty() || !entries.is_array() {
                return Err("matched facts must map archetypes to arrays".to_owned());
            }
            for entry in entries.as_array().into_iter().flatten() {
                if entry.get("id").and_then(Value::as_str).is_none()
                    || entry.get("repo").is_some_and(|repo| !repo.is_string())
                {
                    return Err("matched facts must contain ids and optional repos".to_owned());
                }
                let id = entry.get("id").and_then(Value::as_str);
                let repo = entry.get("repo").and_then(Value::as_str);
                let referenced = report
                    .get("facts")
                    .and_then(|facts| facts.get(archetype))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .any(|fact| {
                        fact.get("id").and_then(Value::as_str) == id
                            && fact.get("repo").and_then(Value::as_str) == repo
                    });
                if !referenced {
                    return Err("matched fact does not resolve in the flow report".to_owned());
                }
            }
        }
        let remediations = denial
            .get("remediations")
            .and_then(Value::as_array)
            .ok_or_else(|| "denial remediations must be an array".to_owned())?;
        for remediation in remediations {
            if remediation.get("name").and_then(Value::as_str).is_none()
                || remediation
                    .get("description")
                    .and_then(Value::as_str)
                    .is_none()
                || remediation
                    .get("change")
                    .and_then(Value::as_object)
                    .is_none()
            {
                return Err("remediation must contain name, description and change".to_owned());
            }
        }
    }
    Ok(())
}

fn validate_top_remediations(report: &Value) -> Result<(), String> {
    for remediation in report
        .get("remediations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let actions = ["adds", "sets", "removes"]
            .iter()
            .filter(|key| remediation.get(**key).is_some())
            .count();
        if remediation.get("name").and_then(Value::as_str).is_none()
            || remediation
                .get("description")
                .and_then(Value::as_str)
                .is_none()
            || actions != 1
        {
            return Err("top-level remediation must contain one declared change".to_owned());
        }
    }
    Ok(())
}

fn dataflow_denied(attempt: u64, report: Value) -> Failure {
    let denials = report_denials(&report).to_vec();
    let facts = report.get("facts").unwrap_or(&Value::Null);
    let detail = denials
        .into_iter()
        .map(|mut denial| {
            let categories = denial
                .pointer("/matched/data_classification")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(|matched| {
                    let id = matched.get("id").and_then(Value::as_str);
                    let repo = matched.get("repo").and_then(Value::as_str);
                    facts
                        .get("data_classification")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter(move |fact| {
                            fact.get("id").and_then(Value::as_str) == id
                                && fact.get("repo").and_then(Value::as_str) == repo
                        })
                        .flat_map(|fact| {
                            fact.pointer("/fields/regulated_categories")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect::<Vec<_>>()
                        })
                })
                .collect::<Vec<_>>();
            if let Some(map) = denial.as_object_mut() {
                map.insert(
                    "data_categories".to_owned(),
                    Value::Array(categories.into_iter().map(Value::String).collect()),
                );
            }
            denial
        })
        .collect::<Vec<_>>();
    let mut details = BTreeMap::from([
        ("attempt".to_owned(), Value::from(attempt)),
        ("denials".to_owned(), Value::Array(detail)),
        ("report".to_owned(), report),
    ]);
    if let Some(array) = details.get_mut("denials").and_then(Value::as_array_mut) {
        for denial in array {
            if denial
                .get("data_categories")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
                && let Some(map) = denial.as_object_mut()
            {
                map.remove("data_categories");
            }
        }
    }
    Failure {
        code: Code::DataflowDenied,
        message: "data-flow rules deny the current design".to_owned(),
        details,
    }
}

fn report_denials(report: &Value) -> &[Value] {
    report
        .get("denials")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

struct FormalRuleTrust<'a> {
    engineering_assurance: &'a str,
    allowed_signers: &'a str,
    gitsign: Option<(&'a str, &'a str, &'a str)>,
}

fn formal_rule_trust(request: &FormalizeInput) -> Result<FormalRuleTrust<'_>, Failure> {
    let valid_path = |value: Option<&String>| {
        value.is_some_and(|path| {
            !path.is_empty() && !path.contains('\0') && Path::new(path).is_absolute()
        })
    };
    if !valid_path(request.engineering_assurance.as_ref())
        || !valid_path(request.allowed_signers.as_ref())
    {
        return Err(fail(
            Code::FormalRuleTrustInputsInvalid,
            "formalization_request requires non-empty absolute engineering_assurance and allowed_signers paths without NUL",
        ));
    }
    let gitsign = match request.gitsign.as_ref() {
        None => None,
        Some(value) => {
            let Some(object) = value.as_object() else {
                return Err(fail(
                    Code::FormalRuleTrustInputsInvalid,
                    "formalization_request.gitsign must be an object",
                ));
            };
            if object.len() != 3
                || object.keys().any(|key| {
                    !matches!(
                        key.as_str(),
                        "certificate_identity" | "certificate_oidc_issuer" | "principal"
                    )
                })
            {
                return Err(fail(
                    Code::FormalRuleTrustInputsInvalid,
                    "formalization_request.gitsign must contain exactly the three FR-040 fields",
                ));
            }
            let identity = object.get("certificate_identity").and_then(Value::as_str);
            let issuer = object
                .get("certificate_oidc_issuer")
                .and_then(Value::as_str);
            let principal = object.get("principal").and_then(Value::as_str);
            let plain = |input: Option<&str>| {
                input.is_some_and(|text| {
                    !text.is_empty()
                        && text.len() <= MAX_GITSIGN_VALUE_BYTES
                        && !text.chars().any(char::is_control)
                })
            };
            if !plain(identity)
                || !plain(issuer)
                || !plain(principal)
                || identity.is_some_and(|text| text.starts_with('-'))
                || issuer.is_some_and(|text| text.starts_with('-'))
            {
                return Err(fail(
                    Code::FormalRuleTrustInputsInvalid,
                    "formalization_request.gitsign values violate FR-040 validation",
                ));
            }
            Some((
                identity.ok_or_else(|| {
                    fail(
                        Code::FormalRuleTrustInputsInvalid,
                        "formalization_request.gitsign.certificate_identity is required",
                    )
                })?,
                issuer.ok_or_else(|| {
                    fail(
                        Code::FormalRuleTrustInputsInvalid,
                        "formalization_request.gitsign.certificate_oidc_issuer is required",
                    )
                })?,
                principal.ok_or_else(|| {
                    fail(
                        Code::FormalRuleTrustInputsInvalid,
                        "formalization_request.gitsign.principal is required",
                    )
                })?,
            ))
        }
    };
    Ok(FormalRuleTrust {
        engineering_assurance: request.engineering_assurance.as_deref().ok_or_else(|| {
            fail(
                Code::FormalRuleTrustInputsInvalid,
                "engineering_assurance is required",
            )
        })?,
        allowed_signers: request.allowed_signers.as_deref().ok_or_else(|| {
            fail(
                Code::FormalRuleTrustInputsInvalid,
                "allowed_signers is required",
            )
        })?,
        gitsign,
    })
}

fn formal_rule_approved(request: &FormalizeInput, context: &Context) -> Check {
    formal_rule_trust(request)?;
    let executable = std::env::current_exe().map_err(|error| {
        fail(
            Code::FormalRuleStatusUnavailable,
            format!("quoin executable is unavailable: {error}"),
        )
    })?;
    formal_rule_approved_with_binary(request, context, &executable)
}

fn formal_rule_approved_with_binary(
    request: &FormalizeInput,
    context: &Context,
    executable: &Path,
) -> Check {
    let mut seen = BTreeSet::new();
    if request.drafted_symbols.is_empty()
        || request
            .drafted_symbols
            .iter()
            .any(|symbol| symbol.trim().is_empty() || !seen.insert(symbol))
    {
        return Err(fail(
            Code::FormalRuleApprovalMissing,
            "the drafted formal-rule set must be non-empty and contain unique symbols",
        ));
    }

    let report = formal_rule_status(request, context, executable)?;
    let Some(rules) = report.get("rules").and_then(Value::as_array) else {
        return Err(fail(
            Code::FormalRuleStatusInvalid,
            "quoin formal-rules status returned no rule list",
        ));
    };
    let all_approved = request.drafted_symbols.iter().all(|symbol| {
        rules.iter().any(|rule| {
            rule.get("symbol").and_then(Value::as_str) == Some(symbol)
                && rule.get("state").and_then(Value::as_str) == Some("approved")
        })
    });
    if !all_approved {
        return Err(fail(
            Code::FormalRuleApprovalMissing,
            "one or more drafted formal rules lack a valid signed approval commit",
        ));
    }
    Ok(())
}

fn retries_exhausted(instance: &Instance) -> Check {
    let failures = instance
        .items
        .get("formalization_failure")
        .map_or(&[][..], Vec::as_slice);
    let mut consecutive_failures = 0;
    for (index, failure) in failures.iter().enumerate() {
        let item: FormalizationFailure =
            serde_json::from_value(failure.clone()).map_err(|error| {
                fail(
                    Code::ItemInvalid,
                    format!("formalization_failure item {index}: {error}"),
                )
            })?;
        if item.diagnostic.trim().is_empty() {
            return Err(fail(
                Code::ItemInvalid,
                "formalization_failure diagnostic must not be empty",
            ));
        }
        if !item.cleared {
            consecutive_failures += 1;
        }
        if item.id.as_deref().is_some_and(str::is_empty) {
            return Err(fail(
                Code::ItemInvalid,
                "formalization_failure id must not be empty",
            ));
        }
    }
    if consecutive_failures < 4 {
        return Err(fail(
            Code::FormalizationRetriesNotExhausted,
            "four consecutive failed formalization attempts are required before stopping",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ClaimSpan {
    authority: String,
    identity: String,
    start: u64,
    end: u64,
    symbol: String,
}

struct AnalysisTools<'a> {
    status: &'a Path,
    analyzer: &'a Path,
    z3: &'a Path,
    cvc5: &'a Path,
}

struct PreparedAnalysis {
    package_path: String,
    converted_package: Option<Vec<u8>>,
    drafts: Vec<ClaimSpan>,
    peers: Vec<ClaimSpan>,
}

fn analysis_consistent_with_approved(
    request: &FormalizeInput,
    context: &Context,
) -> Result<BTreeMap<String, Value>, Failure> {
    validate_analysis_package_path(request)?;
    let status = std::env::current_exe()
        .map_err(|error| fail(Code::FormalRuleStatusUnavailable, error.to_string()))?;
    let analyzer = configured_tool("FILAMENT_ANALYZE_BIN", Code::AnalysisInconclusive)?;
    let z3 = configured_tool("Z3_BIN", Code::AnalysisInconclusive)?;
    let cvc5 = configured_tool("CVC5_BIN", Code::AnalysisInconclusive)?;
    analysis_consistent_with_tools(
        request,
        context,
        &AnalysisTools {
            status: &status,
            analyzer: &analyzer,
            z3: &z3,
            cvc5: &cvc5,
        },
    )
}

fn analysis_consistent_with_tools(
    request: &FormalizeInput,
    context: &Context,
    tools: &AnalysisTools<'_>,
) -> Result<BTreeMap<String, Value>, Failure> {
    let mut prepared = prepare_analysis(request, context, tools.status)?;
    let trust = formal_rule_trust(request)?;
    let report_dir = tempfile::Builder::new()
        .prefix("quoin-analysis-")
        .tempdir_in(&context.config_root)
        .map_err(|error| {
            fail(
                Code::AnalysisInconclusive,
                format!("cannot create report directory: {error}"),
            )
        })?
        .keep();
    if let Some(bytes) = prepared.converted_package.take() {
        let package_path = report_dir.join("analysis-package.json");
        std::fs::write(&package_path, bytes).map_err(|error| {
            fail(
                Code::AnalysisInconclusive,
                format!("cannot write lowered analysis package: {error}"),
            )
        })?;
        prepared.package_path = package_path.to_string_lossy().into_owned();
    }
    let selection_path = write_draft_selection(&report_dir, &prepared.drafts)?;
    run_analysis_modes(
        &prepared,
        &report_dir,
        &selection_path,
        tools,
        trust.engineering_assurance,
    )
}

fn prepare_analysis(
    request: &FormalizeInput,
    context: &Context,
    status_executable: &Path,
) -> Result<PreparedAnalysis, Failure> {
    let package_path = validate_analysis_package_path(request)?;
    let bytes = read_analysis_package_bytes(Path::new(package_path))?;
    let native_claims = match checked_package_version(&bytes).as_deref() {
        Some("filament.checked-package/v2") => {
            Some(analysis_package::claim_spans(&bytes).map_err(checked_package_failure)?)
        }
        Some(version) if version.starts_with("filament.checked-package/") => {
            return Err(fail(
                Code::AnalysisPackageInvalid,
                format!("unsupported checked package version: {version}"),
            ));
        }
        _ => None,
    };
    let contract_package = if native_claims.is_none() {
        Some(read_contract_analysis_package(&bytes)?)
    } else {
        None
    };
    let status = formal_rule_status(request, context, status_executable)?;
    let expected = status_claim_spans(&status, request, context)?;
    let (package, converted_package) = if let Some(claims) = native_claims {
        let selected = bind_native_claims(&expected, &claims, request)?;
        let lowered = analysis_package::lower_selected_claims(&bytes, &selected)
            .map_err(checked_package_failure)?;
        (read_contract_analysis_package(&lowered)?, Some(lowered))
    } else {
        (
            contract_package.ok_or_else(|| {
                fail(
                    Code::AnalysisPackageInvalid,
                    "analysis package reader returned no package",
                )
            })?,
            None,
        )
    };
    let package_spans = package_claim_spans(&package)?;
    let bound = bind_package_claims(&expected, &package_spans, request)?;
    let drafts: Vec<_> = bound
        .iter()
        .filter(|span| request.drafted_symbols.contains(&span.symbol))
        .cloned()
        .collect();
    let peers: Vec<_> = bound
        .iter()
        .filter(|span| !request.drafted_symbols.contains(&span.symbol))
        .cloned()
        .collect();
    if drafts.is_empty() {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "drafted set has no source span for selection",
        ));
    }

    Ok(PreparedAnalysis {
        package_path: package_path.to_owned(),
        converted_package,
        drafts,
        peers,
    })
}

fn bind_native_claims(
    expected: &[ClaimSpan],
    claims: &[analysis_package::CheckedAnalysisClaimSpan],
    request: &FormalizeInput,
) -> Result<Vec<filament_contract_model::CheckedNodeId>, Failure> {
    if claims.is_empty() {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "checked package contains no source-mapped analysis claims",
        ));
    }
    let mut matched = BTreeSet::new();
    let mut selected = BTreeSet::new();
    for claim in claims {
        let candidates: Vec<_> = expected
            .iter()
            .filter(|span| {
                span.identity == claim.identity
                    && claim.start >= span.start
                    && claim.end <= span.end
            })
            .collect();
        if candidates.len() != 1 {
            return Err(fail(
                Code::AnalysisPackageInvalid,
                "checked-package claim does not map uniquely to a drafted or approved declaration",
            ));
        }
        let candidate = candidates.first().ok_or_else(|| {
            fail(
                Code::AnalysisPackageInvalid,
                "checked-package claim does not map uniquely to a drafted or approved declaration",
            )
        })?;
        matched.insert(candidate.symbol.clone());
        selected.insert(claim.node_id.clone());
    }
    if expected.iter().any(|span| !matched.contains(&span.symbol))
        || request
            .drafted_symbols
            .iter()
            .any(|symbol| !matched.contains(symbol))
    {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "checked package omits a drafted or approved claim",
        ));
    }
    Ok(selected.into_iter().collect())
}

fn validate_analysis_package_path(request: &FormalizeInput) -> Result<&str, Failure> {
    let package_path = request
        .analysis_package
        .as_deref()
        .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "analysis_package is required"))?;
    if package_path.is_empty()
        || package_path.contains('\0')
        || !Path::new(package_path).is_absolute()
    {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "analysis_package must be a non-empty absolute path without NUL",
        ));
    }
    Ok(package_path)
}

fn write_draft_selection(directory: &Path, drafts: &[ClaimSpan]) -> Result<PathBuf, Failure> {
    let selection_path = directory.join("selection.json");
    let selected: Vec<_> = drafts.iter().map(|span| serde_json::json!({
        "authority": span.authority, "identity": span.identity, "start": span.start, "end": span.end
    })).collect();
    let selection =
        serde_json::json!({"protocol":"filament-analyze.claim-selection/v1","spans":selected});
    let selection_bytes = serde_json::to_vec(&selection)
        .map_err(|error| fail(Code::AnalysisInconclusive, error.to_string()))?;
    std::fs::write(&selection_path, selection_bytes).map_err(|error| {
        fail(
            Code::AnalysisInconclusive,
            format!("cannot write selection: {error}"),
        )
    })?;
    Ok(selection_path)
}

fn run_analysis_modes(
    prepared: &PreparedAnalysis,
    report_dir: &Path,
    selection_path: &Path,
    tools: &AnalysisTools<'_>,
    producer: &str,
) -> Result<BTreeMap<String, Value>, Failure> {
    let mut reports = Vec::new();
    let invocation = AnalyzerInvocation {
        analyzer: tools.analyzer,
        package: &prepared.package_path,
        selection: selection_path,
        directory: report_dir,
        z3: tools.z3,
        cvc5: tools.cvc5,
        producer,
    };
    let run = run_analysis(&invocation, "consistency")?;
    reports.push(run.path.clone());
    if run.status == "refuted" {
        return Err(fail_with_details(
            Code::DraftClaimsInconsistent,
            "drafted claims are inconsistent",
            BTreeMap::from([("reports".to_owned(), serde_json::json!(reports))]),
        ));
    }
    let mut classification = "consistent";
    if !prepared.peers.is_empty() {
        let run = run_analysis(&invocation, "contradiction")?;
        reports.push(run.path.clone());
        if run.status == "satisfied" {
            return Err(fail_with_details(
                Code::DraftContradictsApproved,
                "draft contradicts an approved rule",
                BTreeMap::from([
                    (
                        "classification".to_owned(),
                        serde_json::json!("contradicts"),
                    ),
                    ("reports".to_owned(), serde_json::json!(reports)),
                ]),
            ));
        }
        if prepared
            .drafts
            .iter()
            .map(|span| &span.symbol)
            .collect::<BTreeSet<_>>()
            .len()
            == 1
        {
            let run = run_analysis(&invocation, "redundancy")?;
            reports.push(run.path.clone());
            if run.status == "satisfied" {
                classification = "redundant";
            }
        }
    }
    Ok(BTreeMap::from([
        (
            "classification".to_owned(),
            serde_json::json!(classification),
        ),
        ("reports".to_owned(), serde_json::json!(reports)),
        ("report_directory".to_owned(), serde_json::json!(report_dir)),
    ]))
}

fn formal_rule_status(
    request: &FormalizeInput,
    context: &Context,
    executable: &Path,
) -> Result<Value, Failure> {
    let trust = formal_rule_trust(request)?;
    let mut command = ProcessCommand::new(executable);
    command
        .args(["formal-rules", "status", "--repo"])
        .arg(&context.repo_root)
        .args(["--engineering-assurance", trust.engineering_assurance])
        .args(["--allowed-signers", trust.allowed_signers, "--json"]);
    if let Some((identity, issuer, principal)) = trust.gitsign {
        command
            .args(["--gitsign-certificate-identity", identity])
            .args(["--gitsign-certificate-oidc-issuer", issuer])
            .args(["--gitsign-principal", principal]);
    }
    let output = command
        .output()
        .map_err(|error| fail(Code::FormalRuleStatusUnavailable, error.to_string()))?;
    if !output.status.success() {
        return Err(fail(
            Code::FormalRuleStatusUnavailable,
            tool_message(&output),
        ));
    }
    let report: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| fail(Code::FormalRuleStatusInvalid, error.to_string()))?;
    if report.get("protocol").and_then(Value::as_str) != Some("quoin.formal-rule-status/v1")
        || report.get("rules").and_then(Value::as_array).is_none()
    {
        return Err(fail(
            Code::FormalRuleStatusInvalid,
            "formal-rules status has an invalid protocol or rules list",
        ));
    }
    Ok(report)
}

fn status_claim_spans(
    status: &Value,
    request: &FormalizeInput,
    context: &Context,
) -> Result<Vec<ClaimSpan>, Failure> {
    let rules = status
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| fail(Code::FormalRuleStatusInvalid, "rules missing"))?;
    let draft = FormalizationDraft::open(request, context)?;
    let mut spans = Vec::new();
    for rule in rules {
        let symbol = rule
            .get("symbol")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(Code::FormalRuleStatusInvalid, "rule symbol missing"))?;
        let selected = request
            .drafted_symbols
            .iter()
            .any(|candidate| candidate == symbol);
        let approved = rule.get("state").and_then(Value::as_str) == Some("approved");
        if !selected && !approved {
            continue;
        }
        if approved && !selected && !rule_formalizes_same_document(rule, &request.requirement_id) {
            continue;
        }
        if selected
            && !draft
                .relative
                .eq(rule.get("path").and_then(Value::as_str).unwrap_or_default())
        {
            return Err(fail(
                Code::FormalRuleStatusInvalid,
                "draft symbol path differs from checked draft",
            ));
        }
        let path = rule
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(Code::FormalRuleStatusInvalid, "rule path missing"))?;
        let range = rule
            .get("declaration_range")
            .ok_or_else(|| fail(Code::FormalRuleStatusInvalid, "declaration_range missing"))?;
        let start = range
            .get("start")
            .and_then(Value::as_u64)
            .ok_or_else(|| fail(Code::FormalRuleStatusInvalid, "declaration start missing"))?;
        let end = range
            .get("end")
            .and_then(Value::as_u64)
            .ok_or_else(|| fail(Code::FormalRuleStatusInvalid, "declaration end missing"))?;
        if end <= start || (selected && rule.get("state").and_then(Value::as_str) != Some("draft"))
        {
            return Err(fail(
                Code::FormalRuleStatusInvalid,
                "invalid declaration range or draft state",
            ));
        }
        spans.push(ClaimSpan {
            authority: "file".to_owned(),
            identity: path.to_owned(),
            start,
            end,
            symbol: symbol.to_owned(),
        });
    }
    if !request
        .drafted_symbols
        .iter()
        .all(|symbol| spans.iter().any(|span| &span.symbol == symbol))
    {
        return Err(fail(
            Code::FormalRuleApprovalMissing,
            "status report omits a drafted symbol",
        ));
    }
    Ok(spans)
}

fn rule_formalizes_same_document(rule: &Value, requirement_id: &str) -> bool {
    let target = requirement_document_id(requirement_id);
    rule.get("formalizes")
        .and_then(Value::as_array)
        .is_some_and(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .any(|id| requirement_document_id(id) == target)
        })
}

fn requirement_document_id(id: &str) -> &str {
    id.split_once("-AC-").map_or(id, |(document, _)| document)
}

fn read_analysis_package_bytes(path: &Path) -> Result<Vec<u8>, Failure> {
    let invalid = |message: String| fail(Code::AnalysisPackageInvalid, message);
    let metadata = std::fs::metadata(path).map_err(|error| invalid(error.to_string()))?;
    if !metadata.is_file() {
        return Err(invalid("analysis package is not a regular file".to_owned()));
    }
    let limits = CheckedPackageReadLimits::bounded();
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| {
            file.take(limits.bytes.saturating_add(1))
                .read_to_end(&mut bytes)
        })
        .map_err(|error| invalid(error.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(invalid(
            "analysis package exceeds the contract reader byte limit".to_owned(),
        ));
    }
    Ok(bytes)
}

fn checked_package_version(bytes: &[u8]) -> Option<String> {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return None;
    };
    value
        .get("contract_version")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn read_contract_analysis_package(bytes: &[u8]) -> Result<ContractPackageV1, Failure> {
    let limits = CheckedPackageReadLimits::bounded();
    admit_analysis_package(ContractPackageWireV1::read(
        bytes,
        limits,
        &BTreeMap::default(),
    ))
}

fn admit_analysis_package(result: ContractPackageReadResult) -> Result<ContractPackageV1, Failure> {
    match result {
        ContractPackageReadResult::Admitted(package) => Ok(*package),
        ContractPackageReadResult::Refused(refusal) => Err(fail(
            Code::AnalysisPackageInvalid,
            format!("contract package was refused: {refusal}"),
        )),
        ContractPackageReadResult::Incomplete(incomplete) => Err(fail(
            Code::AnalysisPackageInvalid,
            format!("contract package exhausted a read limit: {incomplete:?}"),
        )),
        ContractPackageReadResult::CatalogUnavailable(error) => {
            Err(catalog_unavailable_failure(&error))
        }
    }
}

fn catalog_unavailable_failure(error: &CatalogReadError) -> Failure {
    fail(
        Code::CatalogUnavailable,
        format!("contract package reader is unavailable: {error}"),
    )
}

fn checked_package_failure(error: analysis_package::CheckedPackageError) -> Failure {
    match error {
        analysis_package::CheckedPackageError::Invalid(message) => {
            fail(Code::AnalysisPackageInvalid, message)
        }
        analysis_package::CheckedPackageError::CatalogUnavailable(message) => {
            fail(Code::CatalogUnavailable, message)
        }
    }
}

fn package_claim_spans(package: &ContractPackageV1) -> Result<Vec<ClaimSpan>, Failure> {
    let wire = package.wire();
    let nodes = serde_json::to_value(&wire.semantic_graph.nodes).map_err(|error| {
        fail(
            Code::AnalysisPackageInvalid,
            format!("cannot inspect admitted semantic graph: {error}"),
        )
    })?;
    let nodes = nodes.as_array().ok_or_else(|| {
        fail(
            Code::AnalysisPackageInvalid,
            "admitted semantic graph has no nodes",
        )
    })?;
    let map = serde_json::to_value(&wire.source_map.nodes).map_err(|error| {
        fail(
            Code::AnalysisPackageInvalid,
            format!("cannot inspect admitted source map: {error}"),
        )
    })?;
    let map = map.as_array().ok_or_else(|| {
        fail(
            Code::AnalysisPackageInvalid,
            "admitted source map has no nodes",
        )
    })?;
    let claim_ids: BTreeSet<String> = nodes
        .iter()
        .filter(|node| {
            node.get("node_tag").and_then(Value::as_str) == Some("claim")
                && node.get("semantic_form").and_then(Value::as_str) == Some("analysis_claim")
        })
        .map(|node| {
            serde_json::to_string(node.get("node_id").unwrap_or(&Value::Null)).unwrap_or_default()
        })
        .collect();
    if claim_ids.is_empty() {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "package contains no analysis claims",
        ));
    }
    let mut spans = Vec::new();
    for entry in map {
        let id =
            serde_json::to_string(entry.get("node_id").unwrap_or(&Value::Null)).unwrap_or_default();
        if !claim_ids.contains(&id) {
            continue;
        }
        let occurrences = entry
            .get("occurrences")
            .and_then(Value::as_array)
            .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "claim occurrences missing"))?;
        for occurrence in occurrences {
            let regions = occurrence
                .get("regions")
                .and_then(Value::as_array)
                .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "claim regions missing"))?;
            for region in regions {
                let source = region
                    .get("source")
                    .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "claim source missing"))?;
                let authority =
                    source
                        .get("authority")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            fail(Code::AnalysisPackageInvalid, "source authority missing")
                        })?;
                let identity = source
                    .get("identity")
                    .and_then(Value::as_str)
                    .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "source identity missing"))?;
                let start = region
                    .get("start")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "source start missing"))?;
                let end = region
                    .get("end")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "source end missing"))?;
                spans.push(ClaimSpan {
                    authority: authority.to_owned(),
                    identity: identity.to_owned(),
                    start,
                    end,
                    symbol: String::new(),
                });
            }
        }
    }
    if spans.is_empty() {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "package claims have no source spans",
        ));
    }
    Ok(spans)
}

fn bind_package_claims(
    expected: &[ClaimSpan],
    package: &[ClaimSpan],
    request: &FormalizeInput,
) -> Result<Vec<ClaimSpan>, Failure> {
    let mut matched = BTreeSet::new();
    let mut bound = Vec::new();
    for claim in package {
        let candidates: Vec<_> = expected
            .iter()
            .filter(|span| {
                span.identity == claim.identity
                    && claim.start >= span.start
                    && claim.end <= span.end
            })
            .collect();
        if candidates.len() != 1 {
            return Err(fail(
                Code::AnalysisPackageInvalid,
                "claim source span does not map uniquely to a drafted or approved declaration",
            ));
        }
        let candidate = candidates.first().ok_or_else(|| {
            fail(
                Code::AnalysisPackageInvalid,
                "claim source span does not map uniquely to a drafted or approved declaration",
            )
        })?;
        matched.insert(candidate.symbol.clone());
        bound.push(ClaimSpan {
            authority: claim.authority.clone(),
            identity: claim.identity.clone(),
            start: claim.start,
            end: claim.end,
            symbol: candidate.symbol.clone(),
        });
    }
    if expected.iter().any(|span| !matched.contains(&span.symbol))
        || request
            .drafted_symbols
            .iter()
            .any(|symbol| !matched.contains(symbol))
    {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "package omits a drafted or approved claim",
        ));
    }
    Ok(bound)
}

struct AnalysisRun {
    status: String,
    path: PathBuf,
}
struct AnalyzerInvocation<'a> {
    analyzer: &'a Path,
    package: &'a str,
    selection: &'a Path,
    directory: &'a Path,
    z3: &'a Path,
    cvc5: &'a Path,
    producer: &'a str,
}

fn run_analysis(invocation: &AnalyzerInvocation<'_>, mode: &str) -> Result<AnalysisRun, Failure> {
    let path = invocation.directory.join(format!("{mode}.json"));
    let arguments = vec![
        "run-package".to_owned(),
        "--package".to_owned(),
        invocation.package.to_owned(),
        "--output".to_owned(),
        path.to_string_lossy().into_owned(),
        "--z3".to_owned(),
        invocation.z3.to_string_lossy().into_owned(),
        "--cvc5".to_owned(),
        invocation.cvc5.to_string_lossy().into_owned(),
        "--producer-execute".to_owned(),
        invocation.producer.to_owned(),
        "--request".to_owned(),
        mode.to_owned(),
        "--select".to_owned(),
        invocation.selection.to_string_lossy().into_owned(),
    ];
    let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_tool(
        invocation.analyzer,
        &arguments,
        invocation.directory,
        Instant::now() + Duration::from_secs(20),
    )
    .map_err(|error| fail(Code::AnalysisInconclusive, error))?;
    let bytes = std::fs::read(&path).map_err(|error| {
        fail(
            Code::AnalysisInconclusive,
            format!(
                "analysis report unavailable: {error}; {}",
                tool_message(&output)
            ),
        )
    })?;
    let report: Value = serde_json::from_slice(&bytes).map_err(|error| {
        fail(
            Code::AnalysisInconclusive,
            format!("analysis report invalid: {error}"),
        )
    })?;
    let status = analysis_report_status(&report, mode, output.status.code())?;
    Ok(AnalysisRun { status, path })
}

fn analysis_report_status(
    report: &Value,
    mode: &str,
    exit_status: Option<i32>,
) -> Result<String, Failure> {
    if report.get("analysisKind").and_then(Value::as_str) != Some(mode) {
        return Err(fail(
            Code::AnalysisInconclusive,
            "analysis report names a different request mode",
        ));
    }
    let differential = report.get("differential").ok_or_else(|| {
        fail(
            Code::AnalysisInconclusive,
            "analysis report lacks differential",
        )
    })?;
    let selection = report
        .get("selection")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            fail(
                Code::AnalysisInconclusive,
                "analysis report lacks selected and excluded claims",
            )
        })?;
    let selected = selection
        .get("selected")
        .and_then(Value::as_array)
        .filter(|claims| claims.iter().all(Value::is_string))
        .ok_or_else(|| {
            fail(
                Code::AnalysisInconclusive,
                "analysis report selection has invalid selected claims",
            )
        })?;
    if selected.is_empty()
        || !selection
            .get("excluded")
            .and_then(Value::as_array)
            .is_some_and(|claims| claims.iter().all(Value::is_string))
    {
        return Err(fail(
            Code::AnalysisInconclusive,
            "analysis report selection has invalid selected or excluded claims",
        ));
    }
    if differential.get("disposition").and_then(Value::as_str) != Some("agreement") {
        return Err(fail(
            Code::AnalysisInconclusive,
            "analysis engines did not agree",
        ));
    }
    let status = differential
        .get("agreedStatus")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            fail(
                Code::AnalysisInconclusive,
                "analysis report lacks agreedStatus",
            )
        })?;
    if !matches!(status, "satisfied" | "refuted") {
        return Err(fail(
            Code::AnalysisInconclusive,
            "analysis report is inconclusive",
        ));
    }
    let expected_exit = if status == "satisfied" {
        Some(0)
    } else {
        Some(1)
    };
    if exit_status != expected_exit {
        return Err(fail(
            Code::AnalysisInconclusive,
            "analysis exit status disagrees with its report",
        ));
    }
    Ok(status.to_owned())
}

fn formalization_check(request: &FormalizeInput, context: &Context) -> Check {
    let fsl = configured_tool("FSL_DRIVER_BIN", Code::QslCheckUnavailable)?;
    let quire = configured_tool("QUIRE_DRIVER_BIN", Code::FormalizesMarkerUnavailable)?;
    let catalog = Catalog::load(context)?;
    formalization_check_with_tools(request, context, &fsl, &quire, &catalog.modules)
}

fn formalization_check_with_tools(
    request: &FormalizeInput,
    context: &Context,
    fsl: &Path,
    quire: &Path,
    modules: &ModuleSelection,
) -> Check {
    if request.id.as_deref().is_some_and(|id| id != "request")
        || request
            .interview_id
            .as_deref()
            .is_some_and(|id| id != "request")
        || request.answered_at.as_deref().is_some_and(str::is_empty)
    {
        return Err(fail(
            Code::ItemInvalid,
            "formalization_request interview metadata is inconsistent",
        ));
    }
    let draft = FormalizationDraft::open(request, context)?;
    let deadline = Instant::now() + TOOL_DEADLINE;
    let (check_result, trace_result) =
        run_formalization_tools(fsl, quire, &draft, modules, deadline)?;
    let check_output = check_result.map_err(|message| fail(Code::QslCheckUnavailable, message))?;
    check_fsl_result(&check_output, &draft)?;
    let trace_output =
        trace_result.map_err(|message| fail(Code::FormalizesMarkerUnavailable, message))?;
    check_formalizes_result(&trace_output, request, &draft)
}

struct FormalizationDraft {
    root: PathBuf,
    path: PathBuf,
    text: String,
    relative: String,
}

impl FormalizationDraft {
    fn open(request: &FormalizeInput, context: &Context) -> Result<Self, Failure> {
        if request.requirement_id.trim().is_empty() || request.requirement_id.len() > 128 {
            return Err(fail(
                Code::ItemInvalid,
                "formalization_request.requirement_id must contain 1..128 characters",
            ));
        }
        let repo_root = context
            .repo_root
            .canonicalize()
            .map_err(|error| fail(Code::ContextInvalid, error.to_string()))?;
        let draft = scoped_path(&repo_root, &request.draft_path)?;
        if draft.extension().and_then(|value| value.to_str()) != Some("native") {
            return Err(fail(
                Code::PathInvalid,
                "formalization draft must be a .native file",
            ));
        }
        let draft_text = read_bounded(&draft)?;
        let relative = draft
            .strip_prefix(&repo_root)
            .map_err(|_| fail(Code::PathInvalid, "draft escapes repository"))?
            .to_str()
            .ok_or_else(|| fail(Code::PathInvalid, "draft path is not UTF-8"))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        Ok(Self {
            root: repo_root,
            path: draft,
            text: draft_text,
            relative,
        })
    }
}

fn run_formalization_tools(
    fsl: &Path,
    quire: &Path,
    draft: &FormalizationDraft,
    modules: &ModuleSelection,
    deadline: Instant,
) -> Result<ToolResults, Failure> {
    let draft_path = draft
        .path
        .to_str()
        .ok_or_else(|| fail(Code::PathInvalid, "draft path is not UTF-8"))?;
    let module_paths = match modules {
        ModuleSelection::Closed(roots) => roots
            .iter()
            .map(|root| {
                root.as_path().to_str().map(str::to_owned).ok_or_else(|| {
                    fail(
                        Code::FormalizesMarkerUnavailable,
                        "module path is not UTF-8",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        ModuleSelection::ScopeOrAmbient | ModuleSelection::Ambient => {
            return Err(fail(
                Code::FormalizesMarkerUnavailable,
                "catalog did not provide a closed module set",
            ));
        }
    };
    let scope = draft
        .root
        .to_str()
        .ok_or_else(|| fail(Code::ContextInvalid, "repository path is not UTF-8"))?;
    let (check_result, trace_result) = std::thread::scope(|threads| {
        let check = threads.spawn(|| {
            run_tool(
                fsl,
                &["check", draft_path, "--format", "json"],
                &draft.root,
                deadline,
            )
        });
        let trace = threads.spawn(|| {
            let mut arguments = vec!["trace".to_owned(), "--scope".to_owned(), scope.to_owned()];
            for module_path in &module_paths {
                arguments.push("--module".to_owned());
                arguments.push(module_path.clone());
            }
            arguments.extend([
                "--file".to_owned(),
                draft.relative.clone(),
                "--format".to_owned(),
                "json".to_owned(),
            ]);
            let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            run_tool(quire, &arguments, &draft.root, deadline)
        });
        (
            check
                .join()
                .unwrap_or_else(|_| Err("fsl check process panicked".to_owned())),
            trace
                .join()
                .unwrap_or_else(|_| Err("quire trace process panicked".to_owned())),
        )
    });
    Ok((check_result, trace_result))
}

fn check_fsl_result(output: &std::process::Output, draft: &FormalizationDraft) -> Check {
    let check_document: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        fail_with_details(
            Code::QslCheckFailed,
            format!("fsl check returned invalid JSON: {error}"),
            BTreeMap::from([
                ("file".to_owned(), Value::String(draft.relative.clone())),
                (
                    "diagnostics".to_owned(),
                    Value::String(tool_message(output)),
                ),
            ]),
        )
    })?;
    let diagnostic_values = check_document
        .get("diagnostics")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            fail(
                Code::QslCheckFailed,
                "fsl check result has no diagnostics array",
            )
        })?;
    let diagnostics = check_diagnostics(diagnostic_values, &draft.text, &draft.relative);
    let check_passed = output.status.success()
        && check_document.get("format").and_then(Value::as_str) == Some("filament-outcome/1")
        && check_document.get("operation").and_then(Value::as_str) == Some("check")
        && check_document.get("category").and_then(Value::as_str) == Some("success")
        && diagnostics.as_array().is_some_and(Vec::is_empty);
    if !check_passed {
        return Err(fail_with_details(
            Code::QslCheckFailed,
            "fsl check did not return a clean check outcome",
            BTreeMap::from([
                ("file".to_owned(), Value::String(draft.relative.clone())),
                ("diagnostics".to_owned(), diagnostics),
                (
                    "exit_code".to_owned(),
                    output.status.code().map_or(Value::Null, Value::from),
                ),
            ]),
        ));
    }
    Ok(())
}

fn check_formalizes_result(
    output: &std::process::Output,
    request: &FormalizeInput,
    draft: &FormalizationDraft,
) -> Check {
    if !output.status.success() {
        return Err(fail_with_details(
            Code::FormalizesMarkerUnavailable,
            "quire trace could not verify the formalizes marker",
            BTreeMap::from([(
                "diagnostics".to_owned(),
                Value::String(tool_message(output)),
            )]),
        ));
    }
    let trace: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        fail(
            Code::FormalizesMarkerUnavailable,
            format!("quire trace returned invalid JSON: {error}"),
        )
    })?;
    let query_matches = trace
        .get("query")
        .and_then(|query| query.get("kind"))
        .and_then(Value::as_str)
        == Some("file")
        && trace
            .get("query")
            .and_then(|query| query.get("path"))
            .and_then(Value::as_str)
            == Some(draft.relative.as_str());
    if !query_matches {
        return Err(fail(
            Code::FormalizesMarkerUnavailable,
            "quire trace result did not describe the requested draft file",
        ));
    }
    let relations = trace
        .get("claims")
        .and_then(|claims| claims.get("formalizes"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            fail(
                Code::FormalizesMarkerUnavailable,
                "quire trace result has no formalizes relation list",
            )
        })?;
    let marker_matches = relations.iter().any(|relation| {
        relation.get("trace_id").and_then(Value::as_str) == Some(request.requirement_id.as_str())
            && relation.get("path").and_then(Value::as_str) == Some(draft.relative.as_str())
    });
    if !marker_matches {
        return Err(fail_with_details(
            Code::FormalizesMarkerMissing,
            "draft has no bound formalizes relation for the requested requirement",
            BTreeMap::from([
                ("file".to_owned(), Value::String(draft.relative.clone())),
                (
                    "requirement_id".to_owned(),
                    Value::String(request.requirement_id.clone()),
                ),
            ]),
        ));
    }
    Ok(())
}

fn configured_tool(name: &str, code: Code) -> Result<PathBuf, Failure> {
    let path = std::env::var_os(name).map(PathBuf::from).ok_or_else(|| {
        fail(
            code,
            format!("{name} must name an absolute executable path"),
        )
    })?;
    if !path.is_absolute() || !path.is_file() {
        return Err(fail(
            code,
            format!("{name} must name an absolute executable file"),
        ));
    }
    Ok(path)
}

fn run_tool(
    program: &Path,
    arguments: &[&str],
    working_directory: &Path,
    deadline: Instant,
) -> Result<std::process::Output, String> {
    let mut child = ProcessCommand::new(program)
        .args(arguments)
        .current_dir(working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", program.display()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "tool stdout unavailable".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "tool stderr unavailable".to_owned())?;
    let read = |stream: Box<dyn Read + Send>| {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = stream
                .take(MAX_TOOL_OUTPUT + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes);
            let _ = sender.send(result);
        });
        receiver
    };
    let stdout = read(Box::new(stdout));
    let stderr = read(Box::new(stderr));
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child.wait().map_err(|error| error.to_string())?;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout
        .recv_timeout(Duration::from_millis(250))
        .map_err(|_| "tool stdout did not close".to_owned())?
        .map_err(|error| error.to_string())?;
    let stderr = stderr
        .recv_timeout(Duration::from_millis(250))
        .map_err(|_| "tool stderr did not close".to_owned())?
        .map_err(|error| error.to_string())?;
    if timed_out {
        return Err("tool exceeded the 4 second workflow invariant deadline".to_owned());
    }
    if u64::try_from(stdout.len()).unwrap_or(u64::MAX) > MAX_TOOL_OUTPUT
        || u64::try_from(stderr.len()).unwrap_or(u64::MAX) > MAX_TOOL_OUTPUT
    {
        return Err("tool exceeded its 1 MiB output bound".to_owned());
    }
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

fn tool_message(output: &std::process::Output) -> String {
    let text = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    String::from_utf8_lossy(text).chars().take(4096).collect()
}

fn check_diagnostics(diagnostics: &[Value], source: &str, file: &str) -> Value {
    Value::Array(
        diagnostics
            .iter()
            .map(|diagnostic| {
                let locus = diagnostic.get("locus");
                let line = locus
                    .and_then(|locus| locus.get("span"))
                    .and_then(|span| span.get("start"))
                    .and_then(|start| start.get("line"))
                    .and_then(Value::as_u64)
                    .or_else(|| {
                        locus
                            .and_then(|locus| locus.get("start"))
                            .and_then(Value::as_u64)
                            .and_then(|offset| source_line(source, offset))
                    });
                serde_json::json!({
                    "file": file,
                    "line": line,
                    "message": diagnostic.get("message").and_then(Value::as_str).unwrap_or("QSL check diagnostic")
                })
            })
            .collect(),
    )
}

fn source_line(source: &str, offset: u64) -> Option<u64> {
    let offset = usize::try_from(offset).ok()?;
    let prefix = source.as_bytes().get(..offset)?;
    let lines = prefix
        .split(|byte| *byte == b'\n')
        .count()
        .saturating_sub(1);
    u64::try_from(lines + 1).ok()
}

struct Catalog {
    entries: Vec<Value>,
    modules: ModuleSelection,
}

impl Catalog {
    fn load(context: &Context) -> Result<Self, Failure> {
        let arguments = clap::Command::new("quoin")
            .arg(clap::Arg::new("config_root").long("config-root"))
            .try_get_matches_from([
                "quoin".into(),
                "--config-root".into(),
                context.config_root.as_os_str().to_owned(),
            ])
            .map_err(|e| fail(Code::ContextInvalid, e.to_string()))?;
        let response =
            invocation::with_current(invocation::Invocation::from_matches(&arguments), || {
                core_runtime::invoke("catalog.load", &serde_json::json!({}))
            })
            .map_err(|e| fail(Code::CatalogUnavailable, e))?;
        if !response.outcome.carries_payload() {
            return Err(fail(
                Code::CatalogUnavailable,
                response
                    .diagnostics
                    .iter()
                    .map(|d| format!("{d:?}"))
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        let entries = response
            .payload
            .get("entries")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| fail(Code::CatalogUnavailable, "catalog returned no entries"))?;
        let modules = response
            .payload
            .get("modules")
            .and_then(Value::as_array)
            .ok_or_else(|| fail(Code::CatalogUnavailable, "catalog returned no modules"))?
            .iter()
            .map(|module| {
                let path = module
                    .get("root")
                    .and_then(Value::as_str)
                    .ok_or_else(|| fail(Code::CatalogUnavailable, "module has no root"))?;
                ModuleRoot::open(path).map_err(|e| fail(Code::CatalogUnavailable, e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            entries,
            modules: ModuleSelection::Closed(modules),
        })
    }

    fn entry(&self, name: &str) -> Result<&Value, Failure> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.get("name").and_then(Value::as_str) == Some(name))
            .collect();
        if entries.len() != 1 {
            return Err(fail(
                Code::CatalogUnavailable,
                format!("exactly one {name} contract is required"),
            ));
        }
        entries
            .first()
            .copied()
            .ok_or_else(|| fail(Code::CatalogUnavailable, name))
    }

    fn analyses(&self) -> Result<BTreeSet<String>, Failure> {
        let path = self
            .entry("SpecReview")?
            .get("schemaPath")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(Code::CatalogUnavailable, "SpecReview schemaPath is absent"))?;
        let schema: Value = serde_json::from_str(&read_bounded(Path::new(path))?)
            .map_err(|e| fail(Code::CatalogUnavailable, e.to_string()))?;
        schema
            .pointer("/properties/analysis/enum")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                fail(
                    Code::CatalogUnavailable,
                    "SpecReview.analysis has no declared vocabulary",
                )
            })?
            .iter()
            .map(|v| {
                v.as_str().map(str::to_owned).ok_or_else(|| {
                    fail(
                        Code::CatalogUnavailable,
                        "analysis vocabulary must contain strings",
                    )
                })
            })
            .collect()
    }

    fn document(&self, context: &Context, path: &Path, expected: &str) -> Result<Value, Failure> {
        self.entry(expected)?;
        let path = scoped_path(&context.repo_root, path)?;
        let text = read_bounded(&path)?;
        let frontmatter = validate::frontmatter(&text).ok_or_else(|| {
            fail(
                Code::DocumentInvalid,
                format!("{}: frontmatter is absent or malformed", path.display()),
            )
        })?;
        if frontmatter.get("type").and_then(Value::as_str) != Some(expected) {
            return Err(fail(
                Code::DocumentInvalid,
                format!("{} must declare type: {expected}", path.display()),
            ));
        }
        let outcome = validate::run(&validate::Request {
            scope: ScopeRoot::open(&context.repo_root)
                .map_err(|e| fail(Code::ContextInvalid, e.to_string()))?,
            modules: self.modules.clone(),
            documents: vec![path.clone()],
            bundle_package: quoin_config::package_from_git_config(&context.repo_root),
        })
        .map_err(|e| fail(Code::DocumentInvalid, e.to_string()))?;
        if outcome.evaluations.len() != 1
            || outcome
                .evaluations
                .iter()
                .any(|e| e.outcome != validate::DocumentOutcome::Pass)
        {
            return Err(fail(
                Code::DocumentInvalid,
                format!("{}: {:?}", path.display(), outcome.evaluations),
            ));
        }
        Ok(Value::Object(frontmatter))
    }
}

fn selection(review: &ReviewInput, catalog: &Catalog) -> Result<BTreeSet<String>, Failure> {
    let _ = (&review.id, &review.interview_id, &review.answered_at);
    let supported = catalog.analyses()?;
    let chosen: BTreeSet<_> = review.selected_analyses.iter().cloned().collect();
    if chosen.len() != review.selected_analyses.len()
        || chosen.iter().any(|s| !LENSES.contains(&s.as_str()))
    {
        return Err(fail(
            Code::SelectionInvalid,
            "selected analyses must be unique specification lenses",
        ));
    }
    let mut expected = match review.review_set.as_str() {
        "base" if chosen.is_empty() => BTreeSet::new(),
        "all" if chosen.is_empty() || chosen == LENSES.into_iter().map(str::to_owned).collect() => {
            LENSES.into_iter().map(str::to_owned).collect()
        }
        "subset" if !chosen.is_empty() => chosen,
        _ => {
            return Err(fail(
                Code::SelectionInvalid,
                "expected base, all, or a nonempty subset; all cannot be truncated",
            ));
        }
    };
    expected.insert("base".to_owned());
    if !expected.is_subset(&supported) {
        return Err(fail(
            Code::AnalysisUnsupported,
            format!(
                "catalog does not support {:?}",
                expected.difference(&supported).collect::<Vec<_>>()
            ),
        ));
    }
    Ok(expected)
}

fn profiles(review: &ReviewInput, context: &Context, catalog: &Catalog) -> Check {
    if review.profile_disposition.trim().is_empty() {
        return Err(fail(
            Code::ProfileDispositionRequired,
            "record applicable profiles or the reason none applies",
        ));
    }
    if review.assurance_profiles.len() > MAX_DOCUMENTS {
        return Err(fail(Code::DocumentLimit, "too many applicable profiles"));
    }
    for profile in &review.assurance_profiles {
        let doc = catalog.document(context, profile, "AssuranceProfile")?;
        if doc.get("status").and_then(Value::as_str) != Some("active") {
            return Err(fail(
                Code::ProfileInvalid,
                "applicable profiles must be active",
            ));
        }
        if doc.get("review_selection").is_some() {
            return Err(fail(
                Code::ProfileInvalid,
                "review_selection is obsolete; use the installed review_policy contract",
            ));
        }
        // The live schema validates review_policy. Its operations are distinct
        // from specification lenses, so no profile invents a lens selection.
        if let Some(policy) = doc.get("review_policy") {
            let mode = policy.get("mode").and_then(Value::as_str);
            if !matches!(mode, Some("require" | "recommend")) {
                return Err(fail(Code::ProfileInvalid, "invalid review_policy.mode"));
            }
        }
    }
    Ok(())
}

fn review_documents(instance: &Instance, context: &Context) -> Check {
    let review: ReviewInput = one(instance, "operation_request")?;
    let catalog = Catalog::load(context)?;
    profiles(&review, context, &catalog)?;
    let expected = selection(&review, &catalog)?;
    let docs = instance
        .items
        .get("review_doc")
        .map_or(&[][..], Vec::as_slice);
    if docs.len() > MAX_DOCUMENTS {
        return Err(fail(Code::DocumentLimit, "too many review documents"));
    }
    let mut found = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for value in docs {
        let doc: ReviewDoc = serde_json::from_value(value.clone())
            .map_err(|e| fail(Code::ItemInvalid, e.to_string()))?;
        if doc.id.is_empty()
            || !expected.contains(&doc.analysis)
            || !found.insert(doc.analysis.clone())
        {
            return Err(fail(
                Code::ReviewMismatch,
                "unexpected or duplicate analysis record",
            ));
        }
        if !paths.insert(scoped_path(&context.repo_root, &doc.path)?) {
            return Err(fail(
                Code::ReviewMismatch,
                "one document cannot stand for multiple analyses",
            ));
        }
        let frontmatter = catalog.document(context, &doc.path, "SpecReview")?;
        if frontmatter.get("analysis").and_then(Value::as_str) != Some(doc.analysis.as_str()) {
            return Err(fail(
                Code::ReviewMismatch,
                "recorded analysis differs from the document",
            ));
        }
    }
    let missing: Vec<_> = expected.difference(&found).collect();
    if !missing.is_empty() {
        return Err(fail(
            Code::AnalysesMissing,
            format!("missing validated reviews: {missing:?}"),
        ));
    }
    Ok(())
}

fn plan_paths(plan: &PlanInput, context: &Context) -> Check {
    let _ = (&plan.id, &plan.interview_id, &plan.answered_at);
    if plan.readiness_notes.trim().is_empty()
        || plan.task_paths.is_empty()
        || plan.task_paths.len() > MAX_DOCUMENTS
    {
        return Err(fail(
            Code::PlanIncomplete,
            "readiness notes and 1..256 task paths are required",
        ));
    }
    let bundle = scoped_parent(&context.repo_root, &plan.plan_path)?;
    if scoped_parent(&context.repo_root, &plan.index_path)? != bundle {
        return Err(fail(
            Code::PlanIncomplete,
            "plan and index must share a bundle directory",
        ));
    }
    for task in &plan.task_paths {
        if !scoped_parent(&context.repo_root, task)?.starts_with(bundle.join("tasks")) {
            return Err(fail(
                Code::PlanIncomplete,
                "tasks must belong to the bundle's tasks directory",
            ));
        }
    }
    Ok(())
}

fn plan_documents(instance: &Instance, context: &Context) -> Check {
    let plan: PlanInput = one(instance, "plan_answers")?;
    plan_paths(&plan, context)?;
    let catalog = Catalog::load(context)?;
    catalog.document(context, &plan.plan_path, "Plan")?;
    catalog.document(context, &plan.index_path, "index")?;
    let mut paths = BTreeSet::new();
    for task in &plan.task_paths {
        if !paths.insert(scoped_path(&context.repo_root, task)?) {
            return Err(fail(Code::PlanIncomplete, "duplicate task"));
        }
        catalog.document(context, task, "Task")?;
    }
    Ok(())
}

mod paths;
#[cfg(test)]
mod tests;

use paths::{read_bounded, scoped_parent, scoped_path};
