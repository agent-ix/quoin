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
mod flow;
mod invocation;
mod module;
mod plugin;
mod semantic;
mod update;
mod validate;
mod write;

use std::ffi::OsString;
use std::io::Write as _;

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};
use quoin_core::capabilities::Capabilities;
use quoin_core::ops::graph::{change_impact, churn, fan_out};
use quoin_core::protocol::{Diagnostic, Outcome, Response, canonical_json};
use quoin_graph_analysis::OsGraphInputReader;

const EXIT_INVALID: u8 = Outcome::Invalid.code();
const EXIT_INTERNAL: u8 = Outcome::Internal.code();
const EXIT_UNKNOWN_COMMAND: u8 = Outcome::Refused.code();

#[derive(Debug)]
struct ShellError {
    message: String,
    exit: u8,
}

impl ShellError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit: EXIT_INVALID,
        }
    }
}

fn main() -> std::process::ExitCode {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if is_version_request(&arguments) {
        return emit_version();
    }
    match run(arguments) {
        Ok(response) => emit(&response),
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{}", error.message);
            std::process::ExitCode::from(error.exit)
        }
    }
}

fn run(args: impl IntoIterator<Item = OsString>) -> Result<Response, ShellError> {
    let matches = command()
        .try_get_matches_from(args)
        .map_err(|error| ShellError {
            message: error.to_string(),
            exit: if error.kind() == ErrorKind::InvalidSubcommand {
                EXIT_UNKNOWN_COMMAND
            } else {
                EXIT_INVALID
            },
        })?;
    let invocation = invocation::Invocation::from_matches(&matches);
    invocation::with_current(invocation, || dispatch(&matches)).map_err(ShellError::invalid)
}

fn dispatch(matches: &ArgMatches) -> Result<Response, String> {
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
    if let Some(("write", write)) = matches.subcommand() {
        return write::run(write);
    }
    if let Some(("update", update)) = matches.subcommand() {
        return update::run(update);
    }
    if let Some((name, flow_arguments)) = matches.subcommand()
        && matches!(name, "review" | "matrix" | "to-plan")
    {
        return flow::run(name, flow_arguments);
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
        .version(build_version())
        .arg(
            Arg::new("config_root")
                .long("config-root")
                .value_name("DIR")
                .global(true)
                .help("Override the IX configuration and module home"),
        )
        .arg(
            Arg::new("no_project_config")
                .long("no-project-config")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Ignore the project-local .ix configuration layer"),
        )
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
        .subcommand(write::command())
        .subcommand(update::command())
        .subcommand(flow::command("review"))
        .subcommand(flow::command("matrix"))
        .subcommand(flow::command("to-plan"))
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

fn build_version() -> &'static str {
    env!("QUOIN_VERSION")
}

fn is_version_request(arguments: &[OsString]) -> bool {
    arguments
        .get(1)
        .is_some_and(|argument| matches!(argument.to_str(), Some("--version" | "-v" | "version")))
}

fn emit_version() -> std::process::ExitCode {
    if writeln!(std::io::stdout(), "{}", build_version()).is_ok() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(EXIT_INTERNAL)
    }
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

    /// Trace: FR-016, FR-062
    #[test]
    fn tc_373_version_requests_keep_the_bare_native_version_surface() {
        for argument in ["--version", "-v", "version"] {
            assert!(is_version_request(&[
                OsString::from("quoin"),
                OsString::from(argument)
            ]));
        }
        assert!(!is_version_request(&[
            OsString::from("quoin"),
            OsString::from("validate")
        ]));
        assert!(
            !build_version().is_empty(),
            "the native version is always present"
        );
    }

    /// Trace: FR-016, FR-062
    #[test]
    fn tc_373_global_config_flags_are_accepted_after_a_command_path() {
        let matches = command()
            .try_get_matches_from([
                "quoin",
                "catalog",
                "list",
                "--config-root",
                "/tmp/ix",
                "--no-project-config",
            ])
            .expect("retained global flags parse after subcommands");
        let invocation = invocation::Invocation::from_matches(&matches);
        assert_eq!(
            invocation.config_root().map(std::path::PathBuf::as_path),
            Some(std::path::Path::new("/tmp/ix"))
        );
        assert!(invocation.no_project_config());
    }

    /// Trace: FR-005, FR-062
    #[test]
    fn tc_373_unknown_commands_name_the_input_and_keep_the_refusal_status() {
        let error = run([OsString::from("quoin"), OsString::from("bogus")])
            .expect_err("unknown command is refused");
        assert_eq!(error.exit, EXIT_UNKNOWN_COMMAND);
        assert!(error.message.contains("bogus"));
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
