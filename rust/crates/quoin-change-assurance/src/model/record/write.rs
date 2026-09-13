// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Writing a [`ChangeAssuranceRecord`] back out as a document.
//!
//! The inverse of [`super::read`], and deliberately not one traversal shared
//! with it: a writer that reused the reader's walk would agree with the reader
//! by construction, and the digest is taken over what this file emits.

use quoin_store::{CanonicalDigest, JsonValue};

use crate::error::ChangeAssuranceError;
use crate::ids::{RecordId, Revision};
use crate::model::json::{number, object, string_values};
use crate::model::record::{
    DECISION_EVENT_KIND, Definition, ImpactSnapshot, RECORD_TYPE, RecordSubject, ReviewWorkflow,
    SCHEMA_VERSION, SourceConnection, Statement,
};

/// Build the JSON form of a record, with or without its `digest`.
#[allow(
    clippy::too_many_arguments,
    reason = "one argument per top-level member of the closed schema; grouping them \
              into a struct would only re-create the record this builds"
)]
pub(super) fn write_record(
    record_id: &RecordId,
    revision: Revision,
    parent_digest: Option<&CanonicalDigest>,
    digest: Option<&CanonicalDigest>,
    subject: &RecordSubject,
    source_connections: &[SourceConnection],
    impact_snapshot: &ImpactSnapshot,
    definition: &Definition,
    review_workflow: &ReviewWorkflow,
) -> Result<JsonValue, ChangeAssuranceError> {
    let mut members = vec![
        ("schema_version", number(SCHEMA_VERSION)?),
        ("record_type", JsonValue::string(RECORD_TYPE)),
        ("record_id", JsonValue::string(record_id.as_str())),
        ("revision", number(revision.get())?),
        (
            "parent_digest",
            parent_digest.map_or(JsonValue::Null, |value| JsonValue::string(value.as_hex())),
        ),
    ];
    if let Some(digest) = digest {
        members.push(("digest", JsonValue::string(digest.as_hex())));
    }
    members.push((
        "subject",
        object(vec![
            ("repository", JsonValue::string(subject.repository.as_str())),
            (
                "base_revision",
                JsonValue::string(subject.base_revision.as_str()),
            ),
            ("scope", string_values(&subject.scope)),
        ]),
    ));
    members.push((
        "source_connections",
        JsonValue::Array(
            source_connections
                .iter()
                .map(|connection| {
                    object(vec![
                        (
                            "source_id",
                            JsonValue::string(connection.source_id.as_str()),
                        ),
                        ("kind", JsonValue::string(connection.kind.as_str())),
                        ("revision", JsonValue::string(connection.revision.as_str())),
                        ("digest", JsonValue::string(connection.digest.as_hex())),
                    ])
                })
                .collect(),
        ),
    ));
    members.push((
        "impact_snapshot",
        object(vec![
            (
                "identity",
                JsonValue::string(impact_snapshot.identity.as_str()),
            ),
            (
                "revision",
                JsonValue::string(impact_snapshot.revision.as_str()),
            ),
            ("digest", JsonValue::string(impact_snapshot.digest.as_hex())),
            (
                "completeness",
                JsonValue::string(impact_snapshot.completeness.as_str()),
            ),
            ("truncated", JsonValue::Bool(impact_snapshot.truncated)),
            ("gaps", string_values(&impact_snapshot.gaps)),
        ]),
    ));
    members.push(("definition", write_definition(definition)));
    members.push((
        "review_workflow",
        object(vec![
            ("run_id", JsonValue::string(review_workflow.run_id.as_str())),
            (
                "decision_event_kind",
                JsonValue::string(DECISION_EVENT_KIND),
            ),
        ]),
    ));
    Ok(object(members))
}

fn write_definition(definition: &Definition) -> JsonValue {
    let statements = |entries: &[Statement]| {
        JsonValue::Array(
            entries
                .iter()
                .map(|entry| {
                    object(vec![
                        ("id", JsonValue::string(entry.id.as_str())),
                        ("statement", JsonValue::string(entry.statement.as_str())),
                        ("source_ids", string_values(&entry.source_ids)),
                    ])
                })
                .collect(),
        )
    };
    let proofs = JsonValue::Array(
        definition
            .proof_obligations
            .iter()
            .map(|proof| {
                let obligation_ids: Vec<String> = proof
                    .obligation_ids
                    .iter()
                    .map(|id| id.as_str().to_owned())
                    .collect();
                object(vec![
                    ("proof_id", JsonValue::string(proof.proof_id.as_str())),
                    ("statement", JsonValue::string(proof.statement.as_str())),
                    ("obligation_ids", string_values(&obligation_ids)),
                    (
                        "evidence_kind",
                        JsonValue::string(proof.evidence_kind.as_str()),
                    ),
                    ("command", proof.command.to_json()),
                    (
                        "tool_identity",
                        JsonValue::string(proof.tool_identity.as_str()),
                    ),
                    (
                        "configuration_digest",
                        JsonValue::string(proof.configuration_digest.as_hex()),
                    ),
                ])
            })
            .collect(),
    );
    let unknowns = JsonValue::Array(
        definition
            .unknowns
            .iter()
            .map(|unknown| {
                let mut members = vec![
                    ("id", JsonValue::string(unknown.id.as_str())),
                    ("statement", JsonValue::string(unknown.statement.as_str())),
                    (
                        "disposition",
                        JsonValue::string(unknown.disposition.as_str()),
                    ),
                    ("owner", JsonValue::string(unknown.owner.as_str())),
                ];
                if let Some(resolution) = &unknown.resolution {
                    members.push(("resolution", JsonValue::string(resolution.as_str())));
                }
                object(members)
            })
            .collect(),
    );
    object(vec![
        ("requirements", statements(&definition.requirements)),
        (
            "preservation_constraints",
            statements(&definition.preservation_constraints),
        ),
        ("proof_obligations", proofs),
        ("unknowns", unknowns),
    ])
}
