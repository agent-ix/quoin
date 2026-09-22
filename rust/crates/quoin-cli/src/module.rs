// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The Rust `quoin module` command family (quoin#373, Stage 8).
//!
//! Module discovery, reconciliation, installation, and removal stay in the
//! capability-owning core runtime. This adapter preserves the oclif grammar
//! and presentation while the published shell remains in the staged cutover.

use clap::{Arg, ArgMatches, Command};
use quoin_core::protocol::{Outcome, Response};

use crate::core_runtime::invoke;

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");
const RECORD_KEYS: &[&str] = &[
    "name",
    "source",
    "ref",
    "sha",
    "resolvedPath",
    "targetPath",
    "installedAt",
    "semantic",
];
const SOURCE_KEYS: &[&str] = &[
    "type", "repo", "url", "path", "package", "ref", "sha", "version", "registry",
];
const SEMANTIC_KEYS: &[&str] = &["package", "semanticCore", "exports"];
type NestedOrder = fn(&str) -> Option<&'static [&'static str]>;

/// The `module` command and its retained subcommands.
pub(crate) fn command() -> Command {
    Command::new("module")
        .about("Install and manage specification modules")
        .subcommand(Command::new("list").about("List installed specification modules"))
        .subcommand(
            Command::new("install")
                .about("Install or update a specification module")
                .arg(Arg::new("source").value_name("SOURCE").required(true)),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove an installed specification module")
                .arg(Arg::new("name").value_name("NAME").required(true)),
        )
        .subcommand(
            Command::new("ensure-defaults")
                .about("Idempotently install the default specification module set"),
        )
}

/// Execute a parsed module command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let (subcommand, arguments) = matches.subcommand().unwrap_or(("list", matches));
    match subcommand {
        "list" => list(),
        "install" => install(arguments),
        "remove" => remove(arguments),
        "ensure-defaults" => ensure_defaults(),
        _ => Err("an unknown module command reached dispatch".to_owned()),
    }
}

fn list() -> Result<Response, String> {
    let response = invoke("modules.list", &serde_json::json!({}))?;
    render(response, |payload| {
        let plugins = payload
            .get("modules")
            .ok_or_else(|| "modules.list did not return modules".to_owned())?;
        modules_json(plugins, false)
    })
}

fn install(arguments: &ArgMatches) -> Result<Response, String> {
    let source = required(arguments, "source")?;
    let response = invoke("modules.install", &serde_json::json!({ "source": source }))?;
    render(response, |payload| {
        let module = payload
            .get("module")
            .ok_or_else(|| "modules.install did not return module".to_owned())?;
        registry_record_json(module, 0)
    })
}

fn remove(arguments: &ArgMatches) -> Result<Response, String> {
    let name = required(arguments, "name")?;
    let response = invoke("modules.remove", &serde_json::json!({ "name": name }))?;
    render(response, |_| Ok(format!("removed {name}")))
}

fn ensure_defaults() -> Result<Response, String> {
    let mut ensured = invoke(
        "modules.ensure_defaults",
        &serde_json::json!({ "manifest": DEFAULT_MODULES, "mode": "lazy" }),
    )?;
    if !ensured.outcome.carries_payload() {
        return Ok(ensured);
    }
    let listed = invoke("modules.list", &serde_json::json!({}))?;
    if !listed.outcome.carries_payload() {
        return Ok(listed);
    }
    let plugins = listed
        .payload
        .get("modules")
        .ok_or_else(|| "modules.list did not return modules".to_owned())?;
    let rendered = modules_json(plugins, true)?;
    let outcome = if ensured.outcome == Outcome::Partial || listed.outcome == Outcome::Partial {
        Outcome::Partial
    } else {
        Outcome::Ok
    };
    ensured.diagnostics.extend(listed.diagnostics);
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: ensured.diagnostics,
        outcome,
    })
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

fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("module command requires <{name}>"))
}

