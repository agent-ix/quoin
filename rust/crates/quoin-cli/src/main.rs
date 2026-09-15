// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The first Rust-owned `quoin` command slice (FR-062, FR-102).
//!
//! This binary owns syntax and rendering only. The graph analysis remains in
//! `quoin-graph-analysis`; `quoin-core` owns its request validation, response
//! envelope, diagnostics, and exit taxonomy. Keeping that split means the
//! shell cannot acquire a second interpretation of graph inputs while the
//! oclif shell remains published during the staged cutover.

mod advise;
mod assurance;
mod catalog;
mod change_assurance;
mod completeness;
mod config;
mod core_bridge;
mod discharge;
mod evidence;
mod module;
mod plugin;
mod semantic;
mod validate;

use std::ffi::OsString;
use std::io::Write as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::capabilities::Capabilities;
use quoin_core::ops::graph::{change_impact, churn, fan_out};
use quoin_core::protocol::{Diagnostic, Outcome, Response, canonical_json};
use quoin_graph_analysis::OsGraphInputReader;

const EXIT_INVALID: u8 = Outcome::Invalid.code();
const EXIT_INTERNAL: u8 = Outcome::Internal.code();

fn main() -> std::process::ExitCode {
    match run(std::env::args_os()) {
        Ok(response) => emit(&response),
        Err(message) => {
            let _ = writeln!(std::io::stderr(), "{message}");
            std::process::ExitCode::from(EXIT_INVALID)
        }
    }
}

fn run(args: impl IntoIterator<Item = OsString>) -> Result<Response, String> {
    let matches = command()
        .try_get_matches_from(args)
        .map_err(|error| error.to_string())?;
    if let Some(("catalog", catalog)) = matches.subcommand() {
        return catalog::run(catalog);
    }
    if let Some(("advise", advise)) = matches.subcommand() {
        return advise::run(advise);
    }
    if let Some(("assurance", assurance)) = matches.subcommand() {
        return assurance::run(assurance);
    }
    if let Some(("config", config)) = matches.subcommand() {
        return config::run(config);
    }
    if let Some(("completeness", completeness)) = matches.subcommand() {
        return completeness::run(completeness);
    }
    if let Some(("semantic", semantic)) = matches.subcommand() {
        return semantic::run(semantic);
    }
    if let Some(("validate", validate)) = matches.subcommand() {
        return validate::run(validate);
    }
    if let Some(("module", module)) = matches.subcommand() {
        return module::run(module);
    }
    if let Some(("plugin", plugin)) = matches.subcommand() {
        return plugin::run(plugin);
    }
    if let Some(("change-assurance", change_assurance)) = matches.subcommand() {
        return change_assurance::run(change_assurance);
    }
    if let Some(("evidence", evidence)) = matches.subcommand() {
        return evidence::run(evidence);
    }
    if let Some(("discharge", discharge)) = matches.subcommand() {
        return discharge::run(discharge);
    }
    let graph = matches
        .subcommand_matches("graph")
        .ok_or_else(|| "a graph subcommand is required".to_owned())?;
    let (view, arguments) = graph
        .subcommand()
        .ok_or_else(|| "a graph view is required".to_owned())?;
    let request = request(arguments, view)?;
    let reader = OsGraphInputReader;
    let capabilities = Capabilities::with_graph(&reader);
    let response = match view {
        "fan-out" => fan_out(&request, &capabilities),
        "churn" => churn(&request, &capabilities),
        "change-impact" => change_impact(&request, &capabilities),
        _ => Err(quoin_core::error::CoreError::new(
            quoin_core::error::CoreErrorCode::BadUsage,
            "an unknown graph view reached command dispatch",
        )),
    };
    Ok(response.unwrap_or_else(|error| Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic::from(&error)],
        outcome: error.outcome(),
    }))
}

fn command() -> Command {
    Command::new("quoin")
        .about("Quoin assurance tooling")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(catalog::command())
        .subcommand(advise::command())
        .subcommand(assurance::command())
        .subcommand(config::command())
        .subcommand(completeness::command())
        .subcommand(change_assurance::command())
        .subcommand(evidence::command())
        .subcommand(discharge::command())
        .subcommand(module::command())
        .subcommand(plugin::command())
        .subcommand(semantic::command())
        .subcommand(validate::command())
        .subcommand(
            Command::new("graph")
                .about("Read-only evidence graph views")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(graph_view("fan-out"))
                .subcommand(graph_view("churn"))
                .subcommand(
                    graph_view("change-impact")
                        .arg(
                            Arg::new("requirement")
                                .long("requirement")
                                .value_name("ID")
                                .help("Changed requirement id; repeat to add seeds")
                                .required(true)
                                .action(ArgAction::Append),
                        )
                        .arg(
                            Arg::new("relation")
                                .long("relation")
                                .value_name("KIND")
                                .help("Relationship kind; repeat to replace defaults")
                                .action(ArgAction::Append),
                        ),
                ),
        )
}

