// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapters for retained evidence-store commands (quoin#373, Stage 8).

use std::io::Read as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_trace_rs::implements;
use quoin_core::protocol::Response;
use time::OffsetDateTime;

use crate::core_runtime::invoke;

pub(crate) mod audit;
mod mapping_facts;
mod test_result;

const DESCRIPTION: &str = "The artifact of record for verification (FR-030). quoin TRANSCRIBES; the\nconsumer's CI executes — nothing here runs a test.\n\nLayout, under spec/evidence/:\n\n  suites.md        authored suite registry     (validated corpus document)\n  inspections.md   authored inspection acts    (validated corpus document)\n  bindings.json    obligation -> evidence\n  runs/<SUITE-N>/<timestamp>.json  one file = one run of one suite\n  customer-artifacts/<artifact-id>.json  customer-supplied record or document\n  experiments/<recordId>.json      experiment records\n  operational/<recordId>.json       operational evidence\n  discharge/<authority>/<set>/<version>/<clause>.json  attested discharge facts\n\nSubcommands:\n  quoin evidence record     transcribe a suite run\n  quoin evidence trust      record a use-specific producer trust decision\n  quoin evidence record-experiment  publish an immutable experiment record\n  quoin evidence record-operational publish immutable operational evidence\n  quoin evidence record-customer-artifact  retain a record or document the customer supplied\n  quoin evidence record-discharge  store attested clause-discharge facts\n  quoin evidence approval-subject  print the record id and digest an approval tag signs\n  quoin evidence facts-from-mappings  propose discharge facts from effective controls
  quoin evidence audit      read the store and report\n  quoin evidence gc         drop run records superseded by a newer one\n  quoin evidence export-test-result  export runs as an in-toto Test Result statement";

/// The evidence commands whose inputs need no Quire-derived obligation set.
pub(crate) fn command() -> Command {
    Command::new("evidence")
        .about("Record and maintain verification evidence")
        .subcommand(
            Command::new("gc")
                .about("Drop superseded run and scan records, naming any unreadable record it keeps")
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
                .about("Record a use-specific evidence-producer trust decision; a changed decision needs a new id and an id already holding a different decision is refused")
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
            Command::new("record-customer-artifact")
                .about("Retain one record or document the customer supplied, as a description and a locator; the artifact itself is never read")
                .arg(
                    Arg::new("artifact")
                        .long("artifact")
                        .help("customer artifact JSON path, or - for stdin")
                        .required(true),
                )
                .arg(repo_arg())
                .arg(json_arg()),
        )
        .subcommand(
            Command::new("inspect-mocks")
                .about("Inspect test source and record explicit mock substitutions")
                .arg(Arg::new("suite").long("suite").required(true))
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
            Command::new("record")
                .about("Transcribe one suite run into the evidence store")
                .arg(Arg::new("suite").long("suite").required(true))
                .arg(Arg::new("tool").long("tool").required(true))
                .arg(Arg::new("kind").long("kind"))
                .arg(Arg::new("lineage").long("lineage"))
                .arg(Arg::new("discharges").long("discharges"))
                .arg(Arg::new("adapter").long("adapter"))
                .arg(
                    Arg::new("run-kind")
                        .long("run-kind")
                        .value_parser(["analysis", "formal_rule"]),
                )
                .arg(Arg::new("formal-rule-selection").long("formal-rule-selection"))
                .arg(Arg::new("policy").long("policy"))
                .arg(Arg::new("results").long("results").required(true))
                .arg(repo_arg())
                .arg(Arg::new("module").long("module"))
                .arg(Arg::new("timestamp").long("timestamp"))
                .arg(json_arg()),
        )
        .subcommand(crate::discharge::record_command())
        .subcommand(crate::discharge::approval_subject_command())
        .subcommand(mapping_facts::command())
        .subcommand(test_result::command())
        .subcommand(audit::command())
}

