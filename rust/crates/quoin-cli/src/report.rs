// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the retained `quoin report` measurement views (quoin#373,
//! Stage 8).
//!
//! The measurement core owns every store read, calculation, and canonical
//! document. This module owns the former oclif grammar and selects the human
//! or JSON presentation route without recreating report logic.

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::{Response, canonical_json};

use crate::core_bridge::invoke;

/// The `report` command and its retained flags.
pub(crate) fn command() -> Command {
    Command::new("report")
        .about("Render QA plans and measurements from the evidence store")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(repeated("portfolio"))
        .arg(repeated("graph-export"))
        .arg(repeated("graph-premises"))
        .arg(repeated("graph-audit"))
        .arg(repeated("changed"))
        .arg(Arg::new("since").long("since"))
        .arg(Arg::new("series").long("series"))
        .arg(
            Arg::new("format")
                .long("format")
                .default_value("human")
                .value_parser(["human", "json"]),
        )
}

fn repeated(name: &'static str) -> Arg {
    Arg::new(name).long(name).action(ArgAction::Append)
}

/// Execute a parsed report command.
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let since = arguments.get_one::<String>("since");
    let series = arguments.get_one::<String>("series");
    let portfolio = values(arguments, "portfolio");
    let graph_exports = values(arguments, "graph-export");
    let graph_premises = values(arguments, "graph-premises");
    let graph_audits = values(arguments, "graph-audit");
    let changed = values(arguments, "changed");
    let graph_selected = !graph_exports.is_empty()
        || !graph_premises.is_empty()
        || !graph_audits.is_empty()
        || !changed.is_empty();
    if since.is_some() && series.is_some() {
        return Err("--since and --series are mutually exclusive".to_owned());
    }
    if graph_selected && portfolio.is_empty() {
        return Err("graph portfolio mappings require --portfolio".to_owned());
    }
    if !portfolio.is_empty() {
        if since.is_some() || series.is_some() {
            return Err("--portfolio cannot be combined with --since or --series".to_owned());
        }
        let request = portfolio_request(
            &portfolio,
            &graph_exports,
            &graph_premises,
            &graph_audits,
            &changed,
            graph_selected,
        )?;
        return present(
            if graph_selected {
                "measurement.build_graph_portfolio"
            } else {
                "measurement.build_portfolio"
            },
            if graph_selected {
                "measurement.render_graph_portfolio"
            } else {
                "measurement.render_portfolio"
            },
            &request,
            json(arguments),
        );
    }
    let repo = required(arguments, "repo")?;
    if let Some(metric) = series {
        let response = invoke(
            "measurement.build_series",
            &serde_json::json!({ "repo": repo, "metric": metric }),
        )?;
        return series_response(response, metric, json(arguments));
    }
    if let Some(before_revision) = since {
        return present(
            "measurement.build_comparison",
            "measurement.render_comparison",
            &serde_json::json!({ "repo": repo, "before_revision": before_revision }),
            json(arguments),
        );
    }
    present(
        "measurement.build_report",
        "measurement.render_report",
        &serde_json::json!({ "repo": repo }),
        json(arguments),
    )
}

/// Shape the core request for a regular or graph-enriched portfolio view.
///
/// `PortfolioRequest` intentionally denies unknown fields. Graph mapping
/// fields consequently belong only on the graph operation's distinct request
/// shape; sending empty graph fields to a regular portfolio was rejected by
/// the core boundary (quoin#373 Stage 8).
fn portfolio_request(
    portfolio: &[String],
    graph_exports: &[String],
    graph_premises: &[String],
    graph_audits: &[String],
    changed: &[String],
    graph_selected: bool,
) -> Result<serde_json::Value, String> {
    if !graph_selected {
        return Ok(serde_json::json!({ "locations": portfolio }));
    }
    Ok(serde_json::json!({
        "locations": portfolio,
        "graph_exports": graph_exports,
        "graph_premises": graph_premises,
        "graph_audits": graph_audits,
        "changed": changed,
        "cwd": std::env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))?,
    }))
}

