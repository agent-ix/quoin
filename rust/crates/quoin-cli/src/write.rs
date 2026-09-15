// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for retained `quoin write` authoring packs.

use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::core_runtime::invoke;
use crate::invocation;

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");

pub(crate) fn command() -> Command {
    Command::new("write")
        .about("Build an authoring pack for specification files")
        .arg(Arg::new("repo_dir"))
        .arg(Arg::new("types").long("types").action(ArgAction::Append))
        .arg(Arg::new("org").long("org"))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = arguments
        .get_one::<String>("repo_dir")
        .ok_or_else(|| "write requires <repo_dir>".to_owned())?;
    let root = absolute(repo)?;
    if !root.is_dir() {
        return Err(format!("write repo_dir is not a directory: {repo}"));
    }
    let types = types(arguments);
    if types.is_empty() {
        return Err("write requires --types <type[,type...]>".to_owned());
    }
    let ensured = invoke(
        "modules.ensure_defaults",
        &serde_json::json!({ "manifest": DEFAULT_MODULES, "mode": "lazy" }),
    )?;
    if !ensured.outcome.carries_payload() {
        return Ok(ensured);
    }
    let catalog = invoke("catalog.load", &serde_json::json!({}))?;
    if !catalog.outcome.carries_payload() {
        return Ok(catalog);
    }
    let entries = catalog
        .payload
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "catalog.load did not return entries".to_owned())?;
    let contracts = types
        .iter()
        .map(|name| {
            entries
                .iter()
                .find(|entry| {
                    entry
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
                })
                .cloned()
                .ok_or_else(|| available(name, entries))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let org = resolve_org(arguments.get_one::<String>("org"), &root)?;
    let mut pack = serde_json::Map::new();
    pack.insert("repoRoot".to_owned(), serde_json::json!(root));
    if let Some(org) = org.payload.get("org").and_then(serde_json::Value::as_str) {
        pack.insert("org".to_owned(), serde_json::json!(org));
    }
    pack.insert(
        "orgSource".to_owned(),
        org.payload.get("source").cloned().unwrap_or_default(),
    );
    pack.insert(
        "types".to_owned(),
        serde_json::json!(contracts.iter().map(contract).collect::<Vec<_>>()),
    );
    pack.insert("validation".to_owned(), serde_json::json!({ "command": format!("quire validate --scope {} \"spec/**/*.md\"", quote(&root.to_string_lossy())), "scope": root, "globs": ["spec/**/*.md"] }));
    let pack = serde_json::Value::Object(pack);
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string_pretty(&pack).map_err(|error| error.to_string())?
    } else {
        render(&pack, &contracts)
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: org.diagnostics,
        outcome: org.outcome,
    })
}

fn resolve_org(flag: Option<&String>, root: &Path) -> Result<Response, String> {
    let invocation = invocation::current();
    let configured_root = invocation.config_root().cloned().or_else(|| {
        std::env::var_os("IX_CONFIG_ROOT")
            .filter(|root| !root.is_empty())
            .map(PathBuf::from)
    });
    let default_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|root| root.join("ix"));
    let user = configured_root
        .or(default_root)
        .map(|root| root.join("config.d/quoin.yaml"))
        .and_then(|path| read_if_present(&path));
    let project = (!invocation.no_project_config())
        .then(|| root.join(".ix/config.d/quoin.yaml"))
        .and_then(|path| read_if_present(&path));
    let git = git_dir(root)
        .map(|directory| directory.join("config"))
        .and_then(|path| read_if_present(&path));
    invoke(
        "config.resolve_org",
        &serde_json::json!({ "flag": flag, "env": org_environment(std::env::var("QUOIN_ORG").ok()), "user_config": user, "project_config": project, "git_config": git }),
    )
}

/// Build the declared environment object consumed by `config.resolve_org`.
///
/// The core request schema deliberately requires a map. `Value::default()` is
/// JSON `null`, so an unset `QUOIN_ORG` must become `{}` rather than the
/// default JSON value.
fn org_environment(org: Option<String>) -> serde_json::Value {
    org.map_or_else(
        || serde_json::json!({}),
        |value| serde_json::json!({ "QUOIN_ORG": value }),
    )
}

