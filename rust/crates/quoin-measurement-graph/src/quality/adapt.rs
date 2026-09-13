// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Transcribe one retained producer observation into one governed collection.
//!
//! Ports `adaptGraphQualityObservation` and `AdaptGraphQualityInput`
//! (`src/measurement/graph-adapters.ts:470-576`).
//!
//! # The checks, in the retained order
//!
//! The order is load-bearing: the first refusal is the one a caller sees, and
//! a caller fixing an observation should be told about the observation before
//! being told about the attachment.
//!
//! 1. the record satisfies `graph-quality-observation-v1`;
//! 2. its `observation_id` re-derives from its own content;
//! 3. the scorer attachment has a media type;
//! 4. the attachment's digest is the one the record claims;
//! 5. the attestation satisfies its contract, instant included;
//! 6. exactly one active plan governs the observed definition version;
//! 7. the transcription states no partition twice;
//! 8. the collection satisfies the measurement contract.
//!
//! # One check is unrepresentable rather than absent
//!
//! `graph-adapters.ts:491` is `if (!(input.scorerBytes instanceof Uint8Array))`
//! — a guard against a caller who ignored the declared type. Here the member is
//! `&[u8]`, so there is nothing to guard against and the branch is gone rather
//! than dead. An empty slice is still a slice and is still transcribed, exactly
//! as an empty `Uint8Array` is.

use quoin_measurement::{MeasurementCollection, MeasurementPlan, json_bridge, validate};
use quoin_store::digest_bytes_sha256;
use serde_json::{Value, json};

use crate::attestation::InvocationAttestation;
use crate::base64;
use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};

use super::normalize::normalize_graph_quality;
use super::producer::MeasurementPlanReference;

/// Read one member of an already-validated document.
///
/// Every name this is called with is a member
/// [`InvocationAttestation`] requires, so the fallback is unreachable rather
/// than a policy. It is `null` rather than a panic because an unreachable
/// branch that cannot be proven to the compiler should still be total.
fn member(document: &Value, name: &str) -> Value {
    document.get(name).cloned().unwrap_or(Value::Null)
}

/// The metric every plan this adapter can run under governs.
const METRIC: &str = "graph_quality";

/// The tool whose observations this adapter transcribes.
const TOOL_IDENTITY: &str = "agent-ix/quire-code-rs";

/// One transcription: the evidence, and the type that proves it is governed.
///
/// Both halves, because both are load-bearing and neither derives the other.
/// The retained function returns one object that is both, which TypeScript
/// allows because its "type" is a compile-time claim about that object. Here
/// [`MeasurementCollection`] is a parsed value that deliberately does not model
/// every member of the document — `rawEvidence` and `scope` stay opaque — and
/// [`quoin_measurement::write_measurement_collection`] writes the **document**,
/// not a re-serialization of the parsed value, so that nothing this crate does
/// not model is dropped on the way to disk.
#[derive(Clone, Debug)]
pub struct TranscribedCollection {
    document: quoin_store::JsonValue,
    collection: MeasurementCollection,
}

impl TranscribedCollection {
    /// The transcribed evidence, ready for
    /// [`quoin_measurement::write_measurement_collection`].
    #[must_use]
    pub const fn document(&self) -> &quoin_store::JsonValue {
        &self.document
    }

    /// The parsed collection.
    #[must_use]
    pub const fn collection(&self) -> &MeasurementCollection {
        &self.collection
    }

    /// Both halves, for a caller that keeps them apart.
    #[must_use]
    pub fn into_parts(self) -> (quoin_store::JsonValue, MeasurementCollection) {
        (self.document, self.collection)
    }
}

/// Everything one transcription needs.
#[derive(Clone, Copy, Debug)]
pub struct AdaptGraphQualityInput<'a> {
    /// The retained producer observation.
    pub record: &'a Value,
    /// The scorer's complete output, as bytes.
    pub scorer_bytes: &'a [u8],
    /// The media type those bytes are in.
    pub scorer_media_type: &'a str,
    /// The invocation attestation the collection inherits its identity from.
    pub attestation: &'a Value,
    /// Every plan the caller loaded, graph-quality or not.
    pub plans: &'a [MeasurementPlan],
}

