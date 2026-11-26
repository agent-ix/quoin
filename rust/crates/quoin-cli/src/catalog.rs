// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The Rust `quoin catalog` command family (quoin#373, Stage 8).
//!
//! The core owns module discovery, catalog projection, and the method merge.
//! This adapter owns only the retained flag grammar and terminal rendering.

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use ix_trace_rs::implements;
use quoin_core::protocol::Response;

use crate::core_runtime::invoke;

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");

/// The `catalog` command and its retained subcommands.
pub(crate) fn command() -> Command {
    Command::new("catalog")
        .about("Read the active specification-module catalog")
        .arg(json_flag())
        .subcommand(
            Command::new("list")
                .about("List active artifact and object catalog modules")
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("methods")
                .about("List the merged verification-method catalog")
                .arg(json_flag())
                .arg(
                    Arg::new("class")
                        .long("class")
                        .value_name("CLASS")
                        .help("Only methods in this verification class"),
                ),
        )
        .subcommand(
            Command::new("show")
                .about("Show one artifact or object catalog type")
                .arg(Arg::new("type").value_name("TYPE").required(true))
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("validate")
                .about(
                    "Validate that active catalogs have no duplicate types or unreachable methods",
                )
                .arg(json_flag()),
        )
}

fn json_flag() -> Arg {
    Arg::new("json")
        .long("json")
        .help("Emit the full result as JSON")
        .action(ArgAction::SetTrue)
}

/// Execute a parsed catalog command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let (subcommand, arguments) = matches.subcommand().unwrap_or(("list", matches));
    ensure_defaults()?;
    match subcommand {
        "list" => list(arguments),
        "methods" => methods(arguments),
        "show" => show(arguments),
        "validate" => validate(arguments),
        _ => Err("an unknown catalog command reached dispatch".to_owned()),
    }
}

fn ensure_defaults() -> Result<(), String> {
    let response = invoke(
        "modules.ensure_defaults",
        &serde_json::json!({ "manifest": DEFAULT_MODULES, "mode": "lazy" }),
    )?;
    if response.outcome.carries_payload() {
        Ok(())
    } else {
        Err("quoin catalog could not reconcile default modules".to_owned())
    }
}

fn list(arguments: &ArgMatches) -> Result<Response, String> {
    let response = invoke("catalog.load", &serde_json::json!({}))?;
    render_catalog(response, arguments.get_flag("json"), |catalog| {
        let modules = catalog["modules"]
            .as_array()
            .ok_or_else(|| "catalog.load did not return modules".to_owned())?;
        modules
            .iter()
            .map(|module| {
                let name = module["name"].as_str().unwrap_or("unknown");
                let version = module["version"].as_str().unwrap_or("unknown");
                let root = module["root"].as_str().unwrap_or("unknown");
                Ok(format!("{name}@{version} {root}"))
            })
            .collect::<Result<Vec<_>, String>>()
            .map(|lines| lines.join("\n"))
    })
}

fn methods(arguments: &ArgMatches) -> Result<Response, String> {
    let mut response = invoke("catalog.methods", &serde_json::json!({}))?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let class = arguments.get_one::<String>("class");
    if let Some(class) = class {
        let methods = response
            .payload
            .get("methods")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "catalog.methods did not return methods".to_owned())?
            .iter()
            .filter(|method| {
                method["class"]
                    .as_str()
                    .is_some_and(|value| value.to_lowercase() == class.to_lowercase())
            })
            .cloned()
            .collect::<Vec<_>>();
        let payload = response
            .payload
            .as_object_mut()
            .ok_or_else(|| "catalog.methods did not return an object".to_owned())?;
        payload.insert("methods".to_owned(), serde_json::Value::Array(methods));
    }
    render_catalog(response, arguments.get_flag("json"), |catalog| {
        let methods = catalog["methods"]
            .as_array()
            .ok_or_else(|| "catalog.methods did not return methods".to_owned())?;
        if methods.is_empty() {
            return Ok(class.map_or_else(
                || "no active module declares a verification_catalog".to_owned(),
                |class| format!("no methods of class `{class}` in the merged catalog"),
            ));
        }
        let mut lines = Vec::new();
        for method in methods {
            let id = method["id"].as_str().unwrap_or("");
            let category = method["class"].as_str().unwrap_or("");
            let kind = method["evidenceKind"]
                .as_str()
                .map_or_else(String::new, |kind| format!(" → {kind}"));
            lines.push(format!("{id}  [{category}]{kind}"));
            lines.push(format!(
                "    {} — {}",
                method["name"].as_str().unwrap_or(""),
                method["definition"].as_str().unwrap_or("").trim()
            ));
            if let Some(rules) = method["applicability"].as_object() {
                for (rule, values) in rules {
                    let values = values.as_array().map_or_else(Vec::new, |items| {
                        items.iter().filter_map(serde_json::Value::as_str).collect()
                    });
                    lines.push(format!("    when {rule}: {}", values.join(", ")));
                }
            }
        }
        lines.push(String::new());
        let classes = methods
            .iter()
            .filter_map(|method| method["class"].as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        lines.push(format!(
            "{} method(s) across {}",
            methods.len(),
            classes.join(", ")
        ));
        if let Some(duplicates) = catalog["duplicates"].as_array() {
            for duplicate in duplicates {
                let id = duplicate["id"].as_str().unwrap_or("");
                let modules = duplicate["modules"]
                    .as_array()
                    .map_or_else(Vec::new, |items| {
                        items.iter().filter_map(serde_json::Value::as_str).collect()
                    });
                lines.push(format!(
                    "  duplicate `{id}` from {} (first wins)",
                    modules.join(", ")
                ));
            }
        }
        Ok(lines.join("\n"))
    })
}

