// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin anomaly`: the anomaly lifecycle check over git history and the
//! reverse references from a requirement or hazard (FR-136).
//!
//! `quoin anomaly check` reports what no module mapping can express; it never
//! repeats `quire validate`'s dangling-reference or kind diagnostics.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_anomaly::{Context, Severity, anomaly_documents, check_document, references_to};
use quoin_core::protocol::{Diagnostic, Response};

use crate::core_runtime::invoke;

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str =
    "Check anomaly status history and list the anomalies that name an id.";

pub(crate) fn command() -> Command {
    Command::new("anomaly")
        .about(ABOUT)
        .subcommand_required(true)
        .subcommand(
            Command::new("check")
                .about("Check every Anomaly document in the scope against its git history")
                .arg(
                    Arg::new("scope")
                        .value_name("SCOPE")
                        .default_value(".")
                        .help("Directory searched for Anomaly documents"),
                )
                .arg(module_arg()),
        )
        .subcommand(
            Command::new("refs")
                .about("List the anomalies whose assessment names an id")
                .arg(Arg::new("id").value_name("ID").required(true))
                .arg(
                    Arg::new("scope")
                        .long("scope")
                        .value_name("DIR")
                        .default_value(".")
                        .help("Directory holding the corpus and its anomalies"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(ArgAction::SetTrue)
                        .help("Emit one top-level JSON array"),
                ),
        )
}

fn module_arg() -> Arg {
    Arg::new("module")
        .long("module")
        .value_name("DIR")
        .action(ArgAction::Append)
        .help("Module root to look for the `hazard` object type in; repeat to add roots. Without it the installed modules are searched")
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    match arguments.subcommand() {
        Some(("check", check)) => run_check(check),
        Some(("refs", refs)) => run_refs(refs),
        _ => Err("an unknown anomaly command reached dispatch".to_owned()),
    }
}

fn scope_of(arguments: &ArgMatches) -> PathBuf {
    arguments
        .get_one::<String>("scope")
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

/// A refusal that names the path and what is wrong with it. It exits with the
/// refused status, not the "diagnostics found" status of `check`.
fn refuse_missing(what: &str, path: &std::path::Path) -> Response {
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: "anomaly.path_missing".to_owned(),
            message: format!(
                "{what} {} does not exist (no such file or directory)",
                path.display()
            ),
            context: BTreeMap::new(),
        }],
        outcome: Outcome::Refused,
    }
}

fn run_check(arguments: &ArgMatches) -> Result<Response, String> {
    let scope = scope_of(arguments);
    if !scope.is_dir() {
        return Ok(refuse_missing("scope", &scope));
    }
    let roots = arguments
        .get_many::<String>("module")
        .map(|roots| roots.map(PathBuf::from).collect::<Vec<_>>());
    if let Some(missing) = roots.iter().flatten().find(|root| !root.exists()) {
        return Ok(refuse_missing("module root", missing));
    }
    let context = Context {
        hazard_installed: hazard_installed(roots.as_deref())?,
    };
    let mut lines = Vec::new();
    let mut errors = false;
    for (path, text) in anomaly_documents(&scope).map_err(|error| error.to_string())? {
        let checked = check_document(&path, &text, context).map_err(|error| error.to_string())?;
        if !checked.history_applicable {
            lines.push(format!(
                "{}: statusHistory not applicable (the file is not under git)",
                path.display()
            ));
        }
        for diagnostic in checked.diagnostics {
            errors |= diagnostic.severity == Severity::Error;
            let commits = if diagnostic.commits.is_empty() {
                String::new()
            } else {
                format!(" [commits: {}]", diagnostic.commits.join(", "))
            };
            lines.push(format!(
                "{}:{}: {} {} ({}) {}{commits}",
                path.display(),
                diagnostic.line,
                diagnostic.severity.as_str(),
                diagnostic.code.as_str(),
                diagnostic.reason(),
                diagnostic.message,
            ));
        }
    }
    let outcome = if errors {
        Outcome::Partial
    } else {
        Outcome::Ok
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": lines.join("\n") }),
        diagnostics: Vec::new(),
        outcome,
    })
}

fn run_refs(arguments: &ArgMatches) -> Result<Response, String> {
    let id = arguments
        .get_one::<String>("id")
        .ok_or_else(|| "<ID> is required".to_owned())?;
    let scope = scope_of(arguments);
    if !scope.is_dir() {
        return Ok(refuse_missing("scope", &scope));
    }
    let entries = references_to(&scope, id).map_err(|error| error.to_string())?;
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string_pretty(&entries).map_err(|error| error.to_string())?
    } else {
        entries
            .iter()
            .map(|entry| format!("{}\t{}\t{}", entry.anomaly, entry.kind, entry.impact))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

/// Whether the `hazard` object type is installed, asked of the core's module
/// catalog: the explicit roots when given, the host's default discovery
/// otherwise.
fn hazard_installed(roots: Option<&[PathBuf]>) -> Result<bool, String> {
    let request = roots.map_or_else(
        || serde_json::json!({}),
        |roots| serde_json::json!({ "roots": roots }),
    );
    let response = invoke("catalog.load", &request)?;
    if !response.outcome.carries_payload() {
        return Err("the module catalog could not be loaded".to_owned());
    }
    Ok(response
        .payload
        .get("modules")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|module| module.get("objectTypes")?.as_array())
        .flatten()
        .any(|name| name.as_str() == Some("hazard")))
}
