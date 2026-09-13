// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `completeness`: declared-vocabulary coverage and its verdict (FR-037).
//!
//! **Why these operations take content and not a path** (quoin#445).
//! `quoin-core`'s library half is audited as a `ReusableLibrary` by
//! `tests/tc_library_containment.rs`: an operation that takes a directory and
//! walks it acquires a host capability the boundary has said it does not have.
//! So the walk and the reads stay in the command shell, which is where a CLI's
//! I/O belongs, and the decision crosses as bytes. The shell asks [`schema_refs`]
//! which frontmatter schemas a manifest names before it reads them, rather than
//! guessing at the module layout — that layout is a rule this crate owns.
//!
//! [`read_frontmatter`] is here and not in `ops::assurance` for the reason
//! `bundle.rs` gives: ONE reader serves both the completeness sweep (FR-037) and
//! the assurance case view (FR-040), and a second reader over the same files
//! would drift.
//!
//! **There is no error mapping here, and that is a finding rather than an
//! omission.** [`quoin_completeness::CompletenessErrorCode`] carries exactly two
//! conditions, `QCMP-001` (bundle root is not a directory) and `QCMP-002`
//! (bundle root could not be walked), and both are questions about a filesystem
//! this half no longer touches. Neither can be raised across the boundary.
//! `a_third_completeness_error_code_would_need_a_mapping` pins the count so a
//! third condition added upstream fails here rather than silently taking a
//! default arm — the quoin#443 failure mode, where a loop over `ALL` re-ran the
//! same match and agreed with itself.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

/// The largest module manifest this domain will accept, in bytes.
///
/// A bound rather than "as much as arrives": stdin is an untrusted stream
/// (rust-style §11). quoin's own largest module manifest is under 40 KiB, so
/// 1 MiB is far past anything a real module carries and still refuses a stream.
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

/// The request accepted by `completeness.schema_refs`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemaRefsRequest {
    /// One module's `manifest.yaml`, as text.
    pub manifest: String,
}

/// The payload `completeness.schema_refs` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemaRefsPayload {
    /// Every `frontmatter_schema_ref` the manifest's declared coverage reaches
    /// for, deduplicated, in manifest order. Paths are relative to the module
    /// root; the caller reads them and sends the content back.
    pub refs: Vec<String>,
}

/// Answer a `completeness.schema_refs`.
///
/// A manifest that declares no vocabulary coverage, or does not parse at all,
/// yields an empty list rather than an error: this crate's whole posture is
/// that a bad input is **reported, not raised**, and the assessment that
/// follows reports it as an unresolved declaration.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`SchemaRefsRequest`].
/// - [`CoreErrorCode::Refused`] when the manifest exceeds [`MAX_MANIFEST_BYTES`].
pub fn schema_refs(request: &serde_json::Value) -> Result<Response, CoreError> {
    let request: SchemaRefsRequest = serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", "completeness.schema_refs")
    })?;

    if request.manifest.len() > MAX_MANIFEST_BYTES {
        return Err(
            CoreError::new(CoreErrorCode::Refused, "manifest exceeds the accepted size")
                .with_context("op", "completeness.schema_refs")
                .with_context("limit_bytes", MAX_MANIFEST_BYTES.to_string())
                .with_context("observed_bytes", request.manifest.len().to_string()),
        );
    }

    let payload = serde_json::to_value(SchemaRefsPayload {
        refs: quoin_completeness::schema_refs_of(&request.manifest),
    })
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// The request accepted by `completeness.read_frontmatter`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReadFrontmatterRequest {
    /// Every markdown document the caller read, in the order it walked them.
    pub documents: Vec<quoin_completeness::DocumentSource>,
    /// Documents the caller found and could not open, and why. Optional: a walk
    /// that hit no OS error should not have to send an empty list.
    #[serde(default)]
    pub unreadable: Vec<quoin_completeness::UnreadableDocument>,
}

