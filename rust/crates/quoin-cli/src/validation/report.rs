// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Retained validation-report generation and sign-off verification (FR-217, FR-218, FR-220).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_trace_rs::implements;
use quoin_completeness::{
    BundleDocument, DocumentSource, FeatureRatingState, RatingStatus, assess_feature_ratings,
    frontmatter_from_sources,
};
use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::Response;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::EMBEDDED_EXTERNAL_TOOLS_INDEX;
use super::RunEnvironment;

const SIGNATURE_REQUEST: &str = "engineering-assurance.signature-verify-request/v1";
const SIGNATURE_RESULT: &str = "engineering-assurance.signature-verify-result/v1";
const SIGNED_OFF_MEANING: &str = "tool-validation-accepted";
const INDEX_PATH: &str = "spec/validation/external-tools.v1.json";
pub(crate) fn command() -> Command {
    Command::new("report")
        .about("Generate a validation report from retained run records")
        .arg(
            Arg::new("results")
                .long("results")
                .value_name("FILE")
                .action(ArgAction::Append)
                .help("Retained JSON run record; may be repeated"),
        )
        .arg(Arg::new("out").long("out").value_name("DIR"))
        .arg(
            Arg::new("verify-signoff")
                .long("verify-signoff")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("repo").long("repo").value_name("DIR"))
        .arg(
            Arg::new("allowed-signers")
                .long("allowed-signers")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("engineering-assurance")
                .long("engineering-assurance")
                .value_name("FILE")
                .help("Engineering Assurance executable; defaults to PATH"),
        )
}

