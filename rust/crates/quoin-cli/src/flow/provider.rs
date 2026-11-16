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

use quoin_quire::{ModuleRoot, ModuleSelection, ScopeRoot, validate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{core_runtime, invocation};

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
    instance: Instance,
    evaluated_at: String,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<BTreeMap<String, Value>>,
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

fn evaluate(request: &Request) -> ResultEnvelope {
    let outcomes = request
        .invariants
        .iter()
        .map(|name| match check_with_details(&request.instance, name) {
            Ok(None) => Outcome::Passed {
                invariant: name.clone(),
                details: None,
            },
            Ok(Some(details)) => Outcome::Passed {
                invariant: name.clone(),
                details: Some(details),
            },
            Err(error) => Outcome::Failed {
                invariant: name.clone(),
                code: error.code,
                details: error.details,
            },
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
) -> Result<Option<BTreeMap<String, Value>>, Failure> {
    if !matches!(instance.name.as_str(), "review" | "to-plan" | "formalize") {
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
        (_, "human_acceptance_required") => {
            if instance
                .gates
                .get("validated->accepted")
                .map(String::as_str)
                != Some("hitl")
            {
                return Err(fail(
                    Code::HumanGateRequired,
                    "validated->accepted must use hitl",
                ));
            }
            Ok(None)
        }
        _ => Err(fail(Code::InvariantUnknown, name)),
    }
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
    let trust = formal_rule_trust(request)?;
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
    let output = command.output().map_err(|error| {
        fail(
            Code::FormalRuleStatusUnavailable,
            format!("quoin formal-rules status is unavailable: {error}"),
        )
    })?;
    if !output.status.success() {
        return Err(fail(
            Code::FormalRuleStatusUnavailable,
            format!(
                "quoin formal-rules status failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    let report: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        fail(
            Code::FormalRuleStatusInvalid,
            format!("quoin formal-rules status returned invalid JSON: {error}"),
        )
    })?;
    if report.get("protocol").and_then(Value::as_str) != Some("quoin.formal-rule-status/v1") {
        return Err(fail(
            Code::FormalRuleStatusInvalid,
            "quoin formal-rules status returned an unsupported protocol",
        ));
    }
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

fn analysis_consistent_with_approved(
    request: &FormalizeInput,
    context: &Context,
) -> Result<BTreeMap<String, Value>, Failure> {
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
    let package_text = read_bounded(Path::new(package_path))
        .map_err(|error| fail(Code::AnalysisPackageInvalid, error.to_string()))?;
    let package: Value = serde_json::from_str(&package_text).map_err(|error| {
        fail(
            Code::AnalysisPackageInvalid,
            format!("analysis package is invalid JSON: {error}"),
        )
    })?;
    if package.get("contract_version").and_then(Value::as_str)
        != Some("filament.contract-package/v1")
    {
        return Err(fail(
            Code::AnalysisPackageInvalid,
            "analysis_package must use filament.contract-package/v1",
        ));
    }
    let executable = std::env::current_exe()
        .map_err(|error| fail(Code::FormalRuleStatusUnavailable, error.to_string()))?;
    let status = formal_rule_status(request, context, &executable)?;
    let expected = status_claim_spans(&status, request, context)?;
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

    let analyzer = configured_tool("FILAMENT_ANALYZE_BIN", Code::AnalysisInconclusive)?;
    let z3 = configured_tool("Z3_BIN", Code::AnalysisInconclusive)?;
    let cvc5 = configured_tool("CVC5_BIN", Code::AnalysisInconclusive)?;
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
    let selection_path = report_dir.join("selection.json");
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
    let mut reports = Vec::new();
    let run = run_analysis(
        &analyzer,
        package_path,
        &selection_path,
        &report_dir,
        "consistency",
        &z3,
        &cvc5,
        trust.engineering_assurance,
    )?;
    reports.push(run.path.clone());
    if run.status == "refuted" {
        return Err(fail_with_details(
            Code::DraftClaimsInconsistent,
            "drafted claims are inconsistent",
            BTreeMap::from([("reports".to_owned(), serde_json::json!(reports))]),
        ));
    }
    let mut classification = "consistent";
    if !peers.is_empty() {
        let run = run_analysis(
            &analyzer,
            package_path,
            &selection_path,
            &report_dir,
            "contradiction",
            &z3,
            &cvc5,
            trust.engineering_assurance,
        )?;
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
        if drafts
            .iter()
            .map(|span| &span.symbol)
            .collect::<BTreeSet<_>>()
            .len()
            == 1
        {
            let run = run_analysis(
                &analyzer,
                package_path,
                &selection_path,
                &report_dir,
                "redundancy",
                &z3,
                &cvc5,
                trust.engineering_assurance,
            )?;
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

fn package_claim_spans(package: &Value) -> Result<Vec<ClaimSpan>, Failure> {
    let nodes = package
        .pointer("/semantic_graph/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "semantic_graph.nodes missing"))?;
    let map = package
        .pointer("/source_map/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| fail(Code::AnalysisPackageInvalid, "source_map.nodes missing"))?;
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
        matched.insert(candidates[0].symbol.clone());
        let mut claim = claim.clone();
        claim.symbol = candidates[0].symbol.clone();
        bound.push(claim);
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
fn run_analysis(
    analyzer: &Path,
    package: &str,
    selection: &Path,
    directory: &Path,
    mode: &str,
    z3: &Path,
    cvc5: &Path,
    producer: &str,
) -> Result<AnalysisRun, Failure> {
    let path = directory.join(format!("{mode}.json"));
    let arguments = vec![
        "run-package".to_owned(),
        "--package".to_owned(),
        package.to_owned(),
        "--output".to_owned(),
        path.to_string_lossy().into_owned(),
        "--z3".to_owned(),
        z3.to_string_lossy().into_owned(),
        "--cvc5".to_owned(),
        cvc5.to_string_lossy().into_owned(),
        "--producer-execute".to_owned(),
        producer.to_owned(),
        "--request".to_owned(),
        mode.to_owned(),
        "--select".to_owned(),
        selection.to_string_lossy().into_owned(),
    ];
    let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_tool(
        analyzer,
        &arguments,
        directory,
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
    let differential = report.get("differential").ok_or_else(|| {
        fail(
            Code::AnalysisInconclusive,
            "analysis report lacks differential",
        )
    })?;
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
    if output.status.code() != expected_exit {
        return Err(fail(
            Code::AnalysisInconclusive,
            "analysis exit status disagrees with its report",
        ));
    }
    Ok(AnalysisRun {
        status: status.to_owned(),
        path,
    })
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
