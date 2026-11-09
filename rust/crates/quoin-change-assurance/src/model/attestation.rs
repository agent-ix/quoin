// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The proof attestation (FR-064).
//!
//! A faithful port of `validateAttestationShape` and `normalizeAttestation`
//! (`src/change-assurance/attestations.ts`).
//!
//! Two of the oracle's rules are worth naming, because both are about
//! **immutability of the thing attested to** rather than about syntax:
//!
//! * `tool.version` must be a release version or a 40-hex commit - never a
//!   branch name or `latest`. An attestation whose tool version can be
//!   repointed is an attestation that proves nothing later.
//! * `observed_at` must be an instant a calendar actually has. The oracle
//!   enforces this by running the regex **and then** `Date.parse`, which
//!   refuses `2026-02-30`; the regex alone would not.
//!
//! `normalizeAttestation` sorts `environment`'s keys. That is not reproduced
//! as a step here and does not need to be: `JsonObject` is a map with no
//! insertion order, and the JCS serializer sorts member names itself.

use std::collections::BTreeMap;
use std::str::FromStr;

use engineering_assurance::claim_strength::ClaimStrength;
use quoin_store::JsonValue;

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::ids::{AttestationId, NonEmptyText, ProofId, RecordId, Revision};
use crate::model::attestation_shape::{is_a_real_instant, is_an_immutable_version};
use crate::model::json::{Fields, number, object};
use crate::model::record::CommandBinding;

/// The one `record_type` a proof attestation carries.
pub const RECORD_TYPE: &str = "proof_attestation";

/// The schema version every attestation in this family carries.
pub const SCHEMA_VERSION: u64 = 1;

const SUBJECT: Subject = Subject::Attestation;

/// What a producer reported.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProducerResult {
    /// The producer ran and the check held.
    Passed,
    /// The producer ran and the check did not hold.
    Failed,
    /// The producer could not run.
    Unavailable,
    /// The producer ran and computed no verdict.
    NotComputed,
}

impl ProducerResult {
    /// Every result, in the oracle's declaration order.
    pub const ALL: [Self; 4] = [
        Self::Passed,
        Self::Failed,
        Self::Unavailable,
        Self::NotComputed,
    ];

    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
            Self::NotComputed => "not_computed",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// One environment observation.
///
/// Closed at four members because the oracle admits exactly
/// `string | number | boolean | null` and refuses a nested object or array.
#[derive(Clone, Debug, PartialEq)]
pub enum EnvironmentValue {
    /// A recorded absence.
    Null,
    /// A flag.
    Bool(bool),
    /// A finite number.
    Number(f64),
    /// Free text.
    Text(String),
}

impl EnvironmentValue {
    /// The JSON form.
    ///
    /// # Errors
    ///
    /// Never: a non-finite double cannot reach this type, because the parser
    /// refuses one and [`EnvironmentValue::read`] refuses one.
    pub fn to_json(&self) -> Result<JsonValue, ChangeAssuranceError> {
        Ok(match self {
            Self::Null => JsonValue::Null,
            Self::Bool(value) => JsonValue::Bool(*value),
            Self::Number(value) => JsonValue::number(*value)?,
            Self::Text(value) => JsonValue::string(value),
        })
    }

    /// Read one observation, refusing an object, an array and a non-finite
    /// number.
    fn read(value: &JsonValue, name: &str) -> Result<Self, ChangeAssuranceError> {
        match value {
            JsonValue::Null => Ok(Self::Null),
            JsonValue::Bool(flag) => Ok(Self::Bool(*flag)),
            JsonValue::Number(found) if found.get().is_finite() => Ok(Self::Number(found.get())),
            JsonValue::String(text) => Ok(Self::Text(text.clone())),
            _ => Err(ChangeAssuranceError::shape(
                SUBJECT,
                FieldFailure::malformed(format!("environment.{name}")),
            )),
        }
    }
}

/// The tool an attestation pins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolBinding {
    /// The tool's identity.
    pub identity: NonEmptyText,
    /// An immutable version: a release version or a 40-hex commit.
    pub version: NonEmptyText,
    /// The configuration reference it ran under.
    pub configuration_ref: NonEmptyText,
}

