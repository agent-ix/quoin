// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The change-assurance record (FR-063).
//!
//! A faithful port of `validateRecordShape` and `normalizeRecord`
//! (`src/change-assurance/records.ts:212-442`). The record is a **closed**
//! shape at every depth: `exact` refuses an undeclared member as firmly as it
//! refuses an absent one, because a record is named by the digest of its own
//! bytes and a member the reader silently dropped is a member the digest
//! covered.
//!
//! Parsing and validating are one pass here rather than two. The oracle
//! validates a `unknown` and then asserts the type; this module builds the
//! typed value *as* it validates, so there is no window in which a
//! `ChangeAssuranceRecord` exists that has not been checked.

use quoin_store::{CanonicalDigest, JsonValue};

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::ids::{
    ArtifactDigest, ImpactIdentity, NonEmptyText, ObligationId, ProofId, RecordId, Revision, RunId,
    SourceId, StatementId,
};
use crate::model::json::{
    Fields, compare_utf16, number, object, require_sorted, require_unique_values, sort_utf16,
    string_values,
};

/// The one `record_type` a change-assurance record carries.
pub const RECORD_TYPE: &str = "change_assurance";

/// The one `decision_event_kind` a review workflow binds.
pub const DECISION_EVENT_KIND: &str = "change_assurance.review_decided";

/// The schema version every record in this family carries.
pub const SCHEMA_VERSION: u64 = 1;

/// The kind of artifact a source connection points at.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SourceKind {
    /// A requirement document.
    Requirement,
    /// A test.
    Test,
    /// An API surface.
    Api,
    /// An architecture document.
    Architecture,
    /// Retained impact evidence.
    ImpactEvidence,
    /// Retained recovery evidence.
    RecoveryEvidence,
    /// Anything the six above do not name.
    Other,
}

impl SourceKind {
    /// Every kind, in the oracle's declaration order.
    pub const ALL: [Self; 7] = [
        Self::Requirement,
        Self::Test,
        Self::Api,
        Self::Architecture,
        Self::ImpactEvidence,
        Self::RecoveryEvidence,
        Self::Other,
    ];

    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Test => "test",
            Self::Api => "api",
            Self::Architecture => "architecture",
            Self::ImpactEvidence => "impact_evidence",
            Self::RecoveryEvidence => "recovery_evidence",
            Self::Other => "other",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// Whether an impact snapshot claims to have seen everything.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Completeness {
    /// The snapshot saw everything in its scope.
    Complete,
    /// The snapshot did not, and says so.
    Incomplete,
}

impl Completeness {
    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "complete" => Some(Self::Complete),
            "incomplete" => Some(Self::Incomplete),
            _ => None,
        }
    }
}

/// What a reviewer decided about an open unknown.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Disposition {
    /// Still open.
    Open,
    /// Accepted as a risk.
    Accepted,
    /// Deferred to a later revision.
    Deferred,
    /// Answered; the answer is the unknown's `resolution`.
    Resolved,
}

impl Disposition {
    /// Every disposition, in the oracle's declaration order.
    pub const ALL: [Self; 4] = [Self::Open, Self::Accepted, Self::Deferred, Self::Resolved];

    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Accepted => "accepted",
            Self::Deferred => "deferred",
            Self::Resolved => "resolved",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|disposition| disposition.as_str() == value)
    }
}

/// The exact command a proof obligation pins.
///
/// `working_directory` is repository-relative POSIX and normalized: not
/// absolute, no backslash, no trailing slash, and no empty, `.` or `..`
/// segment — with the single exception of `"."`, which names the repository
/// root. `validateCommand` (`records.ts:444`) is the whole rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandBinding {
    /// The literal argument vector, in order. Never empty.
    pub argv: Vec<String>,
    /// The repository-relative working directory.
    pub working_directory: NonEmptyText,
}

