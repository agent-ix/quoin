// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplier feature-rating and retained validation-report commands (PLAT-1636, PLAT-1641).

mod report;

use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
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
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        )
        .subcommand(report::command())
}

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
pub(crate) fn embedded_anomaly_set_value(producer_version: &str) -> serde_json::Value {
    let records = EMBEDDED_ANOMALIES.map(|files| {
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
