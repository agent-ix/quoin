// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a document into a [`ChangeAssuranceRecord`].
//!
//! One pass: every value is parsed AS it is validated, so no partially checked
//! record ever exists. `exact` refuses an undeclared member as firmly as an
//! absent one at every depth, because the record is named by the digest of its
//! own bytes.

use quoin_store::{CanonicalDigest, JsonValue};

use crate::error::{ChangeAssuranceError, FieldFailure};
use crate::ids::{
    ImpactIdentity, NonEmptyText, ObligationId, ProofId, RecordId, Revision, RunId, SourceId,
    StatementId,
};
use crate::model::json::{Fields, number, require_sorted, require_unique_values};
use crate::model::record::{
    CommandBinding, Completeness, DECISION_EVENT_KIND, Definition, Disposition, ImpactSnapshot,
    ProofObligation, RECORD_TYPE, RecordSubject, ReviewWorkflow, SCHEMA_VERSION, SUBJECT,
    SourceConnection, SourceKind, Statement, Unknown, UnsealedRecord,
};

/// Read the record shape, sealed when `digest` is supplied.
pub(super) fn read_record(
    value: &JsonValue,
    digest: Option<&CanonicalDigest>,
) -> Result<UnsealedRecord, ChangeAssuranceError> {
    let root = Fields::read(value, SUBJECT, "record")?;
    let sealed = digest.is_some();
    let allowed: &[&str] = if sealed {
        &[
            "schema_version",
            "record_type",
            "record_id",
            "revision",
            "parent_digest",
            "digest",
            "subject",
            "source_connections",
            "impact_snapshot",
            "definition",
            "review_workflow",
        ]
    } else {
        &[
            "schema_version",
            "record_type",
            "record_id",
            "revision",
            "parent_digest",
            "subject",
            "source_connections",
            "impact_snapshot",
            "definition",
            "review_workflow",
        ]
    };
    root.exact(allowed)?;
    root.equals("schema_version", &number(SCHEMA_VERSION)?)?;
    root.equals("record_type", &JsonValue::string(RECORD_TYPE))?;
    let record_id = RecordId::parse(root.text("record_id")?, SUBJECT, "record_id")?;
    let revision = Revision::new(root.whole_number("revision")?, SUBJECT, "revision")?;
    let parent_digest = root.nullable_digest("parent_digest")?;
    if revision.is_genesis() && parent_digest.is_some() {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("revision 1 parent_digest"),
        ));
    }
    if !revision.is_genesis() && parent_digest.is_none() {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("successor parent_digest"),
        ));
    }

    let subject = read_subject(root)?;
    let source_connections = read_source_connections(root)?;
    let impact_snapshot = read_impact_snapshot(root)?;
    let definition = read_definition(root)?;
    let review_workflow = read_review_workflow(root)?;

    Ok(UnsealedRecord {
        record_id,
        revision,
        parent_digest,
        subject,
        source_connections,
        impact_snapshot,
        definition,
        review_workflow,
    })
}

fn read_subject(root: Fields<'_>) -> Result<RecordSubject, ChangeAssuranceError> {
    let subject = root.nested("subject")?;
    subject.exact(&["repository", "base_revision", "scope"])?;
    let repository = subject.non_empty("repository")?;
    let base_revision = subject.non_empty("base_revision")?;
    let scope = subject.string_array("scope", true, true)?;
    require_sorted(scope.iter().map(String::as_str), SUBJECT, "subject.scope")?;
    Ok(RecordSubject {
        repository,
        base_revision,
        scope,
    })
}

fn read_source_connections(
    root: Fields<'_>,
) -> Result<Vec<SourceConnection>, ChangeAssuranceError> {
    let entries = root.array("source_connections", true)?;
    let mut connections = Vec::with_capacity(entries.len());
    for entry in entries {
        let fields = Fields::read(entry, SUBJECT, "source_connections")?;
        fields.exact(&["source_id", "kind", "revision", "digest"])?;
        let source_id = SourceId::parse(fields.text("source_id")?, SUBJECT, "source_id")?;
        let kind = SourceKind::parse(fields.text("kind")?).ok_or_else(|| {
            ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("source kind"))
        })?;
        let revision = NonEmptyText::parse(fields.text("revision")?, SUBJECT, "source revision")?;
        let digest = fields.artifact_digest("digest").map_err(|_| {
            ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("source digest"))
        })?;
        connections.push(SourceConnection {
            source_id,
            kind,
            revision,
            digest,
        });
    }
    require_unique_values(
        connections.iter().map(|entry| entry.source_id.as_str()),
        SUBJECT,
        "source identity",
    )?;
    require_sorted(
        connections.iter().map(|entry| entry.source_id.as_str()),
        SUBJECT,
        "source_connections",
    )?;
    Ok(connections)
}

