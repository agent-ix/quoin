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
//! * `tool.version` must be a release version, a 40-hex commit or a 64-hex
//!   digest — never a branch name or `latest`. An attestation whose tool
//!   version can be repointed is an attestation that proves nothing later.
//! * `observed_at` must be an instant a calendar actually has. The oracle
//!   enforces this by running the regex **and then** `Date.parse`, which
//!   refuses `2026-02-30`; the regex alone would not.
//!
//! `normalizeAttestation` sorts `environment`'s keys. That is not reproduced
//! as a step here and does not need to be: `JsonObject` is a map with no
//! insertion order, and the JCS serializer sorts member names itself. The
//! oracle's sort is observable only through the digest, and the digest is
//! taken over JCS bytes in both trees.

use std::collections::BTreeMap;

use quoin_store::{CanonicalDigest, JsonValue, RawBytesDigest};

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::ids::{ArtifactDigest, AttestationId, NonEmptyText, ProofId};
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
    /// An immutable version: a release version, a 40-hex commit, or a 64-hex
    /// digest.
    pub version: NonEmptyText,
    /// The configuration it ran under.
    pub configuration_digest: ArtifactDigest,
}

/// The producer output an attestation retains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedOutput {
    /// The output's media type.
    pub media_type: NonEmptyText,
    /// Its blake3 digest over the bytes as produced.
    pub digest: RawBytesDigest,
    /// Its length in bytes.
    pub size_bytes: u64,
}

/// A sealed proof attestation.
#[derive(Clone, Debug, PartialEq)]
pub struct ProofAttestation {
    /// The attestation's own identity, distinct from its digest.
    pub attestation_id: AttestationId,
    /// Its own digest.
    pub digest: CanonicalDigest,
    /// The change-assurance record it attests against.
    pub record_digest: CanonicalDigest,
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
    /// The output retained alongside it.
    pub retained_output: RetainedOutput,
}

/// An attestation read but not yet sealed: everything but the `digest`.
#[derive(Clone, Debug, PartialEq)]
pub struct UnsealedAttestation {
    /// The attestation's own identity.
    pub attestation_id: AttestationId,
    /// The change-assurance record it attests against.
    pub record_digest: CanonicalDigest,
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
    /// The output retained alongside it.
    pub retained_output: RetainedOutput,
}

impl UnsealedAttestation {
    /// Read and validate the unsealed shape.
    ///
    /// # Errors
    ///
    /// Every refusal `validateAttestationShape(value, false)` produces.
    pub fn from_json(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        read_attestation(value, false).map(|(unsealed, _)| unsealed)
    }

    /// The JSON form, without a `digest` member.
    ///
    /// # Errors
    ///
    /// Never in practice: only [`JsonValue::number`]'s non-finite refusal,
    /// which no value of this type can hold.
    pub fn to_json(&self) -> Result<JsonValue, ChangeAssuranceError> {
        write_attestation(self, None)
    }

    /// Attach a digest, producing the sealed attestation.
    #[must_use]
    pub fn seal_with(self, digest: CanonicalDigest) -> ProofAttestation {
        ProofAttestation {
            attestation_id: self.attestation_id,
            digest,
            record_digest: self.record_digest,
            candidate_revision: self.candidate_revision,
            proof_id: self.proof_id,
            command: self.command,
            tool: self.tool,
            environment: self.environment,
            observed_at: self.observed_at,
            result: self.result,
            retained_output: self.retained_output,
        }
    }
}

impl ProofAttestation {
    /// Read and validate the sealed shape, **without** checking the digest.
    ///
    /// Not public: [`crate::attestations::verify_attestation`] is the only way
    /// to obtain a `ProofAttestation` from bytes, and it checks the digest.
    pub(crate) fn read_sealed(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        let (unsealed, digest) = read_attestation(value, true)?;
        let digest = digest
            .ok_or_else(|| ChangeAssuranceError::shape(SUBJECT, FieldFailure::missing("digest")))?;
        Ok(unsealed.seal_with(digest))
    }

    /// Everything but the digest.
    #[must_use]
    pub fn unsealed(&self) -> UnsealedAttestation {
        UnsealedAttestation {
            attestation_id: self.attestation_id.clone(),
            record_digest: self.record_digest.clone(),
            candidate_revision: self.candidate_revision.clone(),
            proof_id: self.proof_id.clone(),
            command: self.command.clone(),
            tool: self.tool.clone(),
            environment: self.environment.clone(),
            observed_at: self.observed_at.clone(),
            result: self.result,
            retained_output: self.retained_output.clone(),
        }
    }

    /// The JSON form, `digest` included.
    ///
    /// # Errors
    ///
    /// As [`UnsealedAttestation::to_json`].
    pub fn to_json(&self) -> Result<JsonValue, ChangeAssuranceError> {
        write_attestation(&self.unsealed(), Some(&self.digest))
    }
}

