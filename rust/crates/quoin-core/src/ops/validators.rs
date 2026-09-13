// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `validators`: deterministic repository QA-gate validation (quoin#412).
//!
//! Stage 2's cutover. `src/validators/` was chosen for the first end-to-end
//! integration precisely because it is trivial — 164 lines behind one command —
//! so the IPC boundary, the exit taxonomy and the delete step are proved on a
//! domain that cannot itself surprise anyone.
//!
//! # Why the request carries the files instead of a path
//!
//! `quoin-core`'s library half must be decidable **without a filesystem**:
//! `tests/tc_library_containment.rs` audits every `src/**.rs` but `main.rs`
//! with `engineering_assurance::source_audit` and fails on a host capability.
//! An operation taking `{ repo }` and walking it acquires `Filesystem` and
//! fails that gate. Pre-resolving the tree in `main.rs` would move the
//! capability to the one file allowed to hold it, but `main.rs` does four
//! things — argv, stdin, dispatch, two writes and an exit — and must keep doing
//! only four; and a request that depends on the filesystem cannot be
//! difftested, because `quoin-difftest` feeds **the same stdin to two
//! processes** and its `Request` enum has no filesystem variant. So the request
//! carries the file map, which is also the shape the golden corpus already uses
//! (`quoin-validators/tests/golden/cases.json`).
//!
//! The classification of those files — which are shell scripts, which are
//! wiring — stays in Rust, in [`quoin_validators::inspect_empty_gates_in`],
//! which re-derives it from the map. A caller that pre-filters its walk
//! therefore cannot change a verdict as long as it sends a superset; its filter
//! is a transport optimisation, not a decision.
//!
//! # Why a finding is not a failure
//!
//! [`quoin_validators::Verdict::Failing`] maps to process exit 1, and so does
//! [`crate::protocol::Outcome::Partial`]. They are not the same 1 and must not
//! be conflated. `Partial` says "the payload is complete but something about
//! the OPERATION was qualified". A repository that has gate findings did not
//! qualify the operation — the operation succeeded and the findings ARE the
//! answer. So this op is [`crate::protocol::Outcome::Ok`] whether the report is
//! empty or not, and `--strict`'s non-zero exit stays a caller policy derived
//! from the payload.
//!
//! That split is already the library's own opinion:
//! [`quoin_validators::GateReport::verdict`] takes `strict` as an argument
//! rather than holding it as state, because strictness belongs to whoever asked.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

/// The largest `validators.run` request this domain will accept, in bytes.
///
/// A bound rather than "as much as arrives": stdin is an untrusted stream
/// (rust-style §11). The caller sends only the files the validator can classify
/// — shell scripts and build wiring — so quoin's own repository arrives in tens
/// of kilobytes; 16 MiB is far past anything a real tree produces and still
/// refuses a stream.
pub const MAX_RUN_REQUEST_BYTES: usize = 16 * 1024 * 1024;

/// The request accepted by `validators.run`.
///
/// A snapshot of the repository, not a path to it. `deny_unknown_fields` so a
/// caller that misspells a field is refused rather than silently ignored — a
/// field the boundary drops is a field the caller believes it sent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    /// Every file the caller found, keyed by repository-relative,
    /// `/`-separated path, holding the file's lines split on `\n`.
    ///
    /// `null` means the caller found the path but could not read it. That is
    /// not the same as omitting it: an unreadable file still classifies as a
    /// shell script or as wiring, and refuses only if the analysis reaches for
    /// its text — which is the on-demand read the TypeScript oracle performs.
    ///
    /// Lines rather than one string because that is the shape the golden corpus
    /// already carries. Joining the lines with a newline reconstructs the file,
    /// so a caller must split on a newline alone and leave any carriage return
    /// on the line.
    pub files: BTreeMap<String, Option<Vec<String>>>,

    /// Directories the caller could not list, repository-relative. The empty
    /// string names the repository root itself.
    ///
    /// Separate from an unreadable file because the walk **aborts** on one: a
    /// subtree nobody could read means the answer would be computed over a
    /// repository nobody has seen, and an empty result must mean the validator
    /// looked and found nothing.
    #[serde(default)]
    pub unlistable: Vec<String>,
}

/// The payload `validators.run` writes to stdout.
///
/// One field, named `findings`, because that is the byte shape
/// `quoin validate --json` has always emitted and the cutover is not licence to
/// change a user-visible document.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunPayload {
    /// Every finding, ordered by `(path, line, obligation)`.
    pub findings: Vec<quoin_validators::EmptyGateFinding>,
}

