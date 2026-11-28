// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Read the version-2 execution provenance an export is judged against
//! (FR-185, FR-186).
//!
//! Pure: the caller supplies the signed bundle bytes and the bytes of
//! `spec/evidence/suites.md`. This module checks the envelope's shape, the
//! suite-declaration digest and the payload's own fields, and yields the
//! required population and the expected run digests. It does NOT verify the
//! DSSE signature: the Engineering Assurance verifier authenticates the
//! bundle under its trust policy before Quoin sees it (FR-069), so a caller
//! that skips that step gets the population it supplied, not an authenticated
//! one.

use std::collections::BTreeSet;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ix_trace_rs::implements;
use serde::Deserialize;
use serde_json::Value;

/// The DSSE payload type of the version-2 execution provenance.
pub const PAYLOAD_TYPE: &str = "application/vnd.agent-ix.execution-provenance.v2+json";
/// The in-toto statement type used by GitHub artifact attestations.
pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
/// DSSE payload type used by GitHub artifact attestations.
pub const DSSE_TYPE: &str = "application/vnd.in-toto+json";

/// One expected run: a suite, where its record lives, and its signed digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedRun {
    /// Suite id.
    pub suite: String,
    /// Record path relative to the evidence directory.
    pub record_path: String,
    /// Lowercase SHA-256 of the exact record bytes; `None` when the run was
    /// not produced.
    pub sha256: Option<String>,
}

/// What a valid provenance payload states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The full resolved commit the matrix and runs belong to.
    pub source_revision: String,
    /// Every active obligation in the signed Test Matrix.
    pub required: BTreeSet<String>,
    /// Expected runs, in payload order.
    pub runs: Vec<ExpectedRun>,
    /// Subject names and SHA-256 digests authenticated by the signed bundle.
    pub subjects: BTreeSet<ProvenanceSubject>,
}

/// One artifact bound by the signed in-toto statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProvenanceSubject {
    /// Artifact name relative to the release-artifact directory.
    pub name: String,
    /// Lowercase SHA-256 digest.
    pub sha256: String,
}

