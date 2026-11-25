// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run the self-test cases embedded in the installed binary (FR-235, PLAT-1639).

use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::Response;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
#[cfg(target_os = "linux")]
use std::sync::OnceLock;
#[cfg(target_os = "linux")]
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const CASE_TIMEOUT: Duration = Duration::from_secs(60);
const TERMINATION_GRACE: Duration = Duration::from_millis(100);

#[cfg(target_os = "linux")]
static PROCESS_SUPERVISION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(target_os = "linux")]
struct SubreaperGuard {
    previous: Option<rustix::process::Pid>,
    _lock: MutexGuard<'static, ()>,
}

#[cfg(target_os = "linux")]
impl SubreaperGuard {
    fn acquire() -> Result<Self, String> {
        let lock = PROCESS_SUPERVISION_LOCK.get_or_init(|| Mutex::new(()));
        let lock = lock
            .lock()
            .map_err(|_| "case process supervision lock is poisoned".to_owned())?;
        let previous = rustix::process::child_subreaper()
            .map_err(|error| format!("read child-subreaper setting: {error}"))?;
        if previous.is_none() {
            let enabled = rustix::process::Pid::from_raw(1)
                .ok_or_else(|| "invalid child-subreaper setting".to_owned())?;
            rustix::process::set_child_subreaper(Some(enabled))
                .map_err(|error| format!("enable child-subreaper cleanup: {error}"))?;
        }
        Ok(Self {
            previous,
            _lock: lock,
        })
    }
}

#[cfg(target_os = "linux")]
impl Drop for SubreaperGuard {
    fn drop(&mut self) {
        let _ = rustix::process::set_child_subreaper(self.previous);
    }
}

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
                .requires("run")
                .conflicts_with("list"),
        )
        .arg(
            Arg::new("out")
                .long("out")
                .value_name("PATH")
                .help("Write the run record to a file")
                .requires("run")
                .conflicts_with("list"),
        )
}

/// Execute `quoin selftest`.
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_flag("list") {
        return Ok(list());
    }

    ensure_supported_platform(std::env::consts::OS)?;

    let selected = select_cases(arguments)?;
    let working_directory =
        std::env::current_dir().map_err(|error| format!("resolve current directory: {error}"))?;
    let evidence_store = find_evidence_store(&working_directory);
    let requested_output = arguments.get_one::<String>("out").map(PathBuf::from);
    require_retained_destination(evidence_store.as_deref(), requested_output.as_deref())?;

    let binary = std::env::current_exe()
        .map_err(|error| format!("resolve running quoin binary: {error}"))?;
    let tool_version = version_output(&binary)?;
    let results = selected
        .iter()
        .map(|case| run_case(case, &binary, CASE_TIMEOUT))
        .collect::<Vec<_>>();
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("format self-test timestamp: {error}"))?;

    let store_path = evidence_store.map(|root| record_path(&root, &timestamp));
    let explicit_output = requested_output
        .as_deref()
        .map(|path| absolute_path(&working_directory, path));
    let retained_location = store_path
        .as_deref()
        .or(explicit_output.as_deref())
        .ok_or_else(|| "self-test run record destination was not selected".to_owned())?;
    let retained_location = retained_location
        .to_str()
        .ok_or_else(|| "self-test run record destination is not valid UTF-8".to_owned())?;
    let record = run_record(tool_version, &timestamp, results, retained_location);
    let mut bytes = serde_json::to_string_pretty(&record)
        .map_err(|error| format!("serialize self-test run record: {error}"))?
        .into_bytes();
    bytes.push(b'\n');

    let stdout_record =
        retain_run_record(&bytes, store_path.as_deref(), explicit_output.as_deref())?;

    let failed = record["results"]
        .as_array()
        .is_some_and(|results| results.iter().any(|result| result["status"] != "pass"));
    let rendered = stdout_record.unwrap_or_default();
    let payload = serde_json::json!({
        "rendered": rendered.trim_end_matches('\n'),
        "renderedStream": "stdout",
    });
    if failed {
        Ok(Response::partial(
            payload,
            &CoreError::new(
                CoreErrorCode::Rejected,
                "one or more self-test cases failed or errored",
            ),
        ))
    } else {
        Ok(Response::ok(payload))
    }
}

