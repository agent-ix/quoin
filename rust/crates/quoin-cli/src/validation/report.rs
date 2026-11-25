// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Retained validation-report generation and sign-off verification (FR-217, FR-218, FR-220).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_completeness::{DocumentSource, frontmatter_from_sources};
use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::Response;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{EMBEDDED_ANOMALIES, EMBEDDED_EXTERNAL_TOOLS_INDEX, EmbeddedFile};

const SIGNATURE_REQUEST: &str = "engineering-assurance.signature-verify-request/v1";
const SIGNATURE_RESULT: &str = "engineering-assurance.signature-verify-result/v1";
const SIGNED_OFF_MEANING: &str = "tool-validation-accepted";
const INDEX_PATH: &str = "spec/validation/external-tools.v1.json";
const REQUIRED_TOOL_IDS: [&str; 9] = [
    "git",
    "ssh",
    "gitsign",
    "syft",
    "osv-scanner",
    "pandoc",
    "typst",
    "z3",
    "cvc5",
];

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
    let report = build_report(&inputs, EMBEDDED_ANOMALIES)?;
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
    external_tools: Vec<ExternalToolEvidence>,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalToolsIndex {
    format_version: String,
    entries: Vec<ExternalToolEntry>,
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
    cases: Vec<String>,
    result: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnownAnomalies {
    provided: bool,
    status: String,
    anomalies: Vec<KnownAnomaly>,
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

fn build_report(
    paths: &[PathBuf],
    embedded_anomalies: Option<&[EmbeddedFile]>,
) -> Result<ValidationReport, String> {
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

    let (intended_use, mut reasons) = validation_documents(&records);
    let mut tools = Vec::new();
    let mut results = Vec::new();
    let mut required_tools = intended_use
        .iter()
        .map(|item| item.tool.clone())
        .collect::<BTreeSet<_>>();
    for record in &records {
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
        });
        required_tools.remove(&record.tool);
        for case in &record.results {
            match case.status.as_str() {
                "fail" => reasons.push(format!("failed case {}: {}", case.id, record.tool)),
                "error" => reasons.push(format!("errored case {}: {}", case.id, record.tool)),
                "not-executed" => reasons.push(format!(
                    "not-executed tool {} case {}",
                    record.tool, case.id
                )),
                _ => {}
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

    let mut feature_ratings_by_tool = BTreeMap::<(String, String), String>::new();
    let mut feature_cases = BTreeMap::<(String, String), Vec<String>>::new();
    for record in &records {
        if let Some(inputs) = &record.validation_inputs {
            feature_ratings_by_tool.extend(
                feature_ratings(&inputs.documents)
                    .into_iter()
                    .map(|(feature, rating)| ((record.tool.clone(), feature), rating)),
            );
        }
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
    let feature_ids = feature_ratings_by_tool
        .keys()
        .chain(feature_cases.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut coverage = Vec::new();
    for (tool, feature) in feature_ids {
        let rating = feature_ratings_by_tool
            .get(&(tool.clone(), feature.clone()))
            .cloned()
            .unwrap_or_else(|| "not provided".to_owned());
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
        if rating == "high_process_risk" && !passed {
            reasons.push(format!("uncovered FR {feature} for {tool}"));
        }
        coverage.push(Coverage {
            tool: tool.clone(),
            fr: feature,
            rating,
            cases,
            result: result.to_owned(),
        });
    }

    let high_risk = coverage
        .iter()
        .filter(|item| item.tool == "quoin")
        .filter(|item| item.rating == "high_process_risk")
        .map(|item| item.fr.clone())
        .collect::<BTreeSet<_>>();
    let known_anomalies = known_anomalies(embedded_anomalies, &high_risk)?;
    let quoin_rating_documents = records
        .iter()
        .filter(|record| record.tool == "quoin")
        .filter_map(|record| record.validation_inputs.as_ref())
        .flat_map(|inputs| inputs.documents.iter().cloned())
        .collect::<Vec<_>>();
    let external_tools_index = embedded_external_tools_index();
    let external_tools = external_tool_report(&records, external_tools_index.as_ref());
    if external_tools_index.is_none() {
        reasons.push("external-tools index missing or invalid".to_owned());
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
        external_tools_index,
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

fn validation_documents(records: &[RunRecord]) -> (Vec<IntendedUse>, Vec<String>) {
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
    let documents = by_path
        .into_iter()
        .map(|((tool, path), raw)| DocumentSource {
            path: format!("{tool}/{path}"),
            raw,
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
    if !parsed
        .documents
        .iter()
        .any(|document| field(&document.frontmatter, "type") == Some("FeatureRiskRating"))
    {
        reasons.push("feature risk ratings not provided".to_owned());
    }
    (intended_use, reasons)
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

fn feature_ratings(documents: &[DocumentSource]) -> BTreeMap<String, String> {
    let parsed = frontmatter_from_sources(documents, &[]);
    let mut ratings = BTreeMap::<String, BTreeSet<String>>::new();
    for document in parsed.documents.iter().filter(|document| {
        field(&document.frontmatter, "type") == Some("FeatureRiskRating")
            && field(&document.frontmatter, "scope") == Some("supplier")
    }) {
        let Some(rating) = field(&document.frontmatter, "rating") else {
            continue;
        };
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
            let Some(feature) = target.rsplit('/').next().filter(|id| is_fr_id(id)) else {
                continue;
            };
            ratings
                .entry(feature.to_owned())
                .or_default()
                .insert(rating.to_owned());
        }
    }
    ratings
        .into_iter()
        .map(|(feature, values)| {
            let rating = if values.len() == 1 {
                values.first().cloned().unwrap_or_default()
            } else {
                "not provided".to_owned()
            };
            (feature, rating)
        })
        .collect()
}

fn known_anomalies(
    embedded: Option<&[EmbeddedFile]>,
    high_risk: &BTreeSet<String>,
) -> Result<KnownAnomalies, String> {
    let Some(files) = embedded else {
        return Ok(KnownAnomalies {
            provided: false,
            status: "not provided".to_owned(),
            anomalies: Vec::new(),
        });
    };
    let temp = tempfile::tempdir().map_err(|error| format!("create anomaly workspace: {error}"))?;
    let scope = temp.path().join("spec/anomalies");
    for file in files {
        let path = scope.join(file.path);
        let parent = path
            .parent()
            .ok_or_else(|| "embedded anomaly path has no parent".to_owned())?;
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create embedded anomaly directory: {error}"))?;
        std::fs::write(&path, file.bytes)
            .map_err(|error| format!("materialize embedded anomaly {}: {error}", file.path))?;
    }
    let report = quoin_anomaly::build_report(&scope, &[]).map_err(|error| error.to_string())?;
    let mut anomalies = report
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
                .filter(|feature| high_risk.contains(*feature))
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
                tool: "quoin".to_owned(),
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
        .collect::<Vec<_>>();
    anomalies.sort_by(|left, right| {
        left.high_process_risk_features
            .is_empty()
            .cmp(&right.high_process_risk_features.is_empty())
            .then_with(|| left.anomaly.cmp(&right.anomaly))
    });
    Ok(KnownAnomalies {
        provided: true,
        status: if anomalies.is_empty() {
            "none known"
        } else {
            "listed"
        }
        .to_owned(),
        anomalies,
    })
}

fn embedded_external_tools_index() -> Option<ExternalToolsIndexReport> {
    let Some(bytes) = EMBEDDED_EXTERNAL_TOOLS_INDEX else {
        return None;
    };
    let Ok(index) = serde_json::from_slice::<ExternalToolsIndex>(bytes) else {
        return None;
    };
    if index.format_version != "1" || index.entries.len() != REQUIRED_TOOL_IDS.len() {
        return None;
    }
    let ids = index
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    let valid = ids.len() == REQUIRED_TOOL_IDS.len()
        && REQUIRED_TOOL_IDS
            .iter()
            .all(|required| ids.contains(required))
        && index
            .entries
            .iter()
            .all(|entry| !entry.executable.is_empty() && !entry.version_invocation.is_empty());
    valid.then(|| ExternalToolsIndexReport {
        path: INDEX_PATH.to_owned(),
        format_version: index.format_version,
        entries: index.entries,
    })
}

fn external_tool_report(
    records: &[RunRecord],
    index: Option<&ExternalToolsIndexReport>,
) -> Vec<ExternalToolReport> {
    let entries = index.map_or(&[][..], |index| index.entries.as_slice());
    let mut evidence_by_id = BTreeMap::<&str, Vec<&ExternalToolEvidence>>::new();
    for evidence in records.iter().flat_map(|record| &record.external_tools) {
        evidence_by_id
            .entry(&evidence.tool_id)
            .or_default()
            .push(evidence);
    }
    let mut out = entries
        .iter()
        .map(|entry| {
            let candidates = evidence_by_id
                .get(entry.id.as_str())
                .cloned()
                .unwrap_or_default();
            let evidence = (candidates.len() == 1).then(|| candidates[0]);
            let valid = evidence.is_some_and(|evidence| {
                evidence.executable == entry.executable
                    && evidence.version_invocation == entry.version_invocation
                    && evidence.index_path == INDEX_PATH
                    && evidence.index_format_version == "1"
                    && matches!(
                        evidence.observed_version_stream.as_str(),
                        "stdout" | "stderr"
                    )
                    && !evidence.observed_version_output.trim().is_empty()
            });
            ExternalToolReport {
                tool_id: entry.id.clone(),
                executable: entry.executable.clone(),
                version_invocation: entry.version_invocation.clone(),
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
                index_path: INDEX_PATH.to_owned(),
                index_format_version: "1".to_owned(),
                status: if valid { "observed" } else { "not provided" }.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|left, right| left.tool_id.cmp(&right.tool_id));
    out
}

fn render_markdown(report: &ValidationReport) -> String {
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
    lines.extend([
        String::new(),
        "## Coverage".to_owned(),
        String::new(),
        "| Tool | FR | Rating | Cases | Result |".to_owned(),
        "| --- | --- | --- | --- | --- |".to_owned(),
    ]);
    for item in &report.coverage {
        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            cell(&item.tool),
            cell(&item.fr),
            cell(&item.rating),
            cell(&item.cases.join(", ")),
            cell(&item.result)
        ));
    }
    lines.extend([
        String::new(),
        "## Known anomalies".to_owned(),
        String::new(),
    ]);
    if !report.known_anomalies.provided {
        lines.push("not provided".to_owned());
    } else if report.known_anomalies.anomalies.is_empty() {
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
                .map(|reason| format!("- {}", reason)),
        );
    }
    lines.extend([
        String::new(),
        "## Sign-off".to_owned(),
        String::new(),
        "Name: ".to_owned(),
        "Date: ".to_owned(),
        "Meaning: ".to_owned(),
        String::new(),
    ]);
    lines.join("\n")
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

fn verify_signoff(
    executable: &Path,
    allowed_signers: &Path,
    repo: &Path,
    report_directory: &Path,
) -> Result<Value, String> {
    let repo = repo
        .canonicalize()
        .map_err(|error| format!("repository is unavailable: {error}"))?;
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
    if revision.is_empty() {
        return Ok(signoff_result(
            "not_accepted",
            Some("unsigned"),
            false,
            false,
            false,
            false,
        ));
    }
    let request = json!({
        "protocol": SIGNATURE_REQUEST,
        "repository": repo,
        "revision": revision,
        "kind": "commit",
        "allowed_signers": allowed_signers,
        "trailer_keys": ["Signed-off-meaning"],
    });
    let result =
        quoin_core::runtime::engineering_assurance_call(executable, "signature-verify", &request)
            .map_err(|error| error.to_string())?;
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
    let meaning = trailer_values(&value, "Signed-off-meaning");
    let principal_matches = !principal.is_empty() && principal == expected_principal;
    let meaning_present = meaning.iter().any(|value| *value == SIGNED_OFF_MEANING);
    let (result_status, reason) = if status != "signed_valid" {
        (
            "not_accepted",
            Some(if status == "unsigned" {
                "unsigned"
            } else {
                "signature_invalid"
            }),
        )
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
        mapping == "mapped",
        status == "signed_valid",
        principal_matches,
        meaning_present,
    ))
}

fn signoff_result(
    status: &str,
    reason: Option<&str>,
    mapped: bool,
    signature_valid: bool,
    principal_matches: bool,
    meaning_present: bool,
) -> Value {
    json!({
        "status": status,
        "reason": reason,
        "signatureValid": signature_valid,
        "signerMapped": mapped,
        "principalMatchesRatedBy": principal_matches,
        "meaningPresent": meaning_present,
    })
}

fn trailer_values<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value
        .get("trailers")
        .and_then(Value::as_object)
        .and_then(|trailers| trailers.get(key))
        .map(|value| {
            value.as_array().map_or_else(
                || value.as_str().into_iter().collect(),
                |values| values.iter().filter_map(Value::as_str).collect(),
            )
        })
        .unwrap_or_default()
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
        let empty_anomalies: [EmbeddedFile; 0] = [];
        let first = build_report(&paths, Some(&empty_anomalies)).expect("report builds");
        let second = build_report(&paths, Some(&empty_anomalies)).expect("repeat report builds");
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

    #[trace("FR-217-AC-2", "FR-218-AC-2", "FR-218-AC-3")]
    #[test]
    fn incomplete_report_names_uncovered_failed_and_missing_tool_evidence() {
        let temp = tempfile::tempdir().expect("fixture directory");
        let quoin = write_run(
            temp.path(),
            "quoin.json",
            &run_input("quoin", "fail", "high_process_risk", true),
        );
        let report = build_report(&[quoin], None).expect("incomplete report still builds");
        assert_eq!(report.status, "incomplete");
        assert!(
            report
                .incomplete_reasons
                .iter()
                .any(|reason| reason.contains("uncovered FR FR-001"))
        );
        assert!(
            report
                .incomplete_reasons
                .iter()
                .any(|reason| reason.contains("fail case case-one"))
        );
        assert!(
            report
                .incomplete_reasons
                .iter()
                .any(|reason| reason.contains("result missing for tool quire-cli"))
        );
        assert!(render_markdown(&report).contains("not provided"));
    }

    #[trace("FR-217-AC-5", "FR-217-AC-6", "FR-217-AC-7")]
    #[test]
    fn known_anomalies_uses_fr162_selection_and_distinguishes_absent_empty_and_unassessed() {
        let files = [
            EmbeddedFile { path: "AN-001-open.md", bytes: b"---\nid: AN-001\ntitle: High risk open\ntype: Anomaly\nstatus: identified\nsafety_impact: impacting\nseverity: major\nassessment:\n  - kind: Requirement\n    target: FR-001\n    impact: Direct\n    rationale: Fixture\n---\n" },
            EmbeddedFile { path: "AN-002-low.md", bytes: b"---\nid: AN-002\ntitle: Low risk open\ntype: Anomaly\nstatus: identified\nsafety_impact: none\nseverity: minor\nassessment:\n  - kind: Requirement\n    target: FR-002\n    impact: Indirect\n    rationale: Fixture\n---\n" },
            EmbeddedFile { path: "AN-003-fixed.md", bytes: b"---\nid: AN-003\ntitle: Fixed\ntype: Anomaly\nstatus: closed\nresolution: fixed\nsafety_impact: impacting\n---\n" },
            EmbeddedFile { path: "AN-004-wont-fix.md", bytes: b"---\nid: AN-004\ntitle: Won't fix\ntype: Anomaly\nstatus: closed\nresolution: wont-fix\nsafety_impact: unassessed\n---\n" },
        ];
        let high_risk = BTreeSet::from(["FR-001".to_owned()]);
        let report = known_anomalies(Some(&files), &high_risk).expect("anomaly selection");
        assert_eq!(
            report
                .anomalies
                .iter()
                .map(|row| row.anomaly.as_str())
                .collect::<Vec<_>>(),
            ["AN-001", "AN-002", "AN-004"]
        );
        assert_eq!(
            report.anomalies[0].highlight.as_deref(),
            Some("affects high-process-risk feature FR-001")
        );
        assert_eq!(report.anomalies[2].safety_impact, "unassessed");
        assert_eq!(report.anomalies[2].severity, "not stated");
        let absent = known_anomalies(None, &BTreeSet::new()).expect("absent set");
        assert_eq!(absent.status, "not provided");
        let empty_files: [EmbeddedFile; 0] = [];
        let empty = known_anomalies(Some(&empty_files), &BTreeSet::new()).expect("empty set");
        assert_eq!(empty.status, "none known");
    }

    #[trace("FR-220-AC-1")]
    #[test]
    fn external_tools_index_has_the_nine_required_entries() {
        let index = embedded_external_tools_index().expect("embedded external tools index");
        let ids = index
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids, REQUIRED_TOOL_IDS.into_iter().collect());
        assert_eq!(index.path, INDEX_PATH);
    }
}