fn read_if_present(path: &Path) -> Option<String> {
    std::fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file() && metadata.len() <= 1024 * 1024)
        .and_then(|_| std::fs::read_to_string(path).ok())
}
fn git_dir(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let pointer_document = std::fs::read_to_string(&dot_git).ok()?;
    let pointer = pointer_document
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:").map(str::trim))?;
    let directory = root.join(pointer);
    let common = read_if_present(&directory.join("commondir"));
    Some(common.map_or(directory.clone(), |common| directory.join(common.trim())))
}
fn contract(entry: &serde_json::Value) -> serde_json::Value {
    let mut contract = serde_json::Map::new();
    for key in [
        "name",
        "kind",
        "moduleName",
        "moduleRoot",
        "schemaPath",
        "skeletonPath",
    ] {
        if let Some(value) = entry.get(key).filter(|value| !value.is_null()) {
            contract.insert(key.to_owned(), value.clone());
        }
    }
    serde_json::Value::Object(contract)
}
fn render(pack: &serde_json::Value, contracts: &[serde_json::Value]) -> String {
    let org = pack
        .get("org")
        .and_then(serde_json::Value::as_str)
        .map_or_else(
            || "Org: unresolved".to_owned(),
            |org| {
                format!(
                    "Org: {org} (from {})",
                    source_label(
                        pack.get("orgSource")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                    )
                )
            },
        );
    let mut lines = vec![
        "quoin write".to_owned(),
        String::new(),
        format!(
            "Repo: {}",
            pack.get("repoRoot")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
        ),
        org,
        String::new(),
        "Authoring contracts:".to_owned(),
    ];
    for entry in contracts {
        lines.push(format!(
            "- {} ({})",
            entry
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            entry
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
        ));
        lines.push(format!(
            "  module: {}",
            entry
                .get("moduleName")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
        ));
        lines.push(format!(
            "  module_root: {}",
            entry
                .get("moduleRoot")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
        ));
        if let Some(path) = entry
            .get("skeletonPath")
            .and_then(serde_json::Value::as_str)
        {
            lines.push(format!("  skeleton: {path}"));
        }
        if let Some(path) = entry.get("schemaPath").and_then(serde_json::Value::as_str) {
            lines.push(format!("  schema: {path}"));
        }
        if entry.get("skeletonPath").is_none() && entry.get("schemaPath").is_none() {
            lines.push("  contract: manifest only".to_owned());
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "Validate command: {}",
        pack.get("validation")
            .and_then(|value| value.get("command"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    ));
    lines.join("\n")
}
fn source_label(source: &str) -> &str {
    match source {
        "flag" => "--org",
        "env" => "QUOIN_ORG",
        "config" => "stored config",
        "git" => "git remote",
        _ => "nothing",
    }
}
fn available(name: &str, entries: &[serde_json::Value]) -> String {
    format!(
        "catalog type not found: {name}\nAvailable types: {}",
        entries
            .iter()
            .filter_map(|entry| entry.get("name").and_then(serde_json::Value::as_str))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
fn types(arguments: &ArgMatches) -> Vec<String> {
    arguments
        .get_many::<String>("types")
        .into_iter()
        .flatten()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}
fn absolute(path: &str) -> Result<PathBuf, String> {
    if Path::new(path).is_absolute() {
        Ok(PathBuf::from(path))
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| error.to_string())
    }
}
fn quote(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_./:-".contains(&byte))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::{command, org_environment, quote, types};

    /// Trace: FR-025, FR-062
    #[test]
    fn tc_373_743_write_preserves_positional_and_repeatable_type_grammar() {
        let matches = command()
            .try_get_matches_from([
                "write",
                "repo",
                "--types",
                "FR,domain",
                "--types",
                "entity",
                "--org",
                "acme",
                "--json",
            ])
            .expect("grammar parses");
        assert_eq!(
            matches.get_one::<String>("repo_dir").map(String::as_str),
            Some("repo")
        );
        assert_eq!(types(&matches), ["FR", "domain", "entity"]);
        assert_eq!(
            matches.get_one::<String>("org").map(String::as_str),
            Some("acme")
        );
        assert!(matches.get_flag("json"));
    }

    /// Trace: FR-025
    #[test]
    fn tc_373_744_quotes_validation_scopes_for_a_posix_shell() {
        assert_eq!(quote("/tmp/my repo"), "'/tmp/my repo'");
        assert_eq!(quote("/tmp/plain"), "/tmp/plain");
    }

    /// Regression: quoin#525.  The normal, unset environment must still cross
    /// the core boundary as the required object, not JSON `null`.
    #[test]
    fn tc_525_001_unset_org_environment_is_an_empty_object() {
        assert_eq!(org_environment(None), serde_json::json!({}));
        assert_eq!(
            org_environment(Some("agent-ix".to_owned())),
            serde_json::json!({ "QUOIN_ORG": "agent-ix" })
        );
    }
}
