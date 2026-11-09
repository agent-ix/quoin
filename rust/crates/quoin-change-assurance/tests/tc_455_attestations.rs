// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The proof-attestation schema and what intake keeps of it (FR-064).
//!
//! An attestation is addressed by its `attestation_id` and carries no digest,
//! so every case here is about what the schema admits and refuses, not about
//! bytes it hashes to.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use engineering_assurance::claim_strength::ClaimStrength;
use quoin_change_assurance::attestations::read_attestation_json;
use quoin_change_assurance::error::{ChangeAssuranceError, FieldFailure};
use quoin_change_assurance::model::attestation::ProducerResult;
use quoin_store::JsonValue;

use crate::common::{OUTPUT, attestation_text, json, names, replacing, without};

/// Trace: FR-064-AC-1
///
/// The attestation schema is closed: the scenario attestation reads, dropping
/// any member refuses it, and a member the schema does not name - a `digest`
/// among them - refuses it.
#[test]
fn tc_455_the_attestation_schema_names_every_member_and_admits_no_other() {
    let attestation = json(&attestation_text());
    read_attestation_json(&attestation).expect("the scenario attestation reads");
    let members = names(&attestation);
    for member_name in &members {
        assert!(
            read_attestation_json(&without(&attestation, member_name)).is_err(),
            "dropping `{member_name}` must refuse the attestation"
        );
    }
    for undeclared in ["undeclared", "digest"] {
        assert!(
            read_attestation_json(&replacing(&attestation, undeclared, JsonValue::string("x")))
                .is_err(),
            "an undeclared `{undeclared}` member must refuse the attestation"
        );
    }
}

/// Trace: FR-064-AC-5
///
/// Each absent member is refused as that member, not as a generic malformed
/// attestation and not by inferring a value for it.
#[test]
fn tc_455_each_absent_member_is_refused_by_name_rather_than_inferred() {
    let attestation = json(&attestation_text());
    let members = names(&attestation);
    for member_name in &members {
        match read_attestation_json(&without(&attestation, member_name)) {
            Err(ChangeAssuranceError::Shape {
                failure: FieldFailure::Missing { field },
                ..
            }) => assert_eq!(
                &field, member_name,
                "dropping `{member_name}` must be refused by that name"
            ),
            other => panic!("dropping `{member_name}` was answered with {other:?}"),
        }
    }
}

/// Trace: FR-064-AC-2
///
/// The four producer results stay four. Each reads back as itself and none is
/// turned into a verification outcome on the way: `result` is the producer's
/// word, retained, and a verification outcome is refused in its place.
#[test]
fn tc_455_the_four_producer_results_stay_four_distinct_states() {
    let attestation = json(&attestation_text());
    let mut seen = Vec::new();
    for result in ProducerResult::ALL {
        let read = read_attestation_json(&replacing(
            &attestation,
            "result",
            JsonValue::string(result.as_str()),
        ))
        .unwrap_or_else(|error| panic!("`{}` must read: {error}", result.as_str()));
        assert_eq!(
            read.result,
            result,
            "`{}` must read back as itself",
            result.as_str()
        );
        seen.push(read.result);
    }
    assert_eq!(seen.len(), 4, "there are four producer states");
    for (index, state) in seen.iter().enumerate() {
        assert!(
            !seen[..index].contains(state),
            "the four results must be four distinct states"
        );
    }
    assert!(
        read_attestation_json(&replacing(
            &attestation,
            "result",
            JsonValue::string("valid")
        ))
        .is_err(),
        "a verification outcome is not a producer result and must be refused"
    );
}

/// Trace: FR-064-AC-7
///
/// A producer that could not run, or ran and computed nothing, still retains
/// its diagnostic output: the attestation reads, and records the size of the
/// bytes kept. What is never created is evidence for a producer that reported
/// nothing at all - that is an absent attestation, not a synthetic one, so an
/// attestation with no `result` is refused rather than defaulted.
#[test]
fn tc_455_a_producer_that_reported_nothing_useful_still_retains_its_output() {
    let attestation = json(&attestation_text());
    for result in [ProducerResult::Unavailable, ProducerResult::NotComputed] {
        let read = read_attestation_json(&replacing(
            &attestation,
            "result",
            JsonValue::string(result.as_str()),
        ))
        .unwrap_or_else(|error| panic!("`{}` must read: {error}", result.as_str()));
        assert_eq!(
            read.retained_output.size_bytes,
            u64::try_from(OUTPUT.len()).unwrap(),
            "`{}`: a diagnostic output must be retained whole",
            result.as_str()
        );
    }
    assert!(
        matches!(
            read_attestation_json(&without(&attestation, "result")),
            Err(ChangeAssuranceError::Shape {
                failure: FieldFailure::Missing { .. },
                ..
            })
        ),
        "no result means no attestation, not a defaulted one"
    );
}

/// Trace: FR-064-AC-10
/// Provenance: PLAT-972
///
/// A proof attestation carries exactly one EA `ClaimStrength`. Every declared
/// wire name reads back as itself, and writing it out again produces the same
/// spelling that was read in.
#[test]
fn tc_455_every_claim_strength_round_trips() {
    let attestation = json(&attestation_text());
    let mut seen = std::collections::BTreeSet::new();
    for strength in ClaimStrength::ALL {
        let read = read_attestation_json(&replacing(
            &attestation,
            "strength",
            JsonValue::string(strength.wire_name()),
        ))
        .unwrap_or_else(|error| panic!("`{}` must read: {error}", strength.wire_name()));
        assert_eq!(
            read.strength.wire_name(),
            strength.wire_name(),
            "`{}` must read back as itself",
            strength.wire_name()
        );
        let written = read.to_json().unwrap();
        assert_eq!(
            written
                .as_object()
                .unwrap()
                .get("strength")
                .and_then(JsonValue::as_str),
            Some(strength.wire_name())
        );
        seen.insert(strength.wire_name());
    }
}

/// Trace: FR-064-AC-10
/// Provenance: PLAT-972
///
/// An attestation with no `strength` is refused by name, not silently
/// defaulted to one of the four values.
#[test]
fn tc_455_an_attestation_with_no_strength_is_refused_not_defaulted() {
    let attestation = json(&attestation_text());
    match read_attestation_json(&without(&attestation, "strength")) {
        Err(ChangeAssuranceError::Shape {
            failure: FieldFailure::Missing { field },
            ..
        }) => assert_eq!(field, "strength"),
        other => panic!("expected a Missing(\"strength\") refusal, got {other:?}"),
    }
}

/// Trace: FR-064-AC-10
/// Provenance: PLAT-972
///
/// A `strength` outside the EA vocabulary is refused as malformed, not
/// coerced to a known value.
#[test]
fn tc_455_an_unknown_strength_is_refused() {
    let attestation = json(&attestation_text());
    match read_attestation_json(&replacing(
        &attestation,
        "strength",
        JsonValue::string("verified"),
    )) {
        Err(ChangeAssuranceError::Shape {
            failure: FieldFailure::Malformed { field },
            ..
        }) => assert_eq!(field, "strength"),
        other => panic!("expected a Malformed(\"strength\") refusal, got {other:?}"),
    }
}
