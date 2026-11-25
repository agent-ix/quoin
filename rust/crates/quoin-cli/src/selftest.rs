// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run the self-test cases embedded in the installed binary (FR-215, PLAT-1639).

use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use quoin_core::protocol::Response;
use std::collections::BTreeSet;
use std::path::{Component, Path};
use std::process::{Command as ProcessCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CASE_TIMEOUT: Duration = Duration::from_secs(60);
const TERMINATION_GRACE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug)]
pub(crate) struct EmbeddedCase {
    id: &'static str,
    files: &'static [EmbeddedCaseFile],
}

#[derive(Clone, Copy, Debug)]
struct EmbeddedCaseFile {
    path: &'static str,
    bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/embedded_selftest.rs"));

/// The concise command description used in root help.
pub(crate) const ABOUT: &str = "Run checks embedded in this quoin binary.";

/// The `quoin selftest` grammar.
pub(crate) fn command() -> Command {
    Command::new("selftest")
        .about(ABOUT)
        .arg(
            Arg::new("list")
                .long("list")
                .help("List embedded self-test case ids")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("run")
                .long("run")
                .help("Run all embedded cases or the selected cases")
                .action(ArgAction::SetTrue),
        )
        .group(
            ArgGroup::new("mode")
                .args(["list", "run"])
                .required(true)
                .multiple(false),
        )
        .arg(
            Arg::new("case")
                .long("case")
                .value_name("ID")
                .help("Select a case to run; may be repeated")
                .action(ArgAction::Append)
                .requires("run"),
        )
        .arg(
            Arg::new("out")
                .long("out")
                .value_name("PATH")
                .help("Write the run record to a file")
                .requires("run"),
        )
}

/// Execute `quoin selftest`.
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_flag("list") {
        return Ok(list());
    }

    let selected = select_cases(arguments)?;
    let _ = selected;
    Err("the self-test runner is not ready".to_owned())
}

fn list() -> Response {
    let rendered = EMBEDDED_CASES
        .iter()
        .map(|case| case.id)
        .collect::<Vec<_>>()
        .join("\n");
    Response::ok(serde_json::json!({
        "rendered": rendered,
        "renderedStream": "stdout",
    }))
}

fn select_cases(arguments: &ArgMatches) -> Result<Vec<&'static EmbeddedCase>, String> {
    let requested = arguments
        .get_many::<String>("case")
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    select_case_ids(EMBEDDED_CASES, &requested)
}

fn select_case_ids<'a>(
    cases: &'a [EmbeddedCase],
    requested: &[String],
) -> Result<Vec<&'a EmbeddedCase>, String> {
    if requested.is_empty() {
        return Ok(cases.iter().collect());
    }

    let available = cases.iter().map(|case| case.id).collect::<BTreeSet<_>>();
    if let Some(unknown) = requested
        .iter()
        .map(String::as_str)
        .find(|id| !available.contains(id))
    {
        return Err(format!("unknown self-test case id {unknown:?}"));
    }

    let requested = requested
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    Ok(cases
        .iter()
        .filter(|case| requested.contains(case.id))
        .collect())
}

fn materialize_case(case: &EmbeddedCase, root: &Path) -> Result<(), String> {
    for file in case.files {
        let relative = Path::new(file.path);
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!(
                "embedded case {:?} has an invalid path {:?}",
                case.id, file.path
            ));
        }
        let destination = root.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("create embedded case directory: {error}"))?;
        }
        std::fs::write(destination, file.bytes)
            .map_err(|error| format!("write embedded case file: {error}"))?;
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct CaseResult {
    id: String,
    status: &'static str,
    criteria: Vec<String>,
    detail: String,
    stderr: String,
}

struct ProcessOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

