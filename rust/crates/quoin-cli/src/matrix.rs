// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin matrix`: the evidence-backed test matrix, rendered (FR-115).
//!
//! Read-only. It assembles the audit exactly as `quoin evidence audit` does
//! (the shared [`assemble`]), hands the three inputs to `matrix.build`, and
//! prints the result. It writes nothing, anywhere, and gates nothing: the
//! static axis is gated by `quire matrix --strict` and the evidence axis by
//! `quoin evidence audit --strict`.

use std::collections::BTreeMap;

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_trace_rs::implements;
use quoin_config::{ConfigService, IncidentLog, ProcessEnvironment, QuoinConfig};
use quoin_core::protocol::{Response, canonical_json};

use crate::core_runtime::{Failure, invoke};
use crate::evidence::audit::{
    Assembly, AuditOptions, assemble_for_matrix_with, module_arg, policy_args,
};
use crate::evidence::{json_arg, repo_arg, required};

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Render the evidence-backed test matrix, per criterion.";
const DEFAULT_MAX_REQUEST_BYTES: usize = 64 * 1024 * 1024;

const HARD_MAX_REQUEST_BYTES: usize = quoin_core::ops::matrix::MAX_MATRIX_BUILD_BYTES;

pub(crate) fn command() -> Command {
    Command::new("matrix")
        .about(ABOUT)
        .arg(repo_arg())
        .arg(
            module_arg().help("Limit assembly to this traceability module (repeatable)"),
        )
        .arg(
            Arg::new("max-request-bytes")
                .long("max-request-bytes")
                .value_name("BYTES")
                .value_parser(clap::value_parser!(usize))
                .help(format!(
                    "Set the serialized matrix request limit in bytes (default {DEFAULT_MAX_REQUEST_BYTES}; hard maximum {HARD_MAX_REQUEST_BYTES}); overrides quoin.matrix_max_request_bytes"
                )),
        )
        .arg(json_arg())
        .arg(
            Arg::new("run-observation")
                .long("run-observation")
                .action(ArgAction::Append)
                .num_args(1)
                .value_name("FILE"),
        )
        .args(crate::formal_rules::trust_arguments())
        .args(policy_args())
}