/// Render module records in the registry field order retained by the oclif CLI.
///
/// `quoin-core` intentionally canonicalizes every payload, which alphabetizes
/// object keys. The old shell restored this record order before `JSON.stringify`
/// because module scripts consume it, so the adapter does the same at the last
/// presentation boundary without changing core's canonical protocol.
fn modules_json(plugins: &serde_json::Value, ensured: bool) -> Result<String, String> {
    let plugins = plugins
        .as_array()
        .ok_or_else(|| "modules.list returned non-array modules".to_owned())?;
    let mut rendered = String::from("{\n");
    if ensured {
        rendered.push_str("  \"ensured\": true,\n");
    }
    rendered.push_str("  \"plugins\": [");
    if !plugins.is_empty() {
        rendered.push('\n');
        for (index, plugin) in plugins.iter().enumerate() {
            rendered.push_str("    ");
            rendered.push_str(&registry_record_json(plugin, 4)?);
            if index + 1 != plugins.len() {
                rendered.push(',');
            }
            rendered.push('\n');
        }
        rendered.push_str("  ");
    }
    rendered.push_str("]\n}");
    Ok(rendered)
}

fn registry_record_json(record: &serde_json::Value, indent: usize) -> Result<String, String> {
    ordered_object_json(record, RECORD_KEYS, indent, Some(registry_nested))
}

fn registry_nested(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "source" => Some(SOURCE_KEYS),
        "semantic" => Some(SEMANTIC_KEYS),
        _ => None,
    }
}

fn ordered_object_json(
    value: &serde_json::Value,
    preferred: &[&str],
    indent: usize,
    nested: Option<NestedOrder>,
) -> Result<String, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "module record is not an object".to_owned())?;
    let mut keys = preferred
        .iter()
        .filter(|key| object.contains_key(**key))
        .copied()
        .collect::<Vec<_>>();
    keys.extend(
        object
            .keys()
            .filter(|key| !preferred.contains(&key.as_str()))
            .map(String::as_str),
    );
    let mut rendered = String::from("{");
    for (index, key) in keys.iter().enumerate() {
        let item = object
            .get(*key)
            .ok_or_else(|| format!("module record lost `{key}` while rendering"))?;
        let value = if let Some(order) = nested.and_then(|nested| nested(key)) {
            ordered_object_json(item, order, indent + 2, None)?
        } else {
            indent_json(
                &serde_json::to_string_pretty(item).map_err(|error| error.to_string())?,
                indent + 2,
            )
        };
        rendered.push('\n');
        rendered.push_str(&" ".repeat(indent + 2));
        rendered.push_str(&serde_json::to_string(key).map_err(|error| error.to_string())?);
        rendered.push_str(": ");
        rendered.push_str(&value);
        if index + 1 != keys.len() {
            rendered.push(',');
        }
    }
    rendered.push('\n');
    rendered.push_str(&" ".repeat(indent));
    rendered.push('}');
    Ok(rendered)
}

fn indent_json(value: &str, indent: usize) -> String {
    value.replace('\n', &format!("\n{}", " ".repeat(indent)))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::{DEFAULT_MODULES, command, modules_json};

    /// Trace: FR-017, FR-101
    #[test]
    fn tc_373_module_defaults_to_list_and_requires_mutation_arguments() {
        assert!(command().try_get_matches_from(["module"]).is_ok());
        assert!(
            command()
                .try_get_matches_from(["module", "install"])
                .is_err()
        );
        assert!(
            command()
                .try_get_matches_from(["module", "remove"])
                .is_err()
        );
        assert!(
            command()
                .try_get_matches_from(["module", "install", "path:custom"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from(["module", "remove", "custom"])
                .is_ok()
        );
    }

    /// Trace: FR-017, FR-101
    #[test]
    fn tc_373_module_output_keeps_the_registry_field_order() {
        let rendered = modules_json(
            &serde_json::json!([{
                "installedAt": "2026-01-01T00:00:00.000Z",
                "name": "custom",
                "ref": "v1",
                "sha": "abc",
                "source": { "path": "custom", "type": "path" },
                "targetPath": "/modules/custom",
            }]),
            false,
        )
        .expect("module output renders");
        assert_eq!(
            rendered,
            r#"{
  "plugins": [
    {
      "name": "custom",
      "source": {
        "type": "path",
        "path": "custom"
      },
      "ref": "v1",
      "sha": "abc",
      "targetPath": "/modules/custom",
      "installedAt": "2026-01-01T00:00:00.000Z"
    }
  ]
}"#
        );
    }

    /// Trace: FR-017
    /// Provenance: quoin#530
    /// The runtime default set must never reintroduce the pinned module whose
    /// `TestMatrix` body shape and traceability status field disagreed.
    #[test]
    fn tc_530_001_default_process_module_avoids_the_split_status_contract() {
        assert!(!DEFAULT_MODULES.contains("375fc2a9c49c37cce0f43384878b8ede16b162a3"));
    }
}