fn run_case(case: &EmbeddedCase, quoin_binary: &Path, timeout: Duration) -> CaseResult {
    let timeout = timeout.min(CASE_TIMEOUT);
    let criteria = case_criteria(case);
    let case_dir = match tempfile::tempdir() {
        Ok(directory) => directory,
        Err(error) => {
            return case_error(
                case,
                criteria,
                format!("create case directory: {error}"),
                "",
            );
        }
    };
    let home = match tempfile::tempdir() {
        Ok(directory) => directory,
        Err(error) => {
            return case_error(case, criteria, format!("create empty HOME: {error}"), "");
        }
    };
    if let Err(error) = materialize_case(case, case_dir.path()) {
        return case_error(case, criteria, error, "");
    }

    let output = match execute_script(
        &case_dir.path().join("run.sh"),
        quoin_binary,
        case_dir.path(),
        home.path(),
        timeout,
    ) {
        Ok(output) => output,
        Err(error) => return case_error(case, criteria, error, ""),
    };
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.timed_out {
        return case_error(
            case,
            criteria,
            format!("timeout after {} seconds", timeout.as_secs_f64()),
            &stderr,
        );
    }

    let expected_exit = match embedded_file(case, "expected/exit_code")
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::trim)
        .and_then(|value| value.parse::<i32>().ok())
    {
        Some(code) => code,
        None => {
            return case_error(
                case,
                criteria,
                "invalid expected exit code".to_owned(),
                &stderr,
            );
        }
    };
    let mut differences = Vec::new();
    if output.status.code() != Some(expected_exit) {
        differences.push(format!(
            "exit code expected {expected_exit}, got {:?}",
            output.status.code()
        ));
    }
    if embedded_file(case, "expected/stdout") != Some(output.stdout.as_slice()) {
        differences.push("stdout bytes differ".to_owned());
    }
    for file in case.files.iter().filter(|file| {
        file.path
            .strip_prefix("expected/files/")
            .is_some_and(|relative| !relative.is_empty())
    }) {
        let Some(relative) = file.path.strip_prefix("expected/files/") else {
            continue;
        };
        let actual_path = case_dir.path().join(relative);
        match std::fs::read(&actual_path) {
            Ok(actual) if actual == file.bytes => {}
            Ok(_) => differences.push(format!("file {relative:?} bytes differ")),
            Err(error) => differences.push(format!("file {relative:?} is missing: {error}")),
        }
    }
    let detail = differences.join("; ");
    CaseResult {
        id: case.id.to_owned(),
        status: if differences.is_empty() {
            "pass"
        } else {
            "fail"
        },
        criteria,
        detail,
        stderr,
    }
}

fn case_error(
    case: &EmbeddedCase,
    criteria: Vec<String>,
    detail: String,
    stderr: &str,
) -> CaseResult {
    CaseResult {
        id: case.id.to_owned(),
        status: "error",
        criteria,
        detail,
        stderr: stderr.to_owned(),
    }
}

fn case_criteria(case: &EmbeddedCase) -> Vec<String> {
    let mut criteria = BTreeSet::new();
    if let Some(script) = embedded_file(case, "run.sh")
        && let Ok(script) = std::str::from_utf8(script)
    {
        for line in script.lines() {
            if let Some(marker) = line.trim().strip_prefix("# Trace:") {
                criteria.extend(
                    marker
                        .split(',')
                        .map(str::trim)
                        .filter(|criterion| !criterion.is_empty())
                        .map(ToOwned::to_owned),
                );
            }
        }
    }
    criteria.into_iter().collect()
}

fn embedded_file<'a>(case: &'a EmbeddedCase, path: &str) -> Option<&'a [u8]> {
    case.files
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.bytes)
}

