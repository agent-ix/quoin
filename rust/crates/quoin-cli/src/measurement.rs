// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapters for measurement collection publication (quoin#373, Stage 8).

mod campaign;
mod source_state;
mod tamper;

use ix_trace_rs::implements;
use std::io::Read as _;

use clap::{Arg, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_runtime::invoke;
use crate::intake_order::intake_order;

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
        .subcommand(
            Command::new("attach")
                .about("Retain a repository file as an immutable content-addressed attachment")
                .arg(repo())
                .arg(
                    Arg::new("path")
                        .long("path")
                        .required(true)
                        .help("The file to retain, relative to the repository (at most 64 MiB)."),
                )
                .arg(
                    Arg::new("media-type")
                        .long("media-type")
                        .required(true)
                        .help("The file's media type."),
                ),
        )
        .subcommand(
            Command::new("verify")
                .about("Decide a plan's verdict from its stored collections, independent of the producer")
                .arg(repo())
                .arg(
                    Arg::new("plan")
                        .long("plan")
                        .required(true)
                        .value_name("MP-ID")
                        .help("The MeasurementPlan to verify."),
                )
                .arg(
                    Arg::new("claimed")
                        .long("claimed")
                        .value_parser(["accept", "reject", "inconclusive"])
                        .help(
                            "A verdict someone claims for the plan. The checker rejects when \
                             its own verdict differs.",
                        ),
                ),
        )
        .subcommand(producer("intervention"))
        .subcommand(producer("operational-release"))
        .subcommand(campaign::command())
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
        "attach" => attach(arguments),
        "verify" => verify(arguments),
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
        "campaign" => campaign::run(arguments),
        _ => Err("an unknown measurement command reached dispatch".to_owned()),
    }
}

fn record(arguments: &ArgMatches) -> Result<Response, String> {
    let input = required(arguments, "input")?;
    let mut record = json_input(&input, "measurement input")?;
    let repo = required(arguments, "repo")?;
    source_state::attest(&repo, &mut record)?;
    publish(
        "measurement.record",
        &serde_json::json!({ "repo": repo, "record": record }),
    )
}

/// `quoin measurement attach` (FR-125): retain a repository file and print the
/// reference to state in a collection's `attachments` member.
#[implements("FR-125")]
fn attach(arguments: &ArgMatches) -> Result<Response, String> {
    let mut response = invoke(
        "measurement.attach",
        &serde_json::json!({
            "repo": required(arguments, "repo")?,
            "path": required(arguments, "path")?,
            "media_type": required(arguments, "media-type")?,
        }),
    )?;
    if response.outcome.carries_payload() {
        let rendered = quoin_core::protocol::canonical_json(&response.payload)
            .map_err(|error| error.to_string())?;
        response.payload = serde_json::json!({ "rendered": rendered });
    }
    Ok(response)
}

/// `quoin measurement verify` (FR-108, PLAT-961): the independent verdict
/// checker. Prints the verdict document; exits 1 when it is not `accept`.
///
/// PLAT-985 adds git-derived tamper facts alongside the intake order,
/// gathered in [`tamper`] for the reason stated there: only this binary has
/// git, and the checker itself stays pure.
#[implements("FR-108")]
fn verify(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let plan = required(arguments, "plan")?;
    let order = intake_order(&repo)?;
    let (deleted, edited) = tamper::deleted_and_edited(&repo)?;
    let request = serde_json::json!({
        "repo": repo,
        "plan": plan,
        "claimed": arguments.get_one::<String>("claimed"),
        "intake_order": order.groups,
        "order_source": order.source,
        "deleted": tamper::to_wire(deleted),
        "edited_collections": tamper::to_wire(edited),
        "definition_changed_without_version_bump": tamper::definition_changed(&repo, &plan),
    });
    let mut response = invoke("measurement.verify", &request)?;
    if response.outcome.carries_payload() {
        let rendered = quoin_core::protocol::canonical_json(&response.payload)
            .map_err(|error| error.to_string())?;
        response.payload = serde_json::json!({ "rendered": rendered, "warning": order.warning });
    }
    Ok(response)
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
    use ix_trace_rs::trace;

    #[trace("FR-061", "FR-102")]
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
                .try_get_matches_from(["measurement", "verify", "--plan", "MP-1"])
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
