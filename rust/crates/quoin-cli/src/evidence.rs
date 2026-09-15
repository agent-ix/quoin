// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapters for retained evidence-store commands (quoin#373, Stage 8).

use std::io::Read as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::core_bridge::invoke;

mod baseline;

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
        .subcommand(
            Command::new("inspect-mocks")
                .about("Inspect test source and record explicit mock substitutions")
                .arg(Arg::new("suite").long("suite").required(true))
                .arg(Arg::new("commit").long("commit").required(true))
                .arg(repo_arg())
                .arg(Arg::new("timestamp").long("timestamp"))
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue),
                )
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("affirm")
                .about("Re-affirm a binding after its statement changed")
                .arg(Arg::new("obligation").long("obligation").required(true))
                .arg(Arg::new("who").long("who").required(true))
                .arg(Arg::new("note").long("note"))
                .arg(Arg::new("commit").long("commit"))
                .arg(repo_arg())
                .arg(Arg::new("module").long("module"))
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("record")
                .about("Transcribe one suite run into the evidence store")
                .arg(Arg::new("suite").long("suite").required(true))
                .arg(Arg::new("commit").long("commit").required(true))
                .arg(Arg::new("tool").long("tool").required(true))
                .arg(Arg::new("kind").long("kind"))
                .arg(Arg::new("lineage").long("lineage"))
                .arg(Arg::new("discharges").long("discharges"))
                .arg(Arg::new("adapter").long("adapter"))
                .arg(Arg::new("results").long("results").required(true))
                .arg(repo_arg())
                .arg(Arg::new("module").long("module"))
                .arg(Arg::new("timestamp").long("timestamp"))
                .arg(json_arg()),
        )
        .subcommand(baseline::command())
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
        "inspect-mocks" => inspect_mocks(arguments),
        "affirm" => affirm(arguments),
        "record" => record_run(arguments),
        "baseline" => baseline::run(arguments),
        _ => Err("an unknown evidence command reached dispatch".to_owned()),
    }
}

fn record_run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let modules = arguments
        .get_one::<String>("module")
        .map_or_else(Vec::new, |module| vec![module.clone()]);
    let coverage = invoke(
        "quire.coverage",
        &serde_json::json!({ "scope": repo, "modules": modules }),
    )?;
    if !coverage.outcome.carries_payload() {
        return Ok(coverage);
    }
    let results = String::from_utf8(input(&required(arguments, "results")?)?)
        .map_err(|error| format!("results are not UTF-8: {error}"))?;
    let lineage = arguments
        .get_one::<String>("lineage")
        .map(|source| json_input(source))
        .transpose()?;
    let timestamp = arguments
        .get_one::<String>("timestamp")
        .cloned()
        .map_or_else(
            || {
                OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .map_err(|error| error.to_string())
            },
            Ok,
        )?;
    let discharges = arguments
        .get_one::<String>("discharges")
        .map_or_else(Vec::new, |value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
                .collect()
        });
    let response = invoke(
        "evidence.record",
        &serde_json::json!({
            "repo": repo, "suite": required(arguments, "suite")?, "commit": required(arguments, "commit")?, "tool": required(arguments, "tool")?,
            "kind": arguments.get_one::<String>("kind"), "lineage": lineage, "timestamp": timestamp, "adapter": arguments.get_one::<String>("adapter"),
            "results": results, "discharges": discharges, "obligations": coverage.payload.get("obligations").cloned().unwrap_or_else(|| serde_json::json!([])),
        }),
    )?;
    render(response, |payload| {
        if arguments.get_flag("json") {
            return serde_json::to_string(payload).map_err(|error| error.to_string());
        }
        match payload.get("kind").and_then(serde_json::Value::as_str) {
            Some("scan") => Ok(format!(
                "recorded scan {} @ {} → {}",
                required(arguments, "suite")?,
                required(arguments, "commit")?.get(..12).unwrap_or(""),
                payload
                    .get("scan_path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            )),
            Some("run") => Ok(format!(
                "recorded {} @ {} → {}",
                required(arguments, "suite")?,
                required(arguments, "commit")?.get(..12).unwrap_or(""),
                payload
                    .get("run_path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            )),
            _ => Err("evidence.record returned an unknown payload kind".to_owned()),
        }
    })
}

fn affirm(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let modules = arguments
        .get_one::<String>("module")
        .map_or_else(Vec::new, |module| vec![module.clone()]);
    let coverage = invoke(
        "quire.coverage",
        &serde_json::json!({ "scope": repo, "modules": modules }),
    )?;
    if !coverage.outcome.carries_payload() {
        return Ok(coverage);
    }
    let obligation = required(arguments, "obligation")?;
    let current = coverage
        .payload
        .get("obligations")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("id").and_then(serde_json::Value::as_str) == Some(obligation.as_str())
            })
        })
        .ok_or_else(|| {
            format!("no obligation `{obligation}` is derived from this specification today")
        })?;
    let statement_hash = current
        .get("statement_hash")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("obligation `{obligation}` has no statement_hash"))?;
    let commit = arguments
        .get_one::<String>("commit")
        .cloned()
        .unwrap_or_else(|| revision(&repo));
    let response = invoke(
        "evidence.affirm",
        &serde_json::json!({ "repo": repo, "obligation": obligation, "statement_hash": statement_hash, "who": required(arguments, "who")?, "commit": commit, "note": arguments.get_one::<String>("note") }),
    )?;
    render(response, |payload| {
        if !payload
            .get("found")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            return Err(format!(
                "no binding exists for `{obligation}`, so there is nothing to affirm"
            ));
        }
        if arguments.get_flag("json") {
            return serde_json::to_string(&serde_json::json!({ "obligation": obligation, "who": required(arguments, "who")?, "commit": commit, "statementHash": statement_hash })).map_err(|error| error.to_string());
        }
        Ok(format!(
            "affirmed {obligation} by {} at {} (hash {}…)",
            required(arguments, "who")?,
            commit.get(..12).unwrap_or(&commit),
            statement_hash.get(..12).unwrap_or(statement_hash)
        ))
    })
}