fn execute_script(
    script: &Path,
    quoin_binary: &Path,
    case_tmp: &Path,
    home: &Path,
    timeout: Duration,
) -> Result<ProcessOutput, String> {
    let mut command = ProcessCommand::new("/bin/sh");
    command
        .arg(script)
        .current_dir(case_tmp)
        .env_clear()
        .env("CASE_TMP", case_tmp)
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", case_tmp)
        .env("QUOIN", quoin_binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("execute case script: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "case stdout pipe was not created".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "case stderr pipe was not created".to_owned())?;
    let stdout_reader = thread::spawn(move || read_pipe(stdout));
    let stderr_reader = thread::spawn(move || read_pipe(stderr));
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                timed_out = true;
                terminate_process_group(&mut child)?;
                break child
                    .wait()
                    .map_err(|error| format!("reap timed-out case process: {error}"))?;
            }
            Err(error) => {
                terminate_process_group(&mut child)?;
                let _ = child.wait();
                return Err(format!("wait for case process: {error}"));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| "case stdout reader panicked".to_owned())?
        .map_err(|error| format!("read case stdout: {error}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "case stderr reader panicked".to_owned())?
        .map_err(|error| format!("read case stderr: {error}"))?;
    Ok(ProcessOutput {
        status,
        stdout,
        stderr,
        timed_out,
    })
}

fn read_pipe(mut pipe: impl std::io::Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(unix)]
fn terminate_process_group(child: &mut std::process::Child) -> Result<(), String> {
    let pid = i32::try_from(child.id()).map_err(|_| "case process id is invalid".to_owned())?;
    let pid =
        rustix::process::Pid::from_raw(pid).ok_or_else(|| "case process id is zero".to_owned())?;
    let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::TERM);
    let deadline = Instant::now() + TERMINATION_GRACE;
    while Instant::now() < deadline {
        if child
            .try_wait()
            .map_err(|error| format!("wait for terminating case process: {error}"))?
            .is_some()
        {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    Ok(())
}

#[cfg(not(unix))]
fn terminate_process_group(_child: &mut std::process::Child) -> Result<(), String> {
    Err("self-tests require a Unix process-group API".to_owned())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic is the failure report"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-215-AC-6")]
    #[test]
    fn tc_215_case_selection_is_unique_and_follows_embedded_order() {
        let matches = command()
            .try_get_matches_from([
                "selftest",
                "--run",
                "--case",
                EMBEDDED_CASES[0].id,
                "--case",
                EMBEDDED_CASES[0].id,
            ])
            .expect("the repeated case flags parse");
        let selected = select_cases(&matches).expect("the case id is embedded");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, EMBEDDED_CASES[0].id);
    }

    #[trace("FR-215-AC-6")]
    #[test]
    fn tc_215_command_requires_one_mode_and_cases_require_run() {
        assert!(command().try_get_matches_from(["selftest"]).is_err());
        assert!(
            command()
                .try_get_matches_from(["selftest", "--list", "--run"])
                .is_err()
        );
        assert!(
            command()
                .try_get_matches_from(["selftest", "--list", "--case", "selftest-list"])
                .is_err()
        );
        assert!(
            command()
                .try_get_matches_from(["selftest", "--run", "--out", "result.json"])
                .is_ok()
        );
    }

    #[trace("FR-215-AC-6")]
    #[test]
    fn tc_215_unknown_case_is_rejected_before_execution() {
        let matches = command()
            .try_get_matches_from(["selftest", "--run", "--case", "missing-case"])
            .expect("case selection parses before semantic validation");
        let error = select_cases(&matches).expect_err("unknown case ids are rejected");
        assert!(error.contains("missing-case"));
    }

    #[trace("FR-215-AC-6")]
    #[test]
    fn tc_215_list_shows_embedded_case_ids_in_declaration_order() {
        let listed = list();
        assert_eq!(
            listed
                .payload
                .get("rendered")
                .and_then(serde_json::Value::as_str),
            Some("selftest-list")
        );
    }

    #[trace("FR-215-AC-1")]
    #[test]
    fn tc_215_runner_compares_exit_stdout_and_expected_files() {
        let case = EmbeddedCase {
            id: "runner-output",
            files: &[
                EmbeddedCaseFile { path: "run.sh", bytes: b"#!/bin/sh\n# Trace: FR-215-AC-1\nprintf 'hello\\n'\nprintf 'saved\\n' > result.txt\n" },
                EmbeddedCaseFile { path: "expected/exit_code", bytes: b"0\n" },
                EmbeddedCaseFile { path: "expected/stdout", bytes: b"hello\n" },
                EmbeddedCaseFile { path: "expected/files/result.txt", bytes: b"saved\n" },
            ],
        };
        let result = run_case(&case, Path::new("/bin/true"), Duration::from_secs(1));
        assert_eq!(result.status, "pass");
        assert!(result.detail.is_empty());
        assert_eq!(result.criteria, ["FR-215-AC-1"]);
    }

    #[trace("FR-215-AC-7")]
    #[test]
    fn tc_215_timeout_terminates_case_process_group() {
        let root = tempfile::tempdir().expect("a temporary test root");
        let marker = root.path().join("descendant-survived");
        let script = format!(
            "#!/bin/sh\n(sleep 0.4; touch '{}') &\nsleep 5\n",
            marker.display()
        );
        let case_tmp = tempfile::tempdir().expect("a temporary case directory");
        let home = tempfile::tempdir().expect("an empty home directory");
        let script_path = case_tmp.path().join("run.sh");
        std::fs::write(&script_path, script).expect("the timeout script is written");
        let output = execute_script(
            &script_path,
            Path::new("/bin/true"),
            case_tmp.path(),
            home.path(),
            Duration::from_millis(100),
        )
        .expect("the timed-out child is terminated and reaped");
        assert!(output.timed_out);
        thread::sleep(Duration::from_millis(450));
        assert!(!marker.exists(), "the background descendant was terminated");
    }

    #[trace("FR-215-CON-1")]
    #[test]
    fn tc_215_materialization_writes_only_case_relative_paths() {
        let root = tempfile::tempdir().expect("a temporary case directory");
        let case = &EMBEDDED_CASES[0];
        materialize_case(case, root.path()).expect("embedded case files materialize");
        assert_eq!(
            std::fs::read(root.path().join("run.sh")).expect("the script was written"),
            case.files
                .iter()
                .find(|file| file.path == "run.sh")
                .expect("the case contains a script")
                .bytes
        );
    }
}