impl CommandBinding {
    /// Read and validate a command binding.
    ///
    /// # Errors
    ///
    /// Refuses an undeclared or absent member, an empty or non-string `argv`,
    /// and a `working_directory` that is not a normalized repository-relative
    /// POSIX path.
    pub fn from_json(value: &JsonValue, subject: Subject) -> Result<Self, ChangeAssuranceError> {
        let fields = Fields::read(value, subject, "command")?;
        fields.exact(&["argv", "working_directory"])?;
        let argv_values = fields.array("argv", true)?;
        let mut argv = Vec::with_capacity(argv_values.len());
        for entry in argv_values {
            let text = entry.as_str().ok_or_else(|| {
                ChangeAssuranceError::shape(subject, FieldFailure::malformed("argv"))
            })?;
            argv.push(text.to_owned());
        }
        let working_directory = fields.non_empty("working_directory")?;
        if !is_normalized_relative_posix_path(working_directory.as_str()) {
            return Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(
                    "working_directory must be normalized repository-relative POSIX path",
                ),
            ));
        }
        Ok(Self {
            argv,
            working_directory,
        })
    }

    /// The JSON form.
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        object(vec![
            ("argv", string_values(&self.argv)),
            (
                "working_directory",
                JsonValue::string(self.working_directory.as_str()),
            ),
        ])
    }
}

/// Whether `path` is what `validateCommand` accepts.
fn is_normalized_relative_posix_path(path: &str) -> bool {
    if path == "." {
        return true;
    }
    !path.starts_with('/')
        && !path.contains('\\')
        && !path.ends_with('/')
        && !path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
}

/// What the record is about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordSubject {
    /// The repository the change lands in.
    pub repository: NonEmptyText,
    /// The revision the change is measured against.
    pub base_revision: NonEmptyText,
    /// The paths in scope, unique and in UTF-16 order.
    pub scope: Vec<String>,
}

/// One artifact the record is connected to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceConnection {
    /// The artifact's identity.
    pub source_id: SourceId,
    /// What kind of artifact it is.
    pub kind: SourceKind,
    /// The revision of it that was read.
    pub revision: NonEmptyText,
    /// Its digest at that revision, as its producer computed it.
    pub digest: ArtifactDigest,
}

/// What the impact analysis saw.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImpactSnapshot {
    /// The analysis's identity.
    pub identity: ImpactIdentity,
    /// The revision it ran at.
    pub revision: NonEmptyText,
    /// The digest of its output, as the analysis computed it.
    pub digest: ArtifactDigest,
    /// Whether it saw everything in scope.
    pub completeness: Completeness,
    /// Whether its output was cut short.
    pub truncated: bool,
    /// What it could not see, in UTF-16 order.
    pub gaps: Vec<String>,
}

/// A reviewed requirement or preservation constraint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    /// The statement's identity within this record.
    pub id: StatementId,
    /// The reviewed text, retained verbatim.
    pub statement: NonEmptyText,
    /// The sources it was drawn from, unique and in UTF-16 order.
    pub source_ids: Vec<String>,
}

/// A proof obligation and the exact evidence it pins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofObligation {
    /// The obligation's identity.
    pub proof_id: ProofId,
    /// What it claims.
    pub statement: NonEmptyText,
    /// The evidence obligations it discharges, unique and in UTF-16 order.
    pub obligation_ids: Vec<ObligationId>,
    /// The kind of evidence that discharges it.
    pub evidence_kind: NonEmptyText,
    /// The exact command that produces that evidence.
    pub command: CommandBinding,
    /// The tool that must have produced it.
    pub tool_identity: NonEmptyText,
    /// The configuration that tool must have run under.
    pub configuration_digest: ArtifactDigest,
}

/// Something the review did not settle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unknown {
    /// The unknown's identity.
    pub id: StatementId,
    /// What is not known.
    pub statement: NonEmptyText,
    /// What the review decided about it.
    pub disposition: Disposition,
    /// Who owns it.
    pub owner: NonEmptyText,
    /// The answer, present exactly when the disposition is `resolved`.
    pub resolution: Option<NonEmptyText>,
}

