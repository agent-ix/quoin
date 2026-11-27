// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Launch Quoin-owned workflows through the maintained generic ix-flow runner.
//! Definitions live beside installed plugin skills; native validation lives here.

pub(crate) mod provider;

use std::ffi::OsStr;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::{Duration, Instant};

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_trace_rs::implements;
use quoin_core::protocol::Response;
use serde_json::Value;

use crate::{ShellError, invocation};

const FLOWS: [&str; 4] = ["formalize", "review", "to-plan", "design-gate"];
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

#[implements("FR-020-AC-3")]
fn launch(name: &str, arguments: &ArgMatches) -> Result<Response, LaunchFailure> {
    if !FLOWS.contains(&name) {
        return Err("an unknown flow command reached dispatch".to_owned().into());
    }
    let path = skill_path(name, arguments)?;
    let home = absolute_home()?;
    let version =
        execute(ProcessCommand::new(runner_program()).arg("--version")).map_err(|failure| {
            match failure {
                LaunchFailure::Local(message) => LaunchFailure::Local(format!(
                    "{message}; install ix-flow with npm install -g @agent-ix/ix-flow@latest"
                )),
                LaunchFailure::Child(exit, message) => LaunchFailure::Child(
                    exit,
                    format!(
                        "{message}; install ix-flow with npm install -g @agent-ix/ix-flow@latest"
                    ),
                ),
            }
        })?;
    let parsed = semver::Version::parse(version.trim().trim_start_matches('v'))
        .map_err(|_| "ix-flow did not report a supported semantic version".to_owned())?;
    if parsed < semver::Version::new(0, 3, 1) {
        return Err(
            "ix-flow >=0.3.1 is required; run npm install -g @agent-ix/ix-flow@latest"
                .to_owned()
                .into(),
        );
    }
    let host_entry = if name == "design-gate" {
        let host_entry = host_entrypoint()?;
        preflight_host_api(&host_entry)?;
        Some(host_entry)
    } else {
        None
    };
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
    let mut context = serde_json::json!({"id":"quoin-launch-context","repo_root":root_text,"config_root":config_text,"targets":target_text});
    let mut command = runner(&home, "run");
    command.arg(name).arg("--path").arg(path);
    if let Some(id) = arguments.get_one::<String>("id") {
        command.arg("--id").arg(id);
    }
    for target in &targets {
        command.arg("--target").arg(target);
    }
    let started = execute(&mut command)?;
    let result = run_result(&started)?;
    let id = instance_id(&result)?;
    if name == "design-gate" {
        context
            .as_object_mut()
            .ok_or_else(|| LaunchFailure::Local("Quoin context is not an object".to_owned()))?
            .insert("id".to_owned(), Value::String(id.to_owned()));
    }
    let recorded = record_context(name, &home, &repo, id, &context, host_entry.as_deref())?;
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

fn record_context(
    name: &str,
    home: &std::path::Path,
    repo: &std::path::Path,
    id: &str,
    context: &Value,
    host_entry: Option<&std::path::Path>,
) -> Result<String, LaunchFailure> {
    let recorded = if name == "design-gate" {
        let host_entry = host_entry.ok_or_else(|| {
            LaunchFailure::Local("ix-flow host API preflight was skipped".to_owned())
        })?;
        execute_host_seed(home, repo, id, context, host_entry)
    } else {
        execute(
            runner(home, "add-item")
                .arg(id)
                .arg("quoin_context")
                .arg("--item")
                .arg(context.to_string()),
        )
    };
    recorded.map_err(|error| match error {
        LaunchFailure::Child(exit, detail) => LaunchFailure::Child(
            exit,
            format!("run {id} remains at intake: context initialization failed: {detail}"),
        ),
        LaunchFailure::Local(detail) => LaunchFailure::Local(format!(
            "run {id} remains at intake: context initialization failed: {detail}"
        )),
    })
}

fn execute_host_seed(
    home: &std::path::Path,
    repo: &std::path::Path,
    id: &str,
    context: &Value,
    host_entry: &std::path::Path,
) -> Result<String, LaunchFailure> {
    let source = r"import { pathToFileURL } from 'node:url';
const { WorkflowCommandRunner } = await import(pathToFileURL(process.argv[1]).href);
const { runId, item, cwd, stateDir } = JSON.parse(process.argv[2]);
const runner = new WorkflowCommandRunner({
  cwd,
  config: { stateDir, defaultDefinition: 'workflow', output: 'json' },
  actor: { kind: 'service', id: 'quoin-host' },
});
const result = await runner.seedSystemOwnedItem({ id: runId, type: 'quoin_context', item });
if (!result.ok) {
  console.error(JSON.stringify(result));
  process.exitCode = 1;
} else {
  console.log(JSON.stringify(result));
}
";
    let input = serde_json::json!({
        "runId": id,
        "item": context,
        "cwd": repo,
        "stateDir": home.join("flows"),
    });
    let mut command = ProcessCommand::new(node_program());
    command
        .arg("--input-type=module")
        .arg("-e")
        .arg(source)
        .arg(host_entry)
        .arg(input.to_string());
    execute(&mut command)
}

fn host_entrypoint() -> Result<PathBuf, LaunchFailure> {
    let runner = executable_path(runner_program())?
        .canonicalize()
        .map_err(|e| LaunchFailure::Local(format!("could not resolve ix-flow executable: {e}")))?;
    let runner_directory = runner
        .parent()
        .ok_or_else(|| LaunchFailure::Local("ix-flow executable has no parent".to_owned()))?;
    let candidates = [
        runner_directory
            .parent()
            .map(|package| package.join("dist/host.js")),
        runner_directory
            .parent()
            .map(|modules| modules.join("@agent-ix/ix-flow/dist/host.js")),
        Some(runner_directory.join("node_modules/@agent-ix/ix-flow/dist/host.js")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            LaunchFailure::Local(format!(
                "installed ix-flow package has no host API near {}; update ix-flow before starting design-gate",
                runner.display(),
            ))
        })
}

fn preflight_host_api(host_entry: &std::path::Path) -> Result<(), LaunchFailure> {
    let source = r"import { pathToFileURL } from 'node:url';
try {
  const host = await import(pathToFileURL(process.argv[1]).href);
  const runner = host.WorkflowCommandRunner;
  if (typeof runner !== 'function' || typeof runner.prototype?.seedSystemOwnedItem !== 'function') {
    throw new Error('WorkflowCommandRunner.seedSystemOwnedItem is unavailable');
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
";
    let mut command = ProcessCommand::new(node_program());
    command
        .arg("--input-type=module")
        .arg("-e")
        .arg(source)
        .arg(host_entry);
    execute(&mut command).map(|_| ()).map_err(|error| {
        let detail = match error {
            LaunchFailure::Child(_, detail) | LaunchFailure::Local(detail) => detail,
        };
        LaunchFailure::Local(format!(
            "installed ix-flow host API is incompatible ({detail}); update ix-flow before starting design-gate"
        ))
    })
}

#[implements("FR-099-AC-10")]
fn executable_path(program: &str) -> Result<PathBuf, LaunchFailure> {
    let path = std::env::var_os("PATH").ok_or_else(|| {
        LaunchFailure::Local("PATH is unavailable while resolving ix-flow".to_owned())
    })?;
    executable_path_in(program, &path)
}

#[implements("FR-099-AC-10")]
fn run_result(stdout: &str) -> Result<Value, LaunchFailure> {
    if stdout.trim().is_empty() {
        return Err("ix-flow returned invalid JSON: empty response"
            .to_owned()
            .into());
    }
    serde_json::from_str(stdout)
        .map_err(|error| format!("ix-flow returned invalid JSON: {error}").into())
}

#[implements("FR-099-AC-10")]
fn executable_path_in(program: &str, path: &OsStr) -> Result<PathBuf, LaunchFailure> {
    std::env::split_paths(path)
        .map(|directory| directory.join(program))
        .find(|candidate| {
            let Ok(metadata) = candidate.metadata() else {
                return false;
            };
            if !metadata.is_file() {
                return false;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                true
            }
        })
        .ok_or_else(|| {
            LaunchFailure::Local("could not resolve ix-flow executable on PATH".to_owned())
        })
}

#[implements("FR-099-AC-10")]
fn resolve_program(program: &OsStr, command: &ProcessCommand) -> Result<PathBuf, LaunchFailure> {
    let path = std::path::Path::new(program);
    let candidate = if path.components().count() > 1 || path.is_absolute() {
        path.to_path_buf()
    } else {
        let program = path.to_str().ok_or_else(|| {
            LaunchFailure::Local(
                "could not resolve ix-flow executable: program name is not UTF-8".to_owned(),
            )
        })?;
        match command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new("PATH"))
        {
            Some((_, Some(path))) => executable_path_in(program, path)?,
            Some((_, None)) => {
                return Err(LaunchFailure::Local(
                    "PATH is unavailable while resolving ix-flow".to_owned(),
                ));
            }
            None => executable_path(program)?,
        }
    };
    candidate.canonicalize().map_err(|error| {
        LaunchFailure::Local(format!(
            "could not resolve ix-flow executable {}: {error}",
            candidate.display()
        ))
    })
}

fn instance_id(result: &Value) -> Result<&str, String> {
    if result.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(result
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .map_or_else(
                || "ix-flow run failed without an error message".to_owned(),
                str::to_owned,
            ));
    }
    result
        .get("instance_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "ix-flow returned no instance_id".to_owned())
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

#[implements("FR-099-AC-10")]
fn execute(command: &mut ProcessCommand) -> Result<String, LaunchFailure> {
    // Resolve PATH once, then spawn the canonical executable so a later PATH
    // change or symlink swap cannot redirect this child launch.
    let resolved = resolve_program(command.get_program(), command)?;
    let mut resolved_command = ProcessCommand::new(resolved);
    resolved_command.args(command.get_args());
    for (key, value) in command.get_envs() {
        if let Some(value) = value {
            resolved_command.env(key, value);
        } else {
            resolved_command.env_remove(key);
        }
    }
    if let Some(directory) = command.get_current_dir() {
        resolved_command.current_dir(directory);
    }
    *command = resolved_command;
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
        let termination = match status.code() {
            Some(code) => format!("ix-flow exited with status {code}"),
            None => {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    status.signal().map_or_else(
                        || "ix-flow terminated without an exit status".to_owned(),
                        |signal| format!("ix-flow terminated by signal {signal}"),
                    )
                }
                #[cfg(not(unix))]
                {
                    "ix-flow terminated without an exit status".to_owned()
                }
            }
        };
        return Err(LaunchFailure::Child(
            status
                .code()
                .and_then(|c| u8::try_from(c).ok())
                .unwrap_or(1),
            if message.is_empty() {
                termination
            } else {
                format!("{termination}: {message}")
            },
        ));
    }
    String::from_utf8(stdout)
        .map_err(|e| LaunchFailure::Local(format!("ix-flow returned non-UTF-8 output: {e}")))
}

pub(crate) fn command(name: &'static str) -> Command {
    Command::new(name)
        .about("Start a Quoin-owned workflow")
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

fn node_program() -> &'static str {
    if cfg!(windows) { "node.exe" } else { "node" }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions may panic")]
mod tests {
    use super::{
        LaunchFailure, ProcessCommand, command, executable_path_in, execute, instance_id, run,
        run_result, skill_path,
    };
    use ix_trace_rs::trace;

    /// Provenance: PLAT-1249
    #[trace("FR-021-AC-2")]
    #[test]
    fn ix_flow_error_envelope_is_reported_before_missing_run_id() {
        let result = serde_json::json!({
            "ok": false,
            "error": {
                "code": "state_write_failed",
                "message": "permission denied writing workflow state"
            }
        });

        assert_eq!(
            instance_id(&result),
            Err("permission denied writing workflow state".to_owned())
        );
    }

    /// Provenance: PLAT-1249
    #[trace("FR-021-AC-2")]
    #[test]
    fn successful_ix_flow_envelope_still_requires_run_id() {
        let result = serde_json::json!({"ok": true});

        assert_eq!(
            instance_id(&result),
            Err("ix-flow returned no instance_id".to_owned())
        );
    }

    /// Provenance: PLAT-1249
    #[trace("FR-021-AC-2")]
    #[test]
    fn successful_ix_flow_envelope_returns_run_id() {
        let result = serde_json::json!({"ok": true, "instance_id": "run-1"});

        assert_eq!(instance_id(&result), Ok("run-1"));
    }

    #[trace("FR-020-AC-2")]
    #[test]
    fn unknown_workflows_are_refused_before_launch() {
        let arguments = command("matrix")
            .try_get_matches_from(["matrix"])
            .expect("arguments");
        assert!(run("matrix", &arguments).is_err_and(|e| e.message.contains("unknown flow")));
    }

    #[trace("FR-020-AC-4")]
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

    #[trace("FR-099-AC-7")]
    #[test]
    fn workflow_path_argument_must_name_the_skill_and_selected_definition() {
        let directory = tempfile::tempdir().expect("skill directory is created");
        let definition = directory.path().join("workflows/review");
        std::fs::create_dir_all(&definition).expect("workflow directory is created");
        std::fs::write(directory.path().join("SKILL.md"), "# Review skill\n")
            .expect("skill document is written");
        std::fs::write(definition.join("def.yaml"), "name: review\n")
            .expect("workflow definition is written");
        let path_text = directory.path().to_string_lossy().into_owned();
        let arguments = command("review")
            .try_get_matches_from(["review", "--path", &path_text])
            .expect("path argument parses");
        assert_eq!(
            skill_path("review", &arguments).expect("complete skill path is accepted"),
            directory.path().canonicalize().expect("canonical path")
        );

        std::fs::remove_file(definition.join("def.yaml")).expect("definition is removed");
        assert!(skill_path("review", &arguments).is_err_and(|error| {
            error.contains("must contain SKILL.md and workflows/review/def.yaml")
        }));
    }

    #[cfg(unix)]
    #[trace("FR-099-AC-10")]
    #[test]
    fn ix_flow_exit_signal_and_spawn_failures_have_distinct_diagnostics() {
        let exited = execute(&mut ProcessCommand::new("sh").arg("-c").arg("exit 7"));
        assert!(
            matches!(exited, Err(LaunchFailure::Child(7, message)) if message.contains("exited with status 7"))
        );

        let signaled = execute(&mut ProcessCommand::new("sh").arg("-c").arg("kill -TERM $$"));
        assert!(
            matches!(signaled, Err(LaunchFailure::Child(_, message)) if message.contains("terminated by signal 15"))
        );

        let mut missing = ProcessCommand::new("definitely-not-an-ix-flow-executable");
        let spawned = execute(&mut missing);
        assert!(
            matches!(spawned, Err(LaunchFailure::Local(message)) if message.contains("could not resolve ix-flow executable"))
        );

        let directory = tempfile::tempdir().expect("non-executable fixture directory");
        let non_executable = directory.path().join("ix-flow");
        std::fs::write(&non_executable, "not executable").expect("fixture is written");
        let mut permissions = std::fs::metadata(&non_executable)
            .expect("fixture metadata")
            .permissions();
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o600);
        std::fs::set_permissions(&non_executable, permissions).expect("fixture is not executable");
        let spawn_failure = execute(&mut ProcessCommand::new(&non_executable));
        assert!(
            matches!(spawn_failure, Err(LaunchFailure::Local(message)) if message.contains("could not start ix-flow"))
        );
    }

    #[cfg(unix)]
    #[trace("FR-099-AC-10")]
    #[test]
    fn ix_flow_path_lookup_skips_non_executable_candidates() {
        let directory = tempfile::tempdir().expect("runner search roots are created");
        let first = directory.path().join("first");
        let second = directory.path().join("second");
        std::fs::create_dir(&first).expect("first search root is created");
        std::fs::create_dir(&second).expect("second search root is created");

        let shadow = first.join("ix-flow");
        std::fs::write(&shadow, "not executable").expect("shadow candidate is written");
        let mut permissions = std::fs::metadata(&shadow)
            .expect("shadow metadata")
            .permissions();
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o600);
        std::fs::set_permissions(&shadow, permissions).expect("shadow is not executable");

        let runner = second.join("ix-flow");
        std::fs::write(&runner, "#!/bin/sh\nexit 0\n").expect("runner candidate is written");
        let mut permissions = std::fs::metadata(&runner)
            .expect("runner metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&runner, permissions).expect("runner is executable");

        let path = std::env::join_paths([first, second]).expect("PATH is valid");
        let selected = executable_path_in("ix-flow", &path);
        assert!(selected.is_ok(), "later executable is selected");
        assert_eq!(
            selected.ok(),
            Some(runner),
            "the later executable must be selected over the non-executable shadow"
        );
    }

    #[cfg(unix)]
    #[trace("FR-099-AC-10")]
    #[test]
    fn ix_flow_resolution_honors_command_path_override() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("runner directory is created");
        let runner = directory.path().join("ix-flow");
        std::fs::write(&runner, "#!/bin/sh\nprintf selected\n").expect("runner fixture is written");
        std::fs::set_permissions(&runner, std::fs::Permissions::from_mode(0o700))
            .expect("runner fixture is executable");

        let mut command = ProcessCommand::new("ix-flow");
        command.env("PATH", directory.path());
        let selected = execute(&mut command);

        assert_eq!(selected.ok().as_deref(), Some("selected"));
    }

    #[trace("FR-099-AC-10")]
    #[test]
    fn ix_flow_empty_success_response_is_rejected_as_invalid_json() {
        let parsed = run_result(" \n\t");

        assert!(matches!(
            parsed,
            Err(LaunchFailure::Local(message))
                if message == "ix-flow returned invalid JSON: empty response"
        ));
    }
}