fn show(arguments: &ArgMatches) -> Result<Response, String> {
    let name = arguments
        .get_one::<String>("type")
        .ok_or_else(|| "catalog show requires <type>".to_owned())?;
    let response = invoke("catalog.load", &serde_json::json!({}))?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let entry = response
        .payload
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .and_then(|entries| {
            entries.iter().find(|entry| {
                entry["name"]
                    .as_str()
                    .is_some_and(|candidate| candidate.to_lowercase() == name.to_lowercase())
            })
        })
        .ok_or_else(|| format!("catalog type not found: {name}"))?;
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string_pretty(entry).map_err(|error| error.to_string())?
    } else {
        format!(
            "{} {} from {}\n{}",
            entry["kind"].as_str().unwrap_or(""),
            entry["name"].as_str().unwrap_or(""),
            entry["moduleName"].as_str().unwrap_or(""),
            entry["moduleRoot"].as_str().unwrap_or(""),
        )
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: response.diagnostics,
        outcome: response.outcome,
    })
}

#[implements("FR-031-AC-13", "FR-031-AC-16")]
fn validate(arguments: &ArgMatches) -> Result<Response, String> {
    let response = invoke("catalog.load", &serde_json::json!({}))?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let methods = invoke("catalog.methods", &serde_json::json!({}))?;
    if !methods.outcome.carries_payload() {
        return Ok(methods);
    }
    let catalog: quoin_auditor::MethodCatalog = serde_json::from_value(methods.payload)
        .map_err(|error| format!("catalog.methods returned an invalid method catalog: {error}"))?;
    let unreachable = quoin_auditor::unreachable_characteristics(&catalog);
    let unjustified_methods = quoin_auditor::unjustified_unreachable_methods(&catalog);
    with_lag_warnings(
        response,
        arguments.get_flag("json"),
        crate::module::lag_diagnostics(),
        unreachable,
        unjustified_methods,
    )
}

/// Build the `validate` response from a loaded catalog and the lag check. The
/// check can add diagnostics but never changes the outcome.
#[implements("FR-031-AC-13", "FR-031-AC-16")]
fn with_lag_warnings(
    response: Response,
    json: bool,
    lag: Result<Vec<quoin_core::protocol::Diagnostic>, String>,
    unreachable_characteristics: Vec<quoin_auditor::UnreachableCharacteristic>,
    unjustified_unreachable_methods: Vec<quoin_auditor::UnjustifiedUnreachableMethod>,
) -> Result<Response, String> {
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let duplicates = response
        .payload
        .get("duplicates")
        .cloned()
        .ok_or_else(|| "catalog.load did not return duplicates".to_owned())?;
    let no_duplicates = duplicates.as_array().is_some_and(Vec::is_empty);
    let modules = response
        .payload
        .get("modules")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "catalog.load did not return modules".to_owned())?
        .len();
    let ok = no_duplicates
        && unreachable_characteristics.is_empty()
        && unjustified_unreachable_methods.is_empty();
    let payload = serde_json::json!({
        "ok": ok,
        "duplicates": duplicates,
        "modules": modules,
        "unreachable_characteristics": unreachable_characteristics,
        "unjustified_unreachable_methods": unjustified_unreachable_methods,
    });
    let rendered = if json || !ok {
        serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?
    } else {
        format!("catalog ok ({modules})")
    };
    let mut diagnostics = response.diagnostics;
    diagnostics.extend(crate::module::lag_diagnostics_or_warning(lag));
    Ok(Response {
        payload: serde_json::json!({
            "rendered": rendered,
            "renderedStream": if ok { "stdout" } else { "stderr" },
        }),
        diagnostics,
        outcome: if ok {
            response.outcome
        } else {
            Outcome::Partial
        },
    })
}

