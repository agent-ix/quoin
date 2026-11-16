// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin dossier generate | check | export` (FR-191, PLAT-1614).
//!
//! This module owns syntax and rendering only. The registry, the writer, the
//! comparison and the `quire export` hand-off live in `quoin-dossier`; the
//! exit status comes from the error code or the report, never from a number
//! chosen here.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_assurance::matrix::MatrixOutput;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_dossier::{
    Comparison, DossierError, ExportOptions, Inputs, MatrixSource, Registry, Report, SourceFailure,
    Status, builtin_registry,
};

use crate::evidence::audit::AuditOptions;
use crate::matrix::Built;

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Generate, check and export the submission dossier documents.";

/// Where documents are written unless `--out` names another directory.
const DEFAULT_OUT: &str = "dossier";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Generate,
    Check,
    Export,
}

fn many(name: &'static str, value: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .value_name(value)
        .action(ArgAction::Append)
        .help(help)
}

fn one(name: &'static str, value: &'static str, help: &'static str) -> Arg {
    Arg::new(name).long(name).value_name(value).help(help)
}

/// The selectors `generate`, `check` and `export` share.
fn selectors(command: Command) -> Command {
    command
        .arg(many(
            "doc",
            "KIND",
            "Document kind to select; repeat to add. Without it every registered kind is selected",
        ))
        .arg(one(
            "out",
            "DIR",
            "Directory holding the documents; defaults to dossier",
        ))
        .arg(one(
            "scope",
            "DIR",
            "Repository scope the records are read from",
        ))
        .arg(many(
            "module",
            "PATH",
            "Module root to read record types from; repeat to add",
        ))
        .arg(many(
            "clause-binding",
            "PATH",
            "Clause-binding report; repeat to add",
        ))
        .arg(one(
            "soup-binding",
            "FILE",
            "Clause binding passed to quoin soup report",
        ))
        .arg(one(
            "soup-records",
            "DIR",
            "Record directory passed to quoin soup report",
        ))
        .arg(one(
            "soup-support",
            "FILE",
            "Support statements passed to quoin soup report",
        ))
        .arg(
            one(
                "pack",
                "PACK",
                "Regulatory pack, for documents that need one",
            )
            .conflicts_with("clause-binding"),
        )
        .arg(one(
            "as-of",
            "RFC-3339",
            "Explicit evaluation instant; required by documents that record time",
        ))
        .arg(one(
            "version",
            "TEXT",
            "Document version written into every generated file; required",
        ))
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Emit the report as JSON instead of text"),
        )
}

pub(crate) fn command() -> Command {
    Command::new("dossier")
        .about(ABOUT)
        .subcommand_required(true)
        .subcommand(selectors(Command::new("generate").about(
            "Write the selected dossier documents; never calls quire export",
        )))
        .subcommand(selectors(Command::new("check").about(
            "Compare the selected documents with the files on disk; writes nothing",
        )))
        .subcommand(
            selectors(Command::new("export").about(
                "Export the selected, committed and current documents through quire export",
            ))
            .arg(
                Arg::new("to")
                    .long("to")
                    .value_name("FORMAT")
                    .required(true)
                    .value_parser(["pdf", "docx"])
                    .help("Output format"),
            )
            .arg(one("output", "PATH", "The file quire export writes").required(true))
            .arg(one("release", "TAG", "Passed to quire export unchanged"))
            .arg(many(
                "approval",
                "PATH",
                "Passed to quire export unchanged; repeat to add",
            ))
            .arg(one("template", "PATH", "Passed to quire export unchanged"))
            .arg(one(
                "quire",
                "PATH",
                "The quire executable; defaults to quire on PATH",
            )),
        )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    run_with(
        builtin_registry(quoin_dossier::Sources {
            matrix: Arc::new(CliMatrix),
            rmm: rmm_source,
            soup: soup_source,
            policy: policy_source,
        }),
        arguments,
    )
}