#[implements("FR-115-AC-27", "FR-115-AC-28")]
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    eprintln!("Building evidence-backed matrix...");
    let run_observation_paths = arguments
        .get_many::<String>("run-observation")
        .map(|paths| paths.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let run_observations = crate::run_observation::read_run_observations(&run_observation_paths)?;
    let signature_verifier = crate::analysis_evidence::signature_verifier(arguments)?;
    let repo = required(arguments, "repo")?;
    let request_limit = selected_request_limit(arguments)?;
    let options = AuditOptions::from_arguments(arguments);
    let built = match build_with_limit(
        &options,
        &repo,
        &run_observations,
        signature_verifier.as_ref(),
        request_limit,
    )? {
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
#[implements("FR-165-AC-21")]
pub(crate) fn build(
    options: &AuditOptions,
    repo: &str,
    run_observations: &[crate::run_observation::RunObservationFile],
    signature_verifier: Option<&crate::analysis_evidence::SignatureVerifier>,
) -> Result<Built, String> {
    build_with_limit(
        options,
        repo,
        run_observations,
        signature_verifier,
        DEFAULT_MAX_REQUEST_BYTES,
    )
}

#[implements("FR-165-AC-21", "FR-115-AC-27")]
fn build_with_limit(
    options: &AuditOptions,
    repo: &str,
    run_observations: &[crate::run_observation::RunObservationFile],
    signature_verifier: Option<&crate::analysis_evidence::SignatureVerifier>,
    request_limit: usize,
) -> Result<Built, String> {
    let audited = match assemble_for_matrix_with(options, repo)? {
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
    let (analysis, design_coverage, design_errors, retained) =
        match crate::analysis_evidence::compute_for_matrix(
            repo,
            options.module_paths(),
            &audited.store_inputs,
            &obligations,
            run_observations,
            signature_verifier,
        ) {
            Ok(computed) => computed,
            Err(Failure::Refused(response)) => return Ok(Built::Declined(response)),
            Err(Failure::Message(message)) => return Err(message),
        };
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
    if let Some(coverage) = evidence_report.get("coverage_matrix") {
        request.insert("coverage".to_owned(), coverage.clone());
    }
    // Display the design rows from the same Quire result as the analysis items.
    request.insert("design_coverage".to_owned(), design_coverage);
    request.insert(
        "analysis_evidence".to_owned(),
        serde_json::json!({
            "record_binders": evidence_report.get("record_binders").cloned().unwrap_or_else(|| serde_json::json!([])),
            "evidence_results": evidence_report.get("evidence_results").cloned().unwrap_or_else(|| serde_json::json!([])),
        }),
    );
    let bindings: Vec<quoin_evidence::types::Binding> =
        serde_json::from_value(audited.bindings.clone())
            .map_err(|error| format!("the binding graph is malformed: {error}"))?;
    request.insert(
        "runs".to_owned(),
        serde_json::to_value(quoin_assurance::matrix::bounded_runs(&bindings, &retained))
            .map_err(|error| format!("the retained runs are malformed: {error}"))?,
    );
    let request = serde_json::Value::Object(request);
    ensure_request_size(&request, request_limit)?;
    let response = invoke("matrix.build", &request)?;
    let mut diagnostics = audited.response.diagnostics;
    diagnostics.extend(design_errors.into_iter().map(|error| {
        quoin_core::protocol::Diagnostic {
            code: "design-matrix-partial".to_owned(),
            message: format!("quire-rs dropped a design relation while retaining valid matrix entries: {error}"),
            context: BTreeMap::default(),
        }
    }));
    Ok(Built::Matrix(MatrixBuilt {
        response,
        audit_diagnostics: diagnostics,
    }))
}

#[implements("FR-115-AC-27")]
fn selected_request_limit(arguments: &ArgMatches) -> Result<usize, String> {
    let configured = if arguments.get_one::<usize>("max-request-bytes").is_none() {
        let environment = ProcessEnvironment;
        let context = crate::config::context()?;
        let service = ConfigService::<QuoinConfig>::for_plugin(&environment, &context);
        service
            .get(&mut IncidentLog::new())
            .value
            .matrix_max_request_bytes
    } else {
        None
    };
    select_request_limit(
        arguments.get_one::<usize>("max-request-bytes").copied(),
        configured,
    )
}

fn select_request_limit(cli: Option<usize>, configured: Option<usize>) -> Result<usize, String> {
    let limit = cli.or(configured).unwrap_or(DEFAULT_MAX_REQUEST_BYTES);
    if limit == 0 {
        return Err("the matrix request limit must be greater than zero".to_owned());
    }
    if limit > HARD_MAX_REQUEST_BYTES {
        return Err(format!(
            "the selected matrix request limit is {limit} bytes; the hard maximum is {HARD_MAX_REQUEST_BYTES} bytes. Reduce the configured value or use repeatable --module PATH to scope the matrix"
        ));
    }
    Ok(limit)
}

#[implements("FR-115-AC-27")]
fn ensure_request_size(request: &serde_json::Value, limit: usize) -> Result<(), String> {
    let observed = serde_json::to_vec(request)
        .map_err(|error| format!("cannot measure the matrix request: {error}"))?
        .len();
    if observed <= limit {
        return Ok(());
    }
    let error = if observed <= HARD_MAX_REQUEST_BYTES {
        format!(
            "matrix.build request is {observed} bytes, over the selected {limit}-byte limit. Raise it with --max-request-bytes {observed}, `quoin config set matrix_max_request_bytes {observed}` for the user setting, or `matrix_max_request_bytes: {observed}` in project .ix/config.d/quoin.yaml (quoin.matrix_max_request_bytes); reduce the request with repeatable --module PATH. The hard maximum is {HARD_MAX_REQUEST_BYTES} bytes"
        )
    } else {
        format!(
            "matrix.build request is {observed} bytes, over the selected {limit}-byte limit and the {HARD_MAX_REQUEST_BYTES}-byte hard maximum. This request cannot be raised further; reduce its size with repeatable --module PATH"
        )
    };
    Err(error)
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_MAX_REQUEST_BYTES, HARD_MAX_REQUEST_BYTES, ensure_request_size,
        select_request_limit,
    };
    use ix_trace_rs::trace;
    use serde_json::json;

    #[trace("FR-115-AC-27")]
    #[test]
    fn request_limit_accepts_hand_written_under_value_and_refuses_over_value_actionably() {
        let request = json!({"request": "fixture"});
        let observed = serde_json::to_vec(&request)
            .expect("request serializes")
            .len();
        assert!(ensure_request_size(&request, observed + 1).is_ok());

        let error = ensure_request_size(&request, observed - 1)
            .expect_err("request exceeds the hand-written limit");
        assert!(error.contains(&format!("request is {observed} bytes")));
        assert!(error.contains(&format!("selected {}-byte limit", observed - 1)));
        assert!(error.contains("--max-request-bytes"));
        assert!(error.contains("quoin.matrix_max_request_bytes"));
        assert!(error.contains("--module PATH"));
    }

    #[trace("FR-115-AC-27")]
    #[test]
    fn request_limit_override_precedence_and_hard_ceiling_are_enforced() {
        assert_eq!(
            select_request_limit(None, None).expect("default limit"),
            DEFAULT_MAX_REQUEST_BYTES
        );
        assert_eq!(
            select_request_limit(None, Some(32)).expect("configured limit"),
            32
        );
        assert_eq!(
            select_request_limit(Some(48), Some(32)).expect("CLI overrides config"),
            48
        );
        assert!(select_request_limit(Some(0), None).is_err());
        let error = select_request_limit(Some(HARD_MAX_REQUEST_BYTES + 1), None)
            .expect_err("hard ceiling cannot be overridden");
        assert!(error.contains("hard maximum"));
    }
}
