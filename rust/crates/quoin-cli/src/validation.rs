// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplier feature-rating and retained validation-report commands (PLAT-1636, PLAT-1641).

mod report;

use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_trace_rs::implements;
use quoin_completeness::{FeatureRatingState, RatingStatus, read_bundle_frontmatter};
use quoin_core::protocol::Response;

/// One build-time embedded validation or anomaly record.
pub(crate) struct EmbeddedFile {
    /// Repository-relative path beneath the record directory.
    pub(crate) path: &'static str,
    /// The record's original bytes.
    pub(crate) bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/embedded_validation.rs"));

#[implements("FR-235-AC-4")]
#[implements("FR-235-AC-11")]
pub(crate) fn command() -> Command {
    Command::new("validation")
        .about("Report authored validation evidence and supplier ratings")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("ratings")
                .about("Report supplier feature ratings for each functional requirement")
                .arg(Arg::new("repo").long("repo").default_value("."))
                .arg(
                    Arg::new("repo_id")
                        .long("repo-id")
                        .value_name("ORG/REPO")
                        .required(true)
                        .help("Repository namespace used to match ix:// requirement targets"),
                )
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("coverage")
                        .long("coverage")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(report::command())
}

#[implements("FR-235-AC-4")]
#[implements("FR-235-AC-11")]
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let Some((subcommand, matches)) = arguments.subcommand() else {
        return Err("validation requires a subcommand".to_owned());
    };
    if subcommand == "report" {
        return report::run(matches);
    }
    if subcommand != "ratings" {
        return Err("validation requires ratings or report".to_owned());
    }
    let ratings = matches;
    let repo = ratings
        .get_one::<String>("repo")
        .map(String::as_str)
        .unwrap_or(".");
    let bundle = Path::new(repo).join("spec");
    if !bundle.is_dir() {
        return Err(format!(
            "spec bundle is not a directory: {}",
            bundle.display()
        ));
    }
    let repository_namespace = ratings
        .get_one::<String>("repo_id")
        .map(String::as_str)
        .ok_or_else(|| "--repo-id is required".to_owned())?;
    validate_repository_namespace(repository_namespace)?;
    let parsed = read_bundle_frontmatter(&bundle);
    let features =
        quoin_completeness::assess_feature_ratings(&parsed.documents, repository_namespace);
    if ratings.get_flag("coverage") {
        return run_coverage(
            repo,
            repository_namespace,
            &parsed.documents,
            &features,
            ratings.get_flag("json"),
        );
    }
    let unreadable = parsed
        .unreadable
        .iter()
        .map(|document| {
            serde_json::json!({
                "path": document.path,
                "reason": document.reason,
            })
        })
        .collect::<Vec<_>>();
    let payload = serde_json::json!({
        "features": &features,
        "unreadable": unreadable,
    });
    let rendered = if ratings.get_flag("json") {
        serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?
    } else {
        render(&features, &parsed.unreadable)
    };
    let has_duplicates = features
        .iter()
        .any(|feature| feature.status == RatingStatus::RatedMoreThanOnce);
    let incomplete = !parsed.unreadable.is_empty() || has_duplicates;
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: Vec::new(),
        outcome: if incomplete {
            ix_cli_kit::exit::Outcome::Partial
        } else {
            ix_cli_kit::exit::Outcome::Ok
        },
    })
}

/// The validation documents embedded in this binary, carried in a run record
/// so report generation does not need a source checkout.
pub(crate) fn embedded_inputs_value() -> serde_json::Value {
    let records = EMBEDDED_VALIDATION_INPUTS.map(|files| {
        files
            .iter()
            .map(|file| {
                serde_json::json!({
                    "path": file.path,
                    "raw": String::from_utf8_lossy(file.bytes),
                })
            })
            .collect::<Vec<_>>()
    });
    serde_json::json!({
        "provided": records.is_some(),
        "documents": records.unwrap_or_default(),
    })
}

/// The anomaly documents and exact version of the binary that embeds them.
pub(crate) fn anomaly_set_value(
    producer_version: &str,
    anomaly_documents: Option<&[EmbeddedFile]>,
) -> serde_json::Value {
    let records = anomaly_documents.map(|files| {
        files
            .iter()
            .map(|file| {
                serde_json::json!({
                    "path": file.path,
                    "raw": String::from_utf8_lossy(file.bytes),
                })
            })
            .collect::<Vec<_>>()
    });
    serde_json::json!({
        "provided": records.is_some(),
        "producerVersion": producer_version,
        "documents": records.unwrap_or_default(),
    })
}