fn read_attestation(
    value: &JsonValue,
    sealed: bool,
) -> Result<(UnsealedAttestation, Option<CanonicalDigest>), ChangeAssuranceError> {
    let root = Fields::read(value, SUBJECT, "attestation")?;
    let allowed: &[&str] = if sealed {
        &[
            "schema_version",
            "record_type",
            "attestation_id",
            "digest",
            "record_digest",
            "candidate_revision",
            "proof_id",
            "command",
            "tool",
            "environment",
            "observed_at",
            "result",
            "retained_output",
        ]
    } else {
        &[
            "schema_version",
            "record_type",
            "attestation_id",
            "record_digest",
            "candidate_revision",
            "proof_id",
            "command",
            "tool",
            "environment",
            "observed_at",
            "result",
            "retained_output",
        ]
    };
    root.exact(allowed)?;
    root.equals("schema_version", &number(SCHEMA_VERSION)?)?;
    root.equals("record_type", &JsonValue::string(RECORD_TYPE))?;
    let attestation_id =
        AttestationId::parse(root.text("attestation_id")?, SUBJECT, "attestation_id")?;
    let digest = if sealed {
        Some(root.digest("digest")?)
    } else {
        None
    };
    let record_digest = root.digest("record_digest")?;
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
    let retained_output = read_retained_output(root)?;

    Ok((
        UnsealedAttestation {
            attestation_id,
            record_digest,
            candidate_revision,
            proof_id,
            command,
            tool,
            environment,
            observed_at,
            result,
            retained_output,
        },
        digest,
    ))
}