/// Map a validator refusal onto the boundary taxonomy.
///
/// The distinction is whose mistake it was.
/// [`quoin_validators::ErrorCode::RepoRootUnreadable`] means the caller could
/// not list the root it asked about — understood, and refused by a stated rule,
/// so [`CoreErrorCode::Refused`] (exit 2). The other two describe a tree that
/// WAS listable failing under the caller mid-walk, which is the environment
/// failing rather than the request being wrong: [`CoreErrorCode::Io`] (exit 4).
///
/// # The wildcard is not laziness, and it is not free
///
/// [`quoin_validators::ErrorCode`] is `#[non_exhaustive]`, so this match MUST
/// carry a `_` arm and the compiler therefore will NOT tell us when a fourth
/// code is added upstream — it will quietly take the `Io` branch and a caller
/// mistake will be reported as a boundary failure. The compiler cannot be the
/// guard here, so a test is: `every_validator_code_has_a_deliberate_mapping`
/// pins `ErrorCode::ALL.len()`, and adding a code upstream fails it until
/// someone decides which side of the taxonomy that code belongs on.
fn refusal(error: &quoin_validators::ValidatorError) -> CoreError {
    let decided = match error.code() {
        quoin_validators::ErrorCode::RepoRootUnreadable => Some(CoreErrorCode::Refused),
        quoin_validators::ErrorCode::DirectoryUnreadable
        | quoin_validators::ErrorCode::FileUnreadable => Some(CoreErrorCode::Io),
        // Not decided here: a code nobody has read yet.
        _ => None,
    };
    // An unnamed refusal is not one we can attribute to the caller, so it is
    // reported as the boundary failing rather than as the request being wrong.
    let code = decided.unwrap_or(CoreErrorCode::Io);
    CoreError::new(code, error.to_string())
        .with_context("op", "validators.run")
        .with_context("validator_code", error.code().as_str().to_owned())
        .with_context("path", error.path().display().to_string())
}

/// Build the in-memory repository the analysis runs over.
fn snapshot(request: RunRequest) -> quoin_validators::MemoryRepo {
    let mut repo = quoin_validators::MemoryRepo::new();
    for (path, lines) in request.files {
        repo = match lines {
            Some(lines) => repo.with_file(path, lines.join("\n")),
            None => repo.with_unreadable_file(path),
        };
    }
    for path in request.unlistable {
        repo = repo.with_unlistable_directory(path);
    }
    repo
}