/// Answer a `completeness.read_frontmatter`.
///
/// One reader for the whole boundary. The completeness sweep (FR-037) and the
/// assurance case view (FR-040) both need every document's leading `---` block
/// and the body under it, and two readers over the same files would drift — the
/// second written by whoever needed a field the first did not expose.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`ReadFrontmatterRequest`].
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_ASSESS_BUNDLE_BYTES`].
pub fn read_frontmatter(request: &serde_json::Value) -> Result<Response, CoreError> {
    let size = request_size(request)?;
    if size > MAX_ASSESS_BUNDLE_BYTES {
        return Err(refusal(
            "completeness.read_frontmatter",
            MAX_ASSESS_BUNDLE_BYTES,
            size,
        ));
    }

    let request: ReadFrontmatterRequest = serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", "completeness.read_frontmatter")
    })?;

    let payload = serde_json::to_value(quoin_completeness::frontmatter_from_sources(
        &request.documents,
        &request.unreadable,
    ))
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// The largest `completeness.assess_bundle` request this domain will accept, in
/// bytes.
///
/// A whole bundle's documents arrive in one request — quoin's own `spec/` is
/// roughly 350 markdown files — plus every module manifest and the frontmatter
/// schemas it names. 16 MiB is past anything a real corpus produces and still
/// refuses a stream. Deliberately the same ceiling as
/// [`MAX_BUILD_CASE_BYTES`](crate::ops::assurance::MAX_BUILD_CASE_BYTES): the
/// two operations read the same bundle, and a bundle that one accepted and the
/// other refused would be a boundary that contradicts itself.
pub const MAX_ASSESS_BUNDLE_BYTES: usize = 16 * 1024 * 1024;

/// Answer a `completeness.assess_bundle`.
///
/// **The request IS the [`AssessInput`]**, with no wrapper object, for the same
/// reason `assurance.build_case` takes a bare `CaseInput`: the retained
/// function's own input type is the contract, and a wrapper would add a key
/// that exists on neither side of it.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an [`AssessInput`].
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_ASSESS_BUNDLE_BYTES`].
///
/// [`AssessInput`]: quoin_completeness::AssessInput
pub fn assess_bundle(request: &serde_json::Value) -> Result<Response, CoreError> {
    let size = request_size(request)?;
    if size > MAX_ASSESS_BUNDLE_BYTES {
        return Err(refusal(
            "completeness.assess_bundle",
            MAX_ASSESS_BUNDLE_BYTES,
            size,
        ));
    }

    let input: quoin_completeness::AssessInput =
        serde_json::from_value(request.clone()).map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string())
                .with_context("op", "completeness.assess_bundle")
        })?;

    let payload = serde_json::to_value(quoin_completeness::assess_sources(&input))
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    fn refs(request: &serde_json::Value) -> Result<serde_json::Value, CoreError> {
        schema_refs(request).map(|response| response.payload)
    }

    fn assess(request: &serde_json::Value) -> Result<serde_json::Value, CoreError> {
        assess_bundle(request).map(|response| response.payload)
    }

    const MANIFEST: &str = "name: iso\n\
        artifact_types:\n\
        \x20 - name: NFR\n\
        \x20   frontmatter_schema_ref: schemas/nfr.json\n\
        traceability:\n\
        \x20 vocabulary_coverage:\n\
        \x20   - name: quality-characteristics\n\
        \x20     from: NFR\n\
        \x20     field: quality_attribute\n\
        \x20     check: vocabulary-coverage\n";

    /// Trace: FR-037, FR-096
    /// Provenance: agent-ix/quoin#445
    #[test]
    fn the_refs_a_manifest_names_come_back_so_the_shell_knows_what_to_read() {
        let payload = refs(&serde_json::json!({ "manifest": MANIFEST })).unwrap();
        assert_eq!(payload["refs"], serde_json::json!(["schemas/nfr.json"]));
    }

    /// Trace: FR-096
    /// Provenance: agent-ix/quoin#445
    #[test]
    fn a_misspelled_schema_refs_field_is_refused_not_ignored() {
        let error = refs(&serde_json::json!({ "manifestt": "" })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
        assert_eq!(error.outcome().code(), 3);
    }

    /// Trace: FR-096
    /// Provenance: agent-ix/quoin#445
    #[test]
    fn an_oversized_manifest_is_refused_before_it_is_parsed() {
        let manifest = "#".repeat(MAX_MANIFEST_BYTES + 1);
        let error = refs(&serde_json::json!({ "manifest": manifest })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(error.outcome().code(), 2);
        assert_eq!(
            error.context.get("limit_bytes").map(String::as_str),
            Some("1048576")
        );
    }

    /// Trace: FR-037, FR-096
    /// Provenance: agent-ix/quoin#445
    ///
    /// The end-to-end shape, over content only: a bundle whose one NFR claims
    /// one of two declared values is `CONDITIONAL`, with the unowned value
    /// named. Zero declarations would be `UNCHECKED`, which is the distinction
    /// FR-037 exists to keep visible, so both are asserted here rather than
    /// only the happy one.
    #[test]
    fn an_assessment_crosses_the_boundary_with_no_filesystem_at_all() {
        let request = serde_json::json!({
            "bundle_root": "spec",
            "strict": false,
            "documents": [
                { "path": "a.md", "raw": "---\ntype: NFR\nquality_attribute: security\n---\nbody\n" }
            ],
            "modules": [
                {
                    "label": "iso",
                    "manifest": MANIFEST,
                    "schemas": {
                        "schemas/nfr.json": {
                            "text": "{\"properties\":{\"quality_attribute\":{\"enum\":[\"security\",\"reliability\"]}}}"
                        }
                    }
                }
            ]
        });
        let payload = assess(&request).unwrap();
        assert_eq!(payload["bundleRoot"], "spec");
        assert_eq!(payload["verdict"], "CONDITIONAL");
        assert_eq!(
            payload["vocabularies"],
            serde_json::json!(["quality-characteristics"])
        );
        assert_eq!(payload["findings"][0]["value"], "reliability");

        let unchecked = serde_json::json!({
            "bundle_root": "spec",
            "strict": false,
            "documents": [],
            "modules": [],
        });
        assert_eq!(assess(&unchecked).unwrap()["verdict"], "UNCHECKED");
    }

    /// Trace: FR-096
    /// Provenance: agent-ix/quoin#445
    #[test]
    fn a_misspelled_assess_field_is_refused_not_ignored() {
        let error = assess(&serde_json::json!({
            "bundle_root": "spec",
            "strict": false,
            "documents": [],
            "modules": [],
            "moduels": [],
        }))
        .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
        assert_eq!(error.outcome().code(), 3);
    }

    /// Trace: FR-096
    /// Provenance: agent-ix/quoin#445
    ///
    /// `unreadable` defaults to empty — a caller whose walk hit no OS error
    /// should not have to send an empty list — but every other field is
    /// required, because a missing `documents` and an empty `documents` are
    /// different claims about the bundle.
    #[test]
    fn unreadable_is_optional_and_documents_is_not() {
        assert!(
            assess(&serde_json::json!({
                "bundle_root": "spec", "strict": false, "documents": [], "modules": []
            }))
            .is_ok()
        );
        let error = assess(&serde_json::json!({
            "bundle_root": "spec", "strict": false, "modules": []
        }))
        .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
    }

    /// Trace: FR-096
    /// Provenance: agent-ix/quoin#445
    ///
    /// A caller's own read failures are reported, not dropped: a file the walk
    /// found and could not open is the same kind of gap as one whose YAML is
    /// broken, and losing it would turn a broken bundle into a clean one.
    #[test]
    fn a_read_failure_the_caller_reports_survives_the_crossing() {
        let payload = assess(&serde_json::json!({
            "bundle_root": "spec",
            "strict": false,
            "documents": [],
            "unreadable": [{ "path": "locked.md", "reason": "EACCES" }],
            "modules": [],
        }))
        .unwrap();
        assert_eq!(payload["unreadable"][0]["path"], "locked.md");
    }

    /// Trace: FR-096
    /// Provenance: agent-ix/quoin#445
    ///
    /// The guard the module docs describe. Not a loop over
    /// `CompletenessErrorCode::all()` — that would re-run the same match and
    /// agree with itself (quoin#443). A count, pinned, so a third code arrives
    /// as a failure here and someone decides its exit status deliberately.
    #[test]
    fn a_third_completeness_error_code_would_need_a_mapping() {
        assert_eq!(
            quoin_completeness::CompletenessErrorCode::all().len(),
            2,
            "both known codes are filesystem conditions this half cannot reach; \
             a third needs a deliberate CoreErrorCode mapping and a test for it"
        );
    }
}
