// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the retained `quoin discharge` command.

use std::io::Read as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_runtime::invoke;
use crate::evidence::{json_arg, repo_arg};

pub(crate) fn command() -> Command {
    Command::new("discharge")
        .about("Partition binding clauses into direct evidence, dispositions, and open work")
        .arg(Arg::new("binding").long("binding").required(true))
        .arg(
            Arg::new("facts")
                .long("facts")
                .required_unless_present("repo")
                .conflicts_with("repo"),
        )
        .arg(
            Arg::new("repo")
                .long("repo")
                .help("Read the facts from this repository's evidence store instead of --facts"),
        )
        .arg(Arg::new("as-of").long("as-of").required(true))
        .arg(
            Arg::new("inward")
                .long("inward")
                .action(ArgAction::SetTrue)
                .help("Report inward completeness: is every binding clause discharged"),
        )
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn record_command() -> Command {
    Command::new("record-discharge")
        .about("Store attested clause-discharge facts for a clause set")
        .arg(Arg::new("binding").long("binding").required(true))
        .arg(Arg::new("facts").long("facts").required(true))
        .arg(Arg::new("recorded-at").long("recorded-at").required(true))
        .arg(repo_arg())
        .arg(json_arg())
}

pub(crate) fn record(arguments: &ArgMatches) -> Result<Response, String> {
    let binding_source = required(arguments, "binding")?;
    let facts_source = required(arguments, "facts")?;
    if binding_source == "-" && facts_source == "-" {
        return Err("--binding and --facts cannot both read stdin".to_owned());
    }
    let facts = json_input(&facts_source, "discharge facts")?;
    if !facts.is_array() {
        return Err("discharge facts must be a JSON array".to_owned());
    }
    let response = invoke(
        "evidence.record_discharge",
        &serde_json::json!({
            "repo": required(arguments, "repo")?,
            "binding": json_input(&binding_source, "clause binding")?,
            "facts": facts,
            "recordedAt": required(arguments, "recorded-at")?,
        }),
    )?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let text = if arguments.get_flag("json") {
        serde_json::to_string_pretty(&response.payload).map_err(|error| error.to_string())?
    } else {
        let recorded = response
            .payload
            .get("recorded")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "evidence.record_discharge did not return recorded".to_owned())?;
        recorded
            .iter()
            .map(|fact| {
                let field = |name: &str| {
                    fact.get(name)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                };
                format!("{} {}", field("clauseId"), field("path"))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": text }),
        diagnostics: response.diagnostics,
        outcome: response.outcome,
    })
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let binding_source = required(arguments, "binding")?;
    let binding = json_input(&binding_source, "clause binding")?;
    let as_of = required(arguments, "as-of")?;
    let built = if let Some(repo) = arguments.get_one::<String>("repo") {
        invoke(
            "evidence.discharge_report",
            &serde_json::json!({ "repo": repo, "binding": binding, "asOf": as_of }),
        )?
    } else {
        let facts_source = required(arguments, "facts")?;
        if binding_source == "-" && facts_source == "-" {
            return Err("--binding and --facts cannot both read stdin".to_owned());
        }
        let facts = json_input(&facts_source, "discharge facts")?;
        if !facts.is_array() {
            return Err("discharge facts must be a JSON array".to_owned());
        }
        invoke(
            "assurance.build_discharge",
            &serde_json::json!({ "binding": binding, "facts": facts, "asOf": as_of }),
        )?
    };
    if !built.outcome.carries_payload() {
        return Ok(built);
    }
    if arguments.get_flag("inward") {
        return inward(&built, arguments.get_flag("json"));
    }
    if arguments.get_flag("json") {
        return Ok(render_json(built));
    }
    let rendered = invoke("assurance.render_discharge", &built.payload)?;
    if !rendered.outcome.carries_payload() {
        return Ok(rendered);
    }
    let text = rendered
        .payload
        .get("rendered")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "assurance.render_discharge did not return rendered text".to_owned())?;
    Ok(Response::ok(serde_json::json!({ "rendered": text })))
}

fn inward(built: &Response, json: bool) -> Result<Response, String> {
    let accounted = invoke("assurance.inward_completeness", &built.payload)?;
    if !accounted.outcome.carries_payload() {
        return Ok(accounted);
    }
    let key = if json { "completeness" } else { "rendered" };
    let member = accounted
        .payload
        .get(key)
        .ok_or_else(|| format!("assurance.inward_completeness did not return {key}"))?;
    let text = match member.as_str() {
        Some(text) => text.to_owned(),
        None => serde_json::to_string_pretty(member).map_err(|error| error.to_string())?,
    };
    Ok(Response::ok(serde_json::json!({ "rendered": text })))
}

fn render_json(response: Response) -> Response {
    if !response.outcome.carries_payload() {
        return response;
    }
    match serde_json::to_string_pretty(&response.payload) {
        Ok(rendered) => Response {
            payload: serde_json::json!({ "rendered": rendered }),
            diagnostics: response.diagnostics,
            outcome: response.outcome,
        },
        Err(_) => response,
    }
}

fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}

fn json_input(source: &str, label: &str) -> Result<serde_json::Value, String> {
    let bytes = if source == "-" {
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read {label}: {error}"))?;
        bytes
    } else {
        std::fs::read(source).map_err(|error| format!("cannot read {source}: {error}"))?
    };
    serde_json::from_slice(&bytes).map_err(|error| format!("{label} is not JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::command;
    use ix_trace_rs::trace;

    #[trace("FR-046")]
    #[test]
    fn tc_373_discharge_requires_the_retained_inputs() {
        assert!(command().try_get_matches_from(["discharge"]).is_err());
        assert!(
            command()
                .try_get_matches_from([
                    "discharge",
                    "--binding",
                    "a.json",
                    "--facts",
                    "f.json",
                    "--as-of",
                    "2026-01-01T00:00:00Z"
                ])
                .is_ok()
        );
    }
}
