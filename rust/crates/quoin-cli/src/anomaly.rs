// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin anomaly`: the anomaly lifecycle check over git history and the
//! reverse references from a requirement or hazard (FR-136).
//!
//! `quoin anomaly check` reports what no module mapping can express; it never
//! repeats `quire validate`'s dangling-reference or kind diagnostics.

use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_anomaly::{Context, anomaly_documents, check_document, references_to};
use quoin_core::protocol::Response;

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
        .help("Module root to look for the `hazard` object type in; repeat to add roots")
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

fn run_check(arguments: &ArgMatches) -> Result<Response, String> {
    let scope = scope_of(arguments);
    let roots = arguments
        .get_many::<String>("module")
        .map_or_else(default_roots, |roots| roots.map(PathBuf::from).collect());
    let context = Context {
        hazard_installed: roots.iter().any(|root| declares_hazard(root)),
    };
    let mut lines = Vec::new();
    for (path, text) in anomaly_documents(&scope).map_err(|error| error.to_string())? {
        for diagnostic in
            check_document(&path, &text, context).map_err(|error| error.to_string())?
        {
            lines.push(format!(
                "{}:{}: error {} ({}) {}",
                path.display(),
                diagnostic.line,
                diagnostic.code.as_str(),
                diagnostic.reason,
                diagnostic.message,
            ));
        }
    }
    let outcome = if lines.is_empty() {
        Outcome::Ok
    } else {
        Outcome::Partial
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
    let entries = references_to(&scope_of(arguments), id).map_err(|error| error.to_string())?;
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

/// The module roots searched by default: `QUOIN_MODULE_PATHS`, then every
/// directory under the IX home's `filament/modules`.
fn default_roots() -> Vec<PathBuf> {
    let mut roots = std::env::var("QUOIN_MODULE_PATHS")
        .ok()
        .map_or_else(Vec::new, |paths| {
            paths
                .split(':')
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
                .collect()
        });
    let home = std::env::var_os("IX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".ix")));
    if let Some(home) = home
        && let Ok(entries) = std::fs::read_dir(home.join("filament/modules"))
    {
        roots.extend(entries.flatten().map(|entry| entry.path()));
    }
    roots
}

/// Whether the module at `root` declares the `hazard` object type.
fn declares_hazard(root: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(root.join("manifest.yaml")) else {
        return false;
    };
    let Ok(manifest) = quoin_yaml::from_str(&raw) else {
        return false;
    };
    manifest
        .get("object_types")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|types| {
            types.iter().any(|entry| {
                entry.get("name").and_then(serde_json::Value::as_str) == Some("hazard")
            })
        })
}
