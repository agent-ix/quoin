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
        render(&shown, all.as_array().map_or(&[], Vec::as_slice))?
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
fn render(shown: &[serde_json::Value], all: &[serde_json::Value]) -> Result<String, String> {
    let mut lines = Vec::with_capacity(shown.len() + 2);
    for advice in shown {
        let inconclusive = advice
            .get("inconclusive")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let conclusion = if inconclusive {
            "inconclusive".to_owned()
        } else {
            format!("recommend: {}", recommendations(advice)?)
        };
        lines.push(format!(
            "{}  authored={}  {}{}{}",
            advice
                .get("obligation")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            advice
                .get("authored")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("—"),
            conclusion,
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
        ));
    }
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
    Ok(lines.join("\n"))
}

/// Render the first three recommendation rows exactly as the retained shell.
fn recommendations(advice: &serde_json::Value) -> Result<String, String> {
    let rows = advice
        .get("recommended")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "auditor.advise entry did not return recommendations".to_owned())?;
    rows.iter()
        .take(3)
        .map(|row| {
            let method = row
                .get("method")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "advice recommendation did not return method".to_owned())?;
            let reasons = row
                .get("reasons")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| "advice recommendation did not return reasons".to_owned())?
                .iter()
                .map(|reason| {
                    reason
                        .get("value")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            "advice recommendation reason did not return value".to_owned()
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("{method} ({})", reasons.join(", ")))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|rows| rows.join("; "))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::render;

    /// Trace: FR-031, FR-062, FR-102
    #[test]
    fn tc_373_advice_human_output_keeps_methods_reasons_and_the_three_row_limit() {
        let rows = serde_json::json!([
            {
                "obligation": "FR-001-AC-1",
                "authored": "Test",
                "inconclusive": false,
                "mismatch": true,
                "uncatalogued": false,
                "recommended": [
                    { "method": "Analysis", "reasons": [{ "value": "characteristic" }, { "value": "property" }] },
                    { "method": "Test", "reasons": [{ "value": "risk" }] },
                    { "method": "Inspection", "reasons": [{ "value": "review" }] },
                    { "method": "Demonstration", "reasons": [{ "value": "ignored" }] }
                ]
            },
            {
                "obligation": "NFR-001",
                "authored": null,
                "inconclusive": true,
                "mismatch": false,
                "uncatalogued": true,
                "recommended": []
            }
        ]);
        let rows = rows.as_array().expect("fixture is an advice array");

        assert_eq!(
            render(rows, rows).expect("valid advice renders"),
            "FR-001-AC-1  authored=Test  recommend: Analysis (characteristic, property); Test (risk); Inspection (review)  ⚠ mismatch\nNFR-001  authored=—  inconclusive  ⚠ uncatalogued\n\n2 of 2 obligation(s) shown. Of all 2: 1 mismatch, 1 uncatalogued, 1 inconclusive. Recommendations, not verdicts: confirm the method in spec review."
        );
    }
}
