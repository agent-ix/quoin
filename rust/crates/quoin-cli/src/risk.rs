// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin risk advise`: the read-only risk advice report (FR-145).
//!
//! The report is built by `quoin-risk-advice`; this module is argv, a refusal
//! for a missing scope, and the choice of format. It writes no file.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::protocol::{Diagnostic, Response};

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str =
    "Advise on risk decisions from the safety records, without deciding.";

pub(crate) fn command() -> Command {
    Command::new("risk").about(ABOUT).subcommand_required(true).subcommand(
        Command::new("advise")
            .about("Print the policy cell of each estimate, its evidence gaps and the undecided decisions")
            .arg(
                Arg::new("scope")
                    .value_name("SCOPE")
                    .default_value(".")
                    .help("Directory holding the safety records, policies, requirements and anomalies"),
            )
            .arg(
                Arg::new("format")
                    .long("format")
                    .value_name("FORMAT")
                    .default_value("markdown")
                    .value_parser(["markdown", "json"])
                    .help("Output format"),
            ),
    )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    match arguments.subcommand() {
        Some(("advise", advise)) => run_advise(advise),
        _ => Err("an unknown risk command reached dispatch".to_owned()),
    }
}

fn run_advise(arguments: &ArgMatches) -> Result<Response, String> {
    let scope = arguments
        .get_one::<String>("scope")
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if !scope.is_dir() {
        return Ok(Response {
            payload: serde_json::Value::Null,
            diagnostics: vec![Diagnostic {
                code: "risk.scope_missing".to_owned(),
                message: format!(
                    "scope {} does not exist (no such file or directory)",
                    scope.display()
                ),
                context: BTreeMap::new(),
            }],
            outcome: Outcome::Refused,
        });
    }
    let advice = quoin_risk_advice::advise(&scope).map_err(|error| error.to_string())?;
    let rendered = match arguments.get_one::<String>("format").map(String::as_str) {
        Some("json") => advice.to_json().map_err(|error| error.to_string())?,
        _ => advice.to_markdown(),
    };
    Ok(Response::ok(
        serde_json::json!({ "rendered": rendered.trim_end() }),
    ))
}