fn read_tool(root: Fields<'_>) -> Result<ToolBinding, ChangeAssuranceError> {
    let tool = root.nested("tool")?;
    tool.exact(&["identity", "version", "configuration_digest"])?;
    let identity = NonEmptyText::parse(tool.text("identity")?, SUBJECT, "tool.identity")?;
    let version = NonEmptyText::parse(tool.text("version")?, SUBJECT, "tool.version")?;
    if !is_an_immutable_version(version.as_str()) {
        return Err(ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("tool.version is not immutable"),
        ));
    }
    let configuration_digest = tool.artifact_digest("configuration_digest")?;
    Ok(ToolBinding {
        identity,
        version,
        configuration_digest,
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
    output.exact(&["media_type", "digest", "size_bytes"])?;
    let media_type = NonEmptyText::parse(
        output.text("media_type")?,
        SUBJECT,
        "retained_output.media_type",
    )?;
    let digest = output.raw_bytes_digest("digest")?;
    let size_bytes = output.whole_number("size_bytes").map_err(|_| {
        ChangeAssuranceError::shape(
            SUBJECT,
            FieldFailure::malformed("retained_output.size_bytes"),
        )
    })?;
    Ok(RetainedOutput {
        media_type,
        digest,
        size_bytes,
    })
}

fn write_attestation(
    attestation: &UnsealedAttestation,
    digest: Option<&CanonicalDigest>,
) -> Result<JsonValue, ChangeAssuranceError> {
    let mut environment = quoin_store::JsonObject::new();
    for (name, value) in &attestation.environment {
        environment.set(name.clone(), value.to_json()?);
    }
    let mut members = vec![
        ("schema_version", number(SCHEMA_VERSION)?),
        ("record_type", JsonValue::string(RECORD_TYPE)),
        (
            "attestation_id",
            JsonValue::string(attestation.attestation_id.as_str()),
        ),
    ];
    if let Some(digest) = digest {
        members.push(("digest", JsonValue::string(digest.as_hex())));
    }
    members.extend([
        (
            "record_digest",
            JsonValue::string(attestation.record_digest.as_hex()),
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
                    "configuration_digest",
                    JsonValue::string(attestation.tool.configuration_digest.as_hex()),
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
            "retained_output",
            object(vec![
                (
                    "media_type",
                    JsonValue::string(attestation.retained_output.media_type.as_str()),
                ),
                (
                    "digest",
                    JsonValue::string(attestation.retained_output.digest.as_hex()),
                ),
                (
                    "size_bytes",
                    number(attestation.retained_output.size_bytes)?,
                ),
            ]),
        ),
    ]);
    Ok(object(members))
}

/// Whether `version` is one of the three immutable spellings the oracle admits.
///
/// `^(?:v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|[a-f0-9]{64})$`.
#[must_use]
pub fn is_an_immutable_version(version: &str) -> bool {
    if is_lowercase_hex(version, 40) || is_lowercase_hex(version, 64) {
        return true;
    }
    let core = version.strip_prefix('v').unwrap_or(version);
    let (numbers, suffix) = match core.find(['-', '+']) {
        Some(at) => {
            let (head, tail) = core.split_at(at);
            (head, Some(&tail[1..]))
        }
        None => (core, None),
    };
    let mut parts = numbers.split('.');
    let triple = [parts.next(), parts.next(), parts.next()];
    if parts.next().is_some() {
        return false;
    }
    for part in triple {
        match part {
            Some(digits) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {}
            _ => return false,
        }
    }
    match suffix {
        None => true,
        Some(tail) => {
            !tail.is_empty()
                && tail
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
        }
    }
}

fn is_lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether `observed` is both the oracle's timestamp shape and an instant a
/// calendar has.
///
/// The oracle runs the regex **and** `Date.parse`, and the second refuses what
/// the first cannot see: `2026-02-30T00:00:00Z` matches the pattern and is not
/// a date. This is the whole of that pair, including ECMAScript's admission of
/// hour `24` only at exactly midnight.
#[must_use]
pub fn is_a_real_instant(observed: &str) -> bool {
    let bytes = observed.as_bytes();
    let digits = |from: usize, count: usize| -> Option<u32> {
        let slice = observed.get(from..from.checked_add(count)?)?;
        if slice.len() == count && slice.bytes().all(|byte| byte.is_ascii_digit()) {
            slice.parse().ok()
        } else {
            None
        }
    };
    if bytes.len() < 20 {
        return false;
    }
    let (Some(year), Some(month), Some(day)) = (digits(0, 4), digits(5, 2), digits(8, 2)) else {
        return false;
    };
    let (Some(hour), Some(minute), Some(second)) = (digits(11, 2), digits(14, 2), digits(17, 2))
    else {
        return false;
    };
    if observed.get(4..5) != Some("-")
        || observed.get(7..8) != Some("-")
        || observed.get(10..11) != Some("T")
        || observed.get(13..14) != Some(":")
        || observed.get(16..17) != Some(":")
    {
        return false;
    }
    let Some(rest) = observed.get(19..) else {
        return false;
    };
    let (fraction, zone) = match rest.strip_prefix('.') {
        Some(tail) => {
            let end = tail
                .find(|character: char| !character.is_ascii_digit())
                .unwrap_or(tail.len());
            if end == 0 {
                return false;
            }
            let (Some(digits), Some(zone)) = (tail.get(..end), tail.get(end..)) else {
                return false;
            };
            (digits, zone)
        }
        None => ("", rest),
    };
    if !is_a_valid_zone(zone) {
        return false;
    }
    if month == 0 || month > 12 || day == 0 || day > days_in_month(year, month) {
        return false;
    }
    if minute > 59 || second > 59 {
        return false;
    }
    // ECMAScript's Date Time String Format admits hour 24, and only as exactly
    // midnight: `T24:00:00` is the following day, `T24:00:01` is not a time.
    if hour > 24
        || (hour == 24 && (minute != 0 || second != 0 || fraction.bytes().any(|b| b != b'0')))
    {
        return false;
    }
    true
}

fn is_a_valid_zone(zone: &str) -> bool {
    if zone == "Z" {
        return true;
    }
    let Some(rest) = zone.strip_prefix(['+', '-']) else {
        return false;
    };
    if rest.len() != 5 || rest.get(2..3) != Some(":") {
        return false;
    }
    let (Some(hours), Some(minutes)) = (rest.get(..2), rest.get(3..)) else {
        return false;
    };
    let digits = |text: &str| -> Option<u32> {
        if text.bytes().all(|byte| byte.is_ascii_digit()) {
            text.parse().ok()
        } else {
            None
        }
    };
    matches!((digits(hours), digits(minutes)), (Some(hours), Some(minutes)) if hours <= 23 && minutes <= 59)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use super::{is_a_real_instant, is_an_immutable_version};

    #[test]
    fn immutable_versions_are_releases_commits_and_digests_and_nothing_else() {
        assert!(is_an_immutable_version("1.2.3"));
        assert!(is_an_immutable_version("v1.2.3"));
        assert!(is_an_immutable_version("1.2.3-rc.1"));
        assert!(is_an_immutable_version("1.2.3+build.5"));
        assert!(is_an_immutable_version(&"a".repeat(40)));
        assert!(is_an_immutable_version(&"0".repeat(64)));
        assert!(!is_an_immutable_version("latest"));
        assert!(!is_an_immutable_version("main"));
        assert!(!is_an_immutable_version("1.2"));
        assert!(!is_an_immutable_version("1.2.3."));
        assert!(!is_an_immutable_version("1.2.3-"));
        assert!(!is_an_immutable_version(&"A".repeat(40)));
    }

    #[test]
    fn a_timestamp_must_be_an_instant_a_calendar_has() {
        assert!(is_a_real_instant("2026-02-28T00:00:00Z"));
        assert!(is_a_real_instant("2024-02-29T23:59:59.123Z"));
        assert!(is_a_real_instant("2026-01-01T00:00:00+05:30"));
        assert!(is_a_real_instant("2026-01-01T24:00:00Z"));
        assert!(!is_a_real_instant("2026-02-30T00:00:00Z"));
        assert!(!is_a_real_instant("2026-13-01T00:00:00Z"));
        assert!(!is_a_real_instant("2026-01-01T24:00:01Z"));
        assert!(!is_a_real_instant("2026-01-01T00:60:00Z"));
        assert!(!is_a_real_instant("2026-01-01T00:00:00"));
        assert!(!is_a_real_instant("2026-01-01T00:00:00+24:00"));
        assert!(!is_a_real_instant("2026-01-01 00:00:00Z"));
    }
}
