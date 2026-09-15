// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the retained `quoin discharge` command.

use std::io::Read as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_bridge::invoke;

pub(crate) fn command() -> Command {
    Command::new("discharge")
        .about("Partition binding clauses into direct evidence, dispositions, and open work")
        .arg(Arg::new("binding").long("binding").required(true))
        .arg(Arg::new("facts").long("facts").required(true))
        .arg(Arg::new("as-of").long("as-of").required(true))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let binding_source = required(arguments, "binding")?;
    let facts_source = required(arguments, "facts")?;
    if binding_source == "-" && facts_source == "-" {
        return Err("--binding and --facts cannot both read stdin".to_owned());
    }
    let binding = json_input(&binding_source, "clause binding")?;
    let facts = json_input(&facts_source, "discharge facts")?;
    if !facts.is_array() {
        return Err("discharge facts must be a JSON array".to_owned());
    }
    let built = invoke(
        "assurance.build_discharge",
        &serde_json::json!({ "binding": binding, "facts": facts, "asOf": required(arguments, "as-of")? }),
    )?;
    if !built.outcome.carries_payload() || arguments.get_flag("json") {
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

    /// Trace: FR-046
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
