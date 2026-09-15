// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for retained `quoin advise`.

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_bridge::invoke;

pub(crate) fn command() -> Command {
    Command::new("advise")
        .about("Recommend a verification method for each obligation, from the catalog")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("module").long("module"))
        .arg(
            Arg::new("mismatch-only")
                .long("mismatch-only")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("inconclusive-only")
                .long("inconclusive-only")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
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
    let properties = invoke(
        "quire.properties",
        &serde_json::json!({ "scope": repo, "modules": [], "documents": [] }),
    )?;
    if !properties.outcome.carries_payload() {
        return Ok(properties);
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
    let methods = field(&catalog.payload, "methods")?;
    if methods.as_array().is_none_or(Vec::is_empty) {
        return Err("no active module declares a `verification_catalog`, so there is nothing to advise from".to_owned());
    }
    let store = invoke(
        "evidence.audit_inputs",
        &serde_json::json!({ "repo": repo }),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(store);
    }
    let advised = invoke(
        "auditor.advise",
        &serde_json::json!({
            "catalog": catalog.payload,
            "obligations": field(&coverage.payload, "obligations")?,
            "shapes": field(&properties.payload, "shapes")?,
            "bindings": field(&store.payload, "bindings")?,
            "runs": field(&store.payload, "runs")?,
            "diagnostics": field(&coverage.payload, "diagnostics")?,
        }),
    )?;
    if !advised.outcome.carries_payload() {
        return Ok(advised);
    }
    let all = field(&advised.payload, "advice")?;
    let shown = all
        .as_array()
        .ok_or_else(|| "auditor.advise did not return advice".to_owned())?
        .iter()
        .filter(|advice| {
            (!arguments.get_flag("mismatch-only") && !arguments.get_flag("inconclusive-only"))
                || (arguments.get_flag("mismatch-only")
                    && advice
                        .get("mismatch")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false))
                || (arguments.get_flag("inconclusive-only")
                    && advice
                        .get("inconclusive")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false))
        })
        .cloned()
        .collect::<Vec<_>>();
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string(&serde_json::json!({ "advice": shown }))
            .map_err(|error| error.to_string())?
    } else {
        render(&shown, all.as_array().map_or(&[], Vec::as_slice))
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

fn field(value: &serde_json::Value, name: &str) -> Result<serde_json::Value, String> {
    value
        .get(name)
        .cloned()
        .ok_or_else(|| format!("core response did not return {name}"))
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn render(shown: &[serde_json::Value], all: &[serde_json::Value]) -> String {
    let mut lines = shown
        .iter()
        .map(|advice| {
            format!(
                "{}  authored={}  {}{}{}",
                advice
                    .get("obligation")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                advice
                    .get("authored")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("—"),
                if advice
                    .get("inconclusive")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    "inconclusive"
                } else {
                    "recommendations available"
                },
                if advice
                    .get("mismatch")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    "  ⚠ mismatch"
                } else {
                    ""
                },
                if advice
                    .get("uncatalogued")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    "  ⚠ uncatalogued"
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>();
    let count = |field: &str| {
        all.iter()
            .filter(|advice| {
                advice
                    .get(field)
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            })
            .count()
    };
    lines.push(String::new());
    lines.push(format!("{} of {} obligation(s) shown. Of all {}: {} mismatch, {} uncatalogued, {} inconclusive. Recommendations, not verdicts: confirm the method in spec review.", shown.len(), all.len(), all.len(), count("mismatch"), count("uncatalogued"), count("inconclusive")));
    lines.join("\n")
}
