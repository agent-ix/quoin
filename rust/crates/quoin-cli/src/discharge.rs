// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the retained `quoin discharge` command.

use std::io::Read as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_runtime::{invoke, invoke_refusable};
use crate::evidence::{json_arg, repo_arg};

/// Maximum size of one JSON file or stdin input accepted by discharge adapters.
const MAX_JSON_INPUT_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn command() -> Command {
    Command::new("discharge")
        .about("Partition binding clauses into direct evidence, dispositions, and open work")
        .arg(
            Arg::new("binding")
                .long("binding")
                .required_unless_present("pack")
                .help("Quire clause-binding-v1 JSON path, or - for stdin. Required unless --pack is given."),
        )
        .arg(
            Arg::new("facts")
                .long("facts")
                .required_unless_present_any(["repo", "pack"])
                .help("JSON array of discharge facts."),
        )
        .arg(Arg::new("repo").long("repo").help(
            "Read the facts stored for the binding report's clause set in this \
             repository's evidence store, instead of --facts.",
        ))
        .arg(
            Arg::new("as-of")
                .long("as-of")
                .required(true)
                .help("Explicit ISO-8601 evaluation instant."),
        )
        .arg(
            Arg::new("inward")
                .long("inward")
                .action(ArgAction::SetTrue)
                .help(
                    "Report inward completeness instead of the partition: binding, \
             discharged, dispositions and the open remainder. Needs --repo.",
                ),
        )
        .arg(Arg::new("rmm").long("rmm").action(ArgAction::SetTrue).help(
            "Render a requirements mapping matrix view: one row per clause with \
             applicability, compliance, evidence and tailoring. Needs --repo.",
        ))
        .arg(Arg::new("pack").long("pack").help(
            "Report on an installed pack: one requirements mapping matrix per clause \
             set behind an index. Needs --rmm, --repo and --as-of.",
        ))
        .arg(
            Arg::new("context")
                .long("context")
                .action(ArgAction::Append)
                .help("KEY=VALUE context fact shared by every clause set. Needs --pack."),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Emit the complete report as JSON."),
        )
}