fn render_catalog(
    response: Response,
    json: bool,
    text: impl FnOnce(&serde_json::Value) -> Result<String, String>,
) -> Result<Response, String> {
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let rendered = if json {
        serde_json::to_string_pretty(&response.payload).map_err(|error| error.to_string())?
    } else {
        text(&response.payload)?
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: response.diagnostics,
        outcome: response.outcome,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::{command, with_lag_warnings};
    use ix_cli_kit::exit::Outcome;
    use ix_trace_rs::trace;
    use quoin_core::protocol::Response;

    #[trace("FR-212-AC-7")]
    #[test]
    fn a_failed_lag_check_leaves_the_validate_outcome_unchanged() {
        let loaded = || {
            Response::ok(serde_json::json!({
                "duplicates": [],
                "modules": [{"name": "m"}],
            }))
        };
        let clean = with_lag_warnings(loaded(), false, Ok(Vec::new()), Vec::new(), Vec::new())
            .expect("validates");
        let failed = with_lag_warnings(
            loaded(),
            false,
            Err("unreadable".to_owned()),
            Vec::new(),
            Vec::new(),
        )
        .expect("a failed lag check is not an error");
        assert_eq!(failed.outcome, clean.outcome);
        assert_eq!(failed.outcome, Outcome::Ok);
        assert_eq!(failed.payload, clean.payload);
        let [warning] = failed.diagnostics.as_slice() else {
            panic!("expected one warning, got {:?}", failed.diagnostics);
        };
        assert_eq!(warning.code, "QM010_REGISTRY_UNREADABLE");
    }

    #[trace("FR-031-AC-13", "FR-031-AC-16")]
    #[test]
    fn unreachable_method_characteristics_fail_catalog_validation_by_name() {
        let response = Response::ok(serde_json::json!({
            "duplicates": [],
            "modules": [{"name": "active-module"}],
        }));
        let checked = with_lag_warnings(
            response,
            false,
            Ok(Vec::new()),
            vec![quoin_auditor::UnreachableCharacteristic {
                characteristic: "removed-characteristic".to_owned(),
                methods: vec!["obsolete-method".to_owned()],
            }],
            vec![quoin_auditor::UnjustifiedUnreachableMethod {
                method: "unreachable-method".to_owned(),
            }],
        )
        .expect("catalog validation returns its finding");
        assert_eq!(checked.outcome, Outcome::Partial);
        assert_eq!(checked.payload["renderedStream"], "stderr");
        let rendered = checked.payload["rendered"]
            .as_str()
            .expect("validation renders text");
        assert!(rendered.contains("removed-characteristic"), "{rendered}");
        assert!(rendered.contains("obsolete-method"), "{rendered}");
        assert!(rendered.contains("unreachable-method"), "{rendered}");
    }

    #[test]
    fn tc_373_catalog_default_and_named_list_preserve_the_retained_grammar() {
        let default = command()
            .try_get_matches_from(["catalog"])
            .expect("catalog without a subcommand is list");
        assert!(default.subcommand().is_none());
        let named = command()
            .try_get_matches_from(["catalog", "list", "--json"])
            .expect("catalog list parses");
        let (name, arguments) = named.subcommand().expect("named subcommand");
        assert_eq!(name, "list");
        assert!(arguments.get_flag("json"));
    }

    #[test]
    fn tc_373_catalog_methods_and_show_keep_their_filter_and_required_argument() {
        let methods = command()
            .try_get_matches_from(["catalog", "methods", "--class", "Analysis", "--json"])
            .expect("catalog methods parses");
        let (_, arguments) = methods.subcommand().expect("methods subcommand");
        assert_eq!(
            arguments.get_one::<String>("class"),
            Some(&"Analysis".to_owned())
        );
        assert!(arguments.get_flag("json"));
        assert!(command().try_get_matches_from(["catalog", "show"]).is_err());
    }
}
