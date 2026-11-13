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
mod anomaly;
mod assurance;
mod catalog;
mod change_assurance;
mod completeness;
mod config;
mod core_runtime;
mod discharge;
mod evidence;
mod flow;
mod help;
mod intake_order;
mod invocation;
mod matrix;
mod measurement;
mod module;
mod plugin;
mod report;
mod semantic;
mod soup;
mod update;
mod validate;
mod write;

use ix_trace_rs::implements;
use std::ffi::OsString;
use std::io::Write as _;

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};
use ix_cli_kit::exit::Outcome;
use quoin_core::capabilities::Capabilities;
use quoin_core::ops::graph::{change_impact, fan_out};
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_graph_analysis::OsGraphInputReader;

const EXIT_INVALID: u8 = Outcome::Invalid.code();
const EXIT_INTERNAL: u8 = Outcome::Internal.code();
const EXIT_UNKNOWN_COMMAND: u8 = Outcome::Refused.code();
const EXIT_COMMAND_FAILURE: u8 = 1;
/// FR-068 reserves exit 1 on the change-assurance surface for a receipt that
/// verified `invalid` or `incomplete`. A document the surface refuses — a
/// usage, parse, or integrity error — exits 2, so a consumer can tell a
/// refused document from a verified-and-rejected one.
const EXIT_DOCUMENT_REFUSED: u8 = Outcome::Refused.code();
const GRAPH_DESCRIPTION: &str = "Analyze an existing, accepted Quire assurance export together with retained\nQuoin evidence and an existing FR-032 audit. These commands run no producer,\nsuite, Quire, Git, or network operation and write nothing.\n\nSubcommands:\n  quoin graph fan-out\n  quoin graph change-impact";

#[derive(Debug)]
struct ShellError {
    message: String,
    exit: u8,
    stream: ShellOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShellOutput {
    Stdout,
    Stderr,
}

impl ShellError {
    /// Retained command implementations surface their own validation and
    /// operational errors as oclif command failures, distinct from parser
    /// rejections produced before a command is selected.
    fn command(message: impl Into<String>) -> Self {
        Self {
            message: format!("    Error: {}", message.into()),
            exit: EXIT_COMMAND_FAILURE,
            stream: ShellOutput::Stderr,
        }
    }

