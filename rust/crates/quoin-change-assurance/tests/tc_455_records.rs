// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The change-assurance record: what the schema requires, what survives
//! reading and writing, and the strict lineage (FR-063).
//!
//! A record is addressed by `(record_id, revision)` and carries no digest, so
//! these cases are about the record's own content and its chain.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use quoin_change_assurance::model::record::{Completeness, Disposition};
use quoin_change_assurance::records::{record_bytes, verify_lineage};
use quoin_change_assurance::{prepare_change_record, read_change_record_json};
use quoin_store::{JsonValue, canonical_bytes};

use crate::common::{json, member, record_text, replacing};

/// The scenario record at `revision`.
fn record_at(revision: u32) -> JsonValue {
    replacing(
        &json(&record_text()),
        "revision",
        JsonValue::number(f64::from(revision)).unwrap(),
    )
}

/// The scenario record under another identity.
fn foreign(revision: u32) -> JsonValue {
    replacing(
        &record_at(revision),
        "record_id",
        JsonValue::string("change-2"),
    )
}

fn object(members: Vec<(&str, JsonValue)>) -> JsonValue {
    let mut fields = quoin_store::JsonObject::new();
    for (name, value) in members {
        fields.set(name, value);
    }
    JsonValue::Object(fields)
}

/// Trace: FR-063-AC-2
///
/// Requirements and proof obligations must be non-empty, while preservation
/// constraints and unknowns may be empty and stay present when they are: an
/// absent collection and an empty one are different claims. A duplicated
/// identity is refused before the record is ever written.
#[test]
fn tc_455_meaningful_collections_are_required_and_empty_ones_stay_present() {
    let record = json(&record_text());
    let definition = member(&record, "definition").clone();
    let with_definition = |replacement: JsonValue| replacing(&record, "definition", replacement);

    for emptied in ["requirements", "proof_obligations"] {
        let replaced = replacing(&definition, emptied, JsonValue::Array(Vec::new()));
        assert!(
            prepare_change_record(&with_definition(replaced)).is_err(),
            "an empty `{emptied}` collection must be refused"
        );
    }

    let typed = prepare_change_record(&record).expect("the scenario record prepares");
    assert!(
        typed.definition.preservation_constraints.is_empty()
            && typed.definition.unknowns.is_empty(),
        "the scenario's empty collections must survive as empty, not as absent"
    );
    let written = record_bytes(&typed).unwrap();
    let reread = read_change_record_json(&quoin_store::parse_strict_json(&written).unwrap())
        .expect("the written bytes read back");
    assert_eq!(reread, typed);
    let written_json = quoin_store::parse_strict_json(&written).unwrap();
    for kept in ["preservation_constraints", "unknowns"] {
        assert!(
            matches!(
                member(member(&written_json, "definition"), kept),
                JsonValue::Array(items) if items.is_empty()
            ),
            "`{kept}` must be written as an empty array"
        );
    }

    let JsonValue::Array(mut entries) = member(&definition, "requirements").clone() else {
        panic!("requirements is not an array");
    };
    entries.push(entries[0].clone());
    let duplicated = replacing(&definition, "requirements", JsonValue::Array(entries));
    assert!(
        prepare_change_record(&with_definition(duplicated)).is_err(),
        "a duplicated requirement identity must be refused"
    );
}

