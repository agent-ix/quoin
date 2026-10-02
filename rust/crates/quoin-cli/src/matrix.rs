// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin matrix`: the evidence-backed test matrix, rendered (FR-115).
//!
//! Read-only. It assembles the audit exactly as `quoin evidence audit` does
//! (the shared [`assemble`]), hands the three inputs to `matrix.build`, and
//! prints the result. It writes nothing, anywhere, and gates nothing: the
//! static axis is gated by `quire matrix --strict` and the evidence axis by
//! `quoin evidence audit --strict`.

use clap::{ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::{Diagnostic, Response, canonical_json};

use crate::core_runtime::invoke;
use crate::evidence::audit::{Assembly, assemble, module_arg, policy_args};
use crate::evidence::{json_arg, repo_arg, required, revision};

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Render the evidence-backed test matrix, per criterion.";

pub(crate) fn command() -> Command {
    Command::new("matrix")
        .about(ABOUT)
        .arg(repo_arg())
        .arg(module_arg())
        .arg(json_arg())
        .args(policy_args())
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let head = revision(&repo);
    if head.is_empty() {
        // An auditor given no HEAD skips its behind-HEAD check, and a binding
        // from months ago would then render as fresh. Refuse instead.
        return Ok(refused(
            &CoreError::new(
                CoreErrorCode::Refused,
                "cannot resolve the repository's HEAD commit; `quoin matrix` needs it to judge whether evidence is current",
            )
            .with_context("op", "matrix")
            .with_context("repo", repo),
        ));
    }
    let audited = match assemble(arguments, &repo, Some(&head))? {
        Assembly::Audited(audited) => audited,
        Assembly::Declined(response) => return Ok(response),
    };
    let report = audited
        .response
        .payload
        .get("report")
        .cloned()
        .ok_or_else(|| "auditor.audit did not return report".to_owned())?;
    let mut request = serde_json::Map::new();
    request.insert(
        "bindings".to_owned(),
        serde_json::json!({
            "schemaVersion": quoin_evidence::STORE_SCHEMA_VERSION,
            "bindings": audited.bindings,
        }),
    );
    request.insert("audit".to_owned(), report);
    // Absent stays absent: `quire.coverage` omits an empty matrix, and that
    // omission is what `matrix.build` renders its reason for.
    if let Some(coverage) = audited.coverage.get("coverage_matrix") {
        request.insert("coverage".to_owned(), coverage.clone());
    }
    let request = serde_json::Value::Object(request);
    let built = invoke("matrix.build", &request)?;
    if !built.outcome.carries_payload() {
        return Ok(built);
    }
    let rendered = if arguments.get_flag("json") {
        canonical_json(&built.payload).map_err(|error| error.to_string())?
    } else {
        let matrix: quoin_assurance::matrix::MatrixOutput =
            serde_json::from_value(built.payload.clone()).map_err(|error| error.to_string())?;
        quoin_assurance::matrix::render_markdown(&matrix)
            .trim_end()
            .to_owned()
    };
    let mut diagnostics = audited.response.diagnostics;
    diagnostics.extend(built.diagnostics);
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics,
        outcome: built.outcome,
    })
}

fn refused(error: &CoreError) -> Response {
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic::from(error)],
        outcome: Outcome::Refused,
    }
}