    /// A change-assurance document the surface refused (FR-068).
    ///
    /// Identical to [`Self::command`] apart from the status: the message text
    /// is the refusal the engine stated, and only the exit number carries the
    /// distinction between a refused document and a rejected receipt.
    #[implements("FR-068")]
    fn document_refused(message: impl Into<String>) -> Self {
        Self {
            message: format!("    Error: {}", message.into()),
            exit: EXIT_DOCUMENT_REFUSED,
            stream: ShellOutput::Stderr,
        }
    }
}

fn main() -> std::process::ExitCode {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if matches!(arguments.as_slice(), [_, command] if command == "workflow-invariants") {
        return std::process::ExitCode::from(flow::provider::serve(
            std::io::stdin().lock(),
            std::io::stdout().lock(),
            std::io::stderr().lock(),
        ));
    }
    if is_version_request(&arguments) {
        return emit_version();
    }
    if is_root_help_request(&arguments) {
        // Oclif terminates the root help page with its final blank line.  The
        // root renderer owns that page rather than Clap, so preserve that byte
        // contract here as well (quoin#520).
        let _ = writeln!(std::io::stdout(), "{}\n", root_usage());
        return std::process::ExitCode::SUCCESS;
    }
    if let Some(rendered) = help::retained_help(&arguments, build_version()) {
        let _ = write!(std::io::stdout(), "{rendered}");
        return std::process::ExitCode::SUCCESS;
    }
    match run(arguments) {
        Ok(response) => emit(&response),
        Err(error) => {
            match error.stream {
                ShellOutput::Stdout => {
                    let _ = writeln!(std::io::stdout(), "{}", error.message);
                }
                ShellOutput::Stderr => {
                    let _ = writeln!(std::io::stderr(), "{}", error.message);
                }
            }
            std::process::ExitCode::from(error.exit)
        }
    }
}

fn is_root_help_request(arguments: &[OsString]) -> bool {
    matches!(arguments, [_, flag] if flag == "--help" || flag == "-h")
}

fn run(args: impl IntoIterator<Item = OsString>) -> Result<Response, ShellError> {
    let args = args.into_iter().collect::<Vec<_>>();
    let change_assurance = routed_family(&args).as_deref() == Some("change-assurance");
    let matches = command().try_get_matches_from(&args).map_err(|error| {
        let rendered_error = error.to_string();
        let is_help = matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        );
        let invalid_subcommand = error.kind() == ErrorKind::InvalidSubcommand;
        let missing_required = error.kind() == ErrorKind::MissingRequiredArgument;
        ShellError {
            message: if invalid_subcommand {
                help::unknown_command(&unknown_command_path(&args))
            } else if missing_required {
                help::missing_required_flags(&rendered_error).unwrap_or(rendered_error)
            } else {
                rendered_error
            },
            exit: if is_help {
                0
            } else if invalid_subcommand || missing_required {
                EXIT_UNKNOWN_COMMAND
            } else if change_assurance {
                // A malformed flag is the "usage error" arm of FR-068's
                // grammar, and it is refused here rather than at dispatch.
                // Without this the surface answered 2 for everything the
                // engine refused and 3 for everything the parser did.
                EXIT_DOCUMENT_REFUSED
            } else {
                EXIT_INVALID
            },
            stream: if is_help {
                ShellOutput::Stdout
            } else {
                ShellOutput::Stderr
            },
        }
    })?;
    let invocation = invocation::Invocation::from_matches(&matches);
    let failure = if change_assurance {
        ShellError::document_refused
    } else {
        ShellError::command
    };
    invocation::with_current(invocation, || {
        if let Some((name @ ("review" | "to-plan"), arguments)) = matches.subcommand() {
            flow::run(name, arguments)
        } else {
            dispatch(&matches).map_err(failure)
        }
    })
}

/// The command family an invocation names, answered even when the invocation
/// does not parse.
///
/// FR-068's 0/1/2 grammar is a property of the change-assurance surface, so
/// which family was named has to be known before the parser's own refusal is
/// given a status. Clap's error-tolerant pass answers that without a second,
/// hand-rolled argv grammar: a scan for the first non-flag token disagrees
/// with the real parser about global flags and their values, and would read
/// `--config-root change-assurance` as the change-assurance surface.
#[implements("FR-068")]
fn routed_family(args: &[OsString]) -> Option<String> {
    command()
        .ignore_errors(true)
        .try_get_matches_from(args)
        .ok()?
        .subcommand_name()
        .map(str::to_owned)
}

/// Preserve oclif's colon-separated command identity for an unresolved path.
/// Options belong to invocation syntax, not the missing command identifier.
fn unknown_command_path(arguments: &[OsString]) -> String {
    let path = arguments
        .iter()
        .skip(1)
        .take_while(|argument| !argument.to_string_lossy().starts_with('-'))
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if path.is_empty() {
        "<non-UTF-8 command>".to_owned()
    } else {
        path.join(":")
    }
}

fn root_usage() -> String {
    help::root_usage(build_version())
}

