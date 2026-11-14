// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence-store adapter for `quoin evidence export-test-result` (FR-185).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use ix_trace_rs::implements;
use quoin_core::protocol::{Diagnostic, Response};
use quoin_evidence::export::test_result::build_statement;
use quoin_evidence::source::DiskEvidence;
use quoin_evidence::store::{latest_runs, read_bindings};
use quoin_store::store::{Content, StorePath, publish};
use serde_json::Value;

use super::required;

pub(super) fn command() -> Command {
    Command::new("export-test-result")
        .about("Export retained test evidence as an in-toto Test Result statement")
        .arg(Arg::new("subject").long("subject").required(true))
        .arg(Arg::new("output").long("output").required(true))
        .arg(
            Arg::new("evidence-dir")
                .long("evidence-dir")
                .default_value("spec/evidence"),
        )
}

#[implements("FR-185")]
pub(super) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let subject_path = required(arguments, "subject")?;
    let output_path = PathBuf::from(required(arguments, "output")?);
    let evidence_path = PathBuf::from(
        arguments
            .get_one::<String>("evidence-dir")
            .map_or("spec/evidence", String::as_str),
    );

    let subject_bytes = match std::fs::read(&subject_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(refused(
                "evidence_subject_invalid",
                format!("cannot read {subject_path}: {error}"),
            ));
        }
    };
    let subject: Value = match serde_json::from_slice(&subject_bytes) {
        Ok(subject) => subject,
        Err(error) => {
            return Ok(refused(
                "evidence_subject_invalid",
                format!("{subject_path}: {error}"),
            ));
        }
    };
    if let Err(error) = build_statement(subject.clone(), &[], &[]) {
        return Ok(refused("evidence_subject_invalid", error.to_string()));
    }

    let source = DiskEvidence::at_path(&evidence_path);
    let bindings_path = evidence_path.join("bindings.json");
    if !bindings_path.is_file() {
        return Ok(store_invalid(format!(
            "{} is missing",
            bindings_path.display()
        )));
    }
    let bindings = match read_bindings(&source) {
        Ok(file) => file.bindings,
        Err(error) => {
            return Ok(store_invalid(format!(
                "{}: {error}",
                bindings_path.display()
            )));
        }
    };
    let mut skipped = Vec::new();
    let runs = match latest_runs(&source, &mut skipped) {
        Ok(runs) => runs,
        Err(error) => {
            return Ok(store_invalid(format!(
                "{}: {error}",
                evidence_path.display()
            )));
        }
    };
    if let Some(path) = skipped.first() {
        return Ok(store_invalid(format!(
            "{}: invalid run record",
            evidence_path.join(path).display()
        )));
    }

    let statement = match build_statement(subject, &bindings, &runs) {
        Ok(statement) => statement,
        Err(error) => return Ok(refused("evidence_subject_invalid", error.to_string())),
    };
    let serialized = serde_json::to_string(&statement).map_err(|error| error.to_string())?;
    let value =
        quoin_store::parse_strict_json_str(&serialized).map_err(|error| error.to_string())?;
    let bytes = quoin_store::canonical_bytes(&value).map_err(|error| error.to_string())?;
    if let Err(error) = publish_output(&output_path, &bytes) {
        return Ok(output_failed(error));
    }
    Ok(Response::ok(serde_json::json!({
        "rendered": format!("Exported in-toto Test Result statement to {}", output_path.display())
    })))
}

fn publish_output(path: &Path, bytes: &[u8]) -> Result<(), String> {
    publish_output_with(path, || {
        publish(
            &StorePath::Named(path.to_path_buf()),
            Content::Artifact(bytes),
        )
        .map(|_| ())
        .map_err(|error| format!("{}: {error}", path.display()))
    })
}