fn present(
    build: &str,
    render: &str,
    request: &serde_json::Value,
    json: bool,
) -> Result<Response, String> {
    let mut response = invoke(if json { build } else { render }, request)?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let rendered = if json {
        canonical_json(&response.payload)
            .map_err(|error| format!("{build} returned non-canonical JSON: {error}"))?
    } else {
        response
            .payload
            .get("rendered")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{render} returned no rendered report"))?
            .to_owned()
    };
    response.payload = serde_json::json!({ "rendered": rendered });
    Ok(response)
}

fn series_response(mut response: Response, metric: &str, json: bool) -> Result<Response, String> {
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let rendered = if json {
        canonical_json(&response.payload).map_err(|error| {
            format!("measurement.build_series returned non-canonical JSON: {error}")
        })?
    } else {
        render_series(metric, &response.payload)?
    };
    response.payload = serde_json::json!({ "rendered": rendered });
    Ok(response)
}

fn render_series(metric: &str, payload: &serde_json::Value) -> Result<String, String> {
    let rows = payload
        .as_array()
        .ok_or_else(|| "measurement.build_series did not return a series array".to_owned())?;
    let mut lines = vec![format!("# {metric} series"), String::new()];
    if rows.is_empty() {
        lines.push("not_computed: no records".to_owned());
    } else {
        for row in rows {
            let row = row
                .as_object()
                .ok_or_else(|| "series entry is not an object".to_owned())?;
            let observation = row
                .get("observation")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| "series entry has no observation".to_owned())?;
            lines.push(format!(
                "- {} {} {} {} (source {}, tool {} {}, corpus {}, gaps {}, config {})",
                value(row, "timestamp"),
                value(observation, "state"),
                value(observation, "value"),
                value(observation, "unit"),
                value(row, "sourceRevision"),
                value(row, "toolIdentity"),
                value(row, "toolVersion"),
                value(row, "corpusRevision"),
                value(row, "corpusGaps"),
                value(row, "configDigest"),
            ));
        }
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn value(object: &serde_json::Map<String, serde_json::Value>, name: &str) -> String {
    object
        .get(name)
        .map_or_else(|| "undefined".to_owned(), stringify)
}

fn stringify(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_owned(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => "[object Object]".to_owned(),
    }
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
        .ok_or_else(|| format!("{name} is required"))
}

fn json(arguments: &ArgMatches) -> bool {
    arguments
        .get_one::<String>("format")
        .is_some_and(|format| format == "json")
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions report failures"
)]
mod tests {
    use super::{command, portfolio_request};

    /// Trace: FR-062, FR-102
    #[test]
    fn tc_373_report_preserves_the_retained_selection_grammar() {
        assert!(
            command()
                .try_get_matches_from([
                    "report",
                    "--portfolio",
                    "one",
                    "--portfolio",
                    "two",
                    "--graph-export",
                    "one=a.json",
                ])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "report",
                    "--repo",
                    ".",
                    "--series",
                    "finding_recall",
                    "--format",
                    "json"
                ])
                .is_ok()
        );
    }

    /// Trace: FR-062, FR-102
    #[test]
    fn tc_373_plain_portfolio_uses_its_strict_core_request_shape() {
        let portfolio = vec!["one".to_owned(), "two".to_owned()];
        let empty = Vec::new();
        let request = portfolio_request(&portfolio, &empty, &empty, &empty, &empty, false)
            .expect("current directory is available");
        assert_eq!(request, serde_json::json!({ "locations": ["one", "two"] }));
    }

    /// Trace: FR-062, FR-102
    #[test]
    fn tc_373_graph_portfolio_carries_only_the_graph_operation_fields() {
        let portfolio = vec!["one".to_owned()];
        let export = vec!["one=export.json".to_owned()];
        let empty = Vec::new();
        let request = portfolio_request(&portfolio, &export, &empty, &empty, &empty, true)
            .expect("current directory is available");
        assert_eq!(request["locations"], serde_json::json!(["one"]));
        assert_eq!(
            request["graph_exports"],
            serde_json::json!(["one=export.json"])
        );
        assert!(request.get("cwd").is_some());
    }
}
