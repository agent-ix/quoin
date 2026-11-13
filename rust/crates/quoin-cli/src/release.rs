// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin release snapshot`: write the evidence state of one release as one
//! file (FR-166, FR-167, FR-168, NFR-036).
//!
//! The command is a view of views. The coverage, freshness and findings come
//! from the code path `quoin matrix` uses, the SOUP section from the one
//! `quoin soup report` uses, the anomalies from the selection and record
//! reading of the anomaly crate, and the change records from the
//! release-history file, taken as data. It runs no git, reads no clock, and
//! computes no digest: every instant is `--as-of` and every commit is the
//! caller's.

mod history;
mod snapshot;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests;

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};

use crate::evidence::audit::AuditOptions;
use crate::soup::{SoupFailure, SoupInputs, soup_section};

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Write the evidence state of a release as one file.";

/// A stable refusal code. The spelling is the API: never renamed, never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Code {
    Exists,
    BaselineCommitInvalid,
    InputUnreadable,
    InputInvalid,
    HistoryInvalid,
    HistoryBaselineMismatch,
    AnomalyUnreadable,
}

impl Code {
    fn as_str(self) -> &'static str {
        match self {
            Self::Exists => "release_snapshot_exists",
            Self::BaselineCommitInvalid => "release_snapshot_baseline_commit_invalid",
            Self::InputUnreadable => "release_snapshot_input_unreadable",
            Self::InputInvalid => "release_snapshot_input_invalid",
            Self::HistoryInvalid => "release_snapshot_history_invalid",
            Self::HistoryBaselineMismatch => "release_snapshot_history_baseline_mismatch",
            Self::AnomalyUnreadable => "release_snapshot_anomaly_unreadable",
        }
    }
}

/// A refusal: exit status 2, no payload, no file.
struct Refusal {
    code: Code,
    message: String,
    context: BTreeMap<String, String>,
}

