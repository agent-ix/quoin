// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplier feature-rating validation command (PLAT-1636).

use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_completeness::{FeatureRatingState, read_bundle_frontmatter};
use quoin_core::protocol::Response;

pub(crate) fn command() -> Command {
    Command::new("validation")
        .about("Report authored validation evidence and supplier rating completeness")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("ratings")
                .about("Report supplier feature ratings for each functional requirement")
                .arg(Arg::new("repo").long("repo").default_value("."))
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
    let parsed = read_bundle_frontmatter(&bundle);
    let features = quoin_completeness::assess_feature_ratings(&parsed.documents);
    let payload = serde_json::json!({
        "features": features,
        "unreadable": parsed.unreadable,
    });
    let rendered = if ratings.get_flag("json") {
        serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?
    } else {
        render(&features)
    };
    let duplicates = features
        .iter()
        .any(|feature| feature.status == quoin_completeness::RatingStatus::RatedMoreThanOnce);
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: Vec::new(),
        outcome: if duplicates {
            ix_cli_kit::exit::Outcome::Partial
        } else {
            ix_cli_kit::exit::Outcome::Ok
        },
    })
}

fn render(features: &[FeatureRatingState]) -> String {
    if features.is_empty() {
        return "supplier feature ratings: no functional requirements found".to_owned();
    }
    features
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
        .collect::<Vec<_>>()
        .join("\n")
}

fn status(feature: &FeatureRatingState) -> &'static str {
    match feature.status {
        quoin_completeness::RatingStatus::Unrated => "unrated",
        quoin_completeness::RatingStatus::Rated => "rated",
        quoin_completeness::RatingStatus::RatedMoreThanOnce => "rated_more_than_once",
    }
}