/// Transcribe a retained producer observation into one governed collection.
///
/// # Errors
///
/// Every code in [`GraphAdapterErrorCode`] except
/// [`GraphAdapterErrorCode::UnknownAdapter`] and
/// [`GraphAdapterErrorCode::InvalidPremise`], which belong to the other two
/// entry points. See the module header for the order they are raised in.
pub fn adapt_graph_quality_observation(
    input: &AdaptGraphQualityInput<'_>,
) -> Result<TranscribedCollection> {
    let record = super::parse(input.record)?;

    let expected = super::identity::graph_quality_observation_id(input.record)?;
    if record.observation_id != expected {
        return Err(GraphAdapterError::new(
            GraphAdapterErrorCode::InvalidObservation,
            format!(
                "observation_id expected {expected}; observed {}",
                record.observation_id
            ),
        ));
    }

    if input.scorer_media_type.is_empty() {
        return Err(GraphAdapterError::new(
            GraphAdapterErrorCode::AttachmentMissing,
            "scorer attachment media type is required",
        ));
    }
    let observed_digest = digest_bytes_sha256(input.scorer_bytes);
    if observed_digest != *record.raw_scorer_output.digest.digest() {
        return Err(GraphAdapterError::new(
            GraphAdapterErrorCode::AttachmentDigestMismatch,
            format!(
                "raw_scorer_output.digest expected {}; observed {}",
                record.raw_scorer_output.digest,
                observed_digest.to_stored()
            ),
        ));
    }

    let attestation = InvocationAttestation::parse(input.attestation)?;
    let plan = governing_plan(input.plans)?;

    let identity = input
        .record
        .get("population")
        .cloned()
        .unwrap_or(Value::Null);
    let observations = normalize_graph_quality(&record, &identity)?;

    let collection = json!({
        "schemaVersion": quoin_measurement::MEASUREMENT_SCHEMA_VERSION,
        "collectionId": format!("graph-quality-{}", record.observation_id.as_hex()),
        "subject": attestation.subject.as_str(),
        // The three opaque members are taken from the *received* document
        // rather than re-serialized from the parsed one. They are opaque by
        // declaration — `z.unknown()` and two `Record`s — so a re-serialization
        // would be this crate inventing a spelling for something it was asked
        // not to interpret. `deny_unknown_fields` is what makes the two the
        // same bytes: nothing reached the parsed value that was not received,
        // and nothing was received that the parsed value dropped.
        "scope": member(input.attestation, "scope"),
        "toolIdentity": TOOL_IDENTITY,
        "toolVersion": record.producer.extractor_revision.as_str(),
        "configDigest": record.producer.configuration_digest.to_stored(),
        "timestamp": attestation.timestamp.as_str(),
        "sourceRevision": record.producer.source_revision.as_str(),
        "corpusRevision": record.producer.corpus_revision.as_str(),
        "environment": member(input.attestation, "environment"),
        "verificationStack": member(input.attestation, "verificationStack"),
        "observations": observations,
        "rawEvidence": {
            "producer": input.record,
            "scorer": {
                "path": record.raw_scorer_output.path.as_str(),
                "digest": record.raw_scorer_output.digest.to_stored(),
                "mediaType": input.scorer_media_type,
                "bytesBase64": base64::encode(input.scorer_bytes),
            },
        },
    });

    let governing: Vec<MeasurementPlan> = input
        .plans
        .iter()
        .filter(|candidate| candidate.metric.as_str() != METRIC)
        .cloned()
        .chain(std::iter::once(plan))
        .collect();
    let document = json_bridge::from_serde(&collection)
        .map_err(|error| GraphAdapterError::new(GraphAdapterErrorCode::Store, error.to_string()))?;
    let collection = validate::measurement_collection(&document, &governing).map_err(|error| {
        GraphAdapterError::new(GraphAdapterErrorCode::CollectionRefused, error.to_string())
    })?;
    Ok(TranscribedCollection {
        document,
        collection,
    })
}

/// The one active plan that governs the observed definition version.
///
/// Two conditions, both from `graph-adapters.ts:531-536`: exactly one active
/// `graph_quality` plan exists at all, and it is the one the record names. The
/// first is what makes the second unambiguous — two active plans for one metric
/// is a governance defect whichever of them matches.
fn governing_plan(plans: &[MeasurementPlan]) -> Result<MeasurementPlan> {
    use quoin_measurement::types::plan::LifecycleStatus;

    let definition_version = crate::wire::GraphQualityDefinitionVersion::SPELLING;
    let active: Vec<&MeasurementPlan> = plans
        .iter()
        .filter(|candidate| {
            candidate.metric.as_str() == METRIC && candidate.status == LifecycleStatus::Active
        })
        .collect();
    let matching: Vec<&MeasurementPlan> = active
        .iter()
        .copied()
        .filter(|candidate| {
            candidate.id.as_str() == MeasurementPlanReference::PLAN_ID
                && candidate.definition_version.as_str() == definition_version
        })
        .collect();
    if let ([_], [only]) = (active.as_slice(), matching.as_slice()) {
        return Ok((*only).clone());
    }

    let observed: Vec<String> = plans
        .iter()
        .filter(|candidate| candidate.metric.as_str() == METRIC)
        .map(|candidate| {
            format!(
                "{}/{}/{}",
                candidate.id,
                candidate.definition_version,
                candidate.status.as_str()
            )
        })
        .collect();
    let observed = if observed.is_empty() {
        "no graph_quality plan".to_owned()
    } else {
        observed.join(", ")
    };
    Err(GraphAdapterError::new(
        GraphAdapterErrorCode::InactivePlan,
        format!(
            "expected exactly one active {}/{definition_version}; observed {observed}",
            MeasurementPlanReference::PLAN_ID
        ),
    ))
}