impl Refusal {
    fn new(code: Code, message: impl Into<String>, context: &[(&str, &str)]) -> Self {
        Self {
            code,
            message: message.into(),
            context: context
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    /// An input named under Inputs that is missing or unreadable.
    fn unreadable(input: &str, why: impl std::fmt::Display) -> Self {
        Self::new(
            Code::InputUnreadable,
            format!("{input} cannot be read: {why}"),
            &[("input", input)],
        )
    }

    /// An input that fails its own format.
    fn invalid(input: &str, cause: &str, message: &str) -> Self {
        Self::new(
            Code::InputInvalid,
            format!("{input} is invalid: {message}"),
            &[("input", input), ("cause", cause)],
        )
    }
}

/// Why the command stopped.
enum Stop {
    Refused(Box<Refusal>),
    /// An operational failure (exit 1).
    Failed(String),
}

impl From<Refusal> for Stop {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(Box::new(refusal))
    }
}

impl From<String> for Stop {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

pub(crate) fn command() -> Command {
    Command::new("release")
        .about(ABOUT)
        .subcommand_required(true)
        .subcommand(
            Command::new("snapshot")
                .about("Write the release evidence snapshot to --out, running no git")
                .arg(
                    Arg::new("repo")
                        .long("repo")
                        .value_name("DIR")
                        .default_value(".")
                        .help("Repository whose spec, evidence store, SOUP records and anomalies are read"),
                )
                .arg(
                    Arg::new("baseline-commit")
                        .long("baseline-commit")
                        .value_name("HASH")
                        .required(true)
                        .help("Full lowercase hexadecimal commit hash of the baseline, stated as supplied"),
                )
                .arg(
                    Arg::new("history")
                        .long("history")
                        .value_name("FILE")
                        .required(true)
                        .help("Release-history file (engineering-assurance.release-history/v1)"),
                )
                .arg(
                    Arg::new("binding")
                        .long("binding")
                        .value_name("FILE")
                        .required(true)
                        .help("Quire clause-binding-v1 JSON report, as `quoin soup report` takes it"),
                )
                .arg(
                    Arg::new("as-of")
                        .long("as-of")
                        .value_name("INSTANT")
                        .required(true)
                        .help("Explicit evaluation instant, stated as supplied"),
                )
                .arg(
                    Arg::new("records")
                        .long("records")
                        .value_name("DIR")
                        .help("Where the soup_record files are read; defaults to <repo>/spec/soup"),
                )
                .arg(
                    Arg::new("support")
                        .long("support")
                        .value_name("FILE")
                        .help("JSON file of public support statements, as `quoin soup report` takes it"),
                )
                .arg(
                    Arg::new("out")
                        .long("out")
                        .value_name("PATH")
                        .required(true)
                        .help("The file to write; it must not exist"),
                ),
        )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let Some(("snapshot", snapshot)) = arguments.subcommand() else {
        return Err("an unknown release command reached dispatch".to_owned());
    };
    match write_snapshot(&Inputs::from_arguments(snapshot)?) {
        Ok(out) => Ok(Response::ok(
            serde_json::json!({ "rendered": out.display().to_string() }),
        )),
        Err(Stop::Refused(refusal)) => Ok(Response {
            payload: serde_json::Value::Null,
            diagnostics: vec![Diagnostic {
                code: refusal.code.as_str().to_owned(),
                message: refusal.message,
                context: refusal.context,
            }],
            outcome: Outcome::Refused,
        }),
        Err(Stop::Failed(message)) => Err(message),
    }
}

/// The flags of `quoin release snapshot`.
struct Inputs {
    repo: PathBuf,
    baseline_commit: String,
    history: PathBuf,
    binding: PathBuf,
    as_of: String,
    records: PathBuf,
    support: Option<PathBuf>,
    out: PathBuf,
}

impl Inputs {
    fn from_arguments(arguments: &ArgMatches) -> Result<Self, String> {
        let text = |name: &str| {
            arguments
                .get_one::<String>(name)
                .cloned()
                .ok_or_else(|| format!("--{name} is required"))
        };
        let repo = PathBuf::from(text("repo")?);
        let records = arguments
            .get_one::<String>("records")
            .map_or_else(|| repo.join("spec/soup"), PathBuf::from);
        Ok(Self {
            baseline_commit: text("baseline-commit")?,
            history: PathBuf::from(text("history")?),
            binding: PathBuf::from(text("binding")?),
            as_of: text("as-of")?,
            support: arguments.get_one::<String>("support").map(PathBuf::from),
            out: PathBuf::from(text("out")?),
            records,
            repo,
        })
    }
}

/// Whether `hash` is a full lowercase hexadecimal commit hash of 40 or 64 digits.
fn is_commit_hash(hash: &str) -> bool {
    matches!(hash.len(), 40 | 64)
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn read_text(input: &str, path: &Path) -> Result<String, Refusal> {
    std::fs::read_to_string(path).map_err(|error| Refusal::unreadable(input, error))
}

/// Build every section, then write `--out` and nothing else.
fn write_snapshot(inputs: &Inputs) -> Result<PathBuf, Stop> {
    if !is_commit_hash(&inputs.baseline_commit) {
        return Err(Refusal::new(
            Code::BaselineCommitInvalid,
            "--baseline-commit is not a full lowercase hexadecimal commit hash of 40 or 64 digits",
            &[("baseline_commit", &inputs.baseline_commit)],
        )
        .into());
    }
    if std::fs::symlink_metadata(&inputs.out).is_ok() {
        return Err(exists(&inputs.out).into());
    }
    if !inputs.repo.is_dir() {
        return Err(Refusal::unreadable("--repo", "not a directory").into());
    }
    let history = history::parse(&read_text("--history", &inputs.history)?).map_err(|invalid| {
        Refusal::new(
            Code::HistoryInvalid,
            invalid.message,
            &[("member", &invalid.member)],
        )
    })?;
    if history.baseline_commit != inputs.baseline_commit {
        return Err(Refusal::new(
            Code::HistoryBaselineMismatch,
            format!(
                "the history file's baseline_commit {} differs from --baseline-commit {}",
                history.baseline_commit, inputs.baseline_commit
            ),
            &[
                ("history_baseline_commit", &history.baseline_commit),
                ("baseline_commit", &inputs.baseline_commit),
            ],
        )
        .into());
    }
    let binding = read_text("--binding", &inputs.binding)?;
    let binding: serde_json::Value = serde_json::from_str(&binding).map_err(|error| {
        Refusal::invalid("--binding", "not-json", &format!("not JSON: {error}"))
    })?;
    if let Some(support) = &inputs.support
        && !support.is_file()
    {
        return Err(Refusal::unreadable("--support", "not a file").into());
    }
    if !inputs.records.is_dir() {
        return Err(Refusal::unreadable("--records", "not a directory").into());
    }

    let soup = soup_section(&SoupInputs {
        repo: &inputs.repo,
        records_dir: &inputs.records,
        as_of: &inputs.as_of,
        binding: &binding,
        support: inputs.support.as_deref(),
    })
    .map_err(|failure| match failure {
        SoupFailure::Refused {
            input,
            code,
            message,
        } => Stop::from(Refusal::invalid(input, &code, &message)),
        SoupFailure::Unreadable { input, message } => {
            Stop::from(Refusal::unreadable(input, message))
        }
        SoupFailure::Failed(message) => Stop::Failed(message),
    })?;
    let matrix = snapshot::matrix_sections(&AuditOptions::default(), &inputs.repo)?;
    let anomalies = snapshot::anomalies(&inputs.repo)?;

    let snapshot = snapshot::Snapshot {
        protocol: snapshot::PROTOCOL.to_owned(),
        as_of: inputs.as_of.clone(),
        baseline_commit: inputs.baseline_commit.clone(),
        previous_release: history.previous_release,
        coverage: matrix.coverage,
        evidence_freshness: matrix.freshness,
        open_findings: matrix.findings,
        soup,
        anomalies,
        change_records: history.records,
    };
    let mut bytes = canonical_json(&snapshot)
        .map_err(|error| Stop::Failed(error.to_string()))?
        .into_bytes();
    bytes.push(b'\n');
    create_new(&inputs.out, &bytes)?;
    Ok(inputs.out.clone())
}

fn exists(out: &Path) -> Refusal {
    Refusal::new(
        Code::Exists,
        format!(
            "--out {} already exists and is left unchanged",
            out.display()
        ),
        &[("path", &out.display().to_string())],
    )
}

/// Write `bytes` to a file that must not exist, removing a partial write.
fn create_new(out: &Path, bytes: &[u8]) -> Result<(), Stop> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Stop::from(exists(out))
            } else {
                Stop::Failed(format!("cannot create {}: {error}", out.display()))
            }
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.flush()) {
        drop(file);
        let _ = std::fs::remove_file(out);
        return Err(Stop::Failed(format!(
            "cannot write {}: {error}",
            out.display()
        )));
    }
    Ok(())
}
