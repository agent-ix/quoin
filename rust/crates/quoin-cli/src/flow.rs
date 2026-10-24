// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Launch Quoin-owned workflows through the maintained generic ix-flow runner.
//! Definitions live beside installed plugin skills; native validation lives here.

pub(crate) mod provider;

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::{Duration, Instant};

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;
use serde_json::Value;

use crate::{ShellError, invocation};

const FLOWS: [&str; 2] = ["review", "to-plan"];
const MAX_OUTPUT: u64 = 1024 * 1024;

pub(crate) fn run(name: &str, arguments: &ArgMatches) -> Result<Response, ShellError> {
    launch(name, arguments).map_err(|error| match error {
        LaunchFailure::Child(exit, message) => ShellError {
            exit,
            message,
            stream: crate::ShellOutput::Stderr,
        },
        LaunchFailure::Local(message) => ShellError::command(message),
    })
}

enum LaunchFailure {
    Local(String),
    Child(u8, String),
}
impl From<String> for LaunchFailure {
    fn from(value: String) -> Self {
        Self::Local(value)
    }
}

fn launch(name: &str, arguments: &ArgMatches) -> Result<Response, LaunchFailure> {
    if !FLOWS.contains(&name) {
        return Err("an unknown flow command reached dispatch".to_owned().into());
    }
    let path = skill_path(name, arguments)?;
    let home = absolute_home()?;
    let version = execute(ProcessCommand::new(runner_program()).arg("--version"))?;
    let parsed = semver::Version::parse(version.trim().trim_start_matches('v'))
        .map_err(|_| "ix-flow did not report a supported semantic version".to_owned())?;
    if parsed < semver::Version::new(0, 3, 1) {
        return Err(
            "ix-flow >=0.3.1 is required; run npm install -g @agent-ix/ix-flow@latest"
                .to_owned()
                .into(),
        );
    }
    let repo = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .map_err(|e| e.to_string())?;
    let targets: Vec<PathBuf> = arguments
        .get_many::<String>("target")
        .into_iter()
        .flatten()
        .map(|p| {
            let path = repo.join(p);
            path.canonicalize()
                .map_err(|e| format!("workflow target {}: {e}", path.display()))
        })
        .collect::<Result<_, _>>()?;
    if targets.len() > 256 {
        return Err("workflow target set exceeds 256 entries".to_owned().into());
    }
    if targets.iter().any(|target| !target.starts_with(&repo)) {
        return Err("workflow targets must belong to the launch repository"
            .to_owned()
            .into());
    }
    let root_text = repo
        .to_str()
        .ok_or_else(|| "workflow repository path is not UTF-8".to_owned())?;
    let config_text = home
        .to_str()
        .ok_or_else(|| "workflow config path is not UTF-8".to_owned())?;
    let target_text = targets
        .iter()
        .map(|path| {
            path.to_str()
                .ok_or_else(|| "workflow target path is not UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let context = serde_json::json!({"id":"quoin-launch-context","repo_root":root_text,"config_root":config_text,"targets":target_text});
    let mut command = runner(&home, "run");
    command.arg(name).arg("--path").arg(path);
    if let Some(id) = arguments.get_one::<String>("id") {
        command.arg("--id").arg(id);
    }
    for target in &targets {
        command.arg("--target").arg(target);
    }
    let started = execute(&mut command)?;
    let result: Value = serde_json::from_str(&started)
        .map_err(|e| format!("ix-flow returned invalid JSON: {e}"))?;
    let id = result
        .get("instance_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "ix-flow returned no instance_id".to_owned())?;
    let recorded = execute(
        runner(&home, "add-item")
            .arg(id)
            .arg("quoin_context")
            .arg("--item")
            .arg(context.to_string()),
    )
    .map_err(|error| match error {
        LaunchFailure::Child(exit, detail) => LaunchFailure::Child(
            exit,
            format!("run {id} remains at intake: context initialization failed: {detail}"),
        ),
        LaunchFailure::Local(detail) => LaunchFailure::Local(format!(
            "run {id} remains at intake: context initialization failed: {detail}"
        )),
    })?;
    let rendered = if arguments.get_flag("json") {
        recorded.trim_end().to_owned()
    } else {
        format!(
            "Started Quoin {name} workflow {id} at intake.\nConfig root: {}\nState directory: {}\nUse ix-flow status {id} --config-root {} --state-dir {} --json",
            home.display(),
            home.join("flows").display(),
            home.display(),
            home.join("flows").display()
        )
    };
    Ok(Response::ok(serde_json::json!({"rendered":rendered})))
}

fn skill_path(name: &str, arguments: &ArgMatches) -> Result<PathBuf, String> {
    let path = arguments
        .get_one::<String>("path")
        .ok_or_else(|| "pass --path <installed-quoin-skill-directory>; install the Quoin agent plugin or use its released source".to_owned())?;
    let path = std::path::Path::new(path)
        .canonicalize()
        .map_err(|e| format!("workflow skill directory {path}: {e}"))?;
    if !path.join("SKILL.md").is_file()
        || !path.join("workflows").join(name).join("def.yaml").is_file()
    {
        return Err(format!(
            "{} must contain SKILL.md and workflows/{name}/def.yaml from the Quoin plugin",
            path.display()
        ));
    }
    Ok(path)
}

fn runner(home: &std::path::Path, operation: &str) -> ProcessCommand {
    let mut command = ProcessCommand::new(runner_program());
    command.arg(operation);
    command
        .arg("--config-root")
        .arg(home)
        .arg("--state-dir")
        .arg(home.join("flows"));
    command
}

fn execute(command: &mut ProcessCommand) -> Result<String, LaunchFailure> {
    let mut child = command
        .arg("--json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start ix-flow: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "ix-flow stdout unavailable".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "ix-flow stderr unavailable".to_owned())?;
    let read = |stream: Box<dyn Read + Send>| {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = stream
                .take(MAX_OUTPUT + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes);
            let _ = sender.send(result);
        });
        receiver
    };
    let out = read(Box::new(stdout));
    let err = read(Box::new(stderr));
    let deadline = Instant::now() + Duration::from_secs(30);
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break (status, false);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            break (child.wait().map_err(|e| e.to_string())?, true);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = out
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| "ix-flow stdout did not close within the launch deadline".to_owned())?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| "ix-flow stderr did not close within the launch deadline".to_owned())?
        .map_err(|e| e.to_string())?;
    if timed_out {
        return Err("ix-flow exceeded the 30 second launch deadline"
            .to_owned()
            .into());
    }
    if u64::try_from(stdout.len()).unwrap_or(u64::MAX) > MAX_OUTPUT
        || u64::try_from(stderr.len()).unwrap_or(u64::MAX) > MAX_OUTPUT
    {
        return Err("ix-flow exceeded its 1 MiB output bound".to_owned().into());
    }
    if !status.success() {
        let message = String::from_utf8_lossy(&stderr);
        let message = if message.trim().is_empty() {
            String::from_utf8_lossy(&stdout).trim().to_owned()
        } else {
            message.trim().to_owned()
        };
        return Err(LaunchFailure::Child(
            status
                .code()
                .and_then(|c| u8::try_from(c).ok())
                .unwrap_or(1),
            if message.is_empty() {
                "ix-flow failed".to_owned()
            } else {
                message
            },
        ));
    }
    String::from_utf8(stdout)
        .map_err(|e| LaunchFailure::Local(format!("ix-flow returned non-UTF-8 output: {e}")))
}

pub(crate) fn command(name: &'static str) -> Command {
    Command::new(name)
        .about("Start the Quoin-owned specification workflow")
        .arg(
            Arg::new("path")
                .long("path")
                .help("Installed Quoin skill directory containing SKILL.md and workflows"),
        )
        .arg(Arg::new("target").long("target").action(ArgAction::Append))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .arg(Arg::new("id").long("id"))
}

fn absolute_home() -> Result<PathBuf, String> {
    let root = invocation::current()
        .config_root()
        .cloned()
        .or_else(|| {
            std::env::var_os("IX_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".ix")))
        .ok_or("cannot determine Quoin config root")?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    root.canonicalize().map_err(|e| e.to_string())
}

fn runner_program() -> &'static str {
    if cfg!(windows) {
        "ix-flow.cmd"
    } else {
        "ix-flow"
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions may panic")]
mod tests {
    use super::{command, run};

    /// Trace: FR-020-AC-2
    #[test]
    fn unknown_workflows_are_refused_before_launch() {
        let arguments = command("matrix")
            .try_get_matches_from(["matrix"])
            .expect("arguments");
        assert!(run("matrix", &arguments).is_err_and(|e| e.message.contains("unknown flow")));
    }

    /// Trace: FR-020-AC-4
    #[test]
    fn missing_plugin_path_has_an_actionable_refusal() {
        let arguments = command("review")
            .try_get_matches_from(["review"])
            .expect("arguments");
        assert!(run("review", &arguments).is_err_and(|e| {
            e.message
                .contains("--path <installed-quoin-skill-directory>")
        }));
    }
}
