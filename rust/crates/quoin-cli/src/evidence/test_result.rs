// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence-store adapter for `quoin evidence export-test-result` (FR-185).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
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
    if let Err(error) = publish(
        &StorePath::Named(output_path.clone()),
        Content::Artifact(&bytes),
    ) {
        return Ok(output_failed(format!("{}: {error}", output_path.display())));
    }
    Ok(Response::ok(serde_json::json!({
        "rendered": format!("Exported in-toto Test Result statement to {}", output_path.display())
    })))
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