#[implements("FR-218-AC-1")]
#[implements("FR-218-AC-2")]
#[implements("FR-218-AC-3")]
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_flag("verify-signoff") {
        if arguments.get_many::<String>("results").is_some() {
            return Err("--verify-signoff cannot be combined with --results".to_owned());
        }
        let repo = required_path(arguments, "repo")?;
        let allowed = required_path(arguments, "allowed-signers")?;
        if !allowed.is_absolute() || !allowed.is_file() {
            return Err("--allowed-signers must name an absolute file".to_owned());
        }
        let report_directory = arguments.get_one::<String>("out").map_or_else(
            || repo.clone(),
            |path| {
                let path = PathBuf::from(path);
                if path.is_absolute() {
                    path
                } else {
                    repo.join(path)
                }
            },
        );
        let executable = arguments
            .get_one::<String>("engineering-assurance")
            .map(PathBuf::from)
            .or_else(|| find_executable("engineering-assurance"))
            .ok_or_else(|| "engineering-assurance is unavailable on PATH".to_owned())?;
        let verification = verify_signoff(&executable, &allowed, &repo, &report_directory)?;
        return Ok(Response::ok(json!({
            "rendered": serde_json::to_string_pretty(&verification)
                .map_err(|error| error.to_string())?
        })));
    }

    if arguments.get_one::<String>("repo").is_some()
        || arguments.get_one::<String>("allowed-signers").is_some()
    {
        return Err("--repo and --allowed-signers require --verify-signoff".to_owned());
    }
    let output = required_path(arguments, "out")?;
    let inputs = arguments
        .get_many::<String>("results")
        .ok_or_else(|| "report requires one or more --results files".to_owned())?
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let report = build_report(&inputs)?;
    let markdown = render_markdown(&report);
    let mut json_bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    json_bytes.push(b'\n');
    write_report(&output, &markdown, &json_bytes)?;
    let payload = json!({
        "rendered": format!(
            "validation report written to {}",
            output.display()
        )
    });
    if report.status == "incomplete" {
        Ok(Response::partial(
            payload,
            &CoreError::new(CoreErrorCode::Rejected, "validation report is incomplete"),
        ))
    } else {
        Ok(Response::ok(payload))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RunRecord {
    schema_version: u32,
    tool: String,
    tool_version: Option<String>,
    os: Option<String>,
    architecture: Option<String>,
    suite: Option<String>,
    timestamp: Option<String>,
    results: Vec<CaseResult>,
    validation_inputs: Option<ValidationInputs>,
    #[serde(default)]
    anomaly_set: Option<AnomalySet>,
    #[serde(default)]
    external_tools: Vec<ExternalToolEvidence>,
    #[serde(default)]
    run_environment: Option<RunEnvironment>,
    retained_location: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CaseResult {
    id: String,
    status: String,
    criteria: Vec<String>,
    detail: String,
    #[serde(default)]
    stderr: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidationInputs {
    provided: bool,
    documents: Vec<DocumentSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AnomalySet {
    provided: bool,
    producer_version: String,
    documents: Vec<DocumentSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalToolEvidence {
    tool_id: String,
    executable: String,
    version_invocation: Vec<String>,
    observed_version_output: String,
    observed_version_stream: String,
    index_path: String,
    index_format_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExternalToolEntry {
    id: String,
    executable: String,
    version_invocation: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidationReport {
    status: String,
    tools: Vec<ToolRun>,
    intended_use: Vec<IntendedUse>,
    results: Vec<ReportCase>,
    coverage: Vec<Coverage>,
    known_anomalies: KnownAnomalies,
    external_tools_index: Option<ExternalToolsIndexReport>,
    external_tools: Vec<ExternalToolReport>,
    incomplete_reasons: Vec<String>,
    rated_by: Option<String>,
    sign_off: SignOff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExternalToolsIndexReport {
    path: String,
    format_version: String,
    entries: Vec<ExternalToolEntry>,
}

#[derive(Debug, Clone)]
struct ExternalToolsIndexState {
    report: Option<ExternalToolsIndexReport>,
    entries: BTreeMap<String, ExternalToolEntry>,
    invalid_ids: BTreeSet<String>,
    format_version: String,
    valid: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolRun {
    name: String,
    version: String,
    os: String,
    architecture: String,
    suite: String,
    timestamp: String,
    retained_location: String,
    run_environment: Option<RunEnvironment>,
}

#[derive(Debug, Clone, Serialize)]
struct IntendedUse {
    tool: String,
    statement: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportCase {
    id: String,
    tool: String,
    criteria: Vec<String>,
    result: String,
    detail: String,
    stderr: String,
}

#[derive(Debug, Clone, Serialize)]
struct Coverage {
    tool: String,
    fr: String,
    rating: String,
    rating_status: String,
    ratings: Vec<RatingEvidence>,
    cases: Vec<String>,
    result: String,
}

#[derive(Debug, Clone, Serialize)]
struct RatingEvidence {
    id: String,
    rating: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnownAnomalies {
    provided: bool,
    status: String,
    producers: Vec<AnomalyProducer>,
    anomalies: Vec<KnownAnomaly>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnomalyProducer {
    tool: String,
    version: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnownAnomaly {
    tool: String,
    anomaly: String,
    title: String,
    severity: String,
    safety_impact: String,
    affected_frs: Vec<String>,
    status: String,
    high_process_risk_features: Vec<String>,
    highlight: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExternalToolReport {
    tool_id: String,
    executable: String,
    version_invocation: Vec<String>,
    observed_version_output: String,
    observed_version_stream: String,
    index_path: String,
    index_format_version: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
struct SignOff {
    name: String,
    date: String,
    meaning: String,
}

#[implements("FR-218-AC-1")]
#[implements("FR-218-AC-2")]
#[implements("FR-218-AC-3")]
#[implements("FR-217-AC-11")]
#[implements("FR-217-AC-12")]
fn build_report(paths: &[PathBuf]) -> Result<ValidationReport, String> {
    let records = read_run_records(paths)?;
    let (intended_use, mut reasons, documents_by_tool) = validation_documents(&records);
    let (tools, results) = summarize_runs(&records, &intended_use, &mut reasons);
    let (coverage, feature_ratings_by_tool) =
        feature_coverage(&records, &documents_by_tool, &results);
    let mut high_risk_by_tool = BTreeMap::<String, BTreeSet<String>>::new();
    for item in &coverage {
        if feature_ratings_by_tool
            .get(&(item.tool.clone(), item.fr.clone()))
            .is_some_and(|rating| rating.high_process_risk)
        {
            high_risk_by_tool
                .entry(item.tool.clone())
                .or_default()
                .insert(item.fr.clone());
        }
    }
    let known_anomalies = known_anomalies(&records, &high_risk_by_tool)?;
    for producer in known_anomalies
        .producers
        .iter()
        .filter(|producer| producer.status != "provided")
    {
        reasons.push(format!(
            "anomaly set {} for {} {}",
            producer.status, producer.tool, producer.version
        ));
    }
    let quoin_rating_documents = records
        .iter()
        .filter(|record| record.tool == "quoin")
        .filter_map(|record| record.validation_inputs.as_ref())
        .flat_map(|inputs| inputs.documents.iter().cloned())
        .collect::<Vec<_>>();
    let external_tools_index = embedded_external_tools_index(EMBEDDED_EXTERNAL_TOOLS_INDEX);
    let external_tools = external_tool_report(&records, &external_tools_index);
    if !external_tools_index.valid {
        reasons.push("external-tools index missing or invalid".to_owned());
    }
    let recorded_external_ids = records
        .iter()
        .flat_map(|record| {
            record
                .external_tools
                .iter()
                .map(|item| item.tool_id.as_str())
        })
        .collect::<BTreeSet<_>>();
    for tool in external_tools.iter().filter(|tool| {
        recorded_external_ids.contains(tool.tool_id.as_str()) && tool.status != "observed"
    }) {
        reasons.push(format!(
            "external tool evidence not provided for {}",
            tool.tool_id
        ));
    }
    reasons.sort();
    reasons.dedup();
    let status = if reasons.is_empty() {
        "complete"
    } else {
        "incomplete"
    };
    Ok(ValidationReport {
        status: status.to_owned(),
        tools,
        intended_use,
        results,
        coverage,
        known_anomalies,
        external_tools_index: external_tools_index.report,
        external_tools,
        incomplete_reasons: reasons,
        rated_by: rated_by(&quoin_rating_documents),
        sign_off: SignOff {
            name: String::new(),
            date: String::new(),
            meaning: String::new(),
        },
    })
}

fn validation_documents(
    records: &[RunRecord],
) -> (
    Vec<IntendedUse>,
    Vec<String>,
    BTreeMap<String, Vec<DocumentSource>>,
) {
    let mut by_path = BTreeMap::<(String, String), String>::new();
    let mut provided = false;
    for record in records {
        if let Some(inputs) = &record.validation_inputs {
            provided |= inputs.provided;
            for document in &inputs.documents {
                by_path
                    .entry((record.tool.clone(), document.path.clone()))
                    .or_insert_with(|| document.raw.clone());
            }
        }
    }
    let mut documents_by_tool = BTreeMap::<String, Vec<DocumentSource>>::new();
    for ((tool, path), raw) in by_path {
        documents_by_tool
            .entry(tool)
            .or_default()
            .push(DocumentSource { path, raw });
    }
    let documents = documents_by_tool
        .iter()
        .flat_map(|(tool, documents)| {
            documents.iter().map(move |document| DocumentSource {
                path: format!("{tool}/{}", document.path),
                raw: document.raw.clone(),
            })
        })
        .collect::<Vec<_>>();
    let parsed = frontmatter_from_sources(&documents, &[]);
    let intended_use = parsed
        .documents
        .iter()
        .filter(|document| field(&document.frontmatter, "type") == Some("IntendedUse"))
        .map(|document| IntendedUse {
            tool: field(&document.frontmatter, "tool")
                .unwrap_or("not stated")
                .to_owned(),
            statement: document.body.clone(),
        })
        .collect::<Vec<_>>();
    let mut reasons = parsed
        .unreadable
        .iter()
        .map(|document| format!("unreadable validation input {}", document.path))
        .collect::<Vec<_>>();
    if !provided {
        reasons.push("validation inputs not provided".to_owned());
    }
    if intended_use.is_empty() {
        reasons.push("intended use not provided".to_owned());
    }
    (intended_use, reasons, documents_by_tool)
}

fn rated_by(documents: &[DocumentSource]) -> Option<String> {
    let parsed = frontmatter_from_sources(documents, &[]);
    let raters = parsed
        .documents
        .iter()
        .filter(|document| {
            field(&document.frontmatter, "type") == Some("FeatureRiskRating")
                && field(&document.frontmatter, "scope") == Some("supplier")
        })
        .filter_map(|document| field(&document.frontmatter, "rated_by"))
        .collect::<BTreeSet<_>>();
    (raters.len() == 1).then(|| raters.first().copied().unwrap_or_default().to_owned())
}

#[derive(Debug, Clone)]
struct FeatureRating {
    status: String,
    display_value: String,
    ratings: Vec<RatingEvidence>,
    high_process_risk: bool,
}

impl FeatureRating {
    fn unrated() -> Self {
        Self {
            status: "unrated".to_owned(),
            display_value: "not provided".to_owned(),
            ratings: Vec::new(),
            high_process_risk: false,
        }
    }
}

fn feature_ratings(
    documents: &[DocumentSource],
    repository_namespace: &str,
) -> BTreeMap<String, FeatureRating> {
    let parsed = frontmatter_from_sources(documents, &[]);
    let mut feature_documents = parsed.documents.clone();
    let mut authored_feature_ids = feature_documents
        .iter()
        .filter(|document| field(&document.frontmatter, "type") == Some("FR"))
        .filter_map(|document| field(&document.frontmatter, "id"))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for document in parsed.documents.iter().filter(|document| {
        field(&document.frontmatter, "type") == Some("FeatureRiskRating")
            && field(&document.frontmatter, "scope") == Some("supplier")
    }) {
        let Some(relations) = document
            .frontmatter
            .get("relationships")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for relation in relations {
            if relation.get("type").and_then(Value::as_str) != Some("rates_feature") {
                continue;
            }
            let Some(target) = relation.get("target").and_then(Value::as_str) else {
                continue;
            };
            let Some(feature) = feature_target(target, repository_namespace) else {
                continue;
            };
            if authored_feature_ids.insert(feature.clone()) {
                let mut frontmatter = serde_json::Map::new();
                frontmatter.insert("id".to_owned(), Value::String(feature.clone()));
                frontmatter.insert("type".to_owned(), Value::String("FR".to_owned()));
                feature_documents.push(BundleDocument {
                    path: format!("retained/{feature}.md"),
                    frontmatter,
                    body: String::new(),
                });
            }
        }
    }
    let states = assess_feature_ratings(&feature_documents, repository_namespace);
    states
        .into_iter()
        .map(|state| {
            let ratings = ratings_for_feature(&parsed.documents, &state, repository_namespace);
            let high_process_risk = ratings
                .iter()
                .any(|rating| rating.rating == "high_process_risk");
            let display_value = match state.status {
                RatingStatus::Rated => ratings
                    .first()
                    .map_or_else(|| "not provided".to_owned(), |rating| rating.rating.clone()),
                RatingStatus::RatedMoreThanOnce => "multiple ratings".to_owned(),
                RatingStatus::Unrated => "not provided".to_owned(),
            };
            let rating_status = match state.status {
                RatingStatus::Rated => "rated",
                RatingStatus::RatedMoreThanOnce => "rated_more_than_once",
                RatingStatus::Unrated => "unrated",
            }
            .to_owned();
            (
                state.feature_id,
                FeatureRating {
                    status: rating_status,
                    display_value,
                    ratings,
                    high_process_risk,
                },
            )
        })
        .collect()
}

fn feature_target(target: &str, repository_namespace: &str) -> Option<String> {
    let target = target.strip_prefix("ix://")?;
    let (namespace, feature) = target.rsplit_once('/')?;
    (namespace == repository_namespace && is_fr_id(feature)).then(|| feature.to_owned())
}

fn ratings_for_feature(
    documents: &[quoin_completeness::BundleDocument],
    state: &FeatureRatingState,
    repository_namespace: &str,
) -> Vec<RatingEvidence> {
    let mut ratings = Vec::new();
    let mut occurrence_by_id = BTreeMap::<String, usize>::new();
    for rating_id in &state.rating_ids {
        let candidates = documents
            .iter()
            .filter(|document| {
                field(&document.frontmatter, "type") == Some("FeatureRiskRating")
                    && field(&document.frontmatter, "scope") == Some("supplier")
                    && field(&document.frontmatter, "id") == Some(rating_id)
                    && document
                        .frontmatter
                        .get("relationships")
                        .and_then(Value::as_array)
                        .is_some_and(|relations| {
                            relations.iter().any(|relation| {
                                relation.get("type").and_then(Value::as_str)
                                    == Some("rates_feature")
                                    && relation
                                        .get("target")
                                        .and_then(Value::as_str)
                                        .and_then(|target| {
                                            feature_target(target, repository_namespace)
                                        })
                                        .as_deref()
                                        == Some(state.feature_id.as_str())
                            })
                        })
            })
            .collect::<Vec<_>>();
        let occurrence = occurrence_by_id.entry(rating_id.clone()).or_default();
        if let Some(document) = candidates.get(*occurrence) {
            ratings.push(RatingEvidence {
                id: rating_id.clone(),
                rating: field(&document.frontmatter, "rating")
                    .unwrap_or("not provided")
                    .to_owned(),
            });
            *occurrence += 1;
        }
    }
    ratings.sort_by(|left, right| (&left.id, &left.rating).cmp(&(&right.id, &right.rating)));
    ratings
}

#[implements("FR-218-AC-1")]
#[implements("FR-218-AC-2")]
#[implements("FR-218-AC-3")]
fn read_run_records(paths: &[PathBuf]) -> Result<Vec<RunRecord>, String> {
    if paths.is_empty() {
        return Err("report requires one or more --results files".to_owned());
    }
    let mut records = Vec::new();
    for path in paths {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read run record {}: {error}", path.display()))?;
        let record: RunRecord = serde_json::from_str(&raw)
            .map_err(|error| format!("cannot parse run record {}: {error}", path.display()))?;
        if record.schema_version != 1 {
            return Err(format!(
                "run record {} has unsupported schemaVersion {}",
                path.display(),
                record.schema_version
            ));
        }
        if record.tool.trim().is_empty() {
            return Err(format!("run record {} has no tool name", path.display()));
        }
        for case in &record.results {
            if !matches!(
                case.status.as_str(),
                "pass" | "fail" | "error" | "not-executed"
            ) {
                return Err(format!(
                    "run record {} has unknown case status {:?}",
                    path.display(),
                    case.status
                ));
            }
        }
        records.push(record);
    }
    records.sort_by(|left, right| {
        (&left.tool, &left.suite, &left.timestamp).cmp(&(
            &right.tool,
            &right.suite,
            &right.timestamp,
        ))
    });

    Ok(records)
}

#[implements("FR-217-AC-1")]
#[implements("FR-218-AC-1")]
#[implements("FR-218-AC-3")]
#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn summarize_runs(
    records: &[RunRecord],
    intended_use: &[IntendedUse],
    reasons: &mut Vec<String>,
) -> (Vec<ToolRun>, Vec<ReportCase>) {
    let mut tools = Vec::new();
    let mut results = Vec::new();
    let mut required_tools = intended_use
        .iter()
        .map(|item| item.tool.clone())
        .collect::<BTreeSet<_>>();
    for record in records {
        let version = record.tool_version.clone().unwrap_or_else(|| {
            reasons.push(format!("missing tool version for {}", record.tool));
            "not provided".to_owned()
        });
        tools.push(ToolRun {
            name: record.tool.clone(),
            version,
            os: record
                .os
                .clone()
                .unwrap_or_else(|| "not provided".to_owned()),
            architecture: record
                .architecture
                .clone()
                .unwrap_or_else(|| "not provided".to_owned()),
            suite: record
                .suite
                .clone()
                .unwrap_or_else(|| "not stated".to_owned()),
            timestamp: record
                .timestamp
                .clone()
                .unwrap_or_else(|| "not stated".to_owned()),
            retained_location: record
                .retained_location
                .clone()
                .unwrap_or_else(|| "not stated".to_owned()),
            run_environment: record.run_environment.clone(),
        });
        required_tools.remove(&record.tool);
        for case in &record.results {
            if case.status == "not-executed" {
                reasons.push(format!(
                    "not-executed tool {} case {}",
                    record.tool, case.id
                ));
            }
            results.push(ReportCase {
                id: case.id.clone(),
                tool: record.tool.clone(),
                criteria: case.criteria.clone(),
                result: case.status.clone(),
                detail: case.detail.clone(),
                stderr: case.stderr.clone(),
            });
        }
    }
    for tool in required_tools {
        reasons.push(format!("result missing for tool {tool}"));
    }
    results.sort_by(|left, right| (&left.tool, &left.id).cmp(&(&right.tool, &right.id)));

    (tools, results)
}

#[implements("FR-217-AC-10")]
#[implements("FR-217-AC-11")]
#[implements("FR-218-AC-3")]
fn feature_coverage(
    records: &[RunRecord],
    documents_by_tool: &BTreeMap<String, Vec<DocumentSource>>,
    results: &[ReportCase],
) -> (Vec<Coverage>, BTreeMap<(String, String), FeatureRating>) {
    let mut feature_cases = BTreeMap::<(String, String), Vec<String>>::new();
    for record in records {
        for case in &record.results {
            for criterion in &case.criteria {
                let feature = criterion.split("-AC-").next().unwrap_or_default();
                if is_fr_id(feature) {
                    feature_cases
                        .entry((record.tool.clone(), feature.to_owned()))
                        .or_default()
                        .push(case.id.clone());
                }
            }
        }
    }
    for cases in feature_cases.values_mut() {
        cases.sort();
        cases.dedup();
    }
    let mut feature_ratings_by_tool = BTreeMap::<(String, String), FeatureRating>::new();
    for (tool, documents) in documents_by_tool {
        let repository_namespace = format!("agent-ix/{tool}");
        feature_ratings_by_tool.extend(
            feature_ratings(documents, &repository_namespace)
                .into_iter()
                .map(|(feature, rating)| ((tool.clone(), feature), rating)),
        );
    }
    let feature_ids = feature_ratings_by_tool
        .keys()
        .chain(feature_cases.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut coverage = Vec::new();
    for (tool, feature) in feature_ids {
        let feature_rating = feature_ratings_by_tool
            .get(&(tool.clone(), feature.clone()))
            .cloned()
            .unwrap_or_else(FeatureRating::unrated);
        let cases = feature_cases
            .remove(&(tool.clone(), feature.clone()))
            .unwrap_or_default();
        let passed = results.iter().any(|case| {
            case.tool == tool
                && case
                    .criteria
                    .iter()
                    .any(|criterion| criterion.starts_with(&format!("{feature}-AC-")))
                && case.result == "pass"
        });
        let result = if passed { "covered" } else { "uncovered" };
        coverage.push(Coverage {
            tool: tool.clone(),
            fr: feature,
            rating: feature_rating.display_value,
            rating_status: feature_rating.status,
            ratings: feature_rating.ratings,
            cases,
            result: result.to_owned(),
        });
    }

    (coverage, feature_ratings_by_tool)
}

#[implements("FR-217-AC-8")]
fn known_anomalies(
    records: &[RunRecord],
    high_risk: &BTreeMap<String, BTreeSet<String>>,
) -> Result<KnownAnomalies, String> {
    let mut producers = Vec::new();
    let mut anomalies = Vec::new();
    for record in records {
        let (producer, rows) = anomalies_for_record(record, high_risk)?;
        producers.push(producer);
        anomalies.extend(rows);
    }
    producers
        .sort_by(|left, right| (&left.tool, &left.version).cmp(&(&right.tool, &right.version)));
    producers.dedup_by(|left, right| {
        left.tool == right.tool && left.version == right.version && left.status == right.status
    });
    anomalies.sort_by(|left, right| {
        left.high_process_risk_features
            .is_empty()
            .cmp(&right.high_process_risk_features.is_empty())
            .then_with(|| (&left.tool, &left.anomaly).cmp(&(&right.tool, &right.anomaly)))
    });
    anomalies.dedup_by(|left, right| left.tool == right.tool && left.anomaly == right.anomaly);
    let all_provided =
        !producers.is_empty() && producers.iter().all(|row| row.status == "provided");
    let any_provided = producers.iter().any(|row| row.status == "provided");
    let status = if all_provided {
        if anomalies.is_empty() {
            "none known"
        } else {
            "listed"
        }
    } else if any_provided {
        "partially provided"
    } else {
        "not provided"
    };
    Ok(KnownAnomalies {
        provided: all_provided,
        status: status.to_owned(),
        producers,
        anomalies,
    })
}

#[implements("FR-217-AC-5")]
#[implements("FR-217-AC-6")]
#[implements("FR-217-AC-7")]
#[implements("FR-217-AC-8")]
#[implements("FR-217-AC-9")]
fn anomalies_for_record(
    record: &RunRecord,
    high_risk: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(AnomalyProducer, Vec<KnownAnomaly>), String> {
    let version = record.tool_version.as_deref().unwrap_or("not provided");
    let Some(set) = &record.anomaly_set else {
        return Ok((
            anomaly_producer(record, version, "not provided"),
            Vec::new(),
        ));
    };
    if !set.provided {
        return Ok((
            anomaly_producer(record, version, "not provided"),
            Vec::new(),
        ));
    }
    if set.producer_version != version || version == "not provided" {
        return Ok((
            anomaly_producer(record, &set.producer_version, "version mismatch"),
            Vec::new(),
        ));
    }
    let report = embedded_anomaly_report(&set.documents)?;
    let invalid = !report.unreadable.is_empty();
    let producer = anomaly_producer(
        record,
        &set.producer_version,
        if invalid { "invalid" } else { "provided" },
    );
    let tool_risk = high_risk.get(&record.tool).cloned().unwrap_or_default();
    let anomalies = report
        .anomalies
        .iter()
        .map(|row| {
            let mut affected_frs = row
                .affected
                .iter()
                .filter(|item| item.kind == "Requirement" && is_fr_id(&item.target))
                .map(|item| item.target.clone())
                .collect::<Vec<_>>();
            affected_frs.sort();
            affected_frs.dedup();
            let high_process_risk_features = affected_frs
                .iter()
                .filter(|feature| tool_risk.contains(*feature))
                .cloned()
                .collect::<Vec<_>>();
            let highlight = (!high_process_risk_features.is_empty()).then(|| {
                format!(
                    "affects high-process-risk feature{} {}",
                    if high_process_risk_features.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    high_process_risk_features.join(", ")
                )
            });
            KnownAnomaly {
                tool: record.tool.clone(),
                anomaly: row.id.clone(),
                title: row.title.clone().unwrap_or_else(|| "not stated".to_owned()),
                severity: row
                    .severity
                    .clone()
                    .unwrap_or_else(|| "not stated".to_owned()),
                safety_impact: row.safety_impact.clone(),
                affected_frs,
                status: row.status.clone(),
                high_process_risk_features,
                highlight,
            }
        })
        .collect();
    Ok((producer, anomalies))
}

#[implements("FR-217-AC-8")]
#[implements("FR-217-AC-9")]
fn anomaly_producer(record: &RunRecord, version: &str, status: &str) -> AnomalyProducer {
    AnomalyProducer {
        tool: record.tool.clone(),
        version: version.to_owned(),
        status: status.to_owned(),
    }
}

#[implements("FR-217-AC-5")]
#[implements("FR-217-AC-6")]
#[implements("FR-217-AC-7")]
#[implements("FR-217-AC-8")]
#[implements("FR-217-AC-9")]
fn embedded_anomaly_report(documents: &[DocumentSource]) -> Result<quoin_anomaly::Report, String> {
    let temp = tempfile::tempdir().map_err(|error| format!("create anomaly workspace: {error}"))?;
    let scope = temp.path().join("spec/anomalies");
    std::fs::create_dir_all(&scope)
        .map_err(|error| format!("create embedded anomaly directory: {error}"))?;
    for document in documents {
        let relative = Path::new(&document.path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(format!("unsafe embedded anomaly path {}", document.path));
        }
        let path = scope.join(relative);
        let parent = path
            .parent()
            .ok_or_else(|| "embedded anomaly path has no parent".to_owned())?;
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create embedded anomaly directory: {error}"))?;
        std::fs::write(&path, document.raw.as_bytes())
            .map_err(|error| format!("materialize embedded anomaly {}: {error}", document.path))?;
    }
    quoin_anomaly::build_report(&scope, &[]).map_err(|error| error.to_string())
}

fn embedded_external_tools_index(bytes: Option<&[u8]>) -> ExternalToolsIndexState {
    let Some(bytes) = bytes else {
        return ExternalToolsIndexState {
            report: None,
            entries: BTreeMap::new(),
            invalid_ids: BTreeSet::new(),
            format_version: "not provided".to_owned(),
            valid: false,
        };
    };
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return ExternalToolsIndexState {
            report: None,
            entries: BTreeMap::new(),
            invalid_ids: BTreeSet::new(),
            format_version: "not provided".to_owned(),
            valid: false,
        };
    };
    let format_version = value
        .get("format_version")
        .and_then(Value::as_str)
        .unwrap_or("not provided")
        .to_owned();
    let raw_entries = value
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut counts = BTreeMap::<String, usize>::new();
    for raw in &raw_entries {
        if let Some(id) = raw
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            *counts.entry(id.to_owned()).or_default() += 1;
        }
    }
    let mut entries = BTreeMap::<String, ExternalToolEntry>::new();
    let mut invalid_ids = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    for raw in &raw_entries {
        let id = raw.get("id").and_then(Value::as_str).unwrap_or_default();
        let Ok(entry) = serde_json::from_value::<ExternalToolEntry>(raw.clone()) else {
            if !id.is_empty() {
                invalid_ids.insert(id.to_owned());
            }
            continue;
        };
        if entry.id.trim().is_empty()
            || entry.executable.trim().is_empty()
            || entry.version_invocation.is_empty()
            || entry.version_invocation.iter().any(String::is_empty)
            || counts.get(&entry.id).copied() != Some(1)
        {
            invalid_ids.insert(entry.id);
            continue;
        }
        entries.insert(entry.id.clone(), entry);
    }
    for id in &invalid_ids {
        entries.remove(id);
    }
    let format_valid = format_version == "1";
    let entries_valid =
        raw_entries.len() == 9 && invalid_ids.is_empty() && entries.len() == raw_entries.len();
    let valid = format_valid && entries_valid;
    let report_entries = entries.values().cloned().collect();
    ExternalToolsIndexState {
        report: Some(ExternalToolsIndexReport {
            path: INDEX_PATH.to_owned(),
            format_version: format_version.clone(),
            entries: report_entries,
        }),
        entries,
        invalid_ids,
        format_version,
        valid,
    }
}

fn external_tool_report(
    records: &[RunRecord],
    index: &ExternalToolsIndexState,
) -> Vec<ExternalToolReport> {
    let mut evidence_by_id = BTreeMap::<&str, Vec<&ExternalToolEvidence>>::new();
    for evidence in records.iter().flat_map(|record| &record.external_tools) {
        evidence_by_id
            .entry(&evidence.tool_id)
            .or_default()
            .push(evidence);
    }
    let mut ids = index.entries.keys().cloned().collect::<BTreeSet<_>>();
    ids.extend(index.invalid_ids.iter().cloned());
    ids.extend(evidence_by_id.keys().map(|id| (*id).to_owned()));
    let mut out = ids
        .into_iter()
        .map(|id| {
            let entry = index.entries.get(&id);
            let candidates = evidence_by_id.get(id.as_str()).cloned().unwrap_or_default();
            let evidence = (candidates.len() == 1)
                .then(|| candidates.first().copied())
                .flatten();
            let valid = index.format_version == "1"
                && !index.invalid_ids.contains(&id)
                && entry.is_some_and(|entry| {
                    evidence.is_some_and(|evidence| {
                        evidence.executable == entry.executable
                            && evidence.version_invocation == entry.version_invocation
                            && evidence.index_path == INDEX_PATH
                            && evidence.index_format_version == "1"
                            && matches!(
                                evidence.observed_version_stream.as_str(),
                                "stdout" | "stderr"
                            )
                            && !evidence.observed_version_output.trim().is_empty()
                    })
                });
            let fallback = evidence;
            ExternalToolReport {
                tool_id: id,
                executable: entry
                    .map(|entry| entry.executable.clone())
                    .or_else(|| fallback.map(|evidence| evidence.executable.clone()))
                    .unwrap_or_else(|| "not provided".to_owned()),
                version_invocation: entry
                    .map(|entry| entry.version_invocation.clone())
                    .or_else(|| fallback.map(|evidence| evidence.version_invocation.clone()))
                    .unwrap_or_default(),
                observed_version_output: if valid {
                    evidence.map_or_else(String::new, |evidence| {
                        evidence.observed_version_output.clone()
                    })
                } else {
                    "not provided".to_owned()
                },
                observed_version_stream: if valid {
                    evidence.map_or_else(String::new, |evidence| {
                        evidence.observed_version_stream.clone()
                    })
                } else {
                    "not provided".to_owned()
                },
                index_path: evidence.map_or_else(
                    || INDEX_PATH.to_owned(),
                    |evidence| evidence.index_path.clone(),
                ),
                index_format_version: evidence.map_or_else(
                    || index.format_version.clone(),
                    |evidence| evidence.index_format_version.clone(),
                ),
                status: if valid { "observed" } else { "not provided" }.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|left, right| left.tool_id.cmp(&right.tool_id));
    out
}

#[implements("FR-217-AC-12")]
#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn render_markdown(report: &ValidationReport) -> String {
    let mut lines = render_tool_section(report);
    append_run_environments(report, &mut lines);
    append_intended_use(report, &mut lines);
    append_results(report, &mut lines);
    append_coverage(report, &mut lines);
    append_known_anomalies(report, &mut lines);
    append_external_tools(report, &mut lines);
    append_incomplete_reasons(report, &mut lines);
    append_signoff(&mut lines);
    lines.join("\n")
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn append_run_environments(report: &ValidationReport, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Run environments".to_owned(),
        String::new(),
    ]);
    for tool in &report.tools {
        lines.push(format!("### {} — {}", cell(&tool.name), cell(&tool.suite)));
        let Some(environment) = &tool.run_environment else {
            lines.extend([String::new(), "not provided".to_owned()]);
            continue;
        };
        lines.extend([
            String::new(),
            "| Required checkout | Observed repository | Revision | Checkout path | Available |"
                .to_owned(),
            "| --- | --- | --- | --- | --- |".to_owned(),
        ]);
        for checkout in &environment.checkouts {
            lines.push(format!(
                "| {} | {} | {} | {} | {} |",
                cell(&checkout.expected_repository),
                cell(checkout.repository.as_deref().unwrap_or("not available")),
                cell(checkout.revision.as_deref().unwrap_or("not available")),
                cell(checkout.path.as_deref().unwrap_or("not available")),
                checkout.available
            ));
        }
        lines.push(format!(
            "Temporary directory: {}; writable: {}",
            cell(
                environment
                    .temporary_directory
                    .path
                    .as_deref()
                    .unwrap_or("not available")
            ),
            environment.temporary_directory.writable
        ));
    }
}

#[implements("FR-217-AC-1")]
fn render_tool_section(report: &ValidationReport) -> Vec<String> {
    let mut lines = vec![
        "# Tool validation report".to_owned(),
        String::new(),
        format!("Status: `{}`", report.status),
        String::new(),
        "## Tools as run".to_owned(),
        String::new(),
        "| Tool | Version | OS | Architecture | Suite | Timestamp | Run record |".to_owned(),
        "| --- | --- | --- | --- | --- | --- | --- |".to_owned(),
    ];
    for tool in &report.tools {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} |",
            cell(&tool.name),
            cell(&tool.version),
            cell(&tool.os),
            cell(&tool.architecture),
            cell(&tool.suite),
            cell(&tool.timestamp),
            cell(&tool.retained_location)
        ));
    }
    lines
}

fn append_intended_use(report: &ValidationReport, lines: &mut Vec<String>) {
    lines.extend([String::new(), "## Intended use".to_owned(), String::new()]);
    if report.intended_use.is_empty() {
        lines.push("not provided".to_owned());
    } else {
        for item in &report.intended_use {
            lines.push(format!("### {}", item.tool));
            lines.push(item.statement.trim_end().to_owned());
            lines.push(String::new());
        }
    }
}

#[implements("FR-217-AC-1")]
#[implements("FR-217-AC-12")]
fn append_results(report: &ValidationReport, lines: &mut Vec<String>) {
    lines.extend([
        "## Results".to_owned(),
        String::new(),
        "| Case | Tool | Criteria | Result | Detail |".to_owned(),
        "| --- | --- | --- | --- | --- |".to_owned(),
    ]);
    for case in &report.results {
        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            cell(&case.id),
            cell(&case.tool),
            cell(&case.criteria.join(", ")),
            cell(&case.result),
            cell(&case.detail)
        ));
    }
}

#[implements("FR-217-AC-1")]
#[implements("FR-217-AC-10")]
#[implements("FR-217-AC-11")]
#[implements("FR-217-AC-12")]
#[implements("FR-218-AC-3")]
fn append_coverage(report: &ValidationReport, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Coverage".to_owned(),
        String::new(),
        "| Tool | FR | Rating | Rating status | Rating IDs | Cases | Result |".to_owned(),
        "| --- | --- | --- | --- | --- | --- | --- |".to_owned(),
    ]);
    for item in &report.coverage {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {}",
            cell(&item.tool),
            cell(&item.fr),
            cell(&item.rating),
            cell(&item.rating_status),
            cell(
                &item
                    .ratings
                    .iter()
                    .map(|rating| rating.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            cell(&item.cases.join(", ")),
            cell(&item.result)
        ));
    }
}

#[implements("FR-217-AC-5")]
#[implements("FR-217-AC-6")]
#[implements("FR-217-AC-7")]
#[implements("FR-217-AC-8")]
#[implements("FR-217-AC-9")]
fn append_known_anomalies(report: &ValidationReport, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Known anomalies".to_owned(),
        String::new(),
    ]);
    for producer in &report.known_anomalies.producers {
        lines.push(format!(
            "Anomaly set: {} {} — {}.",
            cell(&producer.tool),
            cell(&producer.version),
            cell(&producer.status)
        ));
    }
    if !report.known_anomalies.provided && report.known_anomalies.anomalies.is_empty() {
        lines.push("not provided".to_owned());
    } else if report.known_anomalies.anomalies.is_empty() && report.known_anomalies.provided {
        lines.push("none known".to_owned());
    } else {
        lines.extend([
            "| Tool | Anomaly | Title | Severity | Safety impact | Affected FRs | Status |"
                .to_owned(),
            "| --- | --- | --- | --- | --- | --- | --- |".to_owned(),
        ]);
        for anomaly in &report.known_anomalies.anomalies {
            let title = anomaly.highlight.as_ref().map_or_else(
                || anomaly.title.clone(),
                |highlight| format!("{} ({highlight})", anomaly.title),
            );
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} |",
                cell(&anomaly.tool),
                cell(&anomaly.anomaly),
                cell(&title),
                cell(&anomaly.severity),
                cell(&anomaly.safety_impact),
                cell(&anomaly.affected_frs.join(", ")),
                cell(&anomaly.status)
            ));
        }
    }
}

#[implements("FR-217-AC-1")]
fn append_external_tools(report: &ValidationReport, lines: &mut Vec<String>) {
    lines.extend([String::new(), "## External tools".to_owned(), String::new()]);
    if let Some(index) = &report.external_tools_index {
        lines.push(format!(
            "Configuration index: `{}` (format version `{}`).",
            index.path, index.format_version
        ));
        lines.extend([
            String::new(),
            "| Tool | Executable | Version invocation |".to_owned(),
            "| --- | --- | --- |".to_owned(),
        ]);
        for entry in &index.entries {
            lines.push(format!(
                "| {} | {} | {} |",
                cell(&entry.id),
                cell(&entry.executable),
                cell(&entry.version_invocation.join(" "))
            ));
        }
        lines.push(String::new());
    } else {
        lines.push("Configuration index: not provided.".to_owned());
    }
    lines.extend([
        "| Tool | Executable | Version invocation | Observed version | Stream | Status |"
            .to_owned(),
        "| --- | --- | --- | --- | --- | --- |".to_owned(),
    ]);
    for tool in &report.external_tools {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            cell(&tool.tool_id),
            cell(&tool.executable),
            cell(&tool.version_invocation.join(" ")),
            cell(&tool.observed_version_output),
            cell(&tool.observed_version_stream),
            cell(&tool.status)
        ));
    }
}

