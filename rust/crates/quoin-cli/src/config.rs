// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The persisted `quoin config get` and `set` commands.

use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::invocation;

pub(crate) fn command() -> Command {
    Command::new("config")
        .about("Read and write Quoin configuration")
        .subcommand_required(true)
        .subcommand(Command::new("get").arg(Arg::new("key").required(true)))
        .subcommand(
            Command::new("set")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("value").required(true)),
        )
        .subcommand(Command::new("doctor"))
        .subcommand(Command::new("edit"))
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let (name, arguments) = matches
        .subcommand()
        .ok_or_else(|| "a config subcommand is required".to_owned())?;
    match name {
        "get" => get(&required(arguments, "key")?),
        "set" => set(&required(arguments, "key")?, &required(arguments, "value")?),
        "doctor" => Ok(doctor()),
        "edit" => edit(),
        _ => Err("an unknown config command reached dispatch".to_owned()),
    }
}

fn get(key: &str) -> Result<Response, String> {
    validate_key(key)?;
    let value = std::env::var("QUOIN_ORG")
        .ok()
        .map_or_else(read_org, |value| Ok(Some(value)))?;
    let rendered = value.map_or_else(
        || format!("quoin.{key} is unset (file: {})", path().display()),
        |value| format!("quoin.{key}: {value}"),
    );
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

fn set(key: &str, value: &str) -> Result<Response, String> {
    validate_key(key)?;
    if value.is_empty() {
        return Err("org must be a non-empty string".to_owned());
    }
    let path = path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create config directory: {error}"))?;
    }
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    std::fs::write(&path, format!("org: \"{escaped}\"\n"))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(Response::ok(
        serde_json::json!({ "rendered": format!("Set quoin.org in {}.", path.display()) }),
    ))
}

fn doctor() -> Response {
    let path = path();
    let rendered = match read_org() {
        Ok(_) => format!("quoin valid ({})", path.display()),
        Err(error) => format!("quoin invalid ({})\n{error}", path.display()),
    };
    let outcome = if rendered.starts_with("quoin invalid") {
        quoin_core::protocol::Outcome::Partial
    } else {
        quoin_core::protocol::Outcome::Ok
    };
    Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: Vec::new(),
        outcome,
    }
}

fn edit() -> Result<Response, String> {
    let path = path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create config directory: {error}"))?;
    }
    if !path.exists() {
        std::fs::write(&path, "{}\n")
            .map_err(|error| format!("cannot initialize {}: {error}", path.display()))?;
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_owned());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|error| format!("cannot start {editor}: {error}"))?;
    if !status.success() {
        return Err(format!("editor exited non-zero: {status}"));
    }
    read_org()?;
    Ok(Response::ok(
        serde_json::json!({ "rendered": format!("edited {}", path.display()) }),
    ))
}

fn read_org() -> Result<Option<String>, String> {
    let path = path();
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let parsed = quoin_yaml::from_str(&contents)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    match parsed.get("org") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
        Some(_) => Err(format!(
            "{}: org must be a non-empty string",
            path.display()
        )),
    }
}

fn path() -> PathBuf {
    let invocation_root = invocation::current().config_root().cloned();
    let explicit_root = invocation_root.or_else(|| {
        std::env::var_os("IX_CONFIG_ROOT")
            .filter(|root| !root.is_empty())
            .map(PathBuf::from)
    });
    if let Some(root) = explicit_root {
        return root.join("config.d").join("quoin.yaml");
    }
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    root.join("ix").join("config.d").join("quoin.yaml")
}

fn validate_key(key: &str) -> Result<(), String> {
    if key == "org" {
        Ok(())
    } else {
        Err(format!("unknown quoin config key `{key}`"))
    }
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("{name} is required"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions report failures")]
mod tests {
    use std::path::Path;

    use clap::{Arg, Command};

    use super::path;
    use crate::invocation::{Invocation, with_current};

    /// Trace: FR-027, FR-062
    #[test]
    fn tc_373_config_root_selects_the_retained_user_config_location() {
        let matches = Command::new("quoin")
            .arg(Arg::new("config_root").long("config-root"))
            .try_get_matches_from(["quoin", "--config-root", "/tmp/ix-home"])
            .expect("config root parses");
        with_current(Invocation::from_matches(&matches), || {
            assert_eq!(path(), Path::new("/tmp/ix-home/config.d/quoin.yaml"));
        });
    }
}
