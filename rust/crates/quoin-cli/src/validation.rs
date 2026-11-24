// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplier feature-rating validation command (PLAT-1636).

use std::path::Path;
use std::process::Command as ProcessCommand;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_completeness::{FeatureRatingState, RatingStatus, read_bundle_frontmatter};
use quoin_core::protocol::Response;

pub(crate) fn command() -> Command {
    Command::new("validation")
        .about("Report authored validation evidence and supplier rating completeness")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("ratings")
                .about("Report supplier feature ratings for each functional requirement")
                .arg(
                    Arg::new("repo")
                        .long("repo")
                        .default_value(".")
                        .help("Repository whose origin URI defines local requirement identity"),
                )
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let Some(("ratings", ratings)) = arguments.subcommand() else {
        return Err("validation requires a subcommand".to_owned());
    };
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
    let repository_namespace = repository_namespace(Path::new(repo))?;
    let parsed = read_bundle_frontmatter(&bundle);
    let features =
        quoin_completeness::assess_feature_ratings(&parsed.documents, &repository_namespace);
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

fn repository_namespace(repo: &Path) -> Result<String, String> {
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|error| format!("could not read repository origin: {error}"))?;
    if !output.status.success() {
        return Err(
            "could not determine repository identity: origin remote is unavailable".to_owned(),
        );
    }
    let remote = std::str::from_utf8(&output.stdout)
        .map_err(|_| "repository origin URL is not UTF-8".to_owned())?
        .trim();
    let path = if let Some((_, path)) = remote
        .rsplit_once(':')
        .filter(|(prefix, _)| prefix.contains('@') && !prefix.contains("://"))
    {
        path
    } else if let Some((_, rest)) = remote.split_once("://") {
        rest.split_once('/').map_or("", |(_, path)| path)
    } else {
        return Err("repository origin URL has no supported URI form".to_owned());
    };
    let namespace = path
        .trim_matches('/')
        .strip_suffix(".git")
        .unwrap_or(path.trim_matches('/'));
    if namespace.is_empty()
        || namespace
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err("repository origin URL has no valid namespace path".to_owned());
    }
    Ok(namespace.to_owned())
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
