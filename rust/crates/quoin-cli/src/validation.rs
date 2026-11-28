// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplier feature-rating and retained validation-report commands (PLAT-1636, PLAT-1641).

mod report;
mod run_environment;

#[cfg(test)]
pub(crate) use run_environment::{RepositoryCheckout, TemporaryDirectory};
pub(crate) use run_environment::{RunEnvironment, capture_run_environment};

use std::{collections::BTreeMap, path::Path};

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_trace_rs::implements;
use quoin_completeness::{
    BundleDocument, FeatureRatingCoverage, FeatureRatingCoverageReport, FeatureRatingState,
    FeatureRatingValue, RatingStatus, read_bundle_frontmatter,
};
use quoin_core::protocol::{Diagnostic, Response};

/// One build-time embedded validation or anomaly record.
pub(crate) struct EmbeddedFile {
    /// Repository-relative path beneath the record directory.
    pub(crate) path: &'static str,
    /// The record's original bytes.
    pub(crate) bytes: &'static [u8],
}

#[derive(Clone, Copy)]
enum RatingsDiagnosticCode {
    TargetKindMismatch,
}

impl RatingsDiagnosticCode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::TargetKindMismatch => "target-kind-mismatch",
        }
    }
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
#[implements("FR-235-AC-14")]
#[implements("FR-235-AC-15")]
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
        .map_or(".", String::as_str);
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
    let unreadable = repository_relative_unreadable(&parsed.unreadable);
    let diagnostics = wrong_kind_rating_targets(&parsed.documents, repository_namespace);
    if ratings.get_flag("coverage") {
        return run_coverage(
            repository_namespace,
            &parsed.documents,
            &unreadable,
            &features,
            &diagnostics,
            ratings.get_flag("json"),
        );
    }
    let unreadable_json = unreadable
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
        "unreadable": unreadable_json,
    });
    let rendered = if ratings.get_flag("json") {
        serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?
    } else {
        render(&features, &unreadable)
    };
    let has_duplicates = features
        .iter()
        .any(|feature| feature.status == RatingStatus::RatedMoreThanOnce);
    let incomplete = !unreadable.is_empty() || has_duplicates || !diagnostics.is_empty();
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics,
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

#[implements("FR-235-AC-15")]
fn repository_relative_unreadable(
    unreadable: &[quoin_completeness::UnreadableDocument],
) -> Vec<quoin_completeness::UnreadableDocument> {
    unreadable
        .iter()
        .map(|document| quoin_completeness::UnreadableDocument {
            path: repository_spec_path(&document.path),
            reason: document.reason.clone(),
        })
        .collect()
}

#[implements("FR-235-AC-15")]
fn repository_spec_path(bundle_relative_path: &str) -> String {
    format!("spec/{bundle_relative_path}")
}