/// The record's four definition collections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    /// What the change must do. Never empty.
    pub requirements: Vec<Statement>,
    /// What the change must not break. May be empty, and the empty list is a
    /// claim rather than an omission.
    pub preservation_constraints: Vec<Statement>,
    /// How each claim is proved. Never empty.
    pub proof_obligations: Vec<ProofObligation>,
    /// What is not settled. May be empty.
    pub unknowns: Vec<Unknown>,
}

/// The ix-flow run whose decision approves this record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewWorkflow {
    /// The run's identity.
    pub run_id: RunId,
}

/// A sealed change-assurance record.
///
/// Every value of this type has passed the v1 schema *and* carries a `digest`
/// that its own canonical bytes reproduce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeAssuranceRecord {
    /// Identity, constant across revisions.
    pub record_id: RecordId,
    /// This revision's number, from one.
    pub revision: Revision,
    /// The previous revision's digest; absent exactly at revision one.
    pub parent_digest: Option<CanonicalDigest>,
    /// This record's own digest.
    pub digest: CanonicalDigest,
    /// What the record is about.
    pub subject: RecordSubject,
    /// The artifacts it is connected to, in `source_id` order.
    pub source_connections: Vec<SourceConnection>,
    /// What the impact analysis saw.
    pub impact_snapshot: ImpactSnapshot,
    /// The reviewed definition.
    pub definition: Definition,
    /// The review workflow binding.
    pub review_workflow: ReviewWorkflow,
}

/// A record read but not yet sealed: everything but the `digest`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsealedRecord {
    /// Identity, constant across revisions.
    pub record_id: RecordId,
    /// This revision's number, from one.
    pub revision: Revision,
    /// The previous revision's digest; absent exactly at revision one.
    pub parent_digest: Option<CanonicalDigest>,
    /// What the record is about.
    pub subject: RecordSubject,
    /// The artifacts it is connected to, in `source_id` order.
    pub source_connections: Vec<SourceConnection>,
    /// What the impact analysis saw.
    pub impact_snapshot: ImpactSnapshot,
    /// The reviewed definition.
    pub definition: Definition,
    /// The review workflow binding.
    pub review_workflow: ReviewWorkflow,
}

const SUBJECT: Subject = Subject::Record;

impl UnsealedRecord {
    /// Read and validate the unsealed shape.
    ///
    /// # Errors
    ///
    /// Every refusal `validateRecordShape(value, false)` produces.
    pub fn from_json(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        read_record(value, None)
    }

    /// The JSON form, without a `digest` member.
    ///
    /// # Errors
    ///
    /// Refuses only if a count has no finite double, which a `u64` always has.
    pub fn to_json(&self) -> Result<JsonValue, ChangeAssuranceError> {
        write_record(
            &self.record_id,
            self.revision,
            self.parent_digest.as_ref(),
            None,
            &self.subject,
            &self.source_connections,
            &self.impact_snapshot,
            &self.definition,
            &self.review_workflow,
        )
    }

    /// Attach a digest, producing the sealed record.
    #[must_use]
    pub fn seal_with(self, digest: CanonicalDigest) -> ChangeAssuranceRecord {
        ChangeAssuranceRecord {
            record_id: self.record_id,
            revision: self.revision,
            parent_digest: self.parent_digest,
            digest,
            subject: self.subject,
            source_connections: self.source_connections,
            impact_snapshot: self.impact_snapshot,
            definition: self.definition,
            review_workflow: self.review_workflow,
        }
    }
}

impl ChangeAssuranceRecord {
    /// Read and validate the sealed shape, **without** checking the digest.
    ///
    /// [`crate::records::verify_change_record`] is the entry point that also
    /// checks it; this is deliberately not public, so no caller can obtain a
    /// record whose digest was never verified.
    pub(crate) fn read_sealed(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        let digest = Fields::read(value, SUBJECT, "record")?.digest("digest")?;
        Ok(read_record(value, Some(&digest))?.seal_with(digest))
    }

    /// The JSON form, `digest` included.
    ///
    /// # Errors
    ///
    /// Refuses only if a count has no finite double, which a `u64` always has.
    pub fn to_json(&self) -> Result<JsonValue, ChangeAssuranceError> {
        write_record(
            &self.record_id,
            self.revision,
            self.parent_digest.as_ref(),
            Some(&self.digest),
            &self.subject,
            &self.source_connections,
            &self.impact_snapshot,
            &self.definition,
            &self.review_workflow,
        )
    }
}

