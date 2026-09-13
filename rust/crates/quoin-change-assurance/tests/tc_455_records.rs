// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Sealing, verifying and chaining change-assurance records (FR-063), against
//! records the retained TypeScript sealed.
//!
//! Every expected digest and every expected byte here was produced by
//! `sealChangeRecord` and written into `tests/fixtures/oracle.json`. Nothing in
//! this file computes an expectation with the code it is testing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use common::{member, oracle, section, text};
use quoin_change_assurance::model::json::object;
use quoin_change_assurance::model::record::{Completeness, Disposition};
use quoin_change_assurance::records::record_bytes;
use quoin_change_assurance::{seal_change_record, verify_change_record, verify_lineage};
use quoin_store::{JsonValue, canonical_bytes};

/// The captured records, as `(name, input, sealed)`.
fn records() -> Vec<(String, JsonValue, JsonValue)> {
    section(&oracle(), "records")
        .iter()
        .map(|captured| {
            (
                text(captured, "name").to_owned(),
                member(captured, "input").clone(),
                member(captured, "sealed").clone(),
            )
        })
        .collect()
}

/// The three-revision chain, in revision order.
fn chain() -> Vec<JsonValue> {
    records()
        .into_iter()
        .filter(|(name, _, _)| name.starts_with("chain-revision-"))
        .map(|(_, _, sealed)| sealed)
        .collect()
}

/// A copy of `value` with one top-level member replaced.
fn replacing(value: &JsonValue, name: &str, replacement: JsonValue) -> JsonValue {
    let mut members = value.as_object().unwrap().clone();
    members.set(name, replacement);
    JsonValue::Object(members)
}

/// A copy of `value` with one top-level member removed.
fn without(value: &JsonValue, name: &str) -> JsonValue {
    let mut members = value.as_object().unwrap().clone();
    members.remove(name);
    JsonValue::Object(members)
}

/// Trace: FR-063-AC-1
///
/// The schema is closed in both directions: every captured record verifies as
/// sealed, dropping any one of its members refuses it, and adding a member the
/// schema does not name refuses it too. The census walks whatever the record
/// actually carries rather than a list written here, so a member added to the
/// schema later is covered without this test being edited.
#[test]
fn tc_455_the_record_schema_names_every_member_and_admits_no_other() {
    let records = records();
    assert!(
        records.len() >= 4,
        "anti-vacuity floor: at least 4 captured records, saw {}",
        records.len()
    );
    let mut dropped = 0_usize;
    for (name, _, sealed) in &records {
        verify_change_record(sealed).unwrap_or_else(|error| panic!("{name}: {error}"));
        let members: Vec<String> = sealed
            .as_object()
            .unwrap()
            .names()
            .map(str::to_owned)
            .collect();
        for member_name in &members {
            assert!(
                verify_change_record(&without(sealed, member_name)).is_err(),
                "{name}: dropping `{member_name}` must refuse the record"
            );
            dropped += 1;
        }
        assert!(
            verify_change_record(&replacing(sealed, "undeclared", JsonValue::string("x"))).is_err(),
            "{name}: an undeclared member must refuse the record"
        );
    }
    assert!(
        dropped >= 40,
        "anti-vacuity floor: at least 40 member drops exercised, saw {dropped}"
    );
}

/// Trace: FR-063-AC-2
///
/// Requirements and proof obligations must be non-empty, while preservation
/// constraints and unknowns may be empty and stay present when they are: an
/// absent collection and an empty one are different claims, and only one of
/// them is "nothing to say here".
#[test]
fn tc_455_meaningful_collections_are_required_and_empty_ones_stay_present() {
    let sealed = chain().into_iter().next().expect("a captured record");
    let definition = member(&sealed, "definition").clone();
    let with_definition = |replacement: JsonValue| replacing(&sealed, "definition", replacement);

    for emptied in ["requirements", "proof_obligations"] {
        let replaced = replacing(&definition, emptied, JsonValue::Array(Vec::new()));
        assert!(
            seal_change_record(&with_definition(replaced)).is_err(),
            "an empty `{emptied}` collection must be refused"
        );
    }

    let record = verify_change_record(&sealed).expect("the captured record verifies");
    assert!(
        record.definition.preservation_constraints.is_empty()
            && record.definition.unknowns.is_empty(),
        "the captured record's empty collections must survive as empty, not as absent"
    );
    let round_tripped = record_bytes(&record).unwrap();
    assert_eq!(
        round_tripped,
        canonical_bytes(&sealed).unwrap(),
        "re-serializing must reproduce the oracle's bytes, empty collections included"
    );

    // A duplicate identity is refused before a digest is ever taken.
    let requirements = member(&definition, "requirements").clone();
    let JsonValue::Array(mut entries) = requirements else {
        panic!("requirements is not an array");
    };
    entries.push(entries[0].clone());
    let duplicated = replacing(&definition, "requirements", JsonValue::Array(entries));
    assert!(
        seal_change_record(&with_definition(duplicated)).is_err(),
        "a duplicated requirement identity must be refused"
    );
}

