// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Adversarial strict-parse and canonicalization cases, each asserted against
//! an expectation written in this file. Nothing is read from a recorded capture.
//!
//! * Accept/refuse is decided by the strict parser alone.
//! * Canonical bytes are independent of source member order, in both
//!   serializations.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_store::json::parse::parse_strict_json_str;
use quoin_store::json::value::JsonValue;
use quoin_store::{canonical_json, canonicalize_jcs};

/// Strict parsing accepts and refuses by the authored rule.
///
/// Trace: FR-098-AC-9
#[test]
fn tc_380_strict_parse_accepts_and_refuses_the_authored_documents() {
    let refused: [(&str, &str); 5] = [
        ("duplicate member names", r#"{"a":1,"a":2}"#),
        ("leading byte-order mark", "\u{feff}{\"a\":1}"),
        ("unpaired surrogate escape", r#"{"a":"\ud800"}"#),
        ("unescaped control character", "{\"a\":\"x\ty\"}"),
        ("number overflowing to infinity", r#"{"n":1e999}"#),
    ];
    for (why, document) in refused {
        assert!(
            parse_strict_json_str(document).is_err(),
            "{why} must be refused"
        );
    }
    for document in [r#"{"n":1}"#, r#"{"n":-0}"#, r#"{"a":[1,2,{"b":null}]}"#] {
        assert!(
            parse_strict_json_str(document).is_ok(),
            "{document} must be accepted"
        );
    }
}

/// A `__proto__` member is data.
///
/// The Rust port keeps the member. Reproducing a prototype-pollution artefact
/// in a language with no prototypes would be encoding a defect as a contract.
///
#[test]
fn tc_380_proto_member_is_kept_as_data() {
    let value = parse_strict_json_str(r#"{"__proto__":1,"a":2}"#).expect("parses");
    assert_eq!(
        canonicalize_jcs(&value).expect("depth within budget"),
        r#"{"__proto__":1,"a":2}"#
    );
    assert_eq!(
        canonical_json(&value).expect("depth within budget"),
        "{\n  \"__proto__\": 1,\n  \"a\": 2\n}\n"
    );
}

/// Canonicalization makes the bytes independent of source member order.
///
/// Trace: FR-098-CON-1
#[test]
fn tc_380_member_order_in_the_source_does_not_change_the_canonical_bytes() {
    let forward = parse_strict_json_str(r#"{"a":1,"b":2,"c":3}"#).expect("parses");
    let reversed = parse_strict_json_str(r#"{"c":3,"b":2,"a":1}"#).expect("parses");
    assert_eq!(
        canonicalize_jcs(&forward).expect("depth within budget"),
        canonicalize_jcs(&reversed).expect("depth within budget")
    );
}

/// Negative zero and zero are different documents with one canonical form.
///
/// Recorded as a known property, not as a defect: it follows from the frozen
/// ECMAScript number model and is already true of retained evidence.
///
/// Trace: FR-098-CON-1
#[test]
fn tc_380_negative_zero_and_zero_share_one_canonical_form() {
    let negative = parse_strict_json_str(r#"{"n":-0}"#).expect("parses");
    let positive = parse_strict_json_str(r#"{"n":0}"#).expect("parses");
    // The two values are genuinely different doubles: the sign bit differs.
    assert_ne!(member_bits(&negative), member_bits(&positive));
    assert_eq!(member_bits(&negative) ^ member_bits(&positive), 1 << 63);
    assert_eq!(
        canonicalize_jcs(&negative).expect("depth within budget"),
        canonicalize_jcs(&positive).expect("depth within budget")
    );
}

/// The raw IEEE-754 bits of the `n` member.
fn member_bits(value: &JsonValue) -> u64 {
    value
        .as_object()
        .expect("object")
        .get("n")
        .and_then(JsonValue::as_f64)
        .expect("number")
        .to_bits()
}

/// The two serializations order the same members differently, and that
/// difference is deliberate.
///
#[test]
fn tc_380_jcs_and_pretty_orders_are_not_the_same_order() {
    let value = parse_strict_json_str(r#"{"10":1,"2":2,"a":3,"1":4}"#).expect("parses");
    assert_eq!(
        canonicalize_jcs(&value).expect("depth within budget"),
        r#"{"1":4,"10":1,"2":2,"a":3}"#
    );
    assert_eq!(
        canonical_json(&value).expect("depth within budget"),
        "{\n  \"1\": 4,\n  \"2\": 2,\n  \"10\": 1,\n  \"a\": 3\n}\n"
    );
}
