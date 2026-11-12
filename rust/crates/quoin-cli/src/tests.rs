// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Unit tests for the binary root: routing, help and exit rendering.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
use super::*;
use ix_trace_rs::trace;

/// Every native-owned route retained from `src/commands/`.
///
/// This is deliberately a route census rather than a list of only root
/// topics.  The Stage 8 cutover needs a reviewable answer to "which
/// command paths does the native parser own?" before individual argument,
/// rendering, and exit-status fixtures can make that answer executable.
const NATIVE_OWNED_ROUTES: &[&str] = &[
    "quoin advise",
    "quoin anomaly",
    "quoin anomaly check",
    "quoin anomaly refs",
    "quoin assurance",
    "quoin catalog",
    "quoin catalog list",
    "quoin catalog methods",
    "quoin catalog show",
    "quoin catalog validate",
    "quoin change-assurance",
    "quoin change-assurance intake",
    "quoin change-assurance prepare-attestation",
    "quoin change-assurance receipt",
    "quoin change-assurance recover",
    "quoin change-assurance schema",
    "quoin change-assurance verify-receipt",
    "quoin change-assurance write-record",
    "quoin completeness",
    "quoin config",
    "quoin config doctor",
    "quoin config edit",
    "quoin config get",
    "quoin config set",
    "quoin discharge",
    "quoin evidence",
    "quoin evidence audit",
    "quoin evidence gc",
    "quoin evidence inspect-mocks",
    "quoin evidence record",
    "quoin evidence record-discharge",
    "quoin evidence record-experiment",
    "quoin evidence record-operational",
    "quoin evidence trust",
    "quoin graph",
    "quoin graph change-impact",
    "quoin graph fan-out",
    "quoin matrix",
    "quoin measurement",
    "quoin measurement attach",
    "quoin measurement campaign",
    "quoin measurement campaign run",
    "quoin measurement intervention",
    "quoin measurement operational-release",
    "quoin measurement record",
    "quoin measurement verify",
    "quoin module",
    "quoin module ensure-defaults",
    "quoin module inspect",
    "quoin module install",
    "quoin module list",
    "quoin module remove",
    "quoin plugin",
    "quoin plugin ensure-defaults",
    "quoin plugin inspect",
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
    "quoin workflow-invariants",
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

#[trace("FR-062", "FR-102")]
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

#[trace("FR-062", "FR-102", "TC-1650")]
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

#[trace("FR-062", "FR-102", "TC-1650")]
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

#[trace("FR-016", "FR-062")]
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

#[trace("FR-003", "FR-102", "TC-1650")]
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

#[trace("FR-003", "FR-102", "TC-1650")]
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

#[trace("FR-016", "FR-062")]
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

#[trace("FR-102", "TC-1650")]
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
            "anomaly",
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

#[trace("FR-062", "FR-102", "TC-1650")]
#[test]
fn tc_1650_native_shell_route_census_matches_the_retained_owned_surface() {
    let expected = NATIVE_OWNED_ROUTES
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(command_routes(&command()), expected);
}

#[trace("FR-005", "FR-062")]
#[test]
fn tc_373_unknown_commands_name_the_input_and_keep_the_refusal_status() {
    let error = run([OsString::from("quoin"), OsString::from("bogus")])
        .expect_err("unknown command is refused");
    assert_eq!(error.exit, EXIT_UNKNOWN_COMMAND);
    assert_eq!(
        error.message,
        " ›   Error: command bogus not found\n ›\n ›   Usage: quoin <command> [options]\n ›\n ›   Commands: advise, anomaly, assurance, catalog, change-assurance, completeness, \n ›   config, discharge, evidence, graph, matrix, measurement, module, report, \n ›   review, semantic, to-plan, update, validate, write\n ›\n ›   Run `quoin <command> --help` for details."
    );
}

#[trace("FR-005", "FR-102", "TC-1650")]
#[test]
fn tc_1650_command_failures_keep_the_retained_error_envelope() {
    let error = ShellError::command("write requires <repo_dir>");
    assert_eq!(error.exit, EXIT_COMMAND_FAILURE);
    assert_eq!(error.stream, ShellOutput::Stderr);
    assert_eq!(error.message, "    Error: write requires <repo_dir>");
}

#[trace("FR-062", "FR-102", "TC-1650")]
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