#[implements("FR-218-AC-2")]
fn append_incomplete_reasons(report: &ValidationReport, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Incomplete reasons".to_owned(),
        String::new(),
    ]);
    if report.incomplete_reasons.is_empty() {
        lines.push("none".to_owned());
    } else {
        lines.extend(
            report
                .incomplete_reasons
                .iter()
                .map(|reason| format!("- {reason}")),
        );
    }
}

#[implements("FR-217-AC-4")]
fn append_signoff(lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Sign-off".to_owned(),
        String::new(),
        "Name: ".to_owned(),
        "Date: ".to_owned(),
        "Meaning: ".to_owned(),
        String::new(),
    ]);
}

fn write_report(directory: &Path, markdown: &str, json_bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("create report directory {}: {error}", directory.display()))?;
    let markdown_path = directory.join("validation-report.md");
    let json_path = directory.join("validation-report.json");
    let mut markdown_bytes = markdown.as_bytes().to_vec();
    markdown_bytes.push(b'\n');
    std::fs::write(&markdown_path, markdown_bytes)
        .map_err(|error| format!("write {}: {error}", markdown_path.display()))?;
    std::fs::write(&json_path, json_bytes)
        .map_err(|error| format!("write {}: {error}", json_path.display()))?;
    Ok(())
}