fn policy_source(
    inputs: &quoin_dossier::Inputs,
) -> Result<quoin_quire::records::Corpus, quoin_dossier::SourceFailure> {
    let modules = inputs
        .modules
        .iter()
        .map(quoin_quire::ids::ModuleRoot::open)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| quoin_dossier::SourceFailure {
            source: "acceptability policy".to_owned(),
            detail: format!("failed to open module: {error}"),
        })?;
    let selection = if modules.is_empty() {
        quoin_quire::ModuleSelection::ScopeOrAmbient
    } else {
        quoin_quire::ModuleSelection::Closed(modules)
    };
    quoin_quire::records::read(&inputs.scope, &selection, &["acceptability_policy"]).map_err(
        |error| quoin_dossier::SourceFailure {
            source: "acceptability policy".to_owned(),
            detail: format!("failed to read policy records: {error}"),
        },
    )
}

/// Run against `registry`; a registry that could not be built is exit 2, so a
/// generator that collides is reported rather than silently missing.
fn run_with(
    registry: Result<Registry, DossierError>,
    arguments: &ArgMatches,
) -> Result<Response, String> {
    let (mode, selected) = match arguments.subcommand() {
        Some(("generate", selected)) => (Mode::Generate, selected),
        Some(("check", selected)) => (Mode::Check, selected),
        Some(("export", selected)) => (Mode::Export, selected),
        _ => return Err("an unknown dossier command reached dispatch".to_owned()),
    };
    let json = selected.get_flag("json");
    let result = registry.and_then(|registry| execute(&registry, mode, selected));
    finish(mode, json, result)
}

fn strings(arguments: &ArgMatches, name: &str) -> Vec<String> {
    arguments
        .get_many::<String>(name)
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

fn paths(arguments: &ArgMatches, name: &str) -> Vec<PathBuf> {
    strings(arguments, name)
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

fn text(arguments: &ArgMatches, name: &str) -> Option<String> {
    arguments.get_one::<String>(name).cloned()
}

fn execute(
    registry: &Registry,
    mode: Mode,
    arguments: &ArgMatches,
) -> Result<Report, DossierError> {
    let kinds = strings(arguments, "doc");
    let out = PathBuf::from(text(arguments, "out").unwrap_or_else(|| DEFAULT_OUT.to_owned()));
    let inputs = Inputs {
        scope: PathBuf::from(text(arguments, "scope").unwrap_or_else(|| ".".to_owned())),
        modules: paths(arguments, "module"),
        clause_bindings: paths(arguments, "clause-binding"),
        pack: text(arguments, "pack"),
        soup_binding: text(arguments, "soup-binding").map(PathBuf::from),
        soup_records: text(arguments, "soup-records").map(PathBuf::from),
        soup_support: text(arguments, "soup-support").map(PathBuf::from),
        as_of: text(arguments, "as-of"),
        // A missing --version is refused by the library, which owns that rule.
        version: text(arguments, "version").unwrap_or_default(),
    };
    match mode {
        Mode::Generate => quoin_dossier::generate(registry, &kinds, &out, &inputs),
        Mode::Check => quoin_dossier::check(registry, &kinds, &out, &inputs),
        Mode::Export => {
            let options = ExportOptions {
                to: text(arguments, "to").unwrap_or_default(),
                output: PathBuf::from(text(arguments, "output").unwrap_or_default()),
                release: text(arguments, "release"),
                approvals: paths(arguments, "approval"),
                template: text(arguments, "template").map(PathBuf::from),
                quire: PathBuf::from(
                    text(arguments, "quire").unwrap_or_else(|| "quire".to_owned()),
                ),
            };
            quoin_dossier::export(registry, &kinds, &out, &inputs, &options)
        }
    }
}

fn diagnostic(error: &DossierError) -> Diagnostic {
    Diagnostic {
        code: error.code().as_str().to_owned(),
        message: error.message().to_owned(),
        context: error.context().clone(),
    }
}

/// The payload member that marks a report `emit` prints even though the exit
/// status is not 0 or 1. Only this command sets it: exit 3 and 4 name the
/// failed source or the quire failure in the report (FR-191), and every other
/// command's refusal still prints nothing on stdout.
pub(crate) const FAILURE_REPORT: &str = "failureReport";

/// A response whose stdout is the report: JSON when asked for, otherwise nothing.
fn report_response(
    report: &Report,
    json: bool,
    outcome: Outcome,
    diagnostics: Vec<Diagnostic>,
) -> Result<Response, String> {
    let payload = if json {
        let rendered = canonical_json(report).map_err(|error| error.to_string())?;
        serde_json::json!({ "rendered": rendered, FAILURE_REPORT: !outcome.carries_payload() })
    } else {
        serde_json::Value::Null
    };
    Ok(Response {
        payload,
        diagnostics,
        outcome,
    })
}

fn finish(
    mode: Mode,
    json: bool,
    result: Result<Report, DossierError>,
) -> Result<Response, String> {
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            let diagnostics = vec![diagnostic(&error)];
            return match error.source_failure() {
                Some(failure) => {
                    let report = Report {
                        source_failure: Some(failure.clone()),
                        ..Report::default()
                    };
                    report_response(&report, json, error.outcome(), diagnostics)
                }
                None => Ok(Response {
                    payload: serde_json::Value::Null,
                    diagnostics,
                    outcome: error.outcome(),
                }),
            };
        }
    };
    if let Some(quire) = &report.quire {
        let error = DossierError::quire_failed(quire);
        let diagnostics = vec![diagnostic(&error)];
        return report_response(&report, json, error.outcome(), diagnostics);
    }
    let mut diagnostics = Vec::new();
    if mode == Mode::Export {
        for id in report.differing() {
            diagnostics.push(diagnostic(&DossierError::export_stale(id)));
        }
    }
    let outcome = report.outcome();
    if json {
        return report_response(&report, true, outcome, diagnostics);
    }
    Ok(Response {
        payload: serde_json::json!({ "rendered": render_text(mode, &report) }),
        diagnostics,
        outcome,
    })
}

