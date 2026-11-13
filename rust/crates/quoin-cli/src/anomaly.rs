// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin anomaly`: the anomaly lifecycle check over git history and the
//! reverse references from a requirement or hazard (FR-136).
//!
//! `quoin anomaly check` reports what no module mapping can express; it never
//! repeats `quire validate`'s dangling-reference or kind diagnostics.
//! `quoin anomaly report` lists the anomalies a release still carries, with the
//! `--fail-on` gate over them (FR-162 to FR-164).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_anomaly::{
    Context, GateMode, Severity, Verification, anomaly_documents, build_report, check_document,
    evaluates_criteria, references_to,
};
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
        .subcommand(
            Command::new("report")
                .about("Report the anomalies a release still carries, grouped by hazard and requirement")
                .arg(
                    Arg::new("scope")
                        .value_name("SCOPE")
                        .default_value(".")
                        .help("Directory searched for Anomaly documents"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(ArgAction::SetTrue)
                        .help("Emit one JSON object instead of Markdown"),
                )
                .arg(
                    Arg::new("fail-on")
                        .long("fail-on")
                        .value_name("MODE")
                        .action(ArgAction::Append)
                        .value_parser(PossibleValuesParser::new(GateMode::ALL.map(GateMode::as_str)))
                        .help("Exit non-zero when a listed anomaly is unassessed, or a carried impacting anomaly (not closed, or closed as wont-fix) has no release rationale; repeat to evaluate both"),
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
        Some(("report", report)) => run_report(report),
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
    let documents = anomaly_documents(&scope).map_err(|error| error.to_string())?;
    let mut notes = Vec::new();
    let criterion_tests = if documents.iter().any(|(_, text)| evaluates_criteria(text)) {
        criterion_tests(&scope, roots.as_deref()).unwrap_or_else(|reason| {
            // Without the obligations a `Criterion` row still matches its own
            // trace id; only the test cases its Verification cell names are
            // lost, and the output says so.
            notes.push(Diagnostic {
                code: "anomaly.obligations_unavailable".to_owned(),
                message: format!("Criterion rows matched on their own id only: {reason}"),
                context: BTreeMap::new(),
            });
            BTreeMap::new()
        })
    } else {
        BTreeMap::new()
    };
    let verification =
        Verification::read(&scope, criterion_tests).map_err(|error| error.to_string())?;
    let mut lines = Vec::new();
    let mut errors = false;
    for (path, text) in documents {
        let mut checked =
            check_document(&path, &text, context).map_err(|error| error.to_string())?;
        checked.diagnostics.extend(verification.diagnostics(&text));
        if !checked.history_applicable {
            lines.push(format!(
                "{}: statusHistory not applicable (the file is not under git)",
                path.display()
            ));
        }
        for diagnostic in &checked.diagnostics {
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
        if let Some(fixing) = &checked.fixing_change {
            lines.push(format!(
                "{}: fixingChange {}",
                path.display(),
                fixing.markdown()
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
        diagnostics: notes,
        outcome,
    })
}

fn run_report(arguments: &ArgMatches) -> Result<Response, String> {
    let scope = scope_of(arguments);
    if !scope.is_dir() {
        return Ok(refuse_missing("scope", &scope));
    }
    let modes = arguments
        .get_many::<String>("fail-on")
        .into_iter()
        .flatten()
        .map(|mode| {
            GateMode::parse(mode)
                .ok_or_else(|| format!("an unknown --fail-on mode {mode} reached dispatch"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let report = build_report(&scope, &modes).map_err(|error| error.to_string())?;
    let rendered = if arguments.get_flag("json") {
        report.to_json().map_err(|error| error.to_string())?
    } else {
        report.to_markdown()
    };
    let outcome = if report.failed() {
        Outcome::Partial
    } else {
        Outcome::Ok
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
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

/// The test case ids each criterion's obligation names (`target_ids`,
/// quire-rs FR-053-AC-11), asked of the core's coverage derivation over the
/// same module roots as the rest of the check. An id that is itself an
/// obligation id is dropped, as the evidence store drops it (FR-030-AC-15).
fn criterion_tests(
    scope: &std::path::Path,
    roots: Option<&[PathBuf]>,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let response = invoke(
        "quire.coverage",
        &serde_json::json!({ "scope": scope, "modules": roots.unwrap_or_default() }),
    )?;
    if !response.outcome.carries_payload() {
        return Err("the obligations of the corpus could not be derived".to_owned());
    }
    let obligations = response
        .payload
        .get("obligations")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let ids = obligations
        .iter()
        .filter_map(|obligation| obligation.get("id")?.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    Ok(obligations
        .iter()
        .filter_map(|obligation| {
            let id = obligation.get("id")?.as_str()?;
            let tests = obligation
                .get("target_ids")?
                .as_array()?
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|test| !ids.contains(test))
                .map(str::to_owned)
                .collect();
            Some((id.to_owned(), tests))
        })
        .collect())
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
