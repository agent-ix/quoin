// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The persisted `quoin config get` and `set` commands.

use clap::{Arg, ArgMatches, Command};
use quoin_config::{
    ConfigService, DoctorStatus, IncidentLog, ProcessEnvironment, QuoinConfig, RuntimeContext,
};
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
        "doctor" => doctor(),
        "edit" => edit(),
        _ => Err("an unknown config command reached dispatch".to_owned()),
    }
}

fn get(key: &str) -> Result<Response, String> {
    let environment = ProcessEnvironment;
    let context = context()?;
    let service = ConfigService::<QuoinConfig>::for_plugin(&environment, &context);
    let mut incidents = IncidentLog::new();
    let value = service
        .get_key(key, &mut incidents)
        .map_err(|error| error.to_string())?;
    let rendered = value.map_or_else(
        || {
            format!(
                "quoin.{key} is unset (file: {})",
                service.file_path().display()
            )
        },
        |value| format!("quoin.{key}: {}", value.as_str().unwrap_or_default()),
    );
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

fn set(key: &str, value: &str) -> Result<Response, String> {
    let environment = ProcessEnvironment;
    let context = context()?;
    let service = ConfigService::<QuoinConfig>::for_plugin(&environment, &context);
    let mut incidents = IncidentLog::new();
    service
        .set_key(key, value, &mut incidents)
        .map_err(|error| error.to_string())?;
    Ok(Response::ok(
        serde_json::json!({ "rendered": format!("Set quoin.{key} in {}.", service.file_path().display()) }),
    ))
}

fn doctor() -> Result<Response, String> {
    let environment = ProcessEnvironment;
    let context = context()?;
    let service = ConfigService::<QuoinConfig>::for_plugin(&environment, &context);
    let entry = service.doctor();
    let (rendered, outcome) = match entry.status {
        DoctorStatus::Valid { .. } => (
            format!("quoin valid ({})", entry.file_path.display()),
            quoin_core::protocol::Outcome::Ok,
        ),
        DoctorStatus::Invalid { issues } => (
            format!(
                "quoin invalid ({})\n{}",
                entry.file_path.display(),
                issues
                    .into_iter()
                    .map(|issue| issue.message)
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            quoin_core::protocol::Outcome::Partial,
        ),
        DoctorStatus::Unregistered => (
            format!("quoin unregistered ({})", entry.file_path.display()),
            quoin_core::protocol::Outcome::Partial,
        ),
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: Vec::new(),
        outcome,
    })
}

fn edit() -> Result<Response, String> {
    let environment = ProcessEnvironment;
    let context = context()?;
    let service = ConfigService::<QuoinConfig>::for_plugin(&environment, &context);
    let path = service.file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create config directory: {error}"))?;
    }
    if !path.exists() {
        std::fs::write(path, "{}\n")
            .map_err(|error| format!("cannot initialize {}: {error}", path.display()))?;
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_owned());
    let status = std::process::Command::new(&editor)
        .arg(path)
        .status()
        .map_err(|error| format!("cannot start {editor}: {error}"))?;
    if !status.success() {
        return Err(format!("editor exited non-zero: {status}"));
    }
    if matches!(service.doctor().status, DoctorStatus::Invalid { .. }) {
        return Err(format!(
            "edited {} is not valid configuration",
            path.display()
        ));
    }
    Ok(Response::ok(
        serde_json::json!({ "rendered": format!("edited {}", path.display()) }),
    ))
}

fn context() -> Result<RuntimeContext, String> {
    let invocation = invocation::current();
    let cwd = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?;
    let context = RuntimeContext::for_cwd(&cwd, invocation.no_project_config());
    Ok(invocation
        .config_root()
        .cloned()
        .map_or(context.clone(), |root| context.with_config_root(root)))
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
    // Service-level path and layering parity are covered by quoin-config.
}
