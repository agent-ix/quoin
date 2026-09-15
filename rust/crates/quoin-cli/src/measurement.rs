// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapters for measurement collection publication (quoin#373, Stage 8).

use std::io::Read as _;

use clap::{Arg, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_bridge::invoke;

const DESCRIPTION: &str = "Measurement collections connect raw producer output to active\nMeasurementPlans. Use `quoin measurement record` to persist one complete producer\ninvocation; use `quoin report` for current state, comparisons, and series.";

/// The `measurement` command and its retained subcommands.
pub(crate) fn command() -> Command {
    Command::new("measurement")
        .about("Record and inspect versioned QA measurements")
        .subcommand(
            Command::new("record")
                .arg(repo())
                .arg(Arg::new("input").long("input").required(true)),
        )
        .subcommand(producer("intervention"))
        .subcommand(producer("operational-release"))
}

fn repo() -> Arg {
    Arg::new("repo").long("repo").default_value(".")
}

fn producer(name: &'static str) -> Command {
    Command::new(name)
        .arg(repo())
        .arg(Arg::new("definition").long("definition").required(true))
}

/// Execute a parsed measurement command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let Some((name, arguments)) = matches.subcommand() else {
        return Ok(Response::ok(serde_json::json!({ "rendered": DESCRIPTION })));
    };
    match name {
        "record" => record(arguments),
        "intervention" => produce(
            arguments,
            "definition",
            "measurement.produce_agent_eval_intervention",
        ),
        "operational-release" => produce(
            arguments,
            "definition",
            "measurement.produce_github_release_operational",
        ),
        _ => Err("an unknown measurement command reached dispatch".to_owned()),
    }
}

fn record(arguments: &ArgMatches) -> Result<Response, String> {
    let input = required(arguments, "input")?;
    let record = json_input(&input, "measurement input")?;
    publish(
        "measurement.record",
        &serde_json::json!({ "repo": required(arguments, "repo")?, "record": record }),
    )
}

fn produce(arguments: &ArgMatches, field: &str, operation: &str) -> Result<Response, String> {
    let definition = json_input(&required(arguments, field)?, "measurement definition")?;
    publish(
        operation,
        &serde_json::json!({ "repo": required(arguments, "repo")?, "definition": definition }),
    )
}

fn publish(operation: &str, request: &serde_json::Value) -> Result<Response, String> {
    let mut response = invoke(operation, request)?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let path = response
        .payload
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{operation} returned no path"))?;
    response.payload = serde_json::json!({ "rendered": path });
    Ok(response)
}

fn json_input(path: &str, label: &str) -> Result<serde_json::Value, String> {
    let text = if path == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("cannot read {label} from stdin: {error}"))?;
        text
    } else {
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {label}: {error}"))?
    };
    serde_json::from_str(&text).map_err(|error| format!("{label} is not JSON: {error}"))
}

fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("{name} is required"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions report failures")]
mod tests {
    use super::command;

    /// Trace: FR-061, FR-102
    #[test]
    fn tc_373_measurement_preserves_the_retained_grammar() {
        assert!(command().try_get_matches_from(["measurement"]).is_ok());
        assert!(
            command()
                .try_get_matches_from(["measurement", "record", "--input", "record.json"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "measurement",
                    "intervention",
                    "--definition",
                    "producer.json"
                ])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "measurement",
                    "operational-release",
                    "--definition",
                    "release.json"
                ])
                .is_ok()
        );
    }
}
