// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The Rust `quoin module` command family (quoin#373, Stage 8).
//!
//! Module discovery, reconciliation, installation, and removal stay in the
//! capability-owning core runtime. This adapter preserves the oclif grammar
//! and presentation while the published shell remains in the staged cutover.

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::protocol::Response;

use crate::core_runtime::invoke;

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");
const RECORD_KEYS: &[&str] = &[
    "name",
    "source",
    "ref",
    "resolvedPath",
    "targetPath",
    "installedAt",
];
const SOURCE_KEYS: &[&str] = &[
    "type", "repo", "url", "path", "package", "ref", "version", "registry",
];
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
            Command::new("inspect")
                .about("Show each installed module's semantic contract state and its import locks"),
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
        "inspect" => inspect(),
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

/// Generated-package targets: a module declaring any of these is consumed
/// statically as well as dynamically (FR-124).
const COMPILED_TARGETS: &[&str] = &[
    "rust",
    "typescript",
    "python-pydantic-v2",
    "python-dataclass",
];

/// `quoin module inspect` (FR-124): the installed modules, each with the
/// semantic contract it declares and whether its import locks still hold.
///
/// Composed from `modules.list` and `semantic.read_blocks` over every installed
/// root at once, because an import lock is judged against the other installed
/// modules.
fn inspect() -> Result<Response, String> {
    let listed = invoke("modules.list", &serde_json::json!({}))?;
    if !listed.outcome.carries_payload() {
        return Ok(listed);
    }
    let records = listed
        .payload
        .get("modules")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "modules.list did not return modules".to_owned())?;
    let roots = records
        .iter()
        .map(|record| {
            record
                .get("targetPath")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| "a module record has no targetPath".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let read = invoke(
        "semantic.read_blocks",
        &serde_json::json!({ "roots": roots }),
    )?;
    if !read.outcome.carries_payload() {
        return Ok(read);
    }
    let views = read
        .payload
        .get("modules")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "semantic.read_blocks did not return modules".to_owned())?;

    let mut inspected = Vec::with_capacity(records.len());
    let mut diagnostics = Vec::new();
    for (record, view) in records.iter().zip(views) {
        let (item, failures) = inspect_entry(record, view);
        inspected.push(item);
        diagnostics.extend(failures);
    }
    let rendered = serde_json::to_string_pretty(&serde_json::json!({ "modules": inspected }))
        .map_err(|error| error.to_string())?;
    let outcome = if diagnostics.is_empty() {
        Outcome::Ok
    } else {
        Outcome::Partial
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics,
        outcome,
    })
}

/// One installed module's entry, and the stderr diagnostics for its errors.
fn inspect_entry(
    record: &serde_json::Value,
    view: &serde_json::Value,
) -> (serde_json::Value, Vec<quoin_core::protocol::Diagnostic>) {
    let name = record.get("name").cloned().unwrap_or_default();
    let block = view.get("block");
    let unreadable = view.get("unreadable").and_then(serde_json::Value::as_str);
    let mode = match block {
        None if unreadable.is_some() => "unreadable",
        None => "no-semantic-block",
        Some(block) if declares_compiled_target(block) => "compiled",
        Some(_) => "dynamic",
    };
    let unreadable_finding = unreadable.map(|detail| {
        serde_json::json!({
            "code": "inspect.manifest-unreadable",
            "severity": "error",
            "path": "manifest.yaml",
            "message": detail,
        })
    });
    let problems: Vec<&serde_json::Value> = ["diagnostics", "lock_diagnostics"]
        .iter()
        .filter_map(|key| view.get(*key).and_then(serde_json::Value::as_array))
        .flatten()
        .chain(unreadable_finding.as_ref())
        .collect();
    let failures = problems
        .iter()
        .filter(|p| p.get("severity").and_then(serde_json::Value::as_str) == Some("error"))
        .map(|problem| quoin_core::protocol::Diagnostic {
            code: quoin_modules::ModulesErrorCode::InstalledModuleInvalid
                .as_str()
                .to_owned(),
            message: problem
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            context: std::collections::BTreeMap::from([(
                "module".to_owned(),
                name.as_str().unwrap_or_default().to_owned(),
            )]),
        })
        .collect();
    let mut item = serde_json::Map::new();
    item.insert("name".to_owned(), name);
    item.insert("mode".to_owned(), mode.into());
    item.insert(
        "diagnostics".to_owned(),
        serde_json::Value::Array(problems.into_iter().cloned().collect()),
    );
    for (key, value) in [
        ("ref", record.get("ref")),
        ("version", view.get("version")),
        ("semantic", block),
    ] {
        if let Some(value) = value {
            item.insert(key.to_owned(), value.clone());
        }
    }
    (serde_json::Value::Object(item), failures)
}

fn declares_compiled_target(block: &serde_json::Value) -> bool {
    block
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|targets| {
            targets
                .iter()
                .filter_map(serde_json::Value::as_str)
                .any(|target| COMPILED_TARGETS.contains(&target))
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
    use super::{command, modules_json};

    /// Trace: FR-017
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

    /// Trace: FR-017
    #[test]
    fn tc_373_module_output_keeps_the_registry_field_order() {
        let rendered = modules_json(
            &serde_json::json!([{
                "installedAt": "2026-01-01T00:00:00.000Z",
                "name": "custom",
                "ref": "v1",
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
      "targetPath": "/modules/custom",
      "installedAt": "2026-01-01T00:00:00.000Z"
    }
  ]
}"#
        );
    }
}