#[implements("FR-218-AC-5")]
#[implements("FR-218-AC-4")]
fn verify_signoff(
    executable: &Path,
    allowed_signers: &Path,
    repo: &Path,
    report_directory: &Path,
) -> Result<Value, String> {
    let Some((repo, report_directory, revision)) = report_revision(repo, report_directory)? else {
        return Ok(signoff_result(
            "not_accepted",
            Some("unsigned"),
            SignoffChecks {
                signer: SignerChecks {
                    mapped: false,
                    signature_valid: false,
                    principal_matches: false,
                },
                meaning_present: false,
            },
        ));
    };
    let request = json!({
        "protocol": SIGNATURE_REQUEST,
        "repository": repo,
        "revision": revision,
        "kind": "commit",
        "allowed_signers": allowed_signers,
    });
    let result =
        quoin_core::runtime::engineering_assurance_call(executable, "signature-verify", &request)?;
    if !result.status.success() {
        return Err(format!(
            "engineering-assurance signature-verify failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    let value: Value = serde_json::from_slice(&result.stdout)
        .map_err(|error| format!("invalid signature-verify result: {error}"))?;
    if value.get("protocol").and_then(Value::as_str) != Some(SIGNATURE_RESULT)
        || value.get("kind").and_then(Value::as_str) != Some("commit")
    {
        return Err("engineering-assurance returned an unsupported signature result".to_owned());
    }
    let status = value.get("status").and_then(Value::as_str).unwrap_or("");
    let mapping = value
        .pointer("/signer/mapping")
        .and_then(Value::as_str)
        .unwrap_or("");
    let principal = value
        .pointer("/signer/principal")
        .and_then(Value::as_str)
        .unwrap_or("");
    let report_json_path = report_directory.join("validation-report.json");
    let report_json = std::fs::read_to_string(&report_json_path).map_err(|error| {
        format!(
            "validation report JSON is not readable at {}: {error}",
            report_json_path.display()
        )
    })?;
    let report_value: Value = serde_json::from_str(&report_json)
        .map_err(|error| format!("validation report JSON is invalid: {error}"))?;
    let expected_principal = report_value
        .get("ratedBy")
        .and_then(Value::as_str)
        .unwrap_or("");
    let meaning = value
        .pointer("/signed_off_meaning/values")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let principal_matches = !principal.is_empty() && principal == expected_principal;
    let meaning_present = meaning.contains(&SIGNED_OFF_MEANING);
    let (result_status, reason) = if status == "unsigned" {
        ("not_accepted", Some("unsigned"))
    } else if status == "signed_untrusted" && mapping == "unmapped" {
        ("not_accepted", Some("signer_unmapped"))
    } else if status != "signed_valid" {
        ("not_accepted", Some("signature_invalid"))
    } else if mapping != "mapped" {
        ("not_accepted", Some("signer_unmapped"))
    } else if !principal_matches {
        ("not_accepted", Some("signer_mismatch"))
    } else if !meaning_present {
        ("not_accepted", Some("meaning_missing"))
    } else {
        ("accepted", None)
    };
    Ok(signoff_result(
        result_status,
        reason,
        SignoffChecks {
            signer: SignerChecks {
                mapped: mapping == "mapped",
                signature_valid: status == "signed_valid",
                principal_matches,
            },
            meaning_present,
        },
    ))
}

#[implements("FR-218-AC-4")]
#[implements("FR-218-AC-5")]
fn report_revision(
    repo: &Path,
    report_directory: &Path,
) -> Result<Option<(PathBuf, PathBuf, String)>, String> {
    let repo = repo
        .canonicalize()
        .map_err(|error| format!("repository is unavailable: {error}"))?;
    let report_directory = if report_directory.is_absolute() {
        report_directory.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("current directory is unavailable: {error}"))?
            .join(report_directory)
    };
    let report_directory = report_directory.canonicalize().map_err(|error| {
        format!(
            "validation report directory is unavailable at {}: {error}",
            report_directory.display()
        )
    })?;
    let report = report_directory.join("validation-report.md");
    if !report.is_file() {
        return Err(format!(
            "validation report is not present: {}",
            report.display()
        ));
    }
    let report = report
        .strip_prefix(&repo)
        .map_err(|_| "validation report must be inside --repo".to_owned())?;
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["log", "-1", "--format=%H", "--"])
        .arg(report)
        .output()
        .map_err(|error| format!("git is unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot locate report commit: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let revision = String::from_utf8(output.stdout)
        .map_err(|error| format!("git returned a non-UTF-8 commit id: {error}"))?
        .trim()
        .to_owned();
    Ok((!revision.is_empty()).then_some((repo, report_directory, revision)))
}

#[derive(Clone, Copy)]
struct SignoffChecks {
    signer: SignerChecks,
    meaning_present: bool,
}

#[derive(Clone, Copy)]
struct SignerChecks {
    mapped: bool,
    signature_valid: bool,
    principal_matches: bool,
}

#[implements("FR-218-AC-4")]
#[implements("FR-218-AC-5")]
fn signoff_result(status: &str, reason: Option<&str>, checks: SignoffChecks) -> Value {
    json!({
        "status": status,
        "reason": reason,
        "signatureValid": checks.signer.signature_valid,
        "signerMapped": checks.signer.mapped,
        "principalMatchesRatedBy": checks.signer.principal_matches,
        "meaningPresent": checks.meaning_present,
    })
}

fn required_path(arguments: &ArgMatches, name: &str) -> Result<PathBuf, String> {
    arguments
        .get_one::<String>(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("--{name} is required"))
}

fn find_executable(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

fn field<'a>(object: &'a serde_json::Map<String, Value>, name: &str) -> Option<&'a str> {
    object.get(name).and_then(Value::as_str)
}

fn is_fr_id(id: &str) -> bool {
    id.strip_prefix("FR-").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "the test module reports fixture failures directly"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn run_input(tool: &str, case_status: &str, rating: &str, includes_quire_cli: bool) -> Value {
        let mut documents = vec![
            json!({
                "path": format!("IU-{tool}.md"),
                "raw": format!("---\nid: IU-001\ntype: IntendedUse\ntool: {tool}\n---\nPurpose for {tool}.\n"),
            }),
            json!({
                "path": format!("FRR-{tool}.md"),
                "raw": format!("---\nid: FRR-001\ntype: FeatureRiskRating\nscope: supplier\nrating: {rating}\nrated_by: quality\nintended_use: IU-001\nrelationships:\n  - target: ix://agent-ix/{tool}/FR-001\n    type: rates_feature\n---\n"),
            }),
        ];
        if includes_quire_cli {
            documents.push(json!({
                "path": "IU-quire-cli.md",
                "raw": "---\nid: IU-002\ntype: IntendedUse\ntool: quire-cli\n---\nPurpose for quire-cli.\n",
            }));
        }
        json!({
            "schemaVersion": 1,
            "tool": tool,
            "toolVersion": format!("{tool} 1.0.0"),
            "os": "linux",
            "architecture": "x86_64",
            "suite": "SELFTEST",
            "timestamp": "2026-11-25T12:00:00Z",
            "results": [{
                "id": "case-one",
                "status": case_status,
                "criteria": ["FR-001-AC-1"],
                "detail": if case_status == "pass" { "" } else { "fixture failure" },
                "stderr": "",
            }],
            "validationInputs": {"provided": true, "documents": documents},
            "anomalySet": {
                "provided": true,
                "producerVersion": format!("{tool} 1.0.0"),
                "documents": [],
            },
            "externalTools": [],
            "retainedLocation": format!("spec/evidence/runs/{tool}/run.json"),
        })
    }

    fn write_run(directory: &Path, name: &str, record: &Value) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, serde_json::to_vec(record).expect("run serializes"))
            .expect("run record written");
        path
    }

    #[trace(
        "FR-217-AC-1",
        "FR-217-AC-3",
        "FR-217-AC-4",
        "FR-218-AC-1",
        "FR-218-AC-6"
    )]
    #[test]
    fn report_renders_two_tools_coverage_and_signoff_deterministically() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let quoin = write_run(
            temp.path(),
            "quoin.json",
            &run_input("quoin", "pass", "high_process_risk", true),
        );
        let quire_cli = write_run(
            temp.path(),
            "quire-cli.json",
            &run_input("quire-cli", "pass", "not_high_process_risk", false),
        );
        let paths = [quoin, quire_cli];
        let first = build_report(&paths).expect("report builds");
        let second = build_report(&paths).expect("repeat report builds");
        assert_eq!(first.status, "complete");
        assert_eq!(first.tools.len(), 2);
        assert_eq!(first.coverage.len(), 2);
        assert_eq!(first.rated_by.as_deref(), Some("quality"));
        assert_eq!(first.known_anomalies.status, "none known");
        assert_eq!(
            render_markdown(&first).as_bytes(),
            render_markdown(&second).as_bytes()
        );
        let json_first = serde_json::to_vec(&first).expect("report serializes");
        let json_second = serde_json::to_vec(&second).expect("repeat report serializes");
        assert_eq!(json_first, json_second);
        let json_text = String::from_utf8(json_first).expect("report JSON is UTF-8");
        assert!(!json_text.contains("digest"));
        assert!(!json_text.contains("commitId"));
        assert!(
            !render_markdown(&first)
                .to_ascii_lowercase()
                .contains("score")
        );
        assert_eq!(first.sign_off.name, "");
    }

    #[trace("FR-217-AC-8")]
    #[test]
    fn known_anomalies_lists_only_embedded_quire_tool_records() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let mut quire_cli_record = run_input("quire-cli", "pass", "not_high_process_risk", false);
        quire_cli_record["anomalySet"]["documents"] = json!([
            {"path":"AN-CLI-001.md", "raw":"---\nid: AN-CLI-001\ntitle: CLI retained finding\ntype: Anomaly\nstatus: identified\nsafety_impact: impacting\nseverity: major\n---\n"}
        ]);
        let quire_cli = write_run(temp.path(), "quire-cli-anomaly.json", &quire_cli_record);
        let mut quire_rs_record = run_input("quire-rs", "pass", "not_high_process_risk", false);
        quire_rs_record["anomalySet"]["documents"] = json!([
            {"path":"AN-RS-001.md", "raw":"---\nid: AN-RS-001\ntitle: Rust retained finding\ntype: Anomaly\nstatus: identified\nsafety_impact: none\nseverity: minor\n---\n"}
        ]);
        let quire_rs = write_run(temp.path(), "quire-rs-anomaly.json", &quire_rs_record);
        let report = build_report(&[quire_cli, quire_rs]).expect("report builds");
        let producer_names = report
            .known_anomalies
            .producers
            .iter()
            .map(|producer| producer.tool.as_str())
            .collect::<Vec<_>>();
        assert_eq!(producer_names, ["quire-cli", "quire-rs"]);
        assert_eq!(
            report
                .known_anomalies
                .anomalies
                .iter()
                .map(|row| row.anomaly.as_str())
                .collect::<Vec<_>>(),
            ["AN-CLI-001", "AN-RS-001"]
        );

        let markdown = render_markdown(&report);
        assert!(markdown.contains("Anomaly set: quire-cli quire-cli 1.0.0 — provided."));
        assert!(markdown.contains("Anomaly set: quire-rs quire-rs 1.0.0 — provided."));
        assert!(markdown.contains("CLI retained finding"));
        assert!(markdown.contains("Rust retained finding"));
        assert!(!markdown.contains("Anomaly set: quoin"));
    }

    #[trace("FR-219-AC-1")]
    #[test]
    fn report_lists_ea_and_filament_analyze_versions_from_run_records() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let engineering_assurance = write_run(
            temp.path(),
            "engineering-assurance.json",
            &run_input(
                "engineering-assurance",
                "pass",
                "not_high_process_risk",
                false,
            ),
        );
        let filament_analyze = write_run(
            temp.path(),
            "filament-analyze.json",
            &run_input("filament-analyze", "pass", "not_high_process_risk", false),
        );

        let report =
            build_report(&[engineering_assurance, filament_analyze]).expect("report builds");
        let tools = report
            .tools
            .iter()
            .map(|tool| (tool.name.as_str(), tool.version.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            tools,
            [
                ("engineering-assurance", "engineering-assurance 1.0.0"),
                ("filament-analyze", "filament-analyze 1.0.0"),
            ]
        );
    }

    #[trace("FR-217-AC-2", "FR-218-AC-2")]
    #[test]
    fn incomplete_report_keeps_outcomes_separate_from_missing_tool_evidence() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let quoin = write_run(
            temp.path(),
            "quoin.json",
            &run_input("quoin", "fail", "high_process_risk", true),
        );
        let report = build_report(&[quoin]).expect("incomplete report still builds");
        assert_eq!(report.status, "incomplete");
        assert!(
            report
                .incomplete_reasons
                .iter()
                .any(|reason| reason.contains("result missing for tool quire-cli"))
        );
        assert_eq!(report.results[0].result, "fail");
        assert_eq!(report.coverage[0].result, "uncovered");
        assert!(render_markdown(&report).contains("not provided"));
    }

    #[trace("FR-218-AC-1", "FR-218-AC-3")]
    #[test]
    fn test_failures_and_uncovered_requirements_do_not_make_evidence_incomplete() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let quoin = write_run(
            temp.path(),
            "quoin.json",
            &run_input("quoin", "fail", "high_process_risk", true),
        );
        let quire_cli = write_run(
            temp.path(),
            "quire-cli.json",
            &run_input("quire-cli", "pass", "not_high_process_risk", false),
        );
        let mut quoin_record: Value =
            serde_json::from_slice(&std::fs::read(&quoin).expect("Quoin record reads"))
                .expect("Quoin record parses");
        quoin_record["results"]
            .as_array_mut()
            .expect("result list")
            .push(json!({
                "id": "case-error",
                "status": "error",
                "criteria": ["FR-002-AC-1"],
                "detail": "fixture execution error",
                "stderr": "fixture stderr",
            }));
        std::fs::write(
            &quoin,
            serde_json::to_vec(&quoin_record).expect("updated record serializes"),
        )
        .expect("updated Quoin record writes");

        let report = build_report(&[quoin, quire_cli]).expect("complete report builds");
        assert_eq!(report.status, "complete");
        assert_eq!(report.incomplete_reasons, [] as [std::string::String; 0]);
        assert!(report.results.iter().any(|case| case.result == "fail"));
        assert!(report.results.iter().any(|case| case.result == "error"));
        assert!(
            report
                .coverage
                .iter()
                .any(|coverage| { coverage.fr == "FR-001" && coverage.result == "uncovered" })
        );
        assert!(
            report
                .coverage
                .iter()
                .any(|coverage| { coverage.fr == "FR-002" && coverage.result == "uncovered" })
        );
        let markdown = render_markdown(&report);
        assert!(markdown.contains("| case-one | quoin | FR-001-AC-1 | fail |"));
        assert!(markdown.contains("| case-error | quoin | FR-002-AC-1 | error |"));
        assert!(markdown.contains("| quoin | FR-001 | high_process_risk | rated |"));
    }

    #[trace("FR-217-AC-11")]
    #[test]
    fn feature_ratings_do_not_select_tests_or_change_coverage() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let high = write_run(
            temp.path(),
            "high.json",
            &run_input("quoin", "pass", "high_process_risk", false),
        );
        let not_high = write_run(
            temp.path(),
            "not-high.json",
            &run_input("quoin", "pass", "not_high_process_risk", false),
        );
        let high_report = build_report(&[high]).expect("high-rating report builds");
        let not_high_report = build_report(&[not_high]).expect("not-high report builds");
        assert_eq!(high_report.status, "complete");
        assert_eq!(not_high_report.status, "complete");
        assert_eq!(
            serde_json::to_value(&high_report.results).expect("high results serialize"),
            serde_json::to_value(&not_high_report.results).expect("not-high results serialize")
        );
        assert_eq!(
            high_report.coverage[0].cases,
            not_high_report.coverage[0].cases
        );
        assert_eq!(
            high_report.coverage[0].result,
            not_high_report.coverage[0].result
        );
        assert_eq!(high_report.coverage[0].result, "covered");
        assert_eq!(high_report.coverage[0].rating, "high_process_risk");
        assert_eq!(not_high_report.coverage[0].rating, "not_high_process_risk");
    }

    #[trace("FR-217-AC-12")]
    #[test]
    fn producer_execution_details_do_not_supply_test_coverage() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let mut run = run_input("quoin", "pass", "high_process_risk", false);
        run["results"][0]["criteria"] = json!([]);
        run["results"][0]["detail"] =
            json!("producer-execute completed; artifact: reports/producer-result.json");
        let path = write_run(temp.path(), "producer-details.json", &run);
        let report = build_report(&[path]).expect("report builds");
        assert_eq!(report.status, "complete");
        assert_eq!(
            report.results[0].detail,
            "producer-execute completed; artifact: reports/producer-result.json"
        );
        assert_eq!(report.results[0].result, "pass");
        assert_eq!(report.coverage[0].result, "uncovered");
        let markdown = render_markdown(&report);
        assert!(
            markdown.contains("producer-execute completed; artifact: reports/producer-result.json")
        );
    }

    #[trace("FR-217-AC-10")]
    #[test]
    fn duplicate_supplier_records_are_preserved_by_completeness_assessment() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let mut run = run_input("quoin", "pass", "high_process_risk", false);
        run["validationInputs"]["documents"]
            .as_array_mut()
            .expect("documents array")
            .push(json!({
                "path": "FRR-002.md",
                "raw": "---\nid: FRR-002\ntype: FeatureRiskRating\nscope: supplier\nrating: high_process_risk\nrated_by: quality\nintended_use: IU-001\nrelationships:\n  - target: ix://agent-ix/quoin/FR-001\n    type: rates_feature\n---\n",
            }));
        let path = write_run(temp.path(), "quoin.json", &run);
        let report = build_report(&[path]).expect("report builds");
        let coverage = report
            .coverage
            .iter()
            .find(|coverage| coverage.fr == "FR-001")
            .expect("FR coverage");
        assert_eq!(coverage.rating_status, "rated_more_than_once");
        assert_eq!(coverage.rating, "multiple ratings");
        assert_eq!(
            coverage
                .ratings
                .iter()
                .map(|rating| rating.id.as_str())
                .collect::<Vec<_>>(),
            ["FRR-001", "FRR-002"]
        );
    }

    #[trace(
        "FR-217-AC-5",
        "FR-217-AC-6",
        "FR-217-AC-7",
        "FR-217-AC-8",
        "FR-219-AC-3"
    )]
    #[test]
    fn known_anomalies_uses_version_matched_retained_fr162_selection() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let mut run = run_input("quoin", "pass", "high_process_risk", false);
        run["anomalySet"]["documents"] = json!([
            {"path":"AN-001-open.md", "raw":"---\nid: AN-001\ntitle: High risk open\ntype: Anomaly\nstatus: identified\nsafety_impact: impacting\nseverity: major\n---\n# [AN-001] High risk open\n\n## Safety Impact Assessment\n\n| Kind | Target | Impact | Rationale |\n|------|--------|--------|-----------|\n| Requirement | FR-001 | Affects | Fixture |\n"},
            {"path":"AN-002-low.md", "raw":"---\nid: AN-002\ntitle: Low risk open\ntype: Anomaly\nstatus: identified\nsafety_impact: none\nseverity: minor\n---\n# [AN-002] Low risk open\n\n## Safety Impact Assessment\n\n| Kind | Target | Impact | Rationale |\n|------|--------|--------|-----------|\n| Requirement | FR-002 | Affects | Fixture |\n"},
            {"path":"AN-003-fixed.md", "raw":"---\nid: AN-003\ntitle: Fixed\ntype: Anomaly\nstatus: closed\nresolution: fixed\nsafety_impact: impacting\n---\n"},
            {"path":"AN-004-wont-fix.md", "raw":"---\nid: AN-004\ntitle: Won't fix\ntype: Anomaly\nstatus: closed\nresolution: wont-fix\nsafety_impact: unassessed\n---\n"},
            {"path":"linked-engine/AN-005-linked.md", "raw":"---\nid: AN-005\ntitle: Linked engine issue\ntype: Anomaly\nstatus: identified\nsafety_impact: none\nseverity: minor\n---\n# [AN-005] Linked engine issue\n"}
        ]);
        let path = write_run(temp.path(), "quoin.json", &run);
        let validation_report = build_report(&[path]).expect("anomaly selection");
        let report = &validation_report.known_anomalies;
        assert_eq!(
            report
                .anomalies
                .iter()
                .map(|row| row.anomaly.as_str())
                .collect::<Vec<_>>(),
            ["AN-001", "AN-002", "AN-004", "AN-005"]
        );
        assert_eq!(
            report.anomalies[0].highlight.as_deref(),
            Some("affects high-process-risk feature FR-001")
        );
        assert_eq!(report.anomalies[2].safety_impact, "unassessed");
        assert_eq!(report.anomalies[2].severity, "not stated");
        assert_eq!(report.status, "listed");
        let markdown = render_markdown(&validation_report);
        assert!(markdown.contains("High risk open (affects high-process-risk feature FR-001)"));
    }

    #[trace("FR-217-AC-6", "FR-219-AC-2")]
    #[test]
    fn absent_anomaly_set_is_distinct_from_version_matched_empty_set() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let empty = write_run(
            temp.path(),
            "empty.json",
            &run_input("quoin", "pass", "high_process_risk", false),
        );
        let empty_report = build_report(&[empty]).expect("empty set report builds");
        assert_eq!(empty_report.known_anomalies.status, "none known");

        let mut run = run_input("quoin", "pass", "high_process_risk", false);
        run["anomalySet"]["provided"] = json!(false);
        let path = write_run(temp.path(), "absent.json", &run);
        let report = build_report(&[path]).expect("report builds");
        assert_eq!(report.known_anomalies.status, "not provided");
        assert_eq!(report.known_anomalies.producers[0].status, "not provided");
    }

    #[trace("FR-217-AC-9")]
    #[test]
    fn anomaly_set_from_a_different_producer_version_is_not_used() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let mut run = run_input("quoin", "pass", "high_process_risk", false);
        run["anomalySet"]["producerVersion"] = json!("quoin 0.9.0");
        run["anomalySet"]["documents"] = json!([
            {"path":"AN-001.md", "raw":"---\nid: AN-001\ntitle: Old anomaly\ntype: Anomaly\nstatus: identified\nsafety_impact: impacting\n---\n"}
        ]);
        let path = write_run(temp.path(), "quoin.json", &run);
        let report = build_report(&[path]).expect("report builds");
        assert_eq!(report.known_anomalies.status, "not provided");
        assert!(report.known_anomalies.anomalies.is_empty());
        assert!(
            report
                .incomplete_reasons
                .iter()
                .any(|reason| reason.contains("version mismatch"))
        );
    }

    #[trace("FR-220-AC-1")]
    #[test]
    fn external_tools_index_uses_unique_ids_from_the_authoritative_index() {
        let index = embedded_external_tools_index(EMBEDDED_EXTERNAL_TOOLS_INDEX);
        assert!(index.valid);
        assert_eq!(index.entries.len(), 9);
        let ids = index.entries.keys().collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), index.entries.len());
        assert_eq!(index.report.expect("index report").path, INDEX_PATH);
    }

    #[trace("FR-220-AC-2")]
    #[test]
    fn external_tool_evidence_resolves_exactly_and_preserves_observed_version() {
        let index = embedded_external_tools_index(EMBEDDED_EXTERNAL_TOOLS_INDEX);
        let entry = index.entries.values().next().expect("indexed tool");
        let record = run_record_with_evidence(json!({
            "tool_id": entry.id,
            "executable": entry.executable,
            "version_invocation": entry.version_invocation,
            "observed_version_output": "fixture tool 1.2.3",
            "observed_version_stream": "stdout",
            "index_path": INDEX_PATH,
            "index_format_version": "1",
        }));
        let rows = external_tool_report(&[record], &index);
        let row = rows
            .iter()
            .find(|row| row.tool_id == entry.id)
            .expect("tool row");
        assert_eq!(row.status, "observed");
        assert_eq!(row.executable, entry.executable);
        assert_eq!(row.version_invocation, entry.version_invocation);
        assert_eq!(row.observed_version_output, "fixture tool 1.2.3");
        assert_eq!(row.observed_version_stream, "stdout");
        assert_eq!(row.index_path, INDEX_PATH);
        assert_eq!(row.index_format_version, "1");
    }

    #[trace("FR-220-AC-3")]
    #[test]
    fn unavailable_external_tool_case_remains_not_executed_and_not_passing() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let mut run = run_input("quoin", "not-executed", "high_process_risk", false);
        run["results"][0]["detail"] = json!("required external tool git was unavailable");
        let path = write_run(temp.path(), "unavailable.json", &run);
        let report = build_report(&[path]).expect("report builds");
        assert_eq!(report.status, "incomplete");
        assert_eq!(report.results[0].result, "not-executed");
        assert!(
            report
                .incomplete_reasons
                .iter()
                .any(|reason| reason.contains("not-executed tool quoin"))
        );
        assert!(!report.results.iter().any(|result| result.result == "pass"));
    }

    #[trace("FR-220-AC-4")]
    #[test]
    fn invalid_index_entries_and_mismatched_evidence_do_not_hide_valid_entries() {
        let base = embedded_external_tools_index(EMBEDDED_EXTERNAL_TOOLS_INDEX);
        let base_entries = base.entries.values().cloned().collect::<Vec<_>>();
        let duplicate = base_entries[0].clone();
        let missing = base_entries[2].clone();
        let mismatched_identity = base_entries[1].clone();
        let mismatched_invocation = base_entries[4].clone();
        let valid = base_entries[3].clone();
        let mut raw_index: Value =
            serde_json::from_slice(EMBEDDED_EXTERNAL_TOOLS_INDEX.expect("index bytes"))
                .expect("index JSON");
        let entries = raw_index["entries"].as_array_mut().expect("entries array");
        entries.retain(|entry| entry["id"] != missing.id);
        entries.push(serde_json::to_value(&duplicate).expect("duplicate entry serializes"));
        let damaged = serde_json::to_vec(&raw_index).expect("damaged index JSON");
        let index = embedded_external_tools_index(Some(&damaged));
        assert!(!index.valid);
        assert!(index.invalid_ids.contains(&duplicate.id));
        assert!(index.entries.contains_key(&valid.id));
        assert!(index.entries.contains_key(&mismatched_identity.id));
        assert!(index.entries.contains_key(&mismatched_invocation.id));

        let record = run_record_with_evidence(json!([
            {
                "tool_id": duplicate.id,
                "executable": duplicate.executable,
                "version_invocation": duplicate.version_invocation,
                "observed_version_output": "duplicate",
                "observed_version_stream": "stdout",
                "index_path": INDEX_PATH,
                "index_format_version": "1"
            },
            {
                "tool_id": missing.id,
                "executable": missing.executable,
                "version_invocation": missing.version_invocation,
                "observed_version_output": "missing index entry",
                "observed_version_stream": "stdout",
                "index_path": INDEX_PATH,
                "index_format_version": "1"
            },
            {
                "tool_id": mismatched_identity.id,
                "executable": "different-executable",
                "version_invocation": mismatched_identity.version_invocation,
                "observed_version_output": "mismatched identity",
                "observed_version_stream": "stdout",
                "index_path": INDEX_PATH,
                "index_format_version": "1"
            },
            {
                "tool_id": mismatched_invocation.id,
                "executable": mismatched_invocation.executable,
                "version_invocation": ["--wrong-version-flag"],
                "observed_version_output": "mismatched invocation",
                "observed_version_stream": "stdout",
                "index_path": INDEX_PATH,
                "index_format_version": "1"
            },
            {
                "tool_id": valid.id,
                "executable": valid.executable,
                "version_invocation": valid.version_invocation,
                "observed_version_output": "valid tool version",
                "observed_version_stream": "stderr",
                "index_path": INDEX_PATH,
                "index_format_version": "1"
            }
        ]));
        let rows = external_tool_report(&[record], &index);
        for id in [
            duplicate.id.as_str(),
            missing.id.as_str(),
            mismatched_identity.id.as_str(),
            mismatched_invocation.id.as_str(),
        ] {
            let row = rows
                .iter()
                .find(|row| row.tool_id == id)
                .expect("affected tool row");
            assert_eq!(row.status, "not provided");
            assert_eq!(row.observed_version_output, "not provided");
        }
        let valid_row = rows
            .iter()
            .find(|row| row.tool_id == valid.id)
            .expect("valid tool row");
        assert_eq!(valid_row.status, "observed");
        assert_eq!(valid_row.observed_version_output, "valid tool version");
        assert!(
            !index
                .report
                .expect("partial index report")
                .entries
                .is_empty()
        );
    }

    fn run_record_with_evidence(evidence: Value) -> RunRecord {
        let mut value = run_input("quoin", "pass", "high_process_risk", false);
        value["externalTools"] = if evidence.is_array() {
            evidence
        } else {
            json!([evidence])
        };
        serde_json::from_value(value).expect("run record deserializes")
    }
}