pub(super) fn revision(repo: &str) -> String {
    std::process::Command::new("git")
        .args(["-C", repo, "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

fn inspect_mocks(arguments: &ArgMatches) -> Result<Response, String> {
    let timestamp = arguments
        .get_one::<String>("timestamp")
        .cloned()
        .map_or_else(
            || {
                OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .map_err(|error| error.to_string())
            },
            Ok,
        )?;
    let suite = required(arguments, "suite")?;
    let commit = required(arguments, "commit")?;
    let dry_run = arguments.get_flag("dry-run");
    let response = invoke(
        "evidence.inspect_mocks",
        &serde_json::json!({
            "repo": required(arguments, "repo")?, "suite": suite, "commit": commit,
            "tool": concat!("quoin mock-inspection ", env!("CARGO_PKG_VERSION")),
            "timestamp": timestamp, "dry_run": dry_run,
        }),
    )?;
    render(response, |payload| {
        let path = payload
            .get("path")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let injections = payload
            .get("injections")
            .cloned()
            .ok_or_else(|| "evidence.inspect_mocks did not return injections".to_owned())?;
        if arguments.get_flag("json") {
            return serde_json::to_string(
                &serde_json::json!({ "path": path, "injections": injections }),
            )
            .map_err(|error| error.to_string());
        }
        let count = injections.as_array().map_or(0, Vec::len);
        Ok(format!(
            "{} {suite} @ {}{}\n  injections: {count}",
            if dry_run {
                "inspected"
            } else {
                "recorded mock inspection"
            },
            commit.get(..12).unwrap_or(&commit),
            path.as_str().map_or_else(
                || " (dry run; wrote nothing)".to_owned(),
                |path| format!(" -> {path}")
            )
        ))
    })
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

pub(super) fn repo_arg() -> Arg {
    Arg::new("repo").long("repo").default_value(".")
}
pub(super) fn json_arg() -> Arg {
    Arg::new("json").long("json").action(ArgAction::SetTrue)
}
pub(super) fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
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
        assert!(
            command()
                .try_get_matches_from(["evidence", "baseline", "--dry-run"])
                .is_ok()
        );
    }
}