fn run_record(
    tool_version: String,
    timestamp: &str,
    results: Vec<CaseResult>,
    retained_location: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "tool": "quoin",
        "toolVersion": tool_version,
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "suite": "SELFTEST",
        "timestamp": timestamp,
        "results": results,
        "retainedLocation": retained_location,
        "validationInputs": crate::validation::embedded_inputs_value(),
    })
}

fn ensure_supported_platform(os: &str) -> Result<(), String> {
    match os {
        "linux" => Ok(()),
        other => Err(format!(
            "unsupported-platform: quoin self-tests run on Linux only; current platform is {other}"
        )),
    }
}

fn find_evidence_store(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|ancestor| {
        let evidence = ancestor.join("spec/evidence");
        evidence.is_dir().then_some(evidence)
    })
}

fn require_retained_destination(
    evidence_store: Option<&Path>,
    requested_output: Option<&Path>,
) -> Result<(), String> {
    if evidence_store.is_none() && requested_output.is_none() {
        return Err(
            "no repository evidence store; provide --out <path> to retain the run record"
                .to_owned(),
        );
    }
    Ok(())
}

fn record_path(evidence_store: &Path, timestamp: &str) -> PathBuf {
    let key = timestamp
        .bytes()
        .filter(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        .map(char::from)
        .collect::<String>();
    evidence_store
        .join("runs")
        .join("SELFTEST")
        .join(format!("{key}.json"))
}

fn absolute_path(working_directory: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        working_directory.join(path)
    }
}