fn read_impact_snapshot(root: Fields<'_>) -> Result<ImpactSnapshot, ChangeAssuranceError> {
    let impact = root.nested("impact_snapshot")?;
    impact.exact(&[
        "identity",
        "revision",
        "digest",
        "completeness",
        "truncated",
        "gaps",
    ])?;
    let identity = ImpactIdentity::parse(impact.text("identity")?, SUBJECT, "impact identity")?;
    let revision = NonEmptyText::parse(impact.text("revision")?, SUBJECT, "impact revision")?;
    let digest = impact.artifact_digest("digest").map_err(|_| {
        ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("impact digest"))
    })?;
    let completeness = Completeness::parse(impact.text("completeness")?).ok_or_else(|| {
        ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("impact completeness"))
    })?;
    let truncated = impact.boolean("truncated").map_err(|_| {
        ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("impact truncated"))
    })?;
    let gaps = impact.string_array("gaps", false, false).map_err(|_| {
        ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("impact gaps"))
    })?;
    require_sorted(gaps.iter().map(String::as_str), SUBJECT, "impact gaps")?;
    Ok(ImpactSnapshot {
        identity,
        revision,
        digest,
        completeness,
        truncated,
        gaps,
    })
}

fn read_definition(root: Fields<'_>) -> Result<Definition, ChangeAssuranceError> {
    let definition = root.nested("definition")?;
    definition.exact(&[
        "requirements",
        "preservation_constraints",
        "proof_obligations",
        "unknowns",
    ])?;
    let requirements = read_statements(definition, "requirements", true)?;
    let preservation_constraints = read_statements(definition, "preservation_constraints", false)?;
    let proof_obligations = read_proof_obligations(definition)?;
    let unknowns = read_unknowns(definition)?;
    Ok(Definition {
        requirements,
        preservation_constraints,
        proof_obligations,
        unknowns,
    })
}

fn read_statements(
    definition: Fields<'_>,
    name: &str,
    require_non_empty: bool,
) -> Result<Vec<Statement>, ChangeAssuranceError> {
    let entries = definition.array(name, require_non_empty)?;
    let mut statements = Vec::with_capacity(entries.len());
    for entry in entries {
        let fields = Fields::read(entry, SUBJECT, name)?;
        fields.exact(&["id", "statement", "source_ids"])?;
        let id = StatementId::parse(fields.text("id")?, SUBJECT, &format!("{name} id"))?;
        let statement = NonEmptyText::parse(
            fields.text("statement")?,
            SUBJECT,
            &format!("{name} statement"),
        )?;
        let source_ids = fields.string_array("source_ids", true, true)?;
        require_sorted(
            source_ids.iter().map(String::as_str),
            SUBJECT,
            &format!("{name} source_ids"),
        )?;
        statements.push(Statement {
            id,
            statement,
            source_ids,
        });
    }
    require_unique_values(
        statements.iter().map(|entry| entry.id.as_str()),
        SUBJECT,
        &format!("{name} identity"),
    )?;
    require_sorted(
        statements.iter().map(|entry| entry.id.as_str()),
        SUBJECT,
        name,
    )?;
    Ok(statements)
}

