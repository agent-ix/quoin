// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Quoin-owned workflow invariants (FR-020, FR-021, FR-095).
//! ix-flow owns transitions and persistence; this provider reads actual catalog
//! contracts and documents on every evaluation, including final acceptance.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use quoin_quire::{ModuleRoot, ModuleSelection, ScopeRoot, validate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{core_runtime, invocation};

pub(super) const LENSES: [&str; 7] = [
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
const REQUEST_PROTOCOL: &str = "quoin.workflow-invariants/v1";
const RESULT_PROTOCOL: &str = "quoin.workflow-invariants-result/v1";

type Check = Result<(), Failure>;

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
}

fn fail(code: Code, message: impl Into<String>) -> Failure {
    Failure {
        code,
        message: message.into().chars().take(4096).collect(),
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
        details: BTreeMap<&'static str, String>,
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
        .map(|name| match check(&request.instance, name) {
            Ok(()) => Outcome::Passed {
                invariant: name.clone(),
            },
            Err(error) => Outcome::Failed {
                invariant: name.clone(),
                code: error.code,
                details: BTreeMap::from([("message", error.message)]),
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

fn check(instance: &Instance, name: &str) -> Check {
    if !matches!(instance.name.as_str(), "review" | "to-plan") {
        return Err(fail(Code::Unknown, &instance.name));
    }
    let context = context(instance)?;
    match (instance.name.as_str(), name) {
        ("review", "review_selection_consistent") => {
            let review: ReviewInput = one(instance, "operation_request")?;
            let catalog = Catalog::load(&context)?;
            selection(&review, &catalog)?;
            profiles(&review, &context, &catalog)
        }
        ("review", "selected_analyses_covered") => review_documents(instance, &context),
        ("to-plan", "plan_inputs_ready") => {
            let plan: PlanInput = one(instance, "plan_answers")?;
            plan_paths(&plan, &context)?;
            Ok(())
        }
        ("to-plan", "plan_documents_valid") => plan_documents(instance, &context),
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
            Ok(())
        }
        _ => Err(fail(Code::InvariantUnknown, name)),
    }
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

fn scoped_parent(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let parent = path
        .parent()
        .ok_or_else(|| fail(Code::PathInvalid, "path has no parent"))?;
    let canonical = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    if !canonical.starts_with(root)
        || canonical
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(fail(Code::PathInvalid, "path escapes repository"));
    }
    Ok(canonical)
}

fn scoped_path(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = path.canonicalize().map_err(|e| {
        fail(
            Code::DocumentUnavailable,
            format!("{}: {e}", path.display()),
        )
    })?;
    if !canonical.starts_with(root) {
        return Err(fail(Code::PathInvalid, "document escapes repository"));
    }
    Ok(canonical)
}

fn read_bounded(path: &Path) -> Result<String, Failure> {
    let input = std::fs::File::open(path).map_err(|e| {
        fail(
            Code::DocumentUnavailable,
            format!("{}: {e}", path.display()),
        )
    })?;
    let mut bytes = Vec::new();
    input
        .take(MAX_DOCUMENT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| fail(Code::DocumentUnavailable, e.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_DOCUMENT {
        return Err(fail(Code::DocumentLimit, "document exceeds 4 MiB"));
    }
    String::from_utf8(bytes).map_err(|e| fail(Code::DocumentInvalid, e.to_string()))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions may panic"
)]
mod tests {
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
            assert!(output.is_empty());
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
        assert!(output.is_empty());
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
        assert!(errors.is_empty());
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
}