/// Read the record shape, sealed when `digest` is supplied.
fn read_record(
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

/// Build the JSON form of a record, with or without its `digest`.
#[allow(
    clippy::too_many_arguments,
    reason = "one argument per top-level member of the closed schema; grouping them \
              into a struct would only re-create the record this builds"
)]
fn write_record(
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

/// Sort every collection a record declares, as `normalizeRecord` does.
///
/// Operates on the JSON rather than on a typed value because the oracle sorts
/// **before** validating: an input whose collections are out of order is
/// accepted by `sealChangeRecord` and refused by `verifyChangeRecord`, and a
/// port that normalized after parsing could never reproduce that.
///
/// Shapes this does not recognise are left alone; the validation pass that
/// follows is what refuses them, and it produces the better message.
#[must_use]
pub fn normalize_record(value: &JsonValue) -> JsonValue {
    let JsonValue::Object(root) = value else {
        return value.clone();
    };
    let mut normalized = root.clone();
    normalized.remove("digest");
    if let Some(JsonValue::Object(subject)) = normalized.get("subject") {
        let mut subject = subject.clone();
        sort_string_member(&mut subject, "scope");
        normalized.set("subject", JsonValue::Object(subject));
    }
    sort_object_array(&mut normalized, "source_connections", "source_id");
    if let Some(JsonValue::Object(impact)) = normalized.get("impact_snapshot") {
        let mut impact = impact.clone();
        sort_string_member(&mut impact, "gaps");
        normalized.set("impact_snapshot", JsonValue::Object(impact));
    }
    if let Some(JsonValue::Object(definition)) = normalized.get("definition") {
        let mut definition = definition.clone();
        sort_object_array(&mut definition, "requirements", "id");
        sort_object_array(&mut definition, "preservation_constraints", "id");
        sort_object_array(&mut definition, "proof_obligations", "proof_id");
        sort_object_array(&mut definition, "unknowns", "id");
        sort_nested_string_member(&mut definition, "requirements", "source_ids");
        sort_nested_string_member(&mut definition, "preservation_constraints", "source_ids");
        sort_nested_string_member(&mut definition, "proof_obligations", "obligation_ids");
        normalized.set("definition", JsonValue::Object(definition));
    }
    JsonValue::Object(normalized)
}

fn sort_string_member(owner: &mut quoin_store::JsonObject, member: &str) {
    let Some(JsonValue::Array(entries)) = owner.get(member) else {
        return;
    };
    let mut texts: Vec<String> = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(text) = entry.as_str() else {
            return;
        };
        texts.push(text.to_owned());
    }
    sort_utf16(&mut texts);
    owner.set(member, string_values(&texts));
}

fn sort_object_array(owner: &mut quoin_store::JsonObject, member: &str, key: &str) {
    let Some(JsonValue::Array(entries)) = owner.get(member) else {
        return;
    };
    let mut sortable: Vec<(String, JsonValue)> = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(text) = entry
            .as_object()
            .ok()
            .and_then(|fields| fields.get(key))
            .and_then(JsonValue::as_str)
        else {
            return;
        };
        sortable.push((text.to_owned(), entry.clone()));
    }
    sortable.sort_by(|left, right| compare_utf16(&left.0, &right.0));
    owner.set(
        member,
        JsonValue::Array(sortable.into_iter().map(|(_, entry)| entry).collect()),
    );
}

fn sort_nested_string_member(owner: &mut quoin_store::JsonObject, member: &str, nested: &str) {
    let Some(JsonValue::Array(entries)) = owner.get(member) else {
        return;
    };
    let mut rebuilt = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            JsonValue::Object(fields) => {
                let mut fields = fields.clone();
                sort_string_member(&mut fields, nested);
                rebuilt.push(JsonValue::Object(fields));
            }
            other => rebuilt.push(other.clone()),
        }
    }
    owner.set(member, JsonValue::Array(rebuilt));
}