fn record_command(name: &'static str, help: &'static str) -> Command {
    Command::new(name)
        .about("Publish one evidence record")
        .arg(Arg::new("input").long("input").help(help).required(true))
        .arg(repo_arg())
        .arg(json_arg())
}

/// Execute a parsed evidence command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let Some((name, arguments)) = matches.subcommand() else {
        return Ok(Response::ok(serde_json::json!({ "rendered": DESCRIPTION })));
    };
    match name {
        "gc" => gc(arguments),
        "trust" => trust(arguments),
        "record-experiment" => record(arguments, "evidence.record_experiment"),
        "record-operational" => record(arguments, "evidence.record_operational"),
        "record-customer-artifact" => record_customer_artifact(arguments),
        "inspect-mocks" => inspect_mocks(arguments),
        "record" => record_run(arguments),
        "record-discharge" => crate::discharge::record(arguments),
        "approval-subject" => crate::discharge::approval_subject(arguments),
        "facts-from-mappings" => mapping_facts::run(arguments),
        "export-test-result" => test_result::run(arguments),
        "audit" => audit::run(arguments),
        _ => Err("an unknown evidence command reached dispatch".to_owned()),
    }
}

#[implements("FR-030-AC-28")]
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
    let results = String::from_utf8(bounded_input(
        &required(arguments, "results")?,
        MAX_RECORD_RESULTS_BYTES,
    )?)
    .map_err(|error| format!("results are not UTF-8: {error}"))?;
    let formal_rule_selection = read_formal_rule_selection(arguments)?;
    let policy = read_policy(arguments)?;
    let lineage = arguments
        .get_one::<String>("lineage")
        .map(|source| json_input(source))
        .transpose()?;
    let timestamp = arguments
        .get_one::<String>("timestamp")
        .cloned()
        .map_or_else(|| canonical_timestamp(OffsetDateTime::now_utc()), Ok)?;
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
            "repo": repo, "suite": required(arguments, "suite")?, "tool": required(arguments, "tool")?,
            "kind": arguments.get_one::<String>("kind"), "lineage": lineage, "timestamp": timestamp, "adapter": arguments.get_one::<String>("adapter"), "policy": policy,
            "results": results, "discharges": discharges, "obligations": coverage.payload.get("obligations").cloned().unwrap_or_else(|| serde_json::json!([])),
            "run_kind": arguments.get_one::<String>("run-kind"), "formal_rule_selection": formal_rule_selection,
        }),
    )?;
    render(response, |payload| {
        if arguments.get_flag("json") {
            return serde_json::to_string_pretty(payload).map_err(|error| error.to_string());
        }
        match payload.get("kind").and_then(serde_json::Value::as_str) {
            Some("scan") => Ok(format!(
                "recorded scan {} → {}",
                required(arguments, "suite")?,
                payload
                    .get("scan_path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            )),
            Some("run") => {
                let recorded = format!(
                    "recorded {} → {}",
                    required(arguments, "suite")?,
                    payload
                        .get("run_path")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                );
                let named = payload
                    .get("unrepresented")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|item| {
                        let field = |name| item.get(name).and_then(serde_json::Value::as_str);
                        format!(
                            "  not transcribed — {} reported {}: {}",
                            field("symbol").unwrap_or(""),
                            field("state").unwrap_or(""),
                            field("reason").unwrap_or("")
                        )
                    });
                Ok(std::iter::once(recorded)
                    .chain(named)
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            _ => Err("evidence.record returned an unknown payload kind".to_owned()),
        }
    })
}

fn read_formal_rule_selection(arguments: &ArgMatches) -> Result<Option<serde_json::Value>, String> {
    arguments
        .get_one::<String>("formal-rule-selection")
        .map(|path| {
            let bytes = bounded_input(path, MAX_RECORD_RESULTS_BYTES)?;
            serde_json::from_slice(&bytes)
                .map_err(|error| format!("formal-rule-selection is not JSON: {error}"))
        })
        .transpose()
}

fn read_policy(arguments: &ArgMatches) -> Result<Option<String>, String> {
    arguments
        .get_one::<String>("policy")
        .map(|path| {
            String::from_utf8(bounded_input(path, MAX_RECORD_RESULTS_BYTES)?)
                .map_err(|error| format!("policy is not UTF-8: {error}"))
        })
        .transpose()
}

#[implements("FR-030-AC-28")]
fn inspect_mocks(arguments: &ArgMatches) -> Result<Response, String> {
    let timestamp = arguments
        .get_one::<String>("timestamp")
        .cloned()
        .map_or_else(|| canonical_timestamp(OffsetDateTime::now_utc()), Ok)?;
    let suite = required(arguments, "suite")?;
    let dry_run = arguments.get_flag("dry-run");
    let response = invoke(
        "evidence.inspect_mocks",
        &serde_json::json!({
            "repo": required(arguments, "repo")?, "suite": suite,
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
            return serde_json::to_string_pretty(
                &serde_json::json!({ "path": path, "injections": injections }),
            )
            .map_err(|error| error.to_string());
        }
        let count = injections.as_array().map_or(0, Vec::len);
        Ok(format!(
            "{} {suite}{}\n  injections: {count}",
            if dry_run {
                "inspected"
            } else {
                "recorded mock inspection"
            },
            path.as_str().map_or_else(
                || " (dry run; wrote nothing)".to_owned(),
                |path| format!(" -> {path}")
            )
        ))
    })
}

#[implements("FR-030-AC-28")]
fn canonical_timestamp(now: OffsetDateTime) -> Result<String, String> {
    let milliseconds = (now.nanosecond() / 1_000_000) * 1_000_000;
    let timestamp = now
        .replace_nanosecond(milliseconds)
        .map_err(|error| error.to_string())?;
    let format = time::format_description::parse_borrowed::<3>(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
    )
    .map_err(|error| error.to_string())?;
    timestamp
        .to_offset(time::UtcOffset::UTC)
        .format(&format)
        .map_err(|error| error.to_string())
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
        let skipped = payload
            .get("skipped")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "evidence.gc did not return skipped".to_owned())?;
        if arguments.get_flag("json") {
            return serde_json::to_string_pretty(
                &serde_json::json!({ "deleted": deleted, "skipped": skipped, "dryRun": dry_run }),
            )
            .map_err(|error| error.to_string());
        }
        let mut lines: Vec<String> = deleted
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(|path| {
                format!(
                    "{} {path}",
                    if dry_run { "would delete" } else { "deleted" }
                )
            })
            .collect();
        lines.extend(
            skipped
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(|path| format!("kept unreadable {path}")),
        );
        if lines.is_empty() {
            return Ok("nothing to collect".to_owned());
        }
        Ok(lines.join("\n"))
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
            return serde_json::to_string_pretty(payload).map_err(|error| error.to_string());
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
            return serde_json::to_string_pretty(payload).map_err(|error| error.to_string());
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

fn record_customer_artifact(arguments: &ArgMatches) -> Result<Response, String> {
    let operation = "evidence.record_customer_artifact";
    let source = required(arguments, "artifact")?;
    let bytes = bounded_input(&source, MAX_CUSTOMER_ARTIFACT_INPUT_BYTES)?;
    let document: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("input is not JSON: {error}"))?;
    let response = invoke(
        operation,
        &serde_json::json!({ "repo": required(arguments, "repo")?, "document": document }),
    )?;
    render(response, |payload| {
        if arguments.get_flag("json") {
            return serde_json::to_string_pretty(payload).map_err(|error| error.to_string());
        }
        let text = |value: Option<&serde_json::Value>| {
            value
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_owned()
        };
        let id = text(payload.pointer("/record/artifactId"));
        let state = if payload
            .get("created")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            "created"
        } else {
            "exists"
        };
        Ok(format!("{id} {state} {}", text(payload.get("path"))))
    })
}

pub(crate) fn repo_arg() -> Arg {
    Arg::new("repo").long("repo").default_value(".")
}
pub(crate) fn json_arg() -> Arg {
    Arg::new("json").long("json").action(ArgAction::SetTrue)
}
pub(crate) fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn json_input(source: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&input(source)?).map_err(|error| format!("input is not JSON: {error}"))
}
/// The largest `--artifact` document `evidence record-customer-artifact` reads: the
/// core's own bound on the whole request.
const MAX_CUSTOMER_ARTIFACT_INPUT_BYTES: u64 =
    quoin_core::ops::evidence::MAX_CUSTOMER_ARTIFACT_BYTES as u64;