/// The producer output an attestation retains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedOutput {
    /// The output's media type.
    pub media_type: NonEmptyText,
    /// Its length in bytes.
    pub size_bytes: u64,
}

/// A proof attestation.
#[derive(Clone, Debug, PartialEq)]
pub struct ProofAttestation {
    /// The attestation's own identity, assigned by its producer.
    pub attestation_id: AttestationId,
    /// The change-assurance record it attests against, by identity.
    pub record_id: RecordId,
    /// The revision of that record.
    pub record_revision: Revision,
    /// The candidate revision it was taken at.
    pub candidate_revision: NonEmptyText,
    /// The proof obligation it discharges.
    pub proof_id: ProofId,
    /// The exact command that was run.
    pub command: CommandBinding,
    /// The tool that ran it.
    pub tool: ToolBinding,
    /// What was observed about the environment.
    pub environment: BTreeMap<String, EnvironmentValue>,
    /// When it was observed.
    pub observed_at: NonEmptyText,
    /// What the producer reported.
    pub result: ProducerResult,
    /// The kind of support behind this proof (FR-022, EA `ClaimStrength`).
    pub strength: ClaimStrength,
    /// The output retained alongside it.
    pub retained_output: RetainedOutput,
}

impl ProofAttestation {
    /// Read and validate an attestation.
    ///
    /// # Errors
    ///
    /// Every refusal of the v1 attestation schema, including an undeclared
    /// member such as a `digest`.
    pub fn from_json(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        read_attestation(value)
    }

    /// The JSON form.
    ///
    /// # Errors
    ///
    /// Never in practice: only [`JsonValue::number`]'s non-finite refusal,
    /// which no value of this type can hold.
    pub fn to_json(&self) -> Result<JsonValue, ChangeAssuranceError> {
        write_attestation(self)
    }
}

fn read_attestation(value: &JsonValue) -> Result<ProofAttestation, ChangeAssuranceError> {
    let root = Fields::read(value, SUBJECT, "attestation")?;
    root.exact(&[
        "schema_version",
        "record_type",
        "attestation_id",
        "record_id",
        "record_revision",
        "candidate_revision",
        "proof_id",
        "command",
        "tool",
        "environment",
        "observed_at",
        "result",
        "strength",
        "retained_output",
    ])?;
    root.equals("schema_version", &number(SCHEMA_VERSION)?)?;
    root.equals("record_type", &JsonValue::string(RECORD_TYPE))?;
    let attestation_id =
        AttestationId::parse(root.text("attestation_id")?, SUBJECT, "attestation_id")?;
    let record_id = RecordId::parse(root.text("record_id")?, SUBJECT, "record_id")?;
    let record_revision = Revision::new(
        root.whole_number("record_revision")?,
        SUBJECT,
        "record_revision",
    )?;
    let candidate_revision = NonEmptyText::parse(
        root.text("candidate_revision")?,
        SUBJECT,
        "candidate_revision",
    )?;
    let proof_id = ProofId::parse(root.text("proof_id")?, SUBJECT, "proof_id")?;
    let command = CommandBinding::from_json(root.value("command")?, SUBJECT)?;
    let tool = read_tool(root)?;
    let environment = read_environment(root)?;

    let observed_at = NonEmptyText::parse(root.text("observed_at")?, SUBJECT, "observed_at")?;
    if !is_a_real_instant(observed_at.as_str()) {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("observed_at"),
        ));
    }
    let result = ProducerResult::parse(root.text("result")?)
        .ok_or_else(|| ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("result")))?;
    let strength = ClaimStrength::from_str(root.text("strength")?)
        .map_err(|_| ChangeAssuranceError::shape(SUBJECT, FieldFailure::malformed("strength")))?;
    let retained_output = read_retained_output(root)?;

    Ok(ProofAttestation {
        attestation_id,
        record_id,
        record_revision,
        candidate_revision,
        proof_id,
        command,
        tool,
        environment,
        observed_at,
        result,
        strength,
        retained_output,
    })
}

