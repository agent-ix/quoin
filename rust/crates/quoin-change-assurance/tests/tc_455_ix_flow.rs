// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The ix-flow decision event's hash, against the oracle that produced it.
//!
//! # The third serializer, measured
//!
//! `hashIxFlowEvent` (`records.ts:467`) feeds its own JSON serializer,
//! `canonicalIxFlowJson` — the third in the retained TypeScript. This crate
//! does not have a third one: it hashes `quoin-store`'s JCS bytes. Whether
//! that is the same hash is a question of fact, and these tests are where the
//! fact is checked, over events chosen to separate the two serializers if
//! anything can: an astral character, every escape JSON defines, `-0`, `1e21`,
//! `1e-7`, an empty object and array, and keys that cross the surrogate
//! boundary where UTF-16 order and scalar order disagree.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use common::{bytes, member, oracle, section, text};
use quoin_change_assurance::model::decision::{DecisionHistory, RetainedDecisionEvent};
use quoin_change_assurance::model::json::object;
use quoin_store::{JsonValue, canonical_bytes};

/// Every captured event, with its `hash` member put back.
fn events() -> Vec<(String, JsonValue, String, Vec<u8>)> {
    section(&oracle(), "ix_flow")
        .iter()
        .map(|captured| {
            let hash = text(captured, "hash").to_owned();
            let mut sealed = member(captured, "event").as_object().unwrap().clone();
            sealed.set("hash", JsonValue::string(&hash));
            let id = text(member(captured, "event"), "id").to_owned();
            (
                id,
                JsonValue::Object(sealed),
                hash,
                bytes(member(captured, "jcs_bytes")),
            )
        })
        .collect()
}

/// Trace: FR-063-AC-10
///
/// The hash this crate computes for a retained event is the hash the oracle
/// computed for it. Nothing here recomputes the expectation: the hex string
/// compared against was written down by `hashIxFlowEvent` itself.
#[test]
fn tc_455_every_captured_event_hashes_to_what_the_oracle_hashed() {
    let events = events();
    assert!(
        events.len() >= 6,
        "anti-vacuity floor: at least 6 captured events, saw {}",
        events.len()
    );
    for (id, sealed, expected, _) in &events {
        let event = RetainedDecisionEvent::from_json(sealed).expect("a captured event reads");
        assert_eq!(
            event.recompute_hash().unwrap().as_hex(),
            expected,
            "{id}: the recomputed hash disagrees with the oracle's"
        );
    }
}

/// Trace: FR-100-CON-4
///
/// The measurement itself: the bytes this crate hashes are byte-for-byte the
/// JCS bytes the oracle produced beside the hash. Since the hash over them
/// agrees too, `canonicalIxFlowJson` and JCS cannot be distinguished on
/// anything a strict parse admits, and the port is entitled to keep one
/// serializer instead of porting a second.
#[test]
fn tc_455_the_ix_flow_serializer_is_jcs_byte_for_byte() {
    let events = events();
    let mut compared = 0_usize;
    for (id, sealed, _, expected_jcs) in &events {
        let mut unsigned = sealed.as_object().unwrap().clone();
        unsigned.remove("hash");
        assert_eq!(
            &canonical_bytes(&JsonValue::Object(unsigned)).unwrap(),
            expected_jcs,
            "{id}: this crate's canonical bytes are not the oracle's JCS bytes"
        );
        compared += 1;
    }
    assert!(
        compared >= 6,
        "anti-vacuity floor: at least 6 events compared, saw {compared}"
    );
}

/// Trace: FR-063-AC-10
///
/// A chain of captured events links, and one altered byte anywhere in the
/// chain breaks it. The negative half is what makes the positive half mean
/// something: a `chains()` that returned `true` unconditionally would pass the
/// test above and fail this one.
#[test]
fn tc_455_a_decision_chain_links_and_one_altered_member_breaks_it() {
    let (_, sealed, hash, _) = events().into_iter().next().expect("a captured event");
    let history = |events: Vec<JsonValue>| {
        DecisionHistory::from_json(&object(vec![
            ("run_id", JsonValue::string("run-1")),
            ("events", JsonValue::Array(events)),
        ]))
        .expect("the history shape reads")
    };
    assert!(
        history(vec![sealed.clone()]).chains(),
        "a captured event chains from the genesis hash"
    );

    let mut altered = sealed.as_object().unwrap().clone();
    altered.set(
        "kind",
        JsonValue::string("change_assurance.review_decided "),
    );
    assert!(
        !history(vec![JsonValue::Object(altered)]).chains(),
        "a trailing space in `kind` must break the chain"
    );

    let mut relinked = sealed.as_object().unwrap().clone();
    relinked.set("prevHash", JsonValue::string(hash));
    assert!(
        !history(vec![JsonValue::Object(relinked)]).chains(),
        "an event that does not link to the genesis hash must break the chain"
    );
}