/// The largest `--results` document `evidence record` reads, in bytes: the
/// core's own bound on the whole request, since the results are the bulk of it.
const MAX_RECORD_RESULTS_BYTES: u64 = quoin_core::ops::evidence::MAX_RECORD_BYTES as u64;

/// Read `source` (a path, or `-` for stdin) up to `limit` bytes, refusing a
/// longer document without reading the rest of it.
fn bounded_input(source: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let read = if source == "-" {
        std::io::stdin().take(limit + 1).read_to_end(&mut bytes)
    } else {
        std::fs::File::open(source)
            .map_err(|error| format!("cannot read {source}: {error}"))?
            .take(limit + 1)
            .read_to_end(&mut bytes)
    };
    read.map_err(|error| format!("cannot read {source}: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!("{source} is larger than {limit} bytes"));
    }
    Ok(bytes)
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
    // The warning rides beside the rendering: `emit` writes it to stderr.
    let mut payload = serde_json::json!({ "rendered": text(&response.payload)? });
    if let (Some(warning), Some(fields)) =
        (response.payload.get("warning"), payload.as_object_mut())
    {
        fields.insert("warning".to_owned(), warning.clone());
    }
    Ok(Response {
        payload,
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
    use super::{DESCRIPTION, canonical_timestamp, command, run};
    use ix_trace_rs::trace;
    use time::OffsetDateTime;

    #[trace("FR-030", "FR-062", "FR-102")]
    #[test]
    fn tc_373_evidence_without_a_subcommand_keeps_the_retained_description() {
        let matches = command()
            .try_get_matches_from(["evidence"])
            .expect("the retained parent command parses");
        let response = run(&matches).expect("the parent description renders");
        assert_eq!(
            response
                .payload
                .get("rendered")
                .and_then(serde_json::Value::as_str),
            Some(DESCRIPTION)
        );
    }

    #[trace("FR-030", "FR-030-AC-36")]
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
        let artifact = command()
            .try_get_matches_from([
                "evidence",
                "record-customer-artifact",
                "--repo",
                "r",
                "--artifact",
                "-",
            ])
            .unwrap();
        assert_eq!(
            artifact
                .subcommand_matches("record-customer-artifact")
                .and_then(|m| m.get_one::<String>("artifact"))
                .map(String::as_str),
            Some("-")
        );
        assert!(
            command()
                .try_get_matches_from(["evidence", "record-customer-artifact"])
                .is_err(),
            "--artifact is required"
        );
        assert!(
            command()
                .try_get_matches_from([
                    "evidence", "audit", "--module", "first", "--module", "second", "--strict",
                ])
                .is_ok()
        );
    }

    #[trace("FR-030-AC-28")]
    #[test]
    fn generated_evidence_timestamps_are_exact_milliseconds() {
        let now = OffsetDateTime::from_unix_timestamp_nanos(1_123_456_789)
            .expect("fixed timestamp is in range");
        assert_eq!(
            canonical_timestamp(now).expect("timestamp formats"),
            "1970-01-01T00:00:01.123Z"
        );
    }
}