/// Trace: FR-063-AC-3
///
/// A proof obligation pins the exact evidence that discharges it, and the
/// typed reading of it is compared member for member against the retained
/// JSON: the literal argv, the repository-relative working directory, the
/// evidence kind, the tool identity and the configuration digest.
#[test]
fn tc_455_each_proof_obligation_pins_the_exact_evidence_that_discharges_it() {
    let mut checked = 0_usize;
    for (name, _, sealed) in records() {
        let record = verify_change_record(&sealed).expect("a captured record verifies");
        let JsonValue::Array(retained) =
            member(member(&sealed, "definition"), "proof_obligations").clone()
        else {
            panic!("{name}: proof_obligations is not an array");
        };
        assert_eq!(record.definition.proof_obligations.len(), retained.len());
        for (proof, raw) in record.definition.proof_obligations.iter().zip(&retained) {
            assert_eq!(proof.proof_id.as_str(), text(raw, "proof_id"));
            assert_eq!(proof.statement.as_str(), text(raw, "statement"));
            assert_eq!(
                proof.tool_identity.as_str(),
                text(raw, "tool_identity"),
                "{name}: tool identity"
            );
            assert_eq!(
                proof.configuration_digest.as_hex(),
                text(raw, "configuration_digest"),
                "{name}: configuration digest"
            );
            assert_eq!(
                proof.evidence_kind.as_str(),
                text(raw, "evidence_kind"),
                "{name}: evidence kind"
            );
            assert_eq!(
                canonical_bytes(&proof.command.to_json()).unwrap(),
                canonical_bytes(member(raw, "command")).unwrap(),
                "{name}: the command must be retained literally"
            );
            checked += 1;
        }
        let requirements = &record.definition.requirements;
        assert!(
            !requirements.is_empty(),
            "{name}: a record must retain its reviewed requirements"
        );
    }
    assert!(
        checked >= 4,
        "anti-vacuity floor: at least 4 proof obligations compared, saw {checked}"
    );
}

/// Trace: FR-063-AC-4
///
/// An analysis that did not see everything, and a review that did not settle
/// everything, stay visible as themselves. Each of the four dispositions
/// survives a seal-and-read round trip, and so do `incomplete` and
/// `truncated` — none of them collapses into a completeness claim.
#[test]
fn tc_455_admitted_gaps_and_open_unknowns_survive_sealing_as_themselves() {
    let sealed = chain().into_iter().next().expect("a captured record");
    let snapshot = replacing(
        &replacing(
            member(&sealed, "impact_snapshot"),
            "completeness",
            JsonValue::string("incomplete"),
        ),
        "truncated",
        JsonValue::Bool(true),
    );
    let gapped = replacing(&sealed, "impact_snapshot", snapshot);

    let mut unknowns: Vec<JsonValue> = Vec::new();
    for (index, disposition) in Disposition::ALL.iter().enumerate() {
        let mut members = vec![
            ("id", JsonValue::string(format!("unknown-{index}"))),
            ("statement", JsonValue::string("what about it")),
            ("disposition", JsonValue::string(disposition.as_str())),
            ("owner", JsonValue::string("reviewer-1")),
        ];
        // `resolution` is present exactly when the disposition is `resolved`:
        // an unresolved unknown carrying a null resolution would be a fifth
        // member the schema does not name.
        if *disposition == Disposition::Resolved {
            members.push(("resolution", JsonValue::string("answered")));
        }
        unknowns.push(object(members));
    }
    let definition = replacing(
        member(&gapped, "definition"),
        "unknowns",
        JsonValue::Array(unknowns),
    );
    let input = without(&replacing(&gapped, "definition", definition), "digest");

    let record = seal_change_record(&input).expect("the gapped record seals");
    assert_eq!(
        record.impact_snapshot.completeness,
        Completeness::Incomplete
    );
    assert!(record.impact_snapshot.truncated);
    let seen: Vec<Disposition> = record
        .definition
        .unknowns
        .iter()
        .map(|unknown| unknown.disposition)
        .collect();
    for disposition in Disposition::ALL {
        assert!(
            seen.contains(&disposition),
            "`{}` must survive sealing",
            disposition.as_str()
        );
    }
}

/// Trace: FR-063-AC-5
///
/// Sealing produces the oracle's bytes and the oracle's digest. This is the
/// pinned RFC 8785 / BLAKE3 contract: the digest compared against is a hex
/// string `sealChangeRecord` wrote, and the bytes are the canonical bytes of
/// the record it sealed — including one input whose collections arrived
/// unsorted, so the normalization is exercised rather than assumed.
#[test]
fn tc_455_sealing_reproduces_the_oracles_bytes_and_digest() {
    let records = records();
    let mut normalized = 0_usize;
    for (name, input, sealed) in &records {
        let record = seal_change_record(input).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            record.digest.as_hex(),
            text(sealed, "digest"),
            "{name}: the sealed digest disagrees with the oracle's"
        );
        assert_eq!(
            record_bytes(&record).unwrap(),
            canonical_bytes(sealed).unwrap(),
            "{name}: the canonical bytes disagree with the oracle's"
        );
        if canonical_bytes(input).unwrap() != canonical_bytes(&without(sealed, "digest")).unwrap() {
            normalized += 1;
        }
    }
    assert!(
        normalized >= 1,
        "anti-vacuity floor: at least one input must differ from its sealed form, \
         or normalization is untested"
    );
}