pub(crate) fn record_command() -> Command {
    Command::new("record-discharge")
        .about("Store attested clause-discharge facts for a clause set")
        .arg(Arg::new("binding").long("binding").required(true).help(
            "Quire clause-binding-v1 JSON path, or - for stdin. Only its clause set is stored.",
        ))
        .arg(
            Arg::new("facts")
                .long("facts")
                .required(true)
                .help("JSON array of discharge facts, or - for stdin."),
        )
        .arg(repo_arg().help("Repository root. Defaults to the working directory."))
        .arg(json_arg().help("Emit the written paths as JSON."))
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
    if let Some(pack) = arguments.get_one::<String>("pack") {
        return pack_report(arguments, pack);
    }
    if arguments.contains_id("context") {
        return Err("--context needs --pack: the context is evaluated against a pack".to_owned());
    }
    if arguments.contains_id("facts") && arguments.contains_id("repo") {
        return Err("--facts and --repo cannot be combined".to_owned());
    }
    let binding_source = required(arguments, "binding")?;
    let binding = json_input(&binding_source, "clause binding")?;
    let as_of = required(arguments, "as-of")?;
    if arguments.get_flag("inward") && arguments.get_flag("rmm") {
        return Err("--inward and --rmm cannot be combined".to_owned());
    }
    if arguments.get_flag("inward") {
        let repo = arguments
            .get_one::<String>("repo")
            .ok_or_else(|| "--inward needs --repo: the obligations are derived from the binding report and the facts from the store".to_owned())?;
        return view(
            &invoke(
                "evidence.inward_completeness",
                &serde_json::json!({ "repo": repo, "binding": binding, "asOf": as_of }),
            )?,
            arguments.get_flag("json"),
            ("completeness", "evidence.inward_completeness"),
        );
    }
    if arguments.get_flag("rmm") {
        let repo = arguments.get_one::<String>("repo").ok_or_else(|| {
            "--rmm needs --repo: the facts are read from the evidence store".to_owned()
        })?;
        return view(
            &invoke(
                "evidence.requirements_matrix",
                &serde_json::json!({ "repo": repo, "binding": binding, "asOf": as_of }),
            )?,
            arguments.get_flag("json"),
            ("matrix", "evidence.requirements_matrix"),
        );
    }
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

/// `--pack`: the requirements mapping report of an installed pack (FR-140).
fn pack_report(arguments: &ArgMatches, pack: &str) -> Result<Response, String> {
    if !arguments.get_flag("rmm") {
        return Err(
            "--pack needs --rmm: the pack report is a requirements mapping view".to_owned(),
        );
    }
    let repo = arguments.get_one::<String>("repo").ok_or_else(|| {
        "--pack needs --repo: the facts are read from the evidence store".to_owned()
    })?;
    if arguments.contains_id("binding") {
        return Err(
            "--pack cannot be combined with --binding: bindings are evaluated from the pack"
                .to_owned(),
        );
    }
    if arguments.contains_id("facts") {
        return Err(
            "--pack cannot be combined with --facts: facts are read from the evidence store"
                .to_owned(),
        );
    }
    if arguments.get_flag("inward") {
        return Err("--pack cannot be combined with --inward".to_owned());
    }
    let context = arguments
        .get_many::<String>("context")
        .map(|entries| entries.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    view(
        &invoke_refusable(
            "evidence.pack_requirements_matrix",
            &serde_json::json!({
                "pack": pack,
                "repo": repo,
                "asOf": required(arguments, "as-of")?,
                "context": context,
            }),
        )?,
        arguments.get_flag("json"),
        ("report", "evidence.pack_requirements_matrix"),
    )
}

/// Show one of the views an `evidence.*` operation returns beside its
/// rendering: the structured member under `--json`, the Markdown otherwise.
fn view(
    accounted: &Response,
    json: bool,
    (structured, op): (&str, &str),
) -> Result<Response, String> {
    if !accounted.outcome.carries_payload() {
        return Ok(accounted.clone());
    }
    let key = if json { structured } else { "rendered" };
    let member = accounted
        .payload
        .get(key)
        .ok_or_else(|| format!("{op} did not return {key}"))?;
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

pub(crate) fn json_input(source: &str, label: &str) -> Result<serde_json::Value, String> {
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_JSON_INPUT_BYTES + 1)
        .map_err(|error| format!("invalid JSON input limit: {error}"))?;
    if source == "-" {
        std::io::stdin()
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read {label}: {error}"))?;
    } else {
        std::fs::File::open(source)
            .and_then(|file| file.take(limit).read_to_end(&mut bytes))
            .map_err(|error| format!("cannot read {label} {source}: {error}"))?;
    }
    if bytes.len() > MAX_JSON_INPUT_BYTES {
        return Err(format!(
            "{label} exceeds the {MAX_JSON_INPUT_BYTES}-byte input limit"
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("{label} is not JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{MAX_JSON_INPUT_BYTES, command, json_input};
    use ix_trace_rs::trace;

    #[allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    mod bounded_json_input {
        use super::{MAX_JSON_INPUT_BYTES, json_input};
        use ix_trace_rs::trace;

        #[trace("FR-173-AC-5")]
        #[test]
        fn tc_173_json_input_refuses_a_file_above_the_byte_ceiling() {
            let scratch = tempfile::tempdir().unwrap();
            let path = scratch.path().join("too-large.json");
            std::fs::write(&path, vec![b' '; MAX_JSON_INPUT_BYTES + 1]).unwrap();

            let error = json_input(path.to_str().unwrap(), "test JSON").unwrap_err();
            assert!(error.contains("byte input limit"), "{error}");
        }
    }

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
