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
mod core_runtime;
mod discharge;
mod evidence;
mod flow;
mod help;
mod invocation;
mod measurement;
mod module;
mod plugin;
mod report;
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
const EXIT_COMMAND_FAILURE: u8 = 1;
/// FR-068 reserves exit 1 on the change-assurance surface for a receipt that
/// verified `invalid` or `incomplete`. A document the surface refuses — a
/// usage, parse, or integrity error — exits 2, so a consumer can tell a
/// refused document from a verified-and-rejected one.
const EXIT_DOCUMENT_REFUSED: u8 = Outcome::Refused.code();
const GRAPH_DESCRIPTION: &str = "Analyze an existing, accepted Quire assurance export together with retained\nQuoin evidence and an existing FR-032 audit. These commands run no producer,\nsuite, Quire, Git, or network operation and write nothing.\n\nSubcommands:\n  quoin graph fan-out\n  quoin graph change-impact\n  quoin graph churn";

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
    invocation::with_current(invocation, || dispatch(&matches)).map_err(failure)
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
        "churn" => churn(&request, &capabilities),
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
        .subcommand(validate::command())
        .subcommand(write::command())
        .subcommand(update::command())
        .subcommand(flow::command("review"))
        .subcommand(flow::command("matrix"))
        .subcommand(flow::command("to-plan"))
        .subcommand(
            Command::new("graph")
                .about("Read-only evidence graph views")
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
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::*;

    /// Every native-owned route retained from `src/commands/`.
    ///
    /// This is deliberately a route census rather than a list of only root
    /// topics.  The Stage 8 cutover needs a reviewable answer to "which
    /// command paths does the native parser own?" before individual argument,
    /// rendering, and exit-status fixtures can make that answer executable.
    const NATIVE_OWNED_ROUTES: &[&str] = &[
        "quoin advise",
        "quoin assurance",
        "quoin catalog",
        "quoin catalog list",
        "quoin catalog methods",
        "quoin catalog show",
        "quoin catalog validate",
        "quoin change-assurance",
        "quoin change-assurance intake",
        "quoin change-assurance receipt",
        "quoin change-assurance recover",
        "quoin change-assurance schema",
        "quoin change-assurance seal-attestation",
        "quoin change-assurance seal-record",
        "quoin change-assurance verify-receipt",
        "quoin completeness",
        "quoin config",
        "quoin config doctor",
        "quoin config edit",
        "quoin config get",
        "quoin config set",
        "quoin discharge",
        "quoin evidence",
        "quoin evidence affirm",
        "quoin evidence audit",
        "quoin evidence baseline",
        "quoin evidence gc",
        "quoin evidence inspect-mocks",
        "quoin evidence record",
        "quoin evidence record-experiment",
        "quoin evidence record-operational",
        "quoin evidence trust",
        "quoin graph",
        "quoin graph change-impact",
        "quoin graph churn",
        "quoin graph fan-out",
        "quoin matrix",
        "quoin measurement",
        "quoin measurement intervention",
        "quoin measurement operational-release",
        "quoin measurement record",
        "quoin module",
        "quoin module ensure-defaults",
        "quoin module install",
        "quoin module list",
        "quoin module remove",
        "quoin plugin",
        "quoin plugin ensure-defaults",
        "quoin plugin install",
        "quoin plugin list",
        "quoin plugin remove",
        "quoin report",
        "quoin review",
        "quoin semantic",
        "quoin semantic sweep",
        "quoin to-plan",
        "quoin update",
        "quoin validate",
        "quoin write",
    ];

    fn command_routes(command: &Command) -> Vec<String> {
        fn visit(command: &Command, prefix: &str, routes: &mut Vec<String>) {
            for child in command.get_subcommands() {
                let route = format!("{prefix} {}", child.get_name());
                routes.push(route.clone());
                visit(child, &route, routes);
            }
        }

        let mut routes = Vec::new();
        visit(command, "quoin", &mut routes);
        routes.sort_unstable();
        routes
    }

    /// Trace: FR-062, FR-102
    #[test]
    fn tc_373_graph_without_a_view_keeps_the_retained_description() {
        let response = run([OsString::from("quoin"), OsString::from("graph")])
            .expect("the retained parent command renders");
        assert_eq!(
            response
                .payload
                .get("rendered")
                .and_then(serde_json::Value::as_str),
            Some(GRAPH_DESCRIPTION)
        );
    }

    /// Trace: FR-062, FR-102, TC-1650
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

    /// Trace: FR-062, FR-102, TC-1650
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

    #[test]
    fn tc_1650_root_help_uses_the_native_command_inventory() {
        assert!(is_root_help_request(&[
            OsString::from("quoin"),
            OsString::from("--help")
        ]));
        assert!(!is_root_help_request(&[
            OsString::from("quoin"),
            OsString::from("catalog"),
            OsString::from("--help"),
        ]));
        help::assert_catalogue_matches(&command());
        assert!(root_usage().contains("USAGE\n  $ quoin [COMMAND]"));
    }

    /// Trace: FR-003, FR-102, TC-1650
    #[test]
    fn tc_1650_help_is_a_successful_stdout_response_not_a_parse_failure() {
        for arguments in [
            vec![OsString::from("quoin"), OsString::from("--help")],
            vec![
                OsString::from("quoin"),
                OsString::from("config"),
                OsString::from("get"),
                OsString::from("--help"),
            ],
        ] {
            let help = run(arguments).expect_err("clap returns a display response");
            assert_eq!(help.exit, 0);
            assert_eq!(help.stream, ShellOutput::Stdout);
            assert!(help.message.contains("Usage:"));
        }
    }

    /// Trace: FR-003, FR-102, TC-1650
    #[test]
    fn tc_1650_every_native_command_path_accepts_help_before_required_inputs() {
        for route in NATIVE_OWNED_ROUTES {
            let arguments = route
                .split_whitespace()
                .map(OsString::from)
                .chain(std::iter::once(OsString::from("--help")));
            let help = run(arguments)
                .expect_err("every retained route renders help before validating required inputs");
            assert_eq!(help.exit, 0, "{route}");
            assert_eq!(help.stream, ShellOutput::Stdout, "{route}");
            assert!(!help.message.trim().is_empty(), "{route}");
        }
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

    /// Trace: FR-102, TC-1650
    #[test]
    fn tc_1650_native_shell_exposes_every_retained_top_level_command_family() {
        let root = command();
        let mut visible = root
            .get_subcommands()
            .filter(|subcommand| !subcommand.is_hide_set())
            .map(Command::get_name)
            .collect::<Vec<_>>();
        visible.sort_unstable();
        assert_eq!(
            visible,
            [
                "advise",
                "assurance",
                "catalog",
                "change-assurance",
                "completeness",
                "config",
                "discharge",
                "evidence",
                "graph",
                "matrix",
                "measurement",
                "module",
                "report",
                "review",
                "semantic",
                "to-plan",
                "update",
                "validate",
                "write",
            ]
        );
    }

    /// Trace: FR-062, FR-102, TC-1650
    #[test]
    fn tc_1650_native_shell_route_census_matches_the_retained_owned_surface() {
        let expected = NATIVE_OWNED_ROUTES
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(command_routes(&command()), expected);
    }

    /// Trace: FR-005, FR-062
    #[test]
    fn tc_373_unknown_commands_name_the_input_and_keep_the_refusal_status() {
        let error = run([OsString::from("quoin"), OsString::from("bogus")])
            .expect_err("unknown command is refused");
        assert_eq!(error.exit, EXIT_UNKNOWN_COMMAND);
        assert_eq!(
            error.message,
            " ›   Error: command bogus not found\n ›\n ›   Usage: quoin <command> [options]\n ›\n ›   Commands: advise, assurance, catalog, change-assurance, completeness, \n ›   config, discharge, evidence, graph, matrix, measurement, module, report, \n ›   review, semantic, to-plan, update, validate, write\n ›\n ›   Run `quoin <command> --help` for details."
        );
    }

    /// Trace: FR-005, FR-102, TC-1650
    #[test]
    fn tc_1650_command_failures_keep_the_retained_error_envelope() {
        let error = ShellError::command("write requires <repo_dir>");
        assert_eq!(error.exit, EXIT_COMMAND_FAILURE);
        assert_eq!(error.stream, ShellOutput::Stderr);
        assert_eq!(error.message, "    Error: write requires <repo_dir>");
    }

    /// Trace: FR-062, FR-102, TC-1650
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
