// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the retained read-only `quoin assurance` view.

use std::process::Command as ProcessCommand;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::completeness::read_frontmatter;
use crate::core_runtime::invoke;

pub(crate) fn command() -> Command {
    Command::new("assurance")
        .about("Render the assurance case: claim → argument → evidence, from the store")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("module").long("module"))
        .arg(
            Arg::new("claim-type")
                .long("claim-type")
                .action(ArgAction::Append),
        )
        .arg(Arg::new("independence-policy").long("independence-policy"))
        .arg(Arg::new("argument").long("argument"))
        .arg(Arg::new("decisions").long("decisions"))
        .arg(Arg::new("discharge").long("discharge"))
        .arg(Arg::new("evidence").long("evidence"))
        .arg(Arg::new("as-of").long("as-of"))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_one::<String>("argument").is_some() {
        return authored(arguments);
    }
    derived(arguments)
}

fn authored(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_many::<String>("claim-type").is_some() {
        return Err("--argument cannot be combined with --claim-type".to_owned());
    }
    let repo = required(arguments, "repo")?;
    let argument_id = required(arguments, "argument")?;
    let decisions_path = required(arguments, "decisions")
        .map_err(|_| "--argument requires --decisions and --as-of".to_owned())?;
    let as_of = required(arguments, "as-of")
        .map_err(|_| "--argument requires --decisions and --as-of".to_owned())?;
    let frontmatter = read_frontmatter(std::path::Path::new(&format!("{repo}/spec")))?;
    if !frontmatter.outcome.carries_payload() {
        return Ok(frontmatter);
    }
    let documents = field(&frontmatter.payload, "documents")?;
    let matches = documents
        .as_array()
        .ok_or_else(|| "completeness.read_frontmatter did not return documents".to_owned())?
        .iter()
        .filter(|document| {
            document
                .get("frontmatter")
                .and_then(|frontmatter| frontmatter.get("id"))
                .and_then(serde_json::Value::as_str)
                == Some(argument_id.as_str())
        })
        .collect::<Vec<_>>();
    let argument = match matches.as_slice() {
        [document] => field(document, "frontmatter")?,
        [] => {
            return Err(format!(
                "AssuranceArgument {argument_id} was not found under {repo}/spec"
            ));
        }
        _ => {
            return Err(format!(
                "AssuranceArgument {argument_id} is declared more than once"
            ));
        }
    };
    let decisions = json_file(&decisions_path, "sufficiency decisions")?;
    if !decisions.is_array() {
        return Err("sufficiency decisions must be a JSON array".to_owned());
    }
    let discharge = arguments
        .get_one::<String>("discharge")
        .map(|path| json_file(path, "discharge report"))
        .transpose()?;
    // PLAT-966: without an index every cited reference reads as unresolved,
    // so the top claim can only be `supported` when the caller supplies one.
    let evidence = match arguments.get_one::<String>("evidence") {
        Some(path) => {
            let evidence = json_file(path, "evidence index entries")?;
            if !evidence.is_array() {
                return Err("evidence index must be a JSON array".to_owned());
            }
            evidence
        }
        None => serde_json::json!([]),
    };
    let view = invoke(
        "assurance.build_authored_argument",
        &serde_json::json!({ "argument": argument, "decisions": decisions, "asOf": as_of, "discharge": discharge, "evidence": evidence }),
    )?;
    if !view.outcome.carries_payload() {
        return Ok(view);
    }
    present(
        "assurance.render_authored_argument",
        &view.payload,
        arguments.get_flag("json"),
    )
}