/// Trace: FR-063-AC-6
///
/// Changing any semantic leaf invalidates the stored digest. The census walks
/// the record's own members rather than a hand-written list, and the digest
/// member itself is skipped because changing it is the trivial case.
#[test]
fn tc_455_changing_any_semantic_leaf_invalidates_the_stored_digest() {
    let sealed = chain().into_iter().next().expect("a captured record");
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
            verify_change_record(&replacing(&sealed, &name, replacement)).is_err(),
            "changing `{name}` must invalidate the record"
        );
        mutated += 1;
    }
    assert!(
        mutated >= 10,
        "anti-vacuity floor: at least 10 leaves mutated, saw {mutated}"
    );
}

/// Trace: FR-063-AC-7
///
/// The refusals that keep one record from having two spellings: a digest in
/// uppercase hex, a truncated digest, a digest carrying a prefix, a revision
/// that is not an I-JSON integer, and a scope that is not in UTF-16 order. Each
/// is refused on its own.
#[test]
fn tc_455_ambiguous_spellings_of_one_record_are_refused() {
    let sealed = chain().into_iter().next().expect("a captured record");
    let unsealed = without(&sealed, "digest");
    let digest = text(&sealed, "digest").to_owned();

    for (why, spelling) in [
        ("uppercase hex", digest.to_uppercase()),
        ("a truncated digest", digest[..63].to_owned()),
        ("a prefixed digest", format!("blake3:{digest}")),
    ] {
        assert!(
            verify_change_record(&replacing(&sealed, "digest", JsonValue::string(spelling)))
                .is_err(),
            "{why} must be refused"
        );
    }

    for (why, revision) in [
        ("a fractional revision", 1.5_f64),
        ("a zero revision", 0.0),
        ("a negative revision", -1.0),
    ] {
        assert!(
            seal_change_record(&replacing(
                &unsealed,
                "revision",
                JsonValue::number(revision).unwrap()
            ))
            .is_err(),
            "{why} must be refused"
        );
    }

    // "docs/\u{10000}" sorts after "src/evidence" by scalar value and before
    // it by UTF-16 code unit. A record that arrives in scalar order is not in
    // the order this family defines, and `verifyChangeRecord` refuses it.
    let scope = JsonValue::Array(vec![
        JsonValue::string("docs/é"),
        JsonValue::string("src/evidence"),
        JsonValue::string("docs/\u{10000}"),
    ]);
    let subject = replacing(member(&sealed, "subject"), "scope", scope);
    assert!(
        verify_change_record(&replacing(&sealed, "subject", subject)).is_err(),
        "a scope that is not in UTF-16 order must be refused"
    );
}

/// Trace: FR-063-AC-8
///
/// The strict N-1 chain. The captured three-revision chain verifies, and each
/// way of breaking it — a genesis with a parent, a missing link, a skipped
/// revision, a cross-record parent — is refused on its own.
#[test]
fn tc_455_lineage_is_strict_and_every_break_is_refused() {
    let chain = chain();
    assert_eq!(chain.len(), 3, "the capture must hold a three-link chain");
    let read = |value: &JsonValue| verify_change_record(value).expect("a captured record verifies");

    let third = read(&chain[2]);
    let digests = verify_lineage(&third, &chain[..2]).expect("the captured chain verifies");
    assert_eq!(
        digests.len(),
        2,
        "revision 3 must cite exactly its two parents"
    );

    let first = read(&chain[0]);
    assert!(
        verify_lineage(&first, &[])
            .expect("a genesis record has no parents")
            .is_empty(),
        "revision 1 must verify with no parents"
    );
    assert!(
        verify_lineage(&first, &chain[..1]).is_err(),
        "a genesis record with a parent must be refused"
    );
    assert!(
        verify_lineage(&third, &chain[..1]).is_err(),
        "a chain missing a link must be refused"
    );
    assert!(
        verify_lineage(&third, &[chain[0].clone(), chain[0].clone()]).is_err(),
        "a chain that repeats a revision must be refused"
    );

    let foreign = seal_change_record(&without(
        &replacing(&chain[0], "record_id", JsonValue::string("change-2")),
        "digest",
    ))
    .expect("the foreign record seals");
    let second = read(&chain[1]);
    assert!(
        verify_lineage(&second, std::slice::from_ref(&foreign.to_json().unwrap())).is_err(),
        "a parent from another record must be refused"
    );
}