fn validate_repository_namespace(namespace: &str) -> Result<(), String> {
    let mut segments = namespace.split('/');
    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && segment
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    };
    if let (Some(organization), Some(repository), None) =
        (segments.next(), segments.next(), segments.next())
        && valid_segment(organization)
        && valid_segment(repository)
    {
        Ok(())
    } else {
        Err("--repo-id must be an ORG/REPO namespace".to_owned())
    }
}

fn render(
    features: &[FeatureRatingState],
    unreadable: &[quoin_completeness::UnreadableDocument],
) -> String {
    if features.is_empty() {
        let mut lines =
            vec!["supplier feature ratings: no functional requirements found".to_owned()];
        lines.extend(unreadable.iter().map(unreadable_diagnostic));
        return lines.join("\n");
    }
    let mut lines = features
        .iter()
        .map(|feature| {
            let ids = if feature.rating_ids.is_empty() {
                "none".to_owned()
            } else {
                feature.rating_ids.join(", ")
            };
            format!(
                "{}: {} (rating IDs: {})",
                feature.feature_id,
                status(feature),
                ids
            )
        })
        .collect::<Vec<_>>();
    lines.extend(unreadable.iter().map(unreadable_diagnostic));
    lines.join("\n")
}

fn unreadable_diagnostic(document: &quoin_completeness::UnreadableDocument) -> String {
    format!("unreadable: {}: {}", document.path, document.reason)
}

fn status(feature: &FeatureRatingState) -> &'static str {
    match feature.status {
        quoin_completeness::RatingStatus::Unrated => "unrated",
        quoin_completeness::RatingStatus::Rated => "rated",
        quoin_completeness::RatingStatus::RatedMoreThanOnce => "rated_more_than_once",
    }
}

#[implements("FR-235-AC-4")]
#[implements("FR-235-AC-11")]
fn run_coverage(
    repository: &str,
    repository_namespace: &str,
    documents: &[quoin_completeness::BundleDocument],
    features: &[FeatureRatingState],
    json: bool,
) -> Result<Response, String> {
    let functional = documents
        .iter()
        .filter(|document| string_field(&document.frontmatter, "type") == Some("FR"))
        .filter_map(|document| {
            let id = string_field(&document.frontmatter, "id")?.to_owned();
            let criteria = document
                .body
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
                .filter(|token| token.starts_with(&format!("{id}-AC-")))
                .map(str::to_owned)
                .collect::<std::collections::BTreeSet<_>>();
            Some((id, criteria))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let (markers, mut invalid) = read_selftest_markers(Path::new(repository), &functional)?;
    let mut coverage = Vec::new();
    for feature in features {
        let mut risk_values = Vec::new();
        for document in documents {
            if string_field(&document.frontmatter, "type") != Some("FeatureRiskRating")
                || string_field(&document.frontmatter, "scope") != Some("supplier")
                || string_field(&document.frontmatter, "rating").is_none()
            {
                continue;
            }
            let rates_feature = document
                .frontmatter
                .get("relationships")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|relations| {
                    relations.iter().any(|relation| {
                        relation.get("type").and_then(serde_json::Value::as_str)
                            == Some("rates_feature")
                            && relation
                                .get("target")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|target| {
                                    target
                                        == format!(
                                            "ix://{repository_namespace}/{}",
                                            feature.feature_id
                                        )
                                })
                    })
                });
            if rates_feature {
                risk_values.push((
                    string_field(&document.frontmatter, "id").unwrap_or("unknown"),
                    string_field(&document.frontmatter, "rating").unwrap_or("unknown"),
                ));
            }
        }
        let criteria = markers
            .get(&feature.feature_id)
            .cloned()
            .unwrap_or_default();
        let high = risk_values
            .iter()
            .any(|(_, rating)| *rating == "high_process_risk");
        let not_high = risk_values
            .iter()
            .any(|(_, rating)| *rating == "not_high_process_risk");
        let coverage_status = if feature.status == RatingStatus::Unrated {
            "unrated"
        } else if high {
            if criteria.is_empty() {
                "uncovered"
            } else {
                "covered"
            }
        } else if not_high && criteria.is_empty() {
            "release_evidence"
        } else if !criteria.is_empty() {
            "covered"
        } else {
            "unrated"
        };
        coverage.push(serde_json::json!({
            "feature_id": feature.feature_id,
            "status": feature.status,
            "rating_ids": feature.rating_ids,
            "rating_values": risk_values.iter().map(|(id, value)| serde_json::json!({"rating_id": id, "value": value})).collect::<Vec<_>>(),
            "coverage": coverage_status,
            "traced_criteria": criteria,
        }));
    }
    invalid.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
    let uncovered = coverage
        .iter()
        .filter_map(|row| (row["coverage"] == "uncovered").then(|| row["feature_id"].clone()))
        .collect::<Vec<_>>();
    let partial = !uncovered.is_empty() || !invalid.is_empty();
    let payload = serde_json::json!({
        "features": coverage,
        "uncovered": uncovered,
        "invalid_markers": invalid,
    });
    let rendered = if json {
        serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?
    } else {
        let mut lines = coverage
            .iter()
            .map(|row| {
                format!(
                    "{}: {}",
                    row["feature_id"].as_str().unwrap_or("?"),
                    row["coverage"].as_str().unwrap_or("?")
                )
            })
            .collect::<Vec<_>>();
        lines.extend(
            uncovered
                .iter()
                .map(|id| format!("uncovered: {}", id.as_str().unwrap_or("?"))),
        );
        lines.extend(
            invalid
                .iter()
                .map(|marker| format!("invalid marker: {}", marker)),
        );
        lines.join("\n")
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: Vec::new(),
        outcome: if partial {
            ix_cli_kit::exit::Outcome::Partial
        } else {
            ix_cli_kit::exit::Outcome::Ok
        },
    })
}