fn read_proof_obligations(
    definition: Fields<'_>,
) -> Result<Vec<ProofObligation>, ChangeAssuranceError> {
    let entries = definition.array("proof_obligations", true)?;
    let mut proofs = Vec::with_capacity(entries.len());
    for entry in entries {
        let fields = Fields::read(entry, SUBJECT, "proof_obligations")?;
        fields.exact(&[
            "proof_id",
            "statement",
            "obligation_ids",
            "evidence_kind",
            "command",
            "tool_identity",
            "configuration_digest",
        ])?;
        let proof_id = ProofId::parse(fields.text("proof_id")?, SUBJECT, "proof_id")?;
        let statement = NonEmptyText::parse(fields.text("statement")?, SUBJECT, "proof statement")?;
        let obligation_text = fields.string_array("obligation_ids", true, true)?;
        require_sorted(
            obligation_text.iter().map(String::as_str),
            SUBJECT,
            "obligation_ids",
        )?;
        let obligation_ids = obligation_text
            .iter()
            .map(|id| ObligationId::parse(id, SUBJECT, "obligation_ids"))
            .collect::<Result<Vec<_>, _>>()?;
        let evidence_kind =
            NonEmptyText::parse(fields.text("evidence_kind")?, SUBJECT, "evidence_kind")?;
        let command = CommandBinding::from_json(fields.value("command")?, SUBJECT)?;
        let tool_identity =
            NonEmptyText::parse(fields.text("tool_identity")?, SUBJECT, "tool_identity")?;
        let configuration_digest =
            fields
                .artifact_digest("configuration_digest")
                .map_err(|_| {
                    ChangeAssuranceError::shape(
                        SUBJECT,
                        FieldFailure::malformed("configuration_digest"),
                    )
                })?;
        proofs.push(ProofObligation {
            proof_id,
            statement,
            obligation_ids,
            evidence_kind,
            command,
            tool_identity,
            configuration_digest,
        });
    }
    require_unique_values(
        proofs.iter().map(|proof| proof.proof_id.as_str()),
        SUBJECT,
        "proof identity",
    )?;
    require_sorted(
        proofs.iter().map(|proof| proof.proof_id.as_str()),
        SUBJECT,
        "proof_obligations",
    )?;
    Ok(proofs)
}

fn read_unknowns(definition: Fields<'_>) -> Result<Vec<Unknown>, ChangeAssuranceError> {
    let entries = definition.array("unknowns", false)?;
    let mut unknowns = Vec::with_capacity(entries.len());
    for entry in entries {
        let fields = Fields::read(entry, SUBJECT, "unknowns")?;
        let carries_resolution = fields.optional("resolution").is_some();
        if carries_resolution {
            fields.exact(&["id", "statement", "disposition", "owner", "resolution"])?;
        } else {
            fields.exact(&["id", "statement", "disposition", "owner"])?;
        }
        let id = StatementId::parse(fields.text("id")?, SUBJECT, "unknown id")?;
        let statement =
            NonEmptyText::parse(fields.text("statement")?, SUBJECT, "unknown statement")?;
        let owner = NonEmptyText::parse(fields.text("owner")?, SUBJECT, "unknown owner")?;
        let disposition = Disposition::parse(fields.text("disposition")?).ok_or_else(|| {
            ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("unknown disposition"))
        })?;
        let resolution = match (disposition, carries_resolution) {
            (Disposition::Resolved, _) => Some(NonEmptyText::parse(
                fields.text("resolution")?,
                SUBJECT,
                "unknown resolution",
            )?),
            (_, true) => {
                return Err(ChangeAssuranceError::shape(
                    SUBJECT,
                    FieldFailure::malformed("non-resolved unknown resolution"),
                ));
            }
            (_, false) => None,
        };
        unknowns.push(Unknown {
            id,
            statement,
            disposition,
            owner,
            resolution,
        });
    }
    require_unique_values(
        unknowns.iter().map(|unknown| unknown.id.as_str()),
        SUBJECT,
        "unknown identity",
    )?;
    require_sorted(
        unknowns.iter().map(|unknown| unknown.id.as_str()),
        SUBJECT,
        "unknowns",
    )?;
    Ok(unknowns)
}

fn read_review_workflow(root: Fields<'_>) -> Result<ReviewWorkflow, ChangeAssuranceError> {
    let workflow = root.nested("review_workflow")?;
    workflow.exact(&["run_id", "decision_event_kind"])?;
    let run_id = RunId::parse(workflow.text("run_id")?, SUBJECT, "run_id")?;
    workflow.equals(
        "decision_event_kind",
        &JsonValue::string(DECISION_EVENT_KIND),
    )?;
    Ok(ReviewWorkflow { run_id })
}
