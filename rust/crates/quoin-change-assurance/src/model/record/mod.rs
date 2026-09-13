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

//! The file is split on responsibility, not on size (quoin#457, #464): this
//! module holds the shapes and their invariants, [`read`] turns a document into
//! them, [`write`] turns them back into a document, and [`normalize`] orders an
//! unvalidated document's members. Reading and writing are each a single pass
//! over the whole record and neither knows the other exists, which is why they
//! separate cleanly; the shapes are what all three agree about.

mod normalize;
mod read;
mod write;

pub use self::normalize::normalize_record;

use self::read::read_record;
use self::write::write_record;

use quoin_store::{CanonicalDigest, JsonValue};

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::ids::{
    ArtifactDigest, ImpactIdentity, NonEmptyText, ObligationId, ProofId, RecordId, Revision, RunId,
    SourceId, StatementId,
};
use crate::model::json::{Fields, object, string_values};

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

pub(super) const SUBJECT: Subject = Subject::Record;

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