/// Answer a `validators.run`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`RunRequest`].
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_RUN_REQUEST_BYTES`], or when the caller could not list the root.
/// - [`CoreErrorCode::Io`] when a file or directory the caller reported as
///   present turns out to be unreadable where the analysis needs it.
pub fn run(request: &serde_json::Value) -> Result<Response, CoreError> {
    // Measured on the parsed value rather than on raw stdin: the dispatcher has
    // already read and parsed the stream, so this is the honest place to state
    // a size the DOMAIN refuses, distinct from any transport ceiling.
    let size = serde_json::to_vec(request)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?
        .len();
    if size > MAX_RUN_REQUEST_BYTES {
        return Err(
            CoreError::new(CoreErrorCode::Refused, "request exceeds the accepted size")
                .with_context("op", "validators.run")
                .with_context("limit_bytes", MAX_RUN_REQUEST_BYTES.to_string())
                .with_context("observed_bytes", size.to_string()),
        );
    }

    let request: RunRequest = serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", "validators.run")
    })?;

    let findings = quoin_validators::inspect_empty_gates_in(&mut snapshot(request))
        .map_err(|e| refusal(&e))?;

    let payload = serde_json::to_value(RunPayload { findings })
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use crate::protocol::Outcome;

    /// A wired gate that counts forbidden matches and never asserts the count —
    /// the one defect `inspect_empty_gates` reports.
    fn a_repository_with_one_finding() -> serde_json::Value {
        serde_json::json!({
            "files": {
                "Makefile": ["gate:", "\tsh scripts/gate.sh"],
                "scripts/gate.sh": [
                    "#!/bin/sh",
                    "# Gate for FR-001: no unwrap in src",
                    "grep -rn \"unwrap()\" src/ | wc -l"
                ]
            }
        })
    }

    /// A repository with no shell gates at all: the analysis completes and finds
    /// nothing, which is a success carrying an empty list — not an error, and
    /// not an absent field.
    #[test]
    fn a_clean_repository_is_a_success_with_an_empty_findings_list() {
        let response = run(&serde_json::json!({ "files": {} })).unwrap();
        assert_eq!(response.outcome, Outcome::Ok);
        assert_eq!(response.payload["findings"], serde_json::json!([]));
        assert!(response.diagnostics.is_empty());
    }

    /// The load-bearing decision of this module, asserted rather than only
    /// documented: findings do NOT make the operation partial. If this ever
    /// flips to `Partial`, `quoin validate` starts exiting 1 without `--strict`
    /// and every CI job that runs it changes meaning silently.
    #[test]
    fn findings_are_a_successful_answer_not_a_qualified_one() {
        let response = run(&a_repository_with_one_finding()).unwrap();
        assert_eq!(response.outcome, Outcome::Ok);
        assert_eq!(response.outcome.code(), 0);
        assert_eq!(response.payload["findings"][0]["path"], "scripts/gate.sh");
        assert_eq!(response.payload["findings"][0]["line"], 3);
        assert_eq!(response.payload["findings"][0]["wiredBy"], "Makefile");
    }

    /// The classification is re-derived here, so a caller's pre-filter cannot
    /// decide a verdict: sending files the validator ignores changes nothing.
    #[test]
    fn an_over_broad_file_map_yields_the_same_findings() {
        let mut broad = a_repository_with_one_finding();
        let files = broad["files"].as_object_mut().unwrap();
        files.insert("README.md".to_owned(), serde_json::json!(["# quoin"]));
        files.insert(
            "src/index.ts".to_owned(),
            serde_json::json!(["export const x = 1;"]),
        );

        assert_eq!(
            run(&broad).unwrap().payload,
            run(&a_repository_with_one_finding()).unwrap().payload
        );
    }

    /// A root the caller could not list is the caller's mistake, refused by a
    /// stated rule rather than reported as a boundary failure.
    #[test]
    fn an_unlistable_root_is_refused_not_an_internal_error() {
        let error = run(&serde_json::json!({ "files": {}, "unlistable": [""] })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(error.outcome().code(), 2);
        assert_eq!(error.context["validator_code"], "QV-E001");
        assert_eq!(error.context["op"], "validators.run");
    }

    /// An unlistable subtree is the environment failing under a tree that was
    /// listable, which is exit 4 and not exit 2.
    #[test]
    fn an_unlistable_subtree_is_an_io_refusal() {
        let error =
            run(&serde_json::json!({ "files": {}, "unlistable": ["scripts"] })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Io);
        assert_eq!(error.outcome().code(), 4);
        assert_eq!(error.context["validator_code"], "QV-E002");
        assert_eq!(error.context["path"], "scripts");
    }

    /// A file the caller could not read is present for classification and
    /// refuses only when the analysis reaches for its text. `Makefile` is
    /// reached only because a script declares a negative gate claim.
    #[test]
    fn an_unreadable_file_the_analysis_needs_is_an_io_refusal() {
        let error = run(&serde_json::json!({
            "files": {
                "Makefile": null,
                "scripts/gate.sh": [
                    "# Gate for FR-001: no unwrap in src",
                    "grep -rn \"unwrap()\" src/ | wc -l"
                ]
            }
        }))
        .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Io);
        assert_eq!(error.context["validator_code"], "QV-E003");
        assert_eq!(error.context["path"], "Makefile");
    }

    /// The same unreadable `Makefile`, never reached because no script declares
    /// a negative claim: a clean verdict, not a refusal.
    #[test]
    fn an_unreadable_file_the_analysis_never_reads_is_not_a_refusal() {
        let response = run(&serde_json::json!({
            "files": {
                "Makefile": null,
                "scripts/report.sh": ["echo hello"]
            }
        }))
        .unwrap();
        assert_eq!(response.outcome, Outcome::Ok);
        assert_eq!(response.payload["findings"], serde_json::json!([]));
    }

    #[test]
    fn a_misspelled_field_is_refused_not_ignored() {
        let error = run(&serde_json::json!({ "filez": {} })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
        assert_eq!(error.outcome().code(), 3);
    }

    #[test]
    fn a_missing_files_map_is_refused() {
        let error = run(&serde_json::json!({})).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
    }

    /// `unlistable` defaults to empty, so the common request is just the map.
    #[test]
    fn unlistable_is_optional() {
        assert!(run(&serde_json::json!({ "files": {} })).is_ok());
    }

    /// `refusal`'s match needs a `_` arm because [`quoin_validators::ErrorCode`]
    /// is `#[non_exhaustive]`, which means the compiler cannot report a fourth
    /// code as an unhandled case — it would silently take the `Io` branch and
    /// report a caller's mistake as a boundary failure.
    ///
    /// This is the guard that replaces the one the compiler cannot give. It is
    /// deliberately a count and not a loop over `ALL`: a loop would re-run the
    /// same `match` on each variant and agree with itself no matter what was
    /// added, which is the "test that reads the string it guards" failure
    /// quoin#443 was about. When this fails, decide which side of the taxonomy
    /// the new code belongs on, add its arm above, then raise the count.
    #[test]
    fn every_validator_code_has_a_deliberate_mapping() {
        assert_eq!(
            quoin_validators::ErrorCode::ALL.len(),
            3,
            "`quoin-validators` gained or lost an error code. `refusal`'s \
             wildcard will have silently mapped it to CORE_IO; give it an \
             explicit arm and update this count."
        );
    }
}