/// Why provenance cannot be used. Every variant refuses the export.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProvenanceError {
    /// The envelope is not a DSSE envelope with the expected payload type.
    #[error("execution provenance is not a version-2 DSSE envelope: {0}")]
    Envelope(String),
    /// The payload is not the closed version-2 payload.
    #[error("execution provenance payload is malformed: {0}")]
    Payload(String),
    /// `suites.md` does not hash to the signed `suiteDeclarationsSha256`.
    #[error("suite declarations do not match suiteDeclarationsSha256")]
    SuiteDeclarationsMismatch,
    /// The signed matrix names no active obligation.
    #[error("the signed test matrix names no obligation")]
    EmptyMatrix,
    /// The signed in-toto statement has no usable subjects.
    #[error("execution provenance statement subjects are malformed")]
    Subjects,
    /// Caller-provided artifact subjects differ from the signed subjects.
    #[error("caller subjects differ from the signed execution provenance")]
    SubjectMismatch,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SigstoreBundle {
    dsse_envelope: DsseEnvelope,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DsseEnvelope {
    payload_type: String,
    payload: String,
    signatures: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InTotoStatement {
    #[serde(rename = "_type")]
    statement_type: String,
    subject: Vec<Value>,
    #[serde(rename = "predicateType")]
    predicate_type: String,
    predicate: Payload,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Payload {
    version: u64,
    source_revision: String,
    suite_declarations_sha256: String,
    test_matrix: Vec<Value>,
    runs: Vec<RawRun>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawRun {
    suite: String,
    record_path: String,
    sha256: Option<String>,
}

/// Read provenance, checking `suites_md` against its signed digest.
///
/// # Errors
/// [`ProvenanceError`] when the envelope or payload is malformed, the suite
/// declarations differ from the signed digest, or the matrix is empty.
#[implements("FR-185-AC-11", "FR-185-AC-12", "FR-186-AC-7")]
pub fn read_provenance(envelope: &[u8], suites_md: &[u8]) -> Result<Provenance, ProvenanceError> {
    let value: Value = serde_json::from_slice(envelope)
        .map_err(|error| ProvenanceError::Envelope(error.to_string()))?;
    let bundle: SigstoreBundle = serde_json::from_value(value)
        .map_err(|error| ProvenanceError::Envelope(error.to_string()))?;
    if bundle.dsse_envelope.payload_type != DSSE_TYPE || bundle.dsse_envelope.signatures.is_empty()
    {
        return Err(ProvenanceError::Envelope(
            "bundle does not contain a signed in-toto DSSE envelope".to_owned(),
        ));
    }
    let decoded = STANDARD
        .decode(bundle.dsse_envelope.payload.as_bytes())
        .map_err(|error| ProvenanceError::Envelope(format!("payload is not base64: {error}")))?;
    let statement: InTotoStatement = serde_json::from_slice(&decoded)
        .map_err(|error| ProvenanceError::Payload(error.to_string()))?;
    if statement.statement_type != STATEMENT_TYPE || statement.predicate_type != PAYLOAD_TYPE {
        return Err(ProvenanceError::Envelope(
            "bundle statement or predicate type is invalid".to_owned(),
        ));
    }
    let subjects = parse_subjects(statement.subject)?;
    let payload = statement.predicate;
    if payload.version != 2 {
        return Err(ProvenanceError::Payload("version must be 2".to_owned()));
    }
    if !is_commit(&payload.source_revision) {
        return Err(ProvenanceError::Payload(
            "sourceRevision is not a full lowercase hex commit".to_owned(),
        ));
    }
    if !is_sha256(&payload.suite_declarations_sha256) {
        return Err(ProvenanceError::Payload(
            "suiteDeclarationsSha256 is not a lowercase sha256".to_owned(),
        ));
    }
    if quoin_store::attachment::sha256_hex(suites_md) != payload.suite_declarations_sha256 {
        return Err(ProvenanceError::SuiteDeclarationsMismatch);
    }
    let required = active_criteria(&payload.test_matrix)?;
    if required.is_empty() {
        return Err(ProvenanceError::EmptyMatrix);
    }
    let mut runs = Vec::new();
    let mut suites = BTreeSet::new();
    for run in payload.runs {
        if !suites.insert(run.suite.clone()) {
            return Err(ProvenanceError::Payload(format!(
                "suite {} appears twice in runs",
                run.suite
            )));
        }
        if !is_relative_path(&run.record_path) {
            return Err(ProvenanceError::Payload(format!(
                "recordPath {} is not a plain relative path",
                run.record_path
            )));
        }
        if run
            .sha256
            .as_deref()
            .is_some_and(|digest| !is_sha256(digest))
        {
            return Err(ProvenanceError::Payload(format!(
                "sha256 of suite {} is not a lowercase sha256",
                run.suite
            )));
        }
        runs.push(ExpectedRun {
            suite: run.suite,
            record_path: run.record_path,
            sha256: run.sha256,
        });
    }
    Ok(Provenance {
        source_revision: payload.source_revision,
        required,
        runs,
        subjects,
    })
}

/// Compare the caller's artifact subject list with the exact signed subjects.
///
/// # Errors
///
/// Returns [`ProvenanceError::SubjectMismatch`] when the supplied value is not
/// an array of unique, well-formed SHA-256 subjects or its subject set differs
/// from the subjects authenticated by the provenance.
#[implements("FR-185-AC-11")]
pub fn check_signed_subjects(
    subject: &Value,
    provenance: &Provenance,
) -> Result<(), ProvenanceError> {
    let signed = &provenance.subjects;
    let descriptors = subject.as_array().ok_or(ProvenanceError::SubjectMismatch)?;
    let mut supplied = BTreeSet::new();
    for descriptor in descriptors {
        let name = descriptor
            .get("name")
            .and_then(Value::as_str)
            .ok_or(ProvenanceError::SubjectMismatch)?;
        let digest = descriptor
            .pointer("/digest/sha256")
            .and_then(Value::as_str)
            .ok_or(ProvenanceError::SubjectMismatch)?;
        if !is_sha256(digest)
            || !supplied.insert(ProvenanceSubject {
                name: name.to_owned(),
                sha256: digest.to_owned(),
            })
        {
            return Err(ProvenanceError::SubjectMismatch);
        }
    }
    if &supplied == signed {
        Ok(())
    } else {
        Err(ProvenanceError::SubjectMismatch)
    }
}

fn parse_subjects(values: Vec<Value>) -> Result<BTreeSet<ProvenanceSubject>, ProvenanceError> {
    if values.is_empty() {
        return Err(ProvenanceError::Subjects);
    }
    let mut subjects = BTreeSet::new();
    for value in values {
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or(ProvenanceError::Subjects)?;
        let digest = value
            .pointer("/digest/sha256")
            .and_then(Value::as_str)
            .filter(|digest| is_sha256(digest))
            .ok_or(ProvenanceError::Subjects)?;
        if !subjects.insert(ProvenanceSubject {
            name: name.to_owned(),
            sha256: digest.to_owned(),
        }) {
            return Err(ProvenanceError::Subjects);
        }
    }
    Ok(subjects)
}

/// The active criteria of a Quire coverage matrix: every `criteria[].id` whose
/// `status` is not `retired`. A row or criterion of another shape refuses.
fn active_criteria(rows: &[Value]) -> Result<BTreeSet<String>, ProvenanceError> {
    let malformed = |what: &str| ProvenanceError::Payload(format!("testMatrix: {what}"));
    let mut required = BTreeSet::new();
    for row in rows {
        let criteria = row
            .get("criteria")
            .and_then(Value::as_array)
            .ok_or_else(|| malformed("a row has no criteria array"))?;
        for criterion in criteria {
            let id = criterion
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| malformed("a criterion has no id"))?;
            let status = criterion
                .get("status")
                .and_then(Value::as_str)
                .ok_or_else(|| malformed("a criterion has no status"))?;
            if status != "retired" {
                required.insert(id.to_owned());
            }
        }
    }
    Ok(required)
}

/// A full lowercase hex commit id (SHA-1 or SHA-256 object format).
fn is_commit(text: &str) -> bool {
    matches!(text.len(), 40 | 64)
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A path that stays under the evidence directory: no root, no `..`.
fn is_relative_path(text: &str) -> bool {
    !text.is_empty()
        && std::path::Path::new(text)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
}
