// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapters for retained evidence-store commands (quoin#373, Stage 8).

use std::io::Read as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_bridge::invoke;

/// The evidence commands whose inputs need no Quire-derived obligation set.
pub(crate) fn command() -> Command {
    Command::new("evidence")
        .about("Record and maintain verification evidence")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("gc")
                .about("Drop unreferenced run records")
                .arg(repo_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue),
                )
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("trust")
                .about("Record a use-specific evidence-producer trust decision")
                .arg(Arg::new("decision").long("decision").required(true))
                .arg(repo_arg())
                .arg(json_arg()),
        )
        .subcommand(record_command(
            "record-experiment",
            "experiment-record-v1 JSON path",
        ))
        .subcommand(record_command(
            "record-operational",
            "operational-evidence-record-v1 JSON path",
        ))
}

fn record_command(name: &'static str, help: &'static str) -> Command {
    Command::new(name)
        .about("Publish one content-addressed evidence record")
        .arg(Arg::new("input").long("input").help(help).required(true))
        .arg(repo_arg())
        .arg(json_arg())
}

/// Execute a parsed evidence command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let (name, arguments) = matches
        .subcommand()
        .ok_or_else(|| "an evidence subcommand is required".to_owned())?;
    match name {
        "gc" => gc(arguments),
        "trust" => trust(arguments),
        "record-experiment" => record(arguments, "evidence.record_experiment"),
        "record-operational" => record(arguments, "evidence.record_operational"),
        _ => Err("an unknown evidence command reached dispatch".to_owned()),
    }
}

fn gc(arguments: &ArgMatches) -> Result<Response, String> {
    let dry_run = arguments.get_flag("dry-run");
    let response = invoke(
        "evidence.gc",
        &serde_json::json!({ "repo": required(arguments, "repo")?, "dry_run": dry_run }),
    )?;
    render(response, |payload| {
        let deleted = payload
            .get("deleted")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "evidence.gc did not return deleted".to_owned())?;
        if arguments.get_flag("json") {
            return serde_json::to_string(
                &serde_json::json!({ "deleted": deleted, "dryRun": dry_run }),
            )
            .map_err(|error| error.to_string());
        }
        if deleted.is_empty() {
            return Ok("nothing to collect".to_owned());
        }
        Ok(deleted
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(|path| {
                format!(
                    "{} {path}",
                    if dry_run { "would delete" } else { "deleted" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"))
    })
}

fn trust(arguments: &ArgMatches) -> Result<Response, String> {
    let decision = json_input(&required(arguments, "decision")?)?;
    let response = invoke(
        "evidence.trust_decision",
        &serde_json::json!({ "repo": required(arguments, "repo")?, "decision": decision }),
    )?;
    render(response, |payload| {
        if arguments.get_flag("json") {
            return serde_json::to_string(payload).map_err(|error| error.to_string());
        }
        let path = payload
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "evidence.trust_decision did not return path".to_owned())?;
        let assessment = payload
            .get("assessment")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| "evidence.trust_decision did not return assessment".to_owned())?;
        let id = assessment
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let use_id = assessment
            .get("useId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let status = assessment
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        Ok(format!("{id} ({use_id}): {status}\n  {path}"))
    })
}

fn record(arguments: &ArgMatches, operation: &str) -> Result<Response, String> {
    let document = json_input(&required(arguments, "input")?)?;
    let response = invoke(
        operation,
        &serde_json::json!({ "repo": required(arguments, "repo")?, "document": document }),
    )?;
    render(response, |payload| {
        if arguments.get_flag("json") {
            return serde_json::to_string(payload).map_err(|error| error.to_string());
        }
        let record = payload
            .get("record")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| format!("{operation} did not return record"))?;
        let id = record
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let path = payload
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let state = if payload
            .get("created")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            "created"
        } else {
            "exists"
        };
        Ok(format!("{id} {state} {path}"))
    })
}

fn repo_arg() -> Arg {
    Arg::new("repo").long("repo").default_value(".")
}
fn json_arg() -> Arg {
    Arg::new("json").long("json").action(ArgAction::SetTrue)
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn json_input(source: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&input(source)?).map_err(|error| format!("input is not JSON: {error}"))
}
fn input(source: &str) -> Result<Vec<u8>, String> {
    if source != "-" {
        return std::fs::read(source).map_err(|error| format!("cannot read {source}: {error}"));
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}
fn render(
    response: Response,
    text: impl FnOnce(&serde_json::Value) -> Result<String, String>,
) -> Result<Response, String> {
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    Ok(Response {
        payload: serde_json::json!({ "rendered": text(&response.payload)? }),
        diagnostics: response.diagnostics,
        outcome: response.outcome,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::command;
    /// Trace: FR-030, FR-101
    #[test]
    fn tc_373_evidence_store_command_grammar_is_retained() {
        assert!(
            command()
                .try_get_matches_from(["evidence", "gc", "--dry-run"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from(["evidence", "trust"])
                .is_err()
        );
        assert!(
            command()
                .try_get_matches_from(["evidence", "record-experiment", "--input", "record.json"])
                .is_ok()
        );
    }
}