fn render_text(mode: Mode, report: &Report) -> String {
    let mut lines = Vec::new();
    for document in &report.documents {
        let status = match document.status {
            Status::Complete => "complete".to_owned(),
            Status::Incomplete => format!("incomplete, {} gap(s)", document.gaps.len()),
        };
        let comparison = match document.comparison {
            Some(Comparison::Matches) => " matches",
            Some(Comparison::Differs) => " differs",
            None => "",
        };
        let verb = if mode == Mode::Generate { "wrote " } else { "" };
        lines.push(format!(
            "{verb}{} {} ({status}){comparison}",
            document.id, document.path
        ));
    }
    lines.join("\n")
}

/// The matrix `quoin matrix` prints, built through the same
/// [`crate::matrix::build`] path in this process.
struct CliMatrix;

impl MatrixSource for CliMatrix {
    fn matrix(&self, inputs: &Inputs) -> Result<MatrixOutput, SourceFailure> {
        let failure = |detail: String| SourceFailure {
            source: "matrix".to_owned(),
            detail,
        };
        let repo = inputs
            .scope
            .to_str()
            .ok_or_else(|| failure("the scope path is not UTF-8".to_owned()))?;
        let modules = inputs
            .modules
            .iter()
            .map(|module| {
                module
                    .to_str()
                    .map(str::to_owned)
                    .ok_or_else(|| failure("a module path is not UTF-8".to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let options = AuditOptions::for_modules(modules);
        let response = match crate::matrix::build(&options, repo, &[]).map_err(failure)? {
            Built::Matrix(built) => built.response,
            Built::Declined(response) => response,
        };
        if !response.outcome.carries_payload() {
            let messages: Vec<&str> = response
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect();
            return Err(failure(format!(
                "the matrix was not built: {}",
                messages.join("; ")
            )));
        }
        serde_json::from_value(response.payload)
            .map_err(|error| failure(format!("the matrix payload is malformed: {error}")))
    }
}

/// Use discharge's own structured operations, never its Markdown renderer.
fn rmm_source(inputs: &Inputs) -> Result<quoin_dossier::RmmSource, quoin_dossier::SourceFailure> {
    let result = (|| {
        let as_of = inputs
            .as_of
            .as_deref()
            .ok_or_else(|| "--as-of is required for rmm".to_owned())?;
        let repo = inputs.scope.display().to_string();
        let read = |operation: &str,
                    request: &serde_json::Value,
                    field: &str|
         -> Result<serde_json::Value, String> {
            let response = crate::core_runtime::invoke(operation, request)?;
            if !response.outcome.carries_payload() {
                return Err(format!(
                    "discharge source declined: {:?}",
                    response.diagnostics
                ));
            }
            response
                .payload
                .get(field)
                .cloned()
                .ok_or_else(|| format!("{operation} omitted {field}"))
        };
        if let Some(pack) = &inputs.pack {
            let report = read(
                "evidence.pack_requirements_matrix",
                &serde_json::json!({"pack": pack, "repo": repo, "asOf": as_of}),
                "report",
            )?;
            return serde_json::from_value(report)
                .map(quoin_dossier::RmmSource::Pack)
                .map_err(|error| error.to_string());
        }
        let mut matrices = Vec::new();
        for path in &inputs.clause_bindings {
            let binding =
                crate::discharge::json_input(&path.display().to_string(), "clause binding")?;
            let matrix = read(
                "evidence.requirements_matrix",
                &serde_json::json!({"repo": repo, "binding": binding, "asOf": as_of}),
                "matrix",
            )?;
            matrices.push(serde_json::from_value(matrix).map_err(|error| error.to_string())?);
        }
        Ok(quoin_dossier::RmmSource::Bindings(matrices))
    })();
    result.map_err(|detail| quoin_dossier::SourceFailure {
        source: "discharge".to_owned(),
        detail,
    })
}

/// Use FR-144's shared builder with the host's runtime configuration.
fn soup_source(inputs: &Inputs) -> Result<quoin_soup::DossierSource, quoin_dossier::SourceFailure> {
    let result = (|| {
        let binding = inputs
            .soup_binding
            .as_deref()
            .ok_or_else(|| "--soup-binding is required".to_owned())?;
        let as_of = inputs
            .as_of
            .as_deref()
            .ok_or_else(|| "--as-of is required".to_owned())?;
        quoin_soup::dossier_source(
            &inputs.scope,
            inputs.soup_records.as_deref(),
            binding,
            as_of,
            inputs.soup_support.as_deref(),
            &crate::core_runtime::settings(),
        )
        .map_err(|error| error.message().to_owned())
    })();
    result.map_err(|detail| quoin_dossier::SourceFailure {
        source: "quoin soup report".to_owned(),
        detail,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quoin_dossier::DossierErrorCode;

    #[trace("FR-191")]
    #[test]
    fn a_registry_that_cannot_be_built_is_exit_2_with_a_diagnostic_and_writes_nothing() {
        let work = tempfile::tempdir().unwrap();
        let out = work.path().join("dossier");
        let matches = command()
            .try_get_matches_from([
                "dossier",
                "generate",
                "--version",
                "1",
                "--out",
                out.to_str().unwrap(),
            ])
            .unwrap();
        let conflict = DossierError::new(
            DossierErrorCode::RegistryConflict,
            "document srs (DOS-SRS) conflicts with registered srs (DOS-SRS)",
        );
        let response = run_with(Err(conflict), &matches).unwrap();
        assert_eq!(response.outcome.code(), 2);
        assert_eq!(response.diagnostics[0].code, "DOSSIER_REGISTRY_CONFLICT");
        assert!(!out.exists());
    }
}