fn write_record(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("run record path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("create run record directory {}: {error}", parent.display()))?;
    std::fs::write(path, bytes)
        .map_err(|error| format!("write run record {}: {error}", path.display()))
}

fn retain_run_record(
    bytes: &[u8],
    store_path: Option<&Path>,
    explicit_output: Option<&Path>,
) -> Result<Option<String>, String> {
    if let Some(path) = store_path {
        write_record(path, bytes)?;
    }
    if let Some(path) = explicit_output
        && store_path != Some(path)
    {
        write_record(path, bytes)?;
    }
    if explicit_output.is_some() {
        return Ok(None);
    }
    String::from_utf8(bytes.to_vec())
        .map(Some)
        .map_err(|error| format!("canonical run record is not UTF-8: {error}"))
}

fn version_output(binary: &Path) -> Result<String, String> {
    let output = ProcessCommand::new(binary)
        .arg("--version")
        .output()
        .map_err(|error| format!("read quoin version: {error}"))?;
    if !output.status.success() {
        return Err(format!("quoin --version exited with {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("quoin version is not UTF-8: {error}"))
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
    let actual_exit = output.status.code();
    if actual_exit != Some(expected_exit) {
        differences.push(format!(
            "exit code expected {expected_exit}, got {:?}",
            actual_exit
        ));
    }
    if actual_exit.is_some_and(|code| code != 0) {
        differences.push(format!("case exited non-zero with {actual_exit:?}"));
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
    #[cfg(target_os = "linux")]
    let _subreaper = SubreaperGuard::acquire()?;
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
    let mut status = None;
    loop {
        if status.is_none() {
            match child.try_wait() {
                Ok(result) => status = result,
                Err(error) => {
                    terminate_process_group(&mut child)?;
                    let _ = child.wait();
                    return Err(format!("wait for case process: {error}"));
                }
            }
        }
        if status.is_some() && stdout_reader.is_finished() && stderr_reader.is_finished() {
            break;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            terminate_process_group(&mut child)?;
            status = Some(
                child
                    .wait()
                    .map_err(|error| format!("reap timed-out case process: {error}"))?,
            );
            #[cfg(target_os = "linux")]
            terminate_adopted_descendants()?;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let status = status.ok_or_else(|| "case process exited without a status".to_owned())?;
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

#[cfg(target_os = "linux")]
fn terminate_adopted_descendants() -> Result<(), String> {
    loop {
        let tasks = std::fs::read_dir("/proc/self/task")
            .map_err(|error| format!("list case supervisor threads: {error}"))?;
        let mut children = BTreeSet::new();
        for task in tasks {
            let task = task.map_err(|error| format!("read case supervisor thread: {error}"))?;
            let path = task.path().join("children");
            let contents = match std::fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(format!("read adopted case processes: {error}")),
            };
            for child in contents.split_whitespace() {
                let pid = child
                    .parse::<i32>()
                    .map_err(|error| format!("parse adopted case process id: {error}"))?;
                children.insert(pid);
            }
        }
        if children.is_empty() {
            return Ok(());
        }

        for child in &children {
            if let Some(pid) = rustix::process::Pid::from_raw(*child) {
                let _ = rustix::process::kill_process(pid, rustix::process::Signal::KILL);
            }
        }
        for child in children {
            if let Some(pid) = rustix::process::Pid::from_raw(child) {
                rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::empty())
                    .map_err(|error| format!("reap adopted case process: {error}"))?;
            }
        }
    }
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
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => break,
            Ok(None) => thread::sleep(Duration::from_millis(5)),
        }
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

    #[trace("FR-235-AC-6")]
    #[test]
    fn tc_229_case_selection_is_unique_and_follows_embedded_order() {
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

    #[trace("FR-235-AC-6")]
    #[test]
    fn tc_229_command_requires_one_mode_and_cases_require_run() {
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

    #[trace("FR-235-AC-10")]
    #[test]
    fn tc_229_runner_fails_closed_on_unsupported_platforms() {
        assert!(ensure_supported_platform("linux").is_ok());
        let macos_error = ensure_supported_platform("macos").expect_err("macOS is unsupported");
        assert!(macos_error.contains("unsupported-platform"));
        let error = ensure_supported_platform("windows").expect_err("Windows is unsupported");
        assert!(error.contains("unsupported-platform"));
    }

    #[trace("FR-235-AC-6")]
    #[test]
    fn tc_229_unknown_case_is_rejected_before_execution() {
        let matches = command()
            .try_get_matches_from(["selftest", "--run", "--case", "missing-case"])
            .expect("case selection parses before semantic validation");
        let error = select_cases(&matches).expect_err("unknown case ids are rejected");
        assert!(error.contains("missing-case"));
    }

    #[trace("FR-235-AC-6")]
    #[test]
    fn tc_229_list_shows_embedded_case_ids_in_declaration_order() {
        let listed = list();
        assert_eq!(
            listed
                .payload
                .get("rendered")
                .and_then(serde_json::Value::as_str),
            Some("selftest-list")
        );
    }

    #[trace("FR-236-AC-7")]
    #[test]
    fn tc_230_stderr_is_json_safe_and_does_not_affect_case_status() {
        let first = EmbeddedCase {
            id: "runner-stderr-a",
            files: &[
                EmbeddedCaseFile {
                    path: "run.sh",
                    bytes: b"#!/bin/sh\n# Trace: FR-236-AC-7\nprintf 'hello\\n'\nprintf 'first\\377\\n' >&2\nprintf 'saved\\n' > result.txt\n",
                },
                EmbeddedCaseFile {
                    path: "expected/exit_code",
                    bytes: b"0\n",
                },
                EmbeddedCaseFile {
                    path: "expected/stdout",
                    bytes: b"hello\n",
                },
                EmbeddedCaseFile {
                    path: "expected/files/result.txt",
                    bytes: b"saved\n",
                },
            ],
        };
        let second = EmbeddedCase {
            id: "runner-stderr-b",
            files: &[
                EmbeddedCaseFile {
                    path: "run.sh",
                    bytes: b"#!/bin/sh\n# Trace: FR-236-AC-7\nprintf 'hello\\n'\nprintf 'second\\n' >&2\nprintf 'saved\\n' > result.txt\n",
                },
                EmbeddedCaseFile {
                    path: "expected/exit_code",
                    bytes: b"0\n",
                },
                EmbeddedCaseFile {
                    path: "expected/stdout",
                    bytes: b"hello\n",
                },
                EmbeddedCaseFile {
                    path: "expected/files/result.txt",
                    bytes: b"saved\n",
                },
            ],
        };
        let first_result = run_case(&first, Path::new("/bin/true"), Duration::from_secs(1));
        let second_result = run_case(&second, Path::new("/bin/true"), Duration::from_secs(1));
        assert_eq!(first_result.status, "pass");
        assert_eq!(second_result.status, first_result.status);
        assert_eq!(first_result.stderr, "first�\n");
        assert_eq!(second_result.stderr, "second\n");
        let json = serde_json::to_string(&first_result).expect("lossy stderr is JSON-safe");
        assert!(json.contains("first�\\n"));
    }

    #[trace("FR-235-AC-2")]
    #[test]
    fn tc_229_stdout_mismatch_names_case_and_trace() {
        let case = EmbeddedCase {
            id: "stdout-mismatch",
            files: &[
                EmbeddedCaseFile {
                    path: "run.sh",
                    bytes: b"#!/bin/sh\n# Trace: FR-235-AC-2\nprintf 'hello\\n'\n",
                },
                EmbeddedCaseFile {
                    path: "expected/exit_code",
                    bytes: b"0\n",
                },
                EmbeddedCaseFile {
                    path: "expected/stdout",
                    bytes: b"hellO\n",
                },
            ],
        };
        let result = run_case(&case, Path::new("/bin/true"), Duration::from_secs(1));
        assert_eq!(result.id, "stdout-mismatch");
        assert_eq!(result.status, "fail");
        assert_eq!(result.criteria, ["FR-235-AC-2"]);
        assert!(result.detail.contains("stdout bytes differ"));
    }

    #[trace("FR-235-AC-3")]
    #[test]
    fn tc_229_nonzero_exit_fails_even_when_expected() {
        let case = EmbeddedCase {
            id: "nonzero-exit",
            files: &[
                EmbeddedCaseFile {
                    path: "run.sh",
                    bytes: b"#!/bin/sh\n# Trace: FR-235-AC-3\nexit 7\n",
                },
                EmbeddedCaseFile {
                    path: "expected/exit_code",
                    bytes: b"7\n",
                },
                EmbeddedCaseFile {
                    path: "expected/stdout",
                    bytes: b"",
                },
            ],
        };
        let result = run_case(&case, Path::new("/bin/true"), Duration::from_secs(1));
        assert_eq!(result.status, "fail");
        assert!(result.detail.contains("exited non-zero"));
    }

    #[trace("FR-235-AC-7")]
    #[test]
    fn tc_229_timeout_is_an_error_with_timeout_detail() {
        let case = EmbeddedCase {
            id: "timeout-result",
            files: &[
                EmbeddedCaseFile {
                    path: "run.sh",
                    bytes: b"#!/bin/sh\n# Trace: FR-235-AC-7\nsleep 5\n",
                },
                EmbeddedCaseFile {
                    path: "expected/exit_code",
                    bytes: b"0\n",
                },
                EmbeddedCaseFile {
                    path: "expected/stdout",
                    bytes: b"",
                },
            ],
        };
        let result = run_case(&case, Path::new("/bin/true"), Duration::from_millis(50));
        assert_eq!(result.status, "error");
        assert!(result.detail.contains("timeout"));
    }

    #[trace("FR-236-AC-6")]
    #[test]
    fn tc_230_evidence_store_search_uses_nearest_ancestor() {
        let root = tempfile::tempdir().expect("a temporary repository");
        let outer_evidence = root.path().join("spec/evidence");
        std::fs::create_dir_all(&outer_evidence).expect("the outer evidence store exists");
        let nested = root.path().join("nested");
        std::fs::create_dir_all(nested.join("spec/evidence"))
            .expect("the nested evidence store exists");
        let start = nested.join("worktree");
        std::fs::create_dir_all(&start).expect("the nested working directory exists");

        assert_eq!(
            find_evidence_store(&start),
            Some(nested.join("spec/evidence"))
        );
        assert_eq!(
            record_path(Path::new("/repo/spec/evidence"), "2026-11-24T19:30:01.123Z"),
            PathBuf::from("/repo/spec/evidence/runs/SELFTEST/2026-11-24T193001123Z.json")
        );
    }

    #[trace("FR-236-AC-5")]
    #[test]
    fn tc_230_no_store_requires_explicit_run_record_destination() {
        assert!(require_retained_destination(None, None).is_err());
        assert!(require_retained_destination(None, Some(Path::new("result.json"))).is_ok());
        assert!(require_retained_destination(Some(Path::new("spec/evidence")), None).is_ok());
    }

    #[trace("FR-236-AC-1", "FR-236-AC-2", "FR-236-AC-4")]
    #[trace("FR-217-AC-1")]
    #[test]
    fn tc_230_run_record_serializes_run_context_and_case_outcomes() {
        let results = vec![
            CaseResult {
                id: "passing-case".to_owned(),
                status: "pass",
                criteria: vec!["FR-235-AC-1".to_owned()],
                detail: String::new(),
                stderr: String::new(),
            },
            CaseResult {
                id: "unavailable-case".to_owned(),
                status: "not-executed",
                criteria: vec!["FR-235-AC-3".to_owned()],
                detail: "required tool unavailable".to_owned(),
                stderr: String::new(),
            },
        ];
        let record = run_record(
            "quoin 0.1.0".to_owned(),
            "2026-11-25T12:00:00Z",
            results,
            "/repo/spec/evidence/runs/SELFTEST/run.json",
        );

        assert_eq!(record["tool"], "quoin");
        assert_eq!(record["toolVersion"], "quoin 0.1.0");
        assert_eq!(record["os"], std::env::consts::OS);
        assert_eq!(record["architecture"], std::env::consts::ARCH);
        assert_eq!(record["suite"], "SELFTEST");
        assert_eq!(record["timestamp"], "2026-11-25T12:00:00Z");
        assert_eq!(record["results"].as_array().expect("case results").len(), 2);
        assert_eq!(record["results"][0]["status"], "pass");
        assert_eq!(record["results"][1]["status"], "not-executed");
        assert_eq!(record["results"][1]["detail"], "required tool unavailable");
        assert!(record["validationInputs"].is_object());
        let serialized = serde_json::to_string(&record).expect("the record serializes");
        assert!(!serialized.contains("sourceDigest"));
        assert!(!serialized.contains("sourceRevision"));
        assert!(!serialized.contains("reportContentDigest"));
    }

    #[trace("FR-235-AC-8")]
    #[test]
    fn tc_229_out_and_stdout_preserve_exact_run_record_bytes() {
        let root = tempfile::tempdir().expect("a temporary evidence root");
        let bytes = b"{\n  \"status\": \"pass\"\n}\n";
        let store_path = root.path().join("spec/evidence/runs/SELFTEST/run.json");
        let output_path = root.path().join("requested.json");
        let stdout = retain_run_record(bytes, Some(&store_path), Some(&output_path))
            .expect("the run record is retained");
        assert_eq!(stdout, None);
        assert_eq!(
            std::fs::read(&store_path).expect("the evidence store record exists"),
            bytes
        );
        assert_eq!(
            std::fs::read(&output_path).expect("the explicit output record exists"),
            bytes
        );

        let stdout_path = root.path().join("stdout.json");
        let stdout = retain_run_record(bytes, Some(&stdout_path), None)
            .expect("stdout is retained at the evidence store");
        assert_eq!(
            stdout.as_deref(),
            Some(std::str::from_utf8(bytes).expect("record is UTF-8"))
        );
        assert_eq!(
            std::fs::read(&stdout_path).expect("the stdout record exists"),
            bytes
        );
    }

    #[trace("FR-235-AC-7")]
    #[test]
    fn tc_229_timeout_terminates_case_process_group() {
        let root = tempfile::tempdir().expect("a temporary test root");
        let marker = root.path().join("descendant-survived");
        let script = format!(
            "#!/bin/sh\n(sleep 0.4; touch '{}') &\nexit 0\n",
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

    #[cfg(target_os = "linux")]
    #[trace("FR-235-AC-7")]
    #[test]
    fn tc_229_timeout_kills_setsid_descendant_holding_output_pipe() {
        let root = tempfile::tempdir().expect("a temporary test root");
        let marker = root.path().join("escaped-descendant-survived");
        let script = format!(
            "#!/bin/sh\nsetsid sh -c 'sleep 5; touch \"{}\"' &\nexit 0\n",
            marker.display()
        );
        let case_tmp = tempfile::tempdir().expect("a temporary case directory");
        let home = tempfile::tempdir().expect("an empty home directory");
        let script_path = case_tmp.path().join("run.sh");
        std::fs::write(&script_path, script).expect("the escaped-descendant script is written");

        let started = Instant::now();
        let output = execute_script(
            &script_path,
            Path::new("/bin/true"),
            case_tmp.path(),
            home.path(),
            Duration::from_millis(100),
        )
        .expect("the escaped descendant is terminated and reaped");
        assert!(output.timed_out);
        assert!(started.elapsed() < Duration::from_secs(2));
        thread::sleep(Duration::from_millis(150));
        assert!(!marker.exists(), "the setsid descendant was terminated");
    }

    #[trace("FR-235-CON-1")]
    #[test]
    fn tc_229_materialization_writes_only_case_relative_paths() {
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
