// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The adversarial canonicalization suite, asserted against the TypeScript
//! oracle's captured answers.
//!
//! The cases are authored in `oracle/cases.mjs`, each with a statement of what
//! it probes, and the oracle's answers are captured once into
//! `tests/fixtures/jcs-oracle.json` by `oracle/capture-cases.mjs`. No
//! TypeScript runs here: the fixture *is* the oracle, as EPIC #373 AC-5
//! requires.
//!
//! What is asserted, and what is not:
//!
//! * **Accept/refuse must agree exactly.** A document one implementation reads
//!   and the other refuses is the worst failure mode available — it is a store
//!   file that exists for one reader and not the other.
//! * **Canonical bytes must agree exactly**, in both serializations.
//! * **Refusal *messages* are not asserted.** They are diagnostics, and the
//!   Rust port reports typed variants with stable codes rather than the
//!   oracle's prose. The refusal itself is the contract.

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

const FIXTURE: &str = include_str!("fixtures/jcs-oracle.json");

struct Case {
    id: String,
    probes: String,
    expect: String,
    input: String,
    accepted: bool,
    jcs: Option<String>,
    pretty: Option<String>,
    /// The serialization this case is a known, measured divergence in, with the
    /// reason. `None` means the two implementations must agree byte for byte.
    divergence: Option<(String, String)>,
}

fn member(object: &quoin_store::JsonObject, name: &str) -> Option<String> {
    object
        .get(name)
        .and_then(JsonValue::as_str)
        .map(str::to_owned)
}

fn load_cases() -> Vec<Case> {
    let value = parse_strict_json_str(FIXTURE).expect("the captured fixture must be strict JSON");
    let root = value.as_object().expect("fixture root is an object");
    let JsonValue::Array(entries) = root.get("cases").expect("fixture carries cases") else {
        panic!("fixture cases must be an array");
    };
    entries
        .iter()
        .map(|entry| {
            let object = entry.as_object().expect("each case is an object");
            Case {
                id: member(object, "id").expect("case id"),
                probes: member(object, "probes").expect("case probes"),
                expect: member(object, "expect").expect("case expect"),
                input: member(object, "input").expect("case input"),
                accepted: matches!(object.get("accepted"), Some(JsonValue::Bool(true))),
                jcs: member(object, "jcs"),
                pretty: member(object, "pretty"),
                divergence: object.get("divergence").and_then(|value| {
                    let declared = value.as_object().ok()?;
                    Some((member(declared, "form")?, member(declared, "summary")?))
                }),
            }
        })
        .collect()
}

/// Every case carries a statement of what it probes, and the authored intent
/// agrees with what the oracle actually did.
///
/// A case whose `probes` field is empty is a case nobody can maintain: the next
/// reader cannot tell whether a failure is a regression or a corrected
/// expectation.
///
#[test]
fn tc_380_every_case_states_what_it_probes_and_matches_authored_intent() {
    let cases = load_cases();
    assert!(cases.len() >= 50, "the suite must not shrink silently");
    for case in &cases {
        assert!(
            case.probes.len() > 40,
            "{}: probes must say what the case is for",
            case.id
        );
        let actual = if case.accepted { "accept" } else { "refuse" };
        assert_eq!(
            actual, case.expect,
            "{}: authored intent disagrees with the oracle",
            case.id
        );
    }
}

/// Rust and TypeScript accept and refuse exactly the same documents.
///
/// Trace: FR-098-AC-9
#[test]
fn tc_380_acceptance_agrees_with_the_oracle_on_every_case() {
    let mut refused = 0_usize;
    for case in load_cases() {
        let parsed = parse_strict_json_str(&case.input);
        assert_eq!(
            parsed.is_ok(),
            case.accepted,
            "{}: oracle accepted={}, rust accepted={} ({:?})",
            case.id,
            case.accepted,
            parsed.is_ok(),
            parsed.err().map(|error| error.code())
        );
        if !case.accepted {
            refused += 1;
        }
    }
    assert!(
        refused >= 25,
        "the refusal half of the suite must not erode"
    );
}

/// Both canonical serializations are byte-identical to the oracle's, except
/// where the case declares a measured divergence.
///
/// Trace: FR-098-AC-4
#[test]
fn tc_380_both_canonical_serializations_are_byte_identical_to_the_oracle() {
    for case in load_cases() {
        let Ok(value) = parse_strict_json_str(&case.input) else {
            continue;
        };
        let (Some(jcs), Some(pretty)) = (case.jcs, case.pretty) else {
            panic!(
                "{}: an accepted case must carry both serializations",
                case.id
            );
        };
        let diverges_in = case.divergence.as_ref().map(|(form, _)| form.as_str());
        if diverges_in == Some("jcs") {
            assert_ne!(
                canonicalize_jcs(&value).expect("depth within budget"),
                jcs,
                "{}: declared JCS divergence has gone away — re-examine the declaration, do not delete it",
                case.id
            );
        } else {
            assert_eq!(
                canonicalize_jcs(&value).expect("depth within budget"),
                jcs,
                "{}: JCS text",
                case.id
            );
        }
        if diverges_in == Some("pretty") {
            assert_ne!(
                canonical_json(&value).expect("depth within budget"),
                pretty,
                "{}: declared pretty-form divergence has gone away — re-examine the declaration, do not delete it",
                case.id
            );
        } else {
            assert_eq!(
                canonical_json(&value).expect("depth within budget"),
                pretty,
                "{}: pretty text",
                case.id
            );
        }
    }
}

/// The `__proto__` divergence, stated exactly.
///
/// The TypeScript evidence-store writer deletes every `__proto__` member: its
/// `sortKeys` helper rebuilds each object as a plain `{}` and assigns members
/// into it, so `out["__proto__"] = v` reaches the `Object.prototype.__proto__`
/// setter rather than creating an own property. `JSON.parse` *keeps*
/// `__proto__` as an own property, so a store file containing one loses it on
/// any rewrite, with no diagnostic. `canonicalizeJcs` is unaffected — it never
/// assigns into an object — so the JCS bytes are unaffected.
///
/// The Rust port keeps the member. Reproducing a prototype-pollution artefact
/// in a language with no prototypes would be encoding a defect as a contract.
///
#[test]
fn tc_380_proto_member_is_data_in_rust_and_deleted_by_the_typescript_pretty_form() {
    let cases = load_cases();
    let declared: Vec<&Case> = cases
        .iter()
        .filter(|case| case.divergence.is_some())
        .collect();
    assert_eq!(
        declared.len(),
        2,
        "the declared divergence set changed; report it, do not adjust the count"
    );

    let value = parse_strict_json_str(r#"{"__proto__":1,"a":2}"#).expect("parses");
    // Rust keeps the member in both forms.
    assert_eq!(
        canonicalize_jcs(&value).expect("depth within budget"),
        r#"{"__proto__":1,"a":2}"#
    );
    assert_eq!(
        canonical_json(&value).expect("depth within budget"),
        "{\n  \"__proto__\": 1,\n  \"a\": 2\n}\n"
    );
    // And the oracle's captured pretty form for the suite's case drops it.
    let captured = cases
        .iter()
        .find(|case| case.id == "key-prototype-names-are-data")
        .expect("case present");
    let pretty = captured.pretty.clone().expect("captured");
    assert!(
        !pretty.contains("__proto__"),
        "the oracle's pretty form was expected to have dropped __proto__"
    );
    assert!(
        captured
            .jcs
            .clone()
            .expect("captured")
            .contains("__proto__"),
        "the oracle's JCS form keeps __proto__; only the pretty form loses it"
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