fn graph_view(name: &'static str) -> Command {
    Command::new(name)
        .arg(
            Arg::new("repo")
                .long("repo")
                .value_name("PATH")
                .default_value(".")
                .help("Repository root for retained Quoin state"),
        )
        .arg(
            Arg::new("export")
                .long("export")
                .value_name("JSON")
                .required(true)
                .help("Existing Quire assurance-v1 JSON artifact"),
        )
        .arg(
            Arg::new("premises")
                .long("premises")
                .value_name("JSON")
                .required(true)
                .help("Accepted assurance format/module/schema premises JSON"),
        )
        .arg(
            Arg::new("audit")
                .long("audit")
                .value_name("JSON")
                .required(true)
                .help("Source-bound FR-032 audit-envelope JSON"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Emit canonical JSON")
                .action(ArgAction::SetTrue),
        )
}

fn request(arguments: &ArgMatches, view: &str) -> Result<serde_json::Value, String> {
    let required = |name| {
        arguments
            .get_one::<String>(name)
            .cloned()
            .ok_or_else(|| format!("--{name} is required"))
    };
    let mut fields = serde_json::Map::from_iter([
        ("repo".to_owned(), serde_json::json!(required("repo")?)),
        (
            "export_path".to_owned(),
            serde_json::json!(required("export")?),
        ),
        (
            "premises_path".to_owned(),
            serde_json::json!(required("premises")?),
        ),
        (
            "audit_path".to_owned(),
            serde_json::json!(required("audit")?),
        ),
        (
            "json".to_owned(),
            serde_json::json!(arguments.get_flag("json")),
        ),
    ]);
    if view == "change-impact" {
        let requirements = arguments
            .get_many::<String>("requirement")
            .ok_or_else(|| "--requirement is required".to_owned())?
            .cloned()
            .collect::<Vec<_>>();
        fields.insert("requirements".to_owned(), serde_json::json!(requirements));
        if let Some(relations) = arguments.get_many::<String>("relation") {
            fields.insert(
                "relations".to_owned(),
                serde_json::json!(relations.cloned().collect::<Vec<_>>()),
            );
        }
    }
    Ok(serde_json::Value::Object(fields))
}

fn emit(response: &Response) -> std::process::ExitCode {
    if let Some(warning) = response
        .payload
        .get("warning")
        .and_then(serde_json::Value::as_str)
        && writeln!(std::io::stderr(), "{warning}").is_err()
    {
        return std::process::ExitCode::from(EXIT_INTERNAL);
    }
    if response.outcome.carries_payload() {
        let Some(rendered) = response
            .payload
            .get("rendered")
            .and_then(serde_json::Value::as_str)
        else {
            let _ = writeln!(
                std::io::stderr(),
                "quoin graph response carried no rendered report"
            );
            return std::process::ExitCode::from(EXIT_INTERNAL);
        };
        let stream = response
            .payload
            .get("renderedStream")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("stdout");
        let write = if stream == "stderr" {
            writeln!(std::io::stderr(), "{rendered}")
        } else {
            writeln!(std::io::stdout(), "{rendered}")
        };
        if write.is_err() {
            return std::process::ExitCode::from(EXIT_INTERNAL);
        }
    }
    if !response.diagnostics.is_empty() {
        match canonical_json(&response.diagnostics) {
            Ok(diagnostics) => {
                let _ = writeln!(std::io::stderr(), "{diagnostics}");
            }
            Err(_) => return std::process::ExitCode::from(EXIT_INTERNAL),
        }
    }
    std::process::ExitCode::from(response.outcome.code())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::*;

    /// Tracing: FR-062, FR-102, TC-1650
    #[test]
    fn tc_1650_graph_grammar_preserves_the_required_and_repeatable_flags() {
        let matches = command()
            .try_get_matches_from([
                "quoin",
                "graph",
                "change-impact",
                "--export",
                "export.json",
                "--premises",
                "premises.json",
                "--audit",
                "audit.json",
                "--requirement",
                "FR-001",
                "--requirement",
                "FR-002",
                "--relation",
                "requires",
                "--json",
            ])
            .expect("the retained graph grammar parses");
        let (_, graph) = matches.subcommand().expect("graph command");
        let (view, arguments) = graph.subcommand().expect("graph view");
        assert_eq!(view, "change-impact");
        assert_eq!(
            request(arguments, view).expect("request"),
            serde_json::json!({
                "repo": ".",
                "export_path": "export.json",
                "premises_path": "premises.json",
                "audit_path": "audit.json",
                "json": true,
                "requirements": ["FR-001", "FR-002"],
                "relations": ["requires"],
            }),
        );
    }

    /// Tracing: FR-062, FR-102, TC-1650
    #[test]
    fn tc_1650_missing_a_required_graph_input_is_refused_by_the_shell() {
        let error = command().try_get_matches_from([
            "quoin",
            "graph",
            "fan-out",
            "--export",
            "export.json",
            "--premises",
            "premises.json",
        ]);
        assert!(error.is_err());
    }

    /// Tracing: FR-062, FR-102, TC-1650
    #[test]
    fn tc_1650_graph_refusals_keep_the_core_exit_taxonomy() {
        let response = run([
            OsString::from("quoin"),
            OsString::from("graph"),
            OsString::from("fan-out"),
            OsString::from("--export"),
            OsString::from("missing-export.json"),
            OsString::from("--premises"),
            OsString::from("missing-premises.json"),
            OsString::from("--audit"),
            OsString::from("missing-audit.json"),
        ])
        .expect("the parsed shell request reaches the graph operation");
        assert_eq!(response.outcome, Outcome::Refused);
        assert_eq!(response.outcome.code(), 2);
        assert_eq!(response.diagnostics.len(), 1);
    }
}