fn read_tool(root: Fields<'_>) -> Result<ToolBinding, ChangeAssuranceError> {
    let tool = root.nested("tool")?;
    tool.exact(&["identity", "version", "configuration_ref"])?;
    let identity = NonEmptyText::parse(tool.text("identity")?, SUBJECT, "tool.identity")?;
    let version = NonEmptyText::parse(tool.text("version")?, SUBJECT, "tool.version")?;
    if !is_an_immutable_version(version.as_str()) {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("tool.version is not immutable"),
        ));
    }
    let configuration_ref = NonEmptyText::parse(
        tool.text("configuration_ref")?,
        SUBJECT,
        "tool.configuration_ref",
    )?;
    Ok(ToolBinding {
        identity,
        version,
        configuration_ref,
    })
}

fn read_environment(
    root: Fields<'_>,
) -> Result<BTreeMap<String, EnvironmentValue>, ChangeAssuranceError> {
    let environment = root.nested("environment")?;
    if environment.object().is_empty() {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("environment"),
        ));
    }
    let mut observations = BTreeMap::new();
    for (name, value) in environment.object() {
        observations.insert(name.clone(), EnvironmentValue::read(value, name)?);
    }
    Ok(observations)
}

fn read_retained_output(root: Fields<'_>) -> Result<RetainedOutput, ChangeAssuranceError> {
    let output = root.nested("retained_output")?;
    output.exact(&["media_type", "size_bytes"])?;
    let media_type = NonEmptyText::parse(
        output.text("media_type")?,
        SUBJECT,
        "retained_output.media_type",
    )?;
    let size_bytes = output.whole_number("size_bytes").map_err(|_| {
        ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("retained_output.size_bytes"),
        )
    })?;
    Ok(RetainedOutput {
        media_type,
        size_bytes,
    })
}

fn write_attestation(attestation: &ProofAttestation) -> Result<JsonValue, ChangeAssuranceError> {
    let mut environment = quoin_store::JsonObject::new();
    for (name, value) in &attestation.environment {
        environment.set(name.clone(), value.to_json()?);
    }
    Ok(object(vec![
        ("schema_version", number(SCHEMA_VERSION)?),
        ("record_type", JsonValue::string(RECORD_TYPE)),
        (
            "attestation_id",
            JsonValue::string(attestation.attestation_id.as_str()),
        ),
        (
            "record_id",
            JsonValue::string(attestation.record_id.as_str()),
        ),
        (
            "record_revision",
            number(attestation.record_revision.get())?,
        ),
        (
            "candidate_revision",
            JsonValue::string(attestation.candidate_revision.as_str()),
        ),
        ("proof_id", JsonValue::string(attestation.proof_id.as_str())),
        ("command", attestation.command.to_json()),
        (
            "tool",
            object(vec![
                (
                    "identity",
                    JsonValue::string(attestation.tool.identity.as_str()),
                ),
                (
                    "version",
                    JsonValue::string(attestation.tool.version.as_str()),
                ),
                (
                    "configuration_ref",
                    JsonValue::string(attestation.tool.configuration_ref.as_str()),
                ),
            ]),
        ),
        ("environment", JsonValue::Object(environment)),
        (
            "observed_at",
            JsonValue::string(attestation.observed_at.as_str()),
        ),
        ("result", JsonValue::string(attestation.result.as_str())),
        (
            "strength",
            JsonValue::string(attestation.strength.wire_name()),
        ),
        (
            "retained_output",
            object(vec![
                (
                    "media_type",
                    JsonValue::string(attestation.retained_output.media_type.as_str()),
                ),
                (
                    "size_bytes",
                    number(attestation.retained_output.size_bytes)?,
                ),
            ]),
        ),
    ]))
}