/// Trace: FR-063-AC-3
///
/// A proof obligation fixes the exact evidence that discharges it, and the
/// typed reading is compared member for member against the JSON that was
/// given: the literal argv, the repository-relative working directory, the
/// evidence kind, the tool identity and the configuration reference. Every
/// requirement retains its reviewed statement.
#[test]
fn tc_455_each_proof_obligation_fixes_the_exact_evidence_that_discharges_it() {
    let record = json(&record_text());
    let typed = prepare_change_record(&record).unwrap();
    let JsonValue::Array(given) =
        member(member(&record, "definition"), "proof_obligations").clone()
    else {
        panic!("proof_obligations is not an array");
    };
    assert_eq!(typed.definition.proof_obligations.len(), given.len());
    assert!(
        !given.is_empty(),
        "anti-vacuity floor: one obligation compared"
    );
    for (proof, raw) in typed.definition.proof_obligations.iter().zip(&given) {
        let text = |name: &str| member(raw, name).as_str().unwrap().to_owned();
        assert_eq!(proof.proof_id.as_str(), text("proof_id"));
        assert_eq!(proof.statement.as_str(), text("statement"));
        assert_eq!(proof.tool_identity.as_str(), text("tool_identity"));
        assert_eq!(proof.configuration_ref.as_str(), text("configuration_ref"));
        assert_eq!(proof.evidence_kind.as_str(), text("evidence_kind"));
        assert_eq!(
            canonical_bytes(&proof.command.to_json()).unwrap(),
            canonical_bytes(member(raw, "command")).unwrap(),
            "the command must be retained literally"
        );
    }
    let requirement = &typed.definition.requirements[0];
    assert_eq!(requirement.statement.as_str(), "record it");

    // Each fixed member is required: an obligation missing one is refused.
    let definition = member(&record, "definition").clone();
    for dropped in [
        "tool_identity",
        "configuration_ref",
        "command",
        "evidence_kind",
    ] {
        let mut obligation = given[0].as_object().unwrap().clone();
        obligation.remove(dropped);
        let changed = replacing(
            &definition,
            "proof_obligations",
            JsonValue::Array(vec![JsonValue::Object(obligation)]),
        );
        assert!(
            prepare_change_record(&replacing(&record, "definition", changed)).is_err(),
            "an obligation without `{dropped}` must be refused"
        );
    }
}

/// Trace: FR-063-AC-4
///
/// An analysis that did not see everything, and a review that did not settle
/// everything, stay visible as themselves. Each of the four dispositions
/// survives a write-and-read round trip, and so do `incomplete` and
/// `truncated`: none of them collapses into a completeness claim.
#[test]
fn tc_455_admitted_gaps_and_open_unknowns_survive_writing_as_themselves() {
    let record = json(&record_text());
    let snapshot = replacing(
        &replacing(
            member(&record, "impact_snapshot"),
            "completeness",
            JsonValue::string("incomplete"),
        ),
        "truncated",
        JsonValue::Bool(true),
    );
    let gapped = replacing(&record, "impact_snapshot", snapshot);

    let mut unknowns: Vec<JsonValue> = Vec::new();
    for (index, disposition) in Disposition::ALL.iter().enumerate() {
        let mut members = vec![
            ("id", JsonValue::string(format!("unknown-{index}"))),
            ("statement", JsonValue::string("what about it")),
            ("disposition", JsonValue::string(disposition.as_str())),
            ("owner", JsonValue::string("reviewer-1")),
        ];
        // `resolution` is present exactly when the disposition is `resolved`.
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
    let input = replacing(&gapped, "definition", definition);

    let typed = prepare_change_record(&input).expect("the gapped record prepares");
    let written = record_bytes(&typed).unwrap();
    let record = read_change_record_json(&quoin_store::parse_strict_json(&written).unwrap())
        .expect("the written record reads back");
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
            "`{}` must survive writing",
            disposition.as_str()
        );
    }
}

/// Trace: FR-063-AC-8
///
/// The strict N-1 chain. A three-revision chain verifies, and each way of
/// breaking it - a genesis with a parent, a missing link, a repeated
/// revision, a cross-record parent - is refused on its own.
#[test]
fn tc_455_lineage_is_strict_and_every_break_is_refused() {
    let chain: Vec<JsonValue> = (1..=3).map(record_at).collect();
    let read = |value: &JsonValue| prepare_change_record(value).expect("a chain record prepares");

    let third = read(&chain[2]);
    verify_lineage(&third, &chain[..2]).expect("the chain verifies");

    let first = read(&chain[0]);
    verify_lineage(&first, &[]).expect("a genesis record has no parents");
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
    assert!(
        verify_lineage(&third, &[chain[1].clone(), chain[2].clone()]).is_err(),
        "a chain that skips the first revision must be refused"
    );

    let second = read(&chain[1]);
    assert!(
        verify_lineage(&second, &[foreign(1)]).is_err(),
        "a parent from another record must be refused"
    );
    assert!(
        verify_lineage(&second, &[json(r#"{"revision": 1}"#)]).is_err(),
        "a parent that is not a valid record must be refused"
    );
}