#[implements("FR-235-AC-4")]
#[implements("FR-235-AC-11")]
fn read_selftest_markers(
    repository: &Path,
    functional: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
) -> Result<
    (
        std::collections::BTreeMap<String, Vec<String>>,
        Vec<serde_json::Value>,
    ),
    String,
> {
    let mut markers = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut invalid = Vec::new();
    let cases = repository.join("selftest/cases");
    if !cases.exists() {
        return Ok((markers, invalid));
    }
    let entries =
        std::fs::read_dir(&cases).map_err(|error| format!("{}: {error}", cases.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", cases.display()))?;
        let script = entry.path().join("run.sh");
        if !script.is_file() {
            continue;
        }
        let source = std::fs::read_to_string(&script)
            .map_err(|error| format!("{}: {error}", script.display()))?;
        for (line_index, line) in source.lines().enumerate() {
            let Some(raw) = line.trim().strip_prefix("# Trace:") else {
                continue;
            };
            let tokens = raw
                .split(|character: char| {
                    character.is_whitespace() || character == ',' || character == ';'
                })
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>();
            if tokens.is_empty() {
                invalid.push(serde_json::json!({"path": script.display().to_string(), "line": line_index + 1, "marker": raw, "reason": "empty trace marker"}));
                continue;
            }
            for token in tokens {
                let parsed = token
                    .strip_prefix("FR-")
                    .and_then(|rest| rest.split_once("-AC-"))
                    .filter(|(feature, criterion)| {
                        !feature.is_empty()
                            && feature.chars().all(|character| character.is_ascii_digit())
                            && !criterion.is_empty()
                            && criterion
                                .chars()
                                .all(|character| character.is_ascii_digit())
                    });
                let Some((feature_number, _)) = parsed else {
                    invalid.push(serde_json::json!({"path": script.display().to_string(), "line": line_index + 1, "marker": token, "reason": "malformed trace marker"}));
                    continue;
                };
                let feature_id = format!("FR-{feature_number}");
                if !functional
                    .get(&feature_id)
                    .is_some_and(|criteria| criteria.contains(token))
                {
                    invalid.push(serde_json::json!({"path": script.display().to_string(), "line": line_index + 1, "marker": token, "reason": "unknown feature or acceptance criterion"}));
                } else {
                    markers
                        .entry(feature_id)
                        .or_default()
                        .push(token.to_owned());
                }
            }
        }
    }
    for criteria in markers.values_mut() {
        criteria.sort();
        criteria.dedup();
    }
    Ok((markers, invalid))
}

fn string_field<'a>(
    frontmatter: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Option<&'a str> {
    frontmatter.get(name).and_then(serde_json::Value::as_str)
}