#[implements("FR-235-AC-14")]
fn wrong_kind_rating_targets(
    documents: &[BundleDocument],
    repository_namespace: &str,
) -> Vec<Diagnostic> {
    let mut target_kinds = BTreeMap::<String, String>::new();
    for document in documents {
        let (Some(id), Some(kind)) = (
            string_field(&document.frontmatter, "id"),
            string_field(&document.frontmatter, "type"),
        ) else {
            continue;
        };
        target_kinds.insert(format!("ix://{repository_namespace}/{id}"), kind.to_owned());
    }

    let mut found = BTreeMap::<(String, String, String), Diagnostic>::new();
    for document in documents {
        if string_field(&document.frontmatter, "type") != Some("FeatureRiskRating")
            || string_field(&document.frontmatter, "scope") != Some("supplier")
        {
            continue;
        }
        let Some(relations) = document
            .frontmatter
            .get("relationships")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for relation in relations {
            if relation.get("type").and_then(serde_json::Value::as_str) != Some("rates_feature") {
                continue;
            }
            let Some(target) = relation.get("target").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(actual_kind) = target_kinds.get(target) else {
                continue;
            };
            if actual_kind == "FR" {
                continue;
            }
            let key = (
                document.path.clone(),
                target.to_owned(),
                actual_kind.clone(),
            );
            let mut context = BTreeMap::new();
            context.insert("actual_kind".to_owned(), actual_kind.clone());
            context.insert("path".to_owned(), repository_spec_path(&document.path));
            context.insert("target".to_owned(), target.to_owned());
            found.insert(
                key,
                Diagnostic {
                    code: RatingsDiagnosticCode::TargetKindMismatch
                        .as_str()
                        .to_owned(),
                    message: format!(
                        "rates_feature target {target} has kind {actual_kind}; expected FR"
                    ),
                    context,
                },
            );
        }
    }
    found.into_values().collect()
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
#[implements("FR-235-AC-14")]
#[implements("FR-235-AC-15")]
fn run_coverage(
    repository_namespace: &str,
    documents: &[quoin_completeness::BundleDocument],
    unreadable: &[quoin_completeness::UnreadableDocument],
    features: &[FeatureRatingState],
    diagnostics: &[Diagnostic],
    json: bool,
) -> Result<Response, String> {
    let mut coverage = Vec::with_capacity(features.len());
    for feature in features {
        coverage.push(FeatureRatingCoverage {
            rating: feature.clone(),
            rating_values: rating_values_for_feature(
                documents,
                repository_namespace,
                &feature.feature_id,
            ),
        });
    }
    let report = FeatureRatingCoverageReport {
        features: coverage,
        unreadable: unreadable.to_vec(),
    };
    let partial = !report.unreadable.is_empty() || !diagnostics.is_empty();
    let rendered = if json {
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    } else {
        let mut lines = report
            .features
            .iter()
            .map(|feature| {
                let ids = if feature.rating.rating_ids.is_empty() {
                    "none".to_owned()
                } else {
                    feature.rating.rating_ids.join(", ")
                };
                let values = feature
                    .rating_values
                    .iter()
                    .map(|rating| format!("{}={}", rating.rating_id, rating.value))
                    .collect::<Vec<_>>();
                format!(
                    "{}: {} (rating IDs: {}; values: {})",
                    feature.rating.feature_id,
                    status(&feature.rating),
                    ids,
                    if values.is_empty() {
                        "none".to_owned()
                    } else {
                        values.join(", ")
                    }
                )
            })
            .collect::<Vec<_>>();
        lines.extend(
            report
                .unreadable
                .iter()
                .map(|document| format!("unreadable: {}: {}", document.path, document.reason)),
        );
        lines.join("\n")
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: diagnostics.to_vec(),
        outcome: if partial {
            ix_cli_kit::exit::Outcome::Partial
        } else {
            ix_cli_kit::exit::Outcome::Ok
        },
    })
}

#[implements("FR-235-AC-4")]
#[implements("FR-235-AC-11")]
fn rating_values_for_feature(
    documents: &[BundleDocument],
    repository_namespace: &str,
    feature_id: &str,
) -> Vec<FeatureRatingValue> {
    let feature_target = format!("ix://{repository_namespace}/{feature_id}");
    documents
        .iter()
        .filter(|document| {
            string_field(&document.frontmatter, "type") == Some("FeatureRiskRating")
                && string_field(&document.frontmatter, "scope") == Some("supplier")
                && document
                    .frontmatter
                    .get("relationships")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|relations| {
                        relations.iter().any(|relation| {
                            relation.get("type").and_then(serde_json::Value::as_str)
                                == Some("rates_feature")
                                && relation.get("target").and_then(serde_json::Value::as_str)
                                    == Some(feature_target.as_str())
                        })
                    })
        })
        .filter_map(|document| {
            Some(FeatureRatingValue {
                rating_id: string_field(&document.frontmatter, "id")?.to_owned(),
                value: string_field(&document.frontmatter, "rating")?.to_owned(),
            })
        })
        .collect()
}

#[implements("FR-235-AC-4")]
#[implements("FR-235-AC-11")]
fn string_field<'a>(
    frontmatter: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Option<&'a str> {
    frontmatter.get(name).and_then(serde_json::Value::as_str)
}