fn publish_output_with(
    path: &Path,
    publish: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "{}: cannot create output directory: {error}",
            parent.display()
        )
    })?;
    let backup = backup_existing(path, parent)?;
    if let Err(publish_error) = publish() {
        return Err(rollback_output(path, parent, backup)
            .err()
            .map_or(publish_error.clone(), |rollback_error| {
                format!("{publish_error}; rollback failed: {rollback_error}")
            }));
    }
    if let Some(backup) = backup
        && let Err(error) = fs::remove_file(&backup)
    {
        let publish_error = format!(
            "{}: cannot remove output rollback file {}: {error}",
            path.display(),
            backup.display()
        );
        return Err(rollback_output(path, parent, Some(backup))
            .err()
            .map_or(publish_error.clone(), |rollback_error| {
                format!("{publish_error}; rollback failed: {rollback_error}")
            }));
    }
    Ok(())
}

fn backup_existing(path: &Path, parent: &Path) -> Result<Option<PathBuf>, String> {
    static NEXT_BACKUP: AtomicU64 = AtomicU64::new(0);
    if path.file_name().is_none() {
        return Err(format!("{}: output path has no file name", path.display()));
    }
    for _ in 0..16 {
        let sequence = NEXT_BACKUP.fetch_add(1, Ordering::Relaxed);
        let backup = parent.join(format!(".quoin-test-result-rollback-{sequence}"));
        match fs::hard_link(path, &backup) {
            Ok(()) => return Ok(Some(backup)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(format!(
                    "{}: cannot preserve existing output: {error}",
                    path.display()
                ));
            }
        }
    }
    Err(format!(
        "{}: cannot allocate output rollback path",
        path.display()
    ))
}

fn rollback_output(path: &Path, parent: &Path, backup: Option<PathBuf>) -> Result<(), String> {
    match backup {
        Some(backup) => fs::rename(&backup, path).map_err(|error| {
            format!(
                "cannot restore {} from {}: {error}",
                path.display(),
                backup.display()
            )
        })?,
        None => match fs::remove_file(path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(format!("cannot remove {}: {error}", path.display())),
        },
    }
    quoin_store::store::fsync_directory(parent)
        .map_err(|error| format!("cannot sync rollback in {}: {error}", parent.display()))
}

fn refused(code: &str, message: String) -> Response {
    failure(Outcome::Refused, code, message)
}

fn store_invalid(message: String) -> Response {
    failure(Outcome::Invalid, "evidence_store_invalid", message)
}

fn output_failed(message: String) -> Response {
    failure(Outcome::Internal, "evidence_output_write_failed", message)
}

fn failure(outcome: Outcome, code: &str, message: String) -> Response {
    Response {
        payload: Value::Null,
        diagnostics: vec![Diagnostic {
            code: code.to_owned(),
            message,
            context: BTreeMap::new(),
        }],
        outcome,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "temporary output fixtures identify setup failures directly"
)]
mod tests {
    use super::publish_output_with;
    use ix_trace_rs::trace;
    use std::fs;

    #[trace("FR-185-AC-3")]
    #[test]
    fn a_post_publish_error_removes_a_new_output() {
        let root = tempfile::tempdir().expect("temporary directory");
        let output = root.path().join("statement.json");
        let result = publish_output_with(&output, || {
            let temporary = root.path().join("temporary.json");
            fs::write(&temporary, b"published").expect("simulated temporary");
            fs::rename(temporary, &output).expect("simulated rename");
            Err("directory fsync failed".to_owned())
        });
        assert!(result.is_err());
        assert!(!output.exists());
    }

    #[trace("FR-185-AC-3")]
    #[test]
    fn a_post_publish_error_restores_an_existing_output() {
        let root = tempfile::tempdir().expect("temporary directory");
        let output = root.path().join("statement.json");
        fs::write(&output, b"original").expect("original output");
        let result = publish_output_with(&output, || {
            let temporary = root.path().join("temporary.json");
            fs::write(&temporary, b"replacement").expect("simulated temporary");
            fs::rename(temporary, &output).expect("simulated rename");
            Err("directory fsync failed".to_owned())
        });
        assert!(result.is_err());
        assert_eq!(fs::read(output).expect("restored output"), b"original");
    }
}
