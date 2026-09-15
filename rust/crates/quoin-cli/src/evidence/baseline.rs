// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The retained `quoin evidence baseline` adapter.

use std::fmt::Write as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use super::{json_arg, repo_arg, required, revision};
use crate::core_bridge::invoke;

pub(super) fn command() -> Command {
    Command::new("baseline")
        .about("Accept current findings as the ratchet baseline")
        .arg(repo_arg())
        .arg(Arg::new("module").long("module"))
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue),
        )
        .arg(json_arg())
}

pub(super) fn run(arguments: &ArgMatches) -> Result<Response, String> {
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
    let head = revision(&repo);
    let store = invoke(
        "evidence.audit_inputs",
        &serde_json::json!({ "repo": repo, "head_commit": if head.is_empty() { None } else { Some(head.as_str()) }, "independence_policy": null }),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(store);
    }
    let catalog_request = if modules.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::json!({ "roots": modules })
    };
    let catalog = invoke("catalog.methods", &catalog_request)?;
    if !catalog.outcome.carries_payload() {
        return Ok(catalog);
    }
    let input = serde_json::json!({
        "obligations": coverage.payload.get("obligations").cloned().unwrap_or_else(|| serde_json::json!([])),
        "bindings": store.payload.get("bindings").cloned().unwrap_or_else(|| serde_json::json!([])),
        "runs": store.payload.get("runs").cloned().unwrap_or_else(|| serde_json::json!([])),
        "injections": store.payload.get("injections").cloned().unwrap_or_else(|| serde_json::json!([])),
        "mockInspectionSuites": store.payload.get("mock_inspection_suites").cloned().unwrap_or_else(|| serde_json::json!([])),
        "vacuousScanSuites": store.payload.get("vacuous_scan_suites").cloned().unwrap_or_else(|| serde_json::json!([])),
        "catalog": catalog.payload,
        "headCommit": if head.is_empty() { None } else { Some(head.as_str()) },
    });
    let keys = invoke("auditor.baseline", &serde_json::json!({ "input": input }))?;
    if !keys.outcome.carries_payload() {
        return Ok(keys);
    }
    let accepted = keys
        .payload
        .get("accepted")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let dry_run = arguments.get_flag("dry-run");
    let path = if dry_run {
        None
    } else {
        let wrote = invoke(
            "evidence.write_baseline",
            &serde_json::json!({ "repo": repo, "commit": head, "accepted": accepted }),
        )?;
        if !wrote.outcome.carries_payload() {
            return Ok(wrote);
        }
        wrote
            .payload
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let accepted = accepted
        .as_array()
        .ok_or_else(|| "auditor.baseline did not return accepted".to_owned())?;
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string(
            &serde_json::json!({ "accepted": accepted, "commit": head, "dryRun": dry_run }),
        )
        .map_err(|error| error.to_string())?
    } else {
        let mut by_kind = std::collections::BTreeMap::<&str, usize>::new();
        for key in accepted {
            let key = key
                .as_str()
                .ok_or_else(|| "auditor.baseline returned a non-string accepted key".to_owned())?;
            let kind = key.split_once(':').map_or(key, |(kind, _)| kind);
            *by_kind.entry(kind).or_default() += 1;
        }
        let mut rendered = format!(
            "{} {} finding(s) at {}",
            if dry_run { "would accept" } else { "accepted" },
            accepted.len(),
            if head.is_empty() {
                "an unknown commit"
            } else {
                head.get(..12).unwrap_or(&head)
            },
        );
        for (kind, count) in by_kind {
            write!(rendered, "\n  {kind}: {count}").map_err(|error| error.to_string())?;
        }
        if let Some(path) = path {
            write!(rendered, "\n{path}").map_err(|error| error.to_string())?;
        }
        rendered
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}
