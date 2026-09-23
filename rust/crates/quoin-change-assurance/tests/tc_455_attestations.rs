// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Sealing and verifying one producer's proof attestation (FR-064), against
//! attestations the retained TypeScript sealed.
//!
//! An attestation is a producer's report. Nothing in this crate turns one into
//! a verdict, and the tests here are written so that a port which quietly did
//! would fail: the four producer results are checked to survive as four.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use common::{bytes, member, oracle, section, text};
use engineering_assurance::claim_strength::ClaimStrength;
use quoin_change_assurance::attestations::{
    attestation_bytes, seal_attestation, verify_attestation,
};
use quoin_change_assurance::error::{ChangeAssuranceError, FieldFailure};
use quoin_change_assurance::model::attestation::ProducerResult;
use quoin_store::{JsonValue, canonical_bytes};

/// The captured attestations, as `(name, sealed, output)`.
fn attestations() -> Vec<(String, JsonValue, Vec<u8>)> {
    section(&oracle(), "attestations")
        .iter()
        .map(|captured| {
            (
                text(captured, "name").to_owned(),
                member(captured, "sealed").clone(),
                bytes(member(captured, "output")),
            )
        })
        .collect()
}

fn replacing(value: &JsonValue, name: &str, replacement: JsonValue) -> JsonValue {
    let mut members = value.as_object().unwrap().clone();
    members.set(name, replacement);
    JsonValue::Object(members)
}

fn without(value: &JsonValue, name: &str) -> JsonValue {
    let mut members = value.as_object().unwrap().clone();
    members.remove(name);
    JsonValue::Object(members)
}

/// Trace: FR-064-AC-1
///
/// The attestation schema is closed: every captured attestation verifies,
/// dropping any member refuses it, and a member the schema does not name
/// refuses it.
#[test]
fn tc_455_the_attestation_schema_names_every_member_and_admits_no_other() {
    let attestations = attestations();
    assert!(
        attestations.len() >= 4,
        "anti-vacuity floor: at least 4 captured attestations, saw {}",
        attestations.len()
    );
    let mut dropped = 0_usize;
    for (name, sealed, _) in &attestations {
        verify_attestation(sealed).unwrap_or_else(|error| panic!("{name}: {error}"));
        for member_name in sealed
            .as_object()
            .unwrap()
            .names()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        {
            assert!(
                verify_attestation(&without(sealed, &member_name)).is_err(),
                "{name}: dropping `{member_name}` must refuse the attestation"
            );
            dropped += 1;
        }
        assert!(
            verify_attestation(&replacing(sealed, "undeclared", JsonValue::string("x"))).is_err(),
            "{name}: an undeclared member must refuse the attestation"
        );
    }
    assert!(
        dropped >= 40,
        "anti-vacuity floor: at least 40 member drops exercised, saw {dropped}"
    );
}

/// Trace: FR-064-AC-5
///
/// Each absent member is refused *as that member*, not as a generic malformed
/// attestation and not by inferring a value for it. The refusal names the
/// field, which is the difference between a message a reader can act on and
/// one they cannot.
#[test]
fn tc_455_each_absent_member_is_refused_by_name_rather_than_inferred() {
    let (_, sealed, _) = attestations().into_iter().next().expect("an attestation");
    let mut named = 0_usize;
    for member_name in sealed
        .as_object()
        .unwrap()
        .names()
        .map(str::to_owned)
        .collect::<Vec<_>>()
    {
        match verify_attestation(&without(&sealed, &member_name)) {
            Err(ChangeAssuranceError::Shape {
                failure: FieldFailure::Missing { field },
                ..
            }) => {
                assert_eq!(
                    field, member_name,
                    "dropping `{member_name}` must be refused by that name"
                );
                named += 1;
            }
            Err(ChangeAssuranceError::DigestMismatch { .. }) => {
                // Dropping `digest` itself is refused as an absent member
                // before any digest is recomputed; anything else reaching here
                // would mean the shape pass let the drop through.
                assert_eq!(member_name, "digest");
            }
            other => panic!("dropping `{member_name}` was answered with {other:?}"),
        }
    }
    assert!(
        named >= 10,
        "anti-vacuity floor: at least 10 members refused by name, saw {named}"
    );
}

/// Trace: FR-064-AC-2
///
/// The four producer results stay four. Each seals, verifies and reads back as
/// itself, and none of them is turned into a verification outcome on the way:
/// the type that comes out of `verify_attestation` has no outcome member at
/// all, and `result` is the producer's word, retained.
#[test]
fn tc_455_the_four_producer_results_stay_four_distinct_states() {
    let (_, sealed, _) = attestations().into_iter().next().expect("an attestation");
    let mut digests = std::collections::BTreeSet::new();
    for result in ProducerResult::ALL {
        let input = without(
            &replacing(&sealed, "result", JsonValue::string(result.as_str())),
            "digest",
        );
        let attestation = seal_attestation(&input)
            .unwrap_or_else(|error| panic!("`{}` must seal: {error}", result.as_str()));
        assert_eq!(
            attestation.result,
            result,
            "`{}` must read back as itself",
            result.as_str()
        );
        digests.insert(attestation.digest.as_hex().to_owned());
    }
    assert_eq!(
        digests.len(),
        4,
        "the four results must produce four distinct attestations"
    );
    assert!(
        seal_attestation(&without(
            &replacing(&sealed, "result", JsonValue::string("valid")),
            "digest"
        ))
        .is_err(),
        "a verification outcome is not a producer result and must be refused"
    );
}