fn dispatch(matches: &ArgMatches) -> Result<Response, String> {
    if let Some(("catalog", catalog)) = matches.subcommand() {
        return catalog::run(catalog);
    }
    if let Some(("advise", advise)) = matches.subcommand() {
        return advise::run(advise);
    }
    if let Some(("anomaly", anomaly)) = matches.subcommand() {
        return anomaly::run(anomaly);
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
    if let Some(("soup", soup)) = matches.subcommand() {
        return soup::run(soup);
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
    if let Some(("matrix", arguments)) = matches.subcommand() {
        return matrix::run(arguments);
    }
    if let Some(("module", module)) = matches.subcommand() {
        return module::run(module);
    }
    if let Some(("measurement", measurement)) = matches.subcommand() {
        return measurement::run(measurement);
    }
    if let Some(("plugin", plugin)) = matches.subcommand() {
        return plugin::run(plugin);
    }
    if let Some(("report", report)) = matches.subcommand() {
        return report::run(report);
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
        .ok_or_else(|| "an unknown command reached dispatch".to_owned())?;
    let Some((view, arguments)) = graph.subcommand() else {
        return Ok(Response::ok(serde_json::json!({
            "rendered": GRAPH_DESCRIPTION
        })));
    };
    let request = request(arguments, view)?;
    let reader = OsGraphInputReader;
    let capabilities = Capabilities::with_graph(&reader);
    let response = match view {
        "fan-out" => fan_out(&request, &capabilities),
        "change-impact" => change_impact(&request, &capabilities),
        _ => Err(quoin_core::error::CoreError::new(
            quoin_core::error::CoreErrorCode::BadUsage,
            "an unknown graph view reached command dispatch",
        )),
    };
    Ok(trim_graph_rendering(response.unwrap_or_else(|error| {
        Response {
            payload: serde_json::Value::Null,
            diagnostics: vec![Diagnostic::from(&error)],
            outcome: error.outcome(),
        }
    })))
}

/// Match the retained graph command's `askGraph(...).trimEnd()` boundary.
///
/// The graph engine owns a complete rendered document and terminates that
/// document with a newline. Oclif's `this.log` then supplies the command-line
/// newline after trimming the engine's terminator. Without this adapter the
/// native shell wrote both terminators, leaving every human and JSON graph
/// report one blank line longer than the still-retained command (quoin#424).
fn trim_graph_rendering(mut response: Response) -> Response {
    if let Some(payload) = response.payload.as_object_mut()
        && let Some(rendered) = payload
            .get("rendered")
            .and_then(serde_json::Value::as_str)
            .map(str::trim_end)
            .map(str::to_owned)
    {
        payload.insert("rendered".to_owned(), serde_json::Value::String(rendered));
    }
    response
}

fn command() -> Command {
    Command::new("quoin")
        .about("Quoin assurance tooling")
        // Oclif accepts `--help` but does not add a synthetic `help` command
        // to Quoin's published command inventory.
        .disable_help_subcommand(true)
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
        .subcommand(anomaly::command())
        .subcommand(assurance::command())
        .subcommand(config::command())
        .subcommand(completeness::command())
        .subcommand(change_assurance::command())
        .subcommand(evidence::command())
        .subcommand(discharge::command())
        .subcommand(module::command())
        .subcommand(measurement::command())
        .subcommand(plugin::command())
        .subcommand(report::command())
        .subcommand(semantic::command())
        .subcommand(soup::command())
        .subcommand(validate::command())
        .subcommand(write::command())
        .subcommand(update::command())
        .subcommand(
            Command::new("workflow-invariants")
                .hide(true)
                .about("Evaluate Quoin workflow invariants over bounded JSON on stdin"),
        )
        .subcommand(flow::command("review"))
        .subcommand(matrix::command())
        .subcommand(flow::command("to-plan"))
        .subcommand(
            Command::new("graph")
                .about("Read-only evidence graph views")
                .subcommand(graph_view("fan-out"))
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
                .help("FR-032 audit-envelope JSON"),
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
        // Oclif's `this.log()` is not called for an empty listing, so an empty
        // human catalog has no terminal bytes. `writeln!` would manufacture a
        // newline and turn "no modules" into a distinct command contract.
        if !rendered.is_empty() {
            let write = if stream == "stderr" {
                writeln!(std::io::stderr(), "{rendered}")
            } else {
                writeln!(std::io::stdout(), "{rendered}")
            };
            if write.is_err() {
                return std::process::ExitCode::from(EXIT_INTERNAL);
            }
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
mod tests;