fn derived(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let modules = values(arguments, "module");
    let coverage = invoke(
        "quire.coverage",
        &serde_json::json!({ "scope": repo, "modules": modules }),
    )?;
    if !coverage.outcome.carries_payload() {
        return Ok(coverage);
    }
    let obligations = field(&coverage.payload, "obligations")?;
    let policy = policy(arguments, &obligations)?;
    let head = revision(&repo);
    let store = invoke(
        "evidence.audit_inputs",
        &serde_json::json!({ "repo": repo, "head_commit": (!head.is_empty()).then_some(head.as_str()), "independence_policy": policy }),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(store);
    }
    let catalog = invoke(
        "catalog.methods",
        &if modules.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({ "roots": modules })
        },
    )?;
    if !catalog.outcome.carries_payload() {
        return Ok(catalog);
    }
    let audit = invoke(
        "auditor.audit",
        &serde_json::json!({ "input": {
            "obligations": obligations,
            "bindings": field(&store.payload, "bindings")?, "runs": field(&store.payload, "runs")?,
            "scans": field(&store.payload, "scans")?, "injections": field(&store.payload, "injections")?,
            "mockInspectionSuites": field(&store.payload, "mock_inspection_suites")?,
            "vacuousScanSuites": field(&store.payload, "vacuous_scan_suites")?,
            "independence": field(&store.payload, "independence")?, "catalog": catalog.payload,
            "headCommit": (!head.is_empty()).then_some(head.as_str()), "independencePolicy": policy
        }}),
    )?;
    if !audit.outcome.carries_payload() {
        return Ok(audit);
    }
    let frontmatter = read_frontmatter(std::path::Path::new(&format!("{repo}/spec")))?;
    if !frontmatter.outcome.carries_payload() {
        return Ok(frontmatter);
    }
    let trust = invoke(
        "evidence.trust_assessments",
        &serde_json::json!({ "repo": repo }),
    )?;
    if !trust.outcome.carries_payload() {
        return Ok(trust);
    }
    let mut unreadable = field(&frontmatter.payload, "unreadable")?
        .as_array()
        .cloned()
        .unwrap_or_default();
    unreadable.extend(field(&trust.payload, "unreadable")?.as_array().into_iter().flatten().filter_map(serde_json::Value::as_str).map(|path| serde_json::json!({ "path": path, "reason": "trust decision is unreadable or invalid" })));
    let mut case_request = serde_json::json!({
            "documents": field(&frontmatter.payload, "documents")?, "obligations": field(&coverage.payload, "obligations")?,
        "findings": field(&field(&audit.payload, "report")?, "findings")?,
        "unreadable": unreadable, "producer_trust": field(&trust.payload, "assessments")?,
        "evidence_independence": audit.payload.get("independence")
    });
    if let Some(claim_types) = arguments.get_many::<String>("claim-type") {
        case_request
            .as_object_mut()
            .ok_or_else(|| "assurance case request must be an object".to_owned())?
            .insert(
                "claim_types".to_owned(),
                serde_json::json!(claim_types.cloned().collect::<Vec<_>>()),
            );
    }
    let case = invoke("assurance.build_case", &case_request)?;
    if !case.outcome.carries_payload() {
        return Ok(case);
    }
    present(
        "assurance.render_case",
        &case.payload,
        arguments.get_flag("json"),
    )
}

fn present(operation: &str, value: &serde_json::Value, json: bool) -> Result<Response, String> {
    let rendered = if json {
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
    } else {
        let response = invoke(operation, value)?;
        if !response.outcome.carries_payload() {
            return Ok(response);
        }
        field(&response.payload, "rendered")?
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

fn policy(
    arguments: &ArgMatches,
    obligations: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Some(path) = arguments.get_one::<String>("independence-policy") else {
        return Ok(serde_json::Value::Null);
    };
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let known_obligations = obligations
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    let response = invoke(
        "evidence.parse_policy",
        &serde_json::json!({ "text": text, "known_obligations": known_obligations }),
    )?;
    if !response.outcome.carries_payload() {
        return Err("independence policy was refused".to_owned());
    }
    field(&response.payload, "policy")
}

fn json_file(path: &str, label: &str) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("{label} are not readable JSON: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label} are not readable JSON: {error}"))
}
fn revision(repo: &str) -> String {
    ProcessCommand::new("git")
        .args(["-C", repo, "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or_else(String::new, |head| head.trim().to_owned())
}
fn values(arguments: &ArgMatches, name: &str) -> Vec<String> {
    arguments
        .get_many::<String>(name)
        .map_or_else(Vec::new, |values| values.cloned().collect())
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn field(value: &serde_json::Value, name: &str) -> Result<serde_json::Value, String> {
    value
        .get(name)
        .cloned()
        .ok_or_else(|| format!("core response did not return {name}"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::command;

    /// Trace: FR-040, FR-062
    #[test]
    fn tc_447_742_preserves_the_retained_assurance_grammar() {
        let matches = command()
            .try_get_matches_from([
                "assurance",
                "--repo",
                "other",
                "--module",
                "module",
                "--claim-type",
                "StR",
                "--claim-type",
                "hazard",
                "--independence-policy",
                "policy.json",
                "--json",
            ])
            .expect("the retained derived-view grammar parses");

        assert_eq!(
            matches.get_one::<String>("repo").map(String::as_str),
            Some("other")
        );
        assert_eq!(
            matches
                .get_many::<String>("claim-type")
                .expect("claim types")
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["StR", "hazard"]
        );
        assert!(matches.get_flag("json"));
    }

    /// Trace: FR-047, FR-062
    #[test]
    fn tc_447_743_preserves_the_authored_argument_grammar() {
        let matches = command()
            .try_get_matches_from([
                "assurance",
                "--argument",
                "ARG-001",
                "--decisions",
                "decisions.json",
                "--discharge",
                "discharge.json",
                "--evidence",
                "evidence.json",
                "--as-of",
                "2026-09-14T00:00:00Z",
            ])
            .expect("the retained authored-argument grammar parses");

        assert_eq!(
            matches.get_one::<String>("argument").map(String::as_str),
            Some("ARG-001")
        );
        assert_eq!(
            matches.get_one::<String>("evidence").map(String::as_str),
            Some("evidence.json")
        );
        assert_eq!(
            matches.get_one::<String>("as-of").map(String::as_str),
            Some("2026-09-14T00:00:00Z")
        );
    }
}
