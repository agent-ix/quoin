// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin matrix`: the evidence-backed test matrix, rendered (FR-115).
//!
//! Read-only. It assembles the audit exactly as `quoin evidence audit` does
//! (the shared [`assemble`]), hands the three inputs to `matrix.build`, and
//! prints the result. It writes nothing, anywhere, and gates nothing: the
//! static axis is gated by `quire matrix --strict` and the evidence axis by
//! `quoin evidence audit --strict`.

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::{Response, canonical_json};

use crate::core_runtime::invoke;
use crate::evidence::audit::{Assembly, AuditOptions, assemble_with, module_arg, policy_args};
use crate::evidence::{json_arg, repo_arg, required};

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Render the evidence-backed test matrix, per criterion.";

pub(crate) fn command() -> Command {
    Command::new("matrix")
        .about(ABOUT)
        .arg(repo_arg())
        .arg(module_arg())
        .arg(json_arg())
        .arg(
            Arg::new("run-observation")
                .long("run-observation")
                .action(ArgAction::Append)
                .num_args(1)
                .value_name("FILE"),
        )
        .args(policy_args())
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let run_observation_paths = arguments
        .get_many::<String>("run-observation")
        .map(|paths| paths.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let run_observations = crate::run_observation::read_run_observations(&run_observation_paths)?;
    let repo = required(arguments, "repo")?;
    let options = AuditOptions::from_arguments(arguments);
    let built = match build(&options, &repo, &run_observations)? {
        Built::Matrix(built) => built,
        Built::Declined(response) => return Ok(response),
    };
    if !built.response.outcome.carries_payload() {
        return Ok(built.response);
    }
    let rendered = if arguments.get_flag("json") {
        canonical_json(&built.response.payload).map_err(|error| error.to_string())?
    } else {
        let matrix: quoin_assurance::matrix::MatrixOutput =
            serde_json::from_value(built.response.payload.clone())
                .map_err(|error| error.to_string())?;
        quoin_assurance::matrix::render_markdown(&matrix)
            .trim_end()
            .to_owned()
    };
    let mut diagnostics = built.audit_diagnostics;
    diagnostics.extend(built.response.diagnostics);
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics,
        outcome: built.response.outcome,
    })
}

/// The matrix `matrix.build` answered, and the audit diagnostics that came
/// with the assembly.
pub(crate) struct MatrixBuilt {
    /// The `matrix.build` response.
    pub(crate) response: Response,
    /// The diagnostics of the `auditor.audit` step.
    pub(crate) audit_diagnostics: Vec<quoin_core::protocol::Diagnostic>,
}

/// What [`build`] produced.
pub(crate) enum Built {
    /// The matrix step ran; its response may still decline.
    Matrix(MatrixBuilt),
    /// An earlier step declined; its response is the command's answer.
    Declined(Response),
}

/// Assemble the audit over `repo` and join it into the evidence-backed matrix.
///
/// The one path `quoin matrix` and `quoin release snapshot` share (FR-115).
pub(crate) fn build(
    options: &AuditOptions,
    repo: &str,
    run_observations: &[crate::run_observation::RunObservationFile],
) -> Result<Built, String> {
    let audited = match assemble_with(options, repo)? {
        Assembly::Audited(audited) => audited,
        Assembly::Declined(response) => return Ok(Built::Declined(response)),
    };
    let report = audited
        .response
        .payload
        .get("report")
        .cloned()
        .ok_or_else(|| "auditor.audit did not return report".to_owned())?;
    let obligations: Vec<quoin_quire_types::Obligation> = serde_json::from_value(
        audited
            .coverage
            .get("obligations")
            .cloned()
            .ok_or_else(|| "quire.coverage did not return obligations".to_owned())?,
    )
    .map_err(|error| format!("quire.coverage obligations are malformed: {error}"))?;
    let analysis = crate::analysis_evidence::compute(
        repo,
        options.module_paths(),
        &audited.store_inputs,
        &obligations,
        run_observations,
    )?;
    let evidence_report = serde_json::to_value(&analysis.report)
        .map_err(|error| format!("analysis evidence report is malformed: {error}"))?;
    let mut request = serde_json::Map::new();
    request.insert(
        "bindings".to_owned(),
        serde_json::json!({
            "schemaVersion": quoin_evidence::STORE_SCHEMA_VERSION,
            "bindings": audited.bindings,
        }),
    );
    request.insert("audit".to_owned(), report);
    let coverage = evidence_report
        .get("coverage_matrix")
        .cloned()
        .ok_or_else(|| "analysis evidence report omitted coverage_matrix".to_owned())?;
    request.insert("coverage".to_owned(), coverage);
    request.insert(
        "analysis_evidence".to_owned(),
        serde_json::json!({
            "record_binders": evidence_report.get("record_binders").cloned().unwrap_or_else(|| serde_json::json!([])),
            "evidence_results": evidence_report.get("evidence_results").cloned().unwrap_or_else(|| serde_json::json!([])),
        }),
    );
    let request = serde_json::Value::Object(request);
    let response = invoke("matrix.build", &request)?;
    Ok(Built::Matrix(MatrixBuilt {
        response,
        audit_diagnostics: audited.response.diagnostics,
    }))
}
