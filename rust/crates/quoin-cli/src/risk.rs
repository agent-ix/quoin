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
use quoin_quire::{ModuleRoot, ModuleSelection};

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Advise on risk decisions without making them.";

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
            )
            .arg(
                Arg::new("module")
                    .long("module")
                    .value_name("DIR")
                    .action(clap::ArgAction::Append)
                    .help("Module root that declares the safety record types; repeat to add roots. Without it the installed modules are searched"),
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
        return Ok(refused(
            "risk.scope_missing",
            &format!(
                "scope {} does not exist (no such file or directory)",
                scope.display()
            ),
        ));
    }
    let modules =
        arguments
            .get_many::<String>("module")
            .map_or(Ok(ModuleSelection::Ambient), |roots| {
                roots
                    .map(ModuleRoot::open)
                    .collect::<Result<Vec<_>, _>>()
                    .map(ModuleSelection::Closed)
            });
    let modules = match modules {
        Ok(modules) => modules,
        Err(error) => return Ok(refused("risk.module_missing", &error.to_string())),
    };
    let advice = match quoin_risk_advice::advise(&scope, &modules) {
        Ok(advice) => advice,
        Err(error) => {
            return Ok(refused(
                &format!("risk.{}", error.code.as_str()),
                &error.message,
            ));
        }
    };
    let diagnostics = advice
        .unreadable
        .iter()
        .map(|file| Diagnostic {
            code: "risk.document_unreadable".to_owned(),
            message: format!("{file}; the file is absent from the report"),
            context: BTreeMap::new(),
        })
        .collect();
    let rendered = match arguments.get_one::<String>("format").map(String::as_str) {
        Some("json") => advice.to_json().map_err(|error| error.to_string())?,
        _ => advice.to_markdown(),
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered.trim_end() }),
        diagnostics,
        outcome: Outcome::Ok,
    })
}

/// A refusal: exit status 2 with one diagnostic, and nothing on stdout.
fn refused(code: &str, message: &str) -> Response {
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: code.to_owned(),
            message: message.to_owned(),
            context: BTreeMap::new(),
        }],
        outcome: Outcome::Refused,
    }
}