/// Trace: FR-064-AC-7
///
/// A producer that could not run, or ran and computed nothing, still retains
/// its diagnostic output: the bytes are kept and the attestation over them
/// verifies. What is never created is evidence for a producer that reported
/// nothing at all — that is an absent attestation, not a synthetic one.
#[test]
fn tc_455_a_producer_that_reported_nothing_useful_still_retains_its_output() {
    let mut retained = 0_usize;
    for result in [ProducerResult::Unavailable, ProducerResult::NotComputed] {
        for (name, sealed, output) in attestations() {
            if output.is_empty() {
                continue;
            }
            let input = without(
                &replacing(&sealed, "result", JsonValue::string(result.as_str())),
                "digest",
            );
            let attestation = seal_attestation(&input)
                .unwrap_or_else(|error| panic!("{name}/{}: {error}", result.as_str()));
            assert_eq!(
                attestation.retained_output.size_bytes,
                u64::try_from(output.len()).unwrap(),
                "{name}: a diagnostic output must be retained whole"
            );
            retained += 1;
        }
    }
    assert!(
        retained >= 4,
        "anti-vacuity floor: at least 4 diagnostic outputs retained, saw {retained}"
    );
}

/// Trace: FR-064-AC-4
///
/// The attestation digest is the same pinned RFC 8785 / BLAKE3 contract the
/// record uses: sealing reproduces the oracle's digest and bytes, and changing
/// any member invalidates it.
#[test]
fn tc_455_the_attestation_digest_is_the_pinned_contract_and_every_change_breaks_it() {
    let attestations = attestations();
    for (name, sealed, _) in &attestations {
        let resealed = seal_attestation(&without(sealed, "digest"))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            resealed.digest.as_hex(),
            text(sealed, "digest"),
            "{name}: the sealed digest disagrees with the oracle's"
        );
        assert_eq!(
            attestation_bytes(&resealed).unwrap(),
            canonical_bytes(sealed).unwrap(),
            "{name}: the canonical bytes disagree with the oracle's"
        );
    }

    let (_, sealed, _) = attestations.into_iter().next().expect("an attestation");
    let mut mutated = 0_usize;
    for name in sealed
        .as_object()
        .unwrap()
        .names()
        .map(str::to_owned)
        .collect::<Vec<_>>()
    {
        if name == "digest" {
            continue;
        }
        let replacement = match sealed.as_object().unwrap().get(&name) {
            Some(JsonValue::String(text)) => JsonValue::string(format!("{text} ")),
            Some(JsonValue::Number(_)) => JsonValue::number(99.0).unwrap(),
            Some(JsonValue::Bool(flag)) => JsonValue::Bool(!flag),
            Some(JsonValue::Null) => JsonValue::string("a".repeat(64)),
            Some(JsonValue::Object(members)) => {
                let mut altered = members.clone();
                altered.set("undeclared", JsonValue::string("x"));
                JsonValue::Object(altered)
            }
            Some(JsonValue::Array(entries)) => {
                let mut altered = entries.clone();
                altered.push(JsonValue::string("x"));
                JsonValue::Array(altered)
            }
            None => unreachable!("the name came from this object"),
        };
        assert!(
            verify_attestation(&replacing(&sealed, &name, replacement)).is_err(),
            "changing `{name}` must invalidate the attestation"
        );
        mutated += 1;
    }
    assert!(
        mutated >= 10,
        "anti-vacuity floor: at least 10 members mutated, saw {mutated}"
    );
}

/// Trace: FR-064-AC-10
/// Provenance: PLAT-972
///
/// A proof attestation carries exactly one EA `ClaimStrength` (FR-022,
/// PLAT-971). Every declared wire name seals and reads back as itself.
#[test]
fn tc_455_every_claim_strength_round_trips() {
    let (_, sealed, _) = attestations().into_iter().next().expect("an attestation");
    let mut seen = std::collections::BTreeSet::new();
    for strength in ClaimStrength::ALL {
        let input = without(
            &replacing(&sealed, "strength", JsonValue::string(strength.wire_name())),
            "digest",
        );
        let attestation = seal_attestation(&input)
            .unwrap_or_else(|error| panic!("`{}` must seal: {error}", strength.wire_name()));
        assert_eq!(
            attestation.strength.wire_name(),
            strength.wire_name(),
            "`{}` must read back as itself",
            strength.wire_name()
        );
        // Round-trip through the sealed bytes too: writing it back out
        // produces the same wire spelling that was read in.
        let json = attestation.to_json().unwrap();
        assert_eq!(
            json.as_object()
                .unwrap()
                .get("strength")
                .and_then(JsonValue::as_str),
            Some(strength.wire_name())
        );
        seen.insert(strength.wire_name());
    }
    assert_eq!(
        seen.len(),
        4,
        "anti-vacuity floor: all four claim strengths must round-trip"
    );
}

/// Trace: FR-064-AC-10
/// Provenance: PLAT-972
///
/// An attestation with no `strength` is refused by name, not silently
/// defaulted to one of the four values.
#[test]
fn tc_455_an_attestation_with_no_strength_is_refused_not_defaulted() {
    let (_, sealed, _) = attestations().into_iter().next().expect("an attestation");
    let input = without(&without(&sealed, "digest"), "strength");
    match seal_attestation(&input) {
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
    let (_, sealed, _) = attestations().into_iter().next().expect("an attestation");
    let input = without(
        &replacing(&sealed, "strength", JsonValue::string("verified")),
        "digest",
    );
    match seal_attestation(&input) {
        Err(ChangeAssuranceError::Shape {
            failure: FieldFailure::Malformed { field },
            ..
        }) => assert_eq!(field, "strength"),
        other => panic!("expected a Malformed(\"strength\") refusal, got {other:?}"),
    }
}
