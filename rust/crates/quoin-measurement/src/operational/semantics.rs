// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What the schema cannot say about an operational record.
//!
//! Ports `operational.ts:320-364` (`semanticFindings`) and its scalar helpers.
//!
//! # Why this reads JSON and not the typed record
//!
//! Three of these checks are about members the typed record cannot carry.
//! `clock_support` with `supported: false` is refused when it has **any other
//! member**, and a `not_applicable` clock is refused when it has more than two
//! — neither is a statement about a value, both are statements about the
//! object's shape, and a shape that deserialises into
//! [`crate::operational::record::OperationalEvidenceRecord`] has already
//! dropped the members they are about. So the pass runs before the record is
//! typed, on the document, which is what `operational.ts` does too.
//!
//! The pins check is the opposite case and shows the boundary: it asks whether
//! a `*_pin` control kind has a matching pin, and the set of pin kinds is
//! [`VersionPinKind`]'s, read through `from_code` rather than respelled as a
//! second literal set.

use serde_json::{Map, Value};

use crate::operational::clock::{check_clock, instant_at};
use crate::operational::record::VersionPinKind;

/// Every semantic finding against one candidate record.
///
/// The candidate is whatever the caller was handed: a value that is not an
/// object yields no findings here, because the schema pass has already refused
/// it and a second refusal would only duplicate the finding.
pub(super) fn semantic_findings(value: &Value, findings: &mut Vec<String>) {
    let Some(record) = value.as_object() else {
        return;
    };
    let observed = instant_at(record.get("observed_at"), "/observed_at", findings);
    let capability = record.get("capability").and_then(Value::as_object);
    let exercise = record.get("exercise").and_then(Value::as_object);
    match record.get("record_shape").and_then(Value::as_str) {
        Some("standing_capability") => {
            if capability.is_none() || exercise.is_some() {
                findings.push("/: standing_capability requires only capability payload".to_owned());
            }
            if let Some(support) = capability
                .and_then(|payload| payload.get("clock_support"))
                .and_then(Value::as_object)
            {
                check_clock_support(support, findings);
            }
        }
        Some("exercise") => {
            if exercise.is_none() || capability.is_some() {
                findings.push("/: exercise requires only exercise payload".to_owned());
            }
            check_exercise(exercise, observed, findings);
        }
        _ => {}
    }
    check_pins(record, findings);
}

/// `operational.ts:331-347`.
fn check_clock_support(support: &Map<String, Value>, findings: &mut Vec<String>) {
    match support.get("supported") {
        Some(Value::Bool(true)) => {
            let bounded = truthy(support.get("start_event"))
                && truthy(support.get("completion_event"))
                && positive_integer(support.get("deadline_seconds"));
            if !bounded {
                findings.push(
                    "/capability/clock_support: supported clock requires events and positive \
                     deadline"
                        .to_owned(),
                );
            }
        }
        Some(Value::Bool(false)) if support.len() != 1 => {
            findings.push(
                "/capability/clock_support: unsupported clock excludes event/deadline fields"
                    .to_owned(),
            );
        }
        _ => {}
    }
}

/// `operational.ts:349-362`.
fn check_exercise(
    exercise: Option<&Map<String, Value>>,
    observed: Option<i64>,
    findings: &mut Vec<String>,
) {
    let started = instant_at(
        exercise.and_then(|payload| payload.get("started_at")),
        "/exercise/started_at",
        findings,
    );
    let completed = instant_at(
        exercise.and_then(|payload| payload.get("completed_at")),
        "/exercise/completed_at",
        findings,
    );
    if let (Some(started), Some(completed)) = (started, completed)
        && completed < started
    {
        findings.push("/exercise/completed_at: precedes exercise start".to_owned());
    }
    if let (Some(observed), Some(completed)) = (observed, completed)
        && observed < completed
    {
        findings.push("/observed_at: precedes exercise completion".to_owned());
    }
    if let Some(payload) = exercise {
        check_clock(payload, observed, findings);
    }
}

/// `operational.ts:364-384`: no two pins pin the same thing, and a `*_pin`
/// control kind has the pin it is named after.
fn check_pins(record: &Map<String, Value>, findings: &mut Vec<String>) {
    let pins: Vec<&Map<String, Value>> = record
        .get("configuration")
        .and_then(Value::as_object)
        .and_then(|configuration| configuration.get("version_pins"))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_object).collect())
        .unwrap_or_default();
    let mut seen: Vec<(String, String)> = Vec::new();
    for (index, pin) in pins.iter().enumerate() {
        let key = (rendered(pin.get("kind")), rendered(pin.get("identity")));
        if seen.contains(&key) {
            findings.push(format!(
                "/configuration/version_pins/{index}: duplicate kind/identity"
            ));
        }
        seen.push(key);
    }
    let Some(control_kind) = record.get("control_kind").and_then(Value::as_str) else {
        return;
    };
    let Some(expected) = control_kind.strip_suffix("_pin") else {
        return;
    };
    let matched = VersionPinKind::from_code(expected).is_some()
        && pins
            .iter()
            .any(|pin| pin.get("kind").and_then(Value::as_str) == Some(expected));
    if !matched {
        findings.push(format!(
            "/configuration/version_pins: {control_kind} requires a matching pin kind"
        ));
    }
}

/// ECMAScript truthiness, for the three members `operational.ts:336` tests with
/// a bare `!`.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null | Value::Bool(false)) => false,
        Some(Value::Number(number)) => number.as_f64().is_some_and(|n| n != 0.0),
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
    }
}

/// `Number.isInteger(value) && value > 0`. `operational.ts:491-493`.
fn positive_integer(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_f64)
        .is_some_and(|number| number.fract() == 0.0 && number > 0.0)
}

/// `String(value)` for the scalars a pin's discriminant can be.
///
/// The schema constrains both members to strings, so a composite value only
/// reaches here inside a record the schema pass already refused; it is rendered
/// as one opaque spelling rather than walked, because the only use of the
/// rendering is telling two pins apart.
fn rendered(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_owned(),
        Some(Value::Null) => "null".to_owned(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(_)) => "[object Array]".to_owned(),
        Some(Value::Object(_)) => "[object Object]".to_owned(),
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::semantic_findings;

    fn findings_for(value: &serde_json::Value) -> Vec<String> {
        let mut findings = Vec::new();
        semantic_findings(value, &mut findings);
        findings
    }

    /// An unsupported clock that still carries its bounds is refused, and this
    /// is the check the typed record cannot make.
    #[test]
    fn an_unsupported_clock_carrying_bounds_is_refused() {
        let findings = findings_for(&json!({
            "record_shape": "standing_capability",
            "observed_at": "2026-01-01T00:00:00Z",
            "control_kind": "release",
            "capability": { "clock_support": { "supported": false, "deadline_seconds": 1 } },
        }));
        assert_eq!(
            findings,
            ["/capability/clock_support: unsupported clock excludes event/deadline fields"]
        );
    }

    #[test]
    fn a_supported_clock_without_a_positive_deadline_is_refused() {
        let findings = findings_for(&json!({
            "record_shape": "standing_capability",
            "observed_at": "2026-01-01T00:00:00Z",
            "control_kind": "release",
            "capability": { "clock_support": {
                "supported": true,
                "start_event": "a",
                "completion_event": "b",
                "deadline_seconds": 0
            } },
        }));
        assert_eq!(
            findings,
            ["/capability/clock_support: supported clock requires events and positive deadline"]
        );
    }

    /// The pin kinds are `VersionPinKind`'s, not a second literal set.
    #[test]
    fn a_pin_control_kind_needs_the_pin_it_is_named_after() {
        let without = findings_for(&json!({
            "record_shape": "standing_capability",
            "observed_at": "2026-01-01T00:00:00Z",
            "control_kind": "model_pin",
            "capability": {},
            "configuration": { "version_pins": [{ "kind": "policy", "identity": "p" }] },
        }));
        assert!(without.contains(
            &"/configuration/version_pins: model_pin requires a matching pin kind".to_owned()
        ));
        let with = findings_for(&json!({
            "record_shape": "standing_capability",
            "observed_at": "2026-01-01T00:00:00Z",
            "control_kind": "model_pin",
            "capability": {},
            "configuration": { "version_pins": [{ "kind": "model", "identity": "m" }] },
        }));
        assert_eq!(with, Vec::<String>::new());
    }

    #[test]
    fn two_pins_on_one_thing_are_refused_at_the_second() {
        let findings = findings_for(&json!({
            "record_shape": "standing_capability",
            "observed_at": "2026-01-01T00:00:00Z",
            "control_kind": "release",
            "capability": {},
            "configuration": { "version_pins": [
                { "kind": "model", "identity": "m" },
                { "kind": "model", "identity": "m" }
            ] },
        }));
        assert_eq!(
            findings,
            ["/configuration/version_pins/1: duplicate kind/identity"]
        );
    }

    #[test]
    fn an_exercise_that_completed_before_it_started_is_refused() {
        let findings = findings_for(&json!({
            "record_shape": "exercise",
            "observed_at": "2026-01-01T00:00:10Z",
            "control_kind": "release",
            "exercise": {
                "started_at": "2026-01-01T00:00:05Z",
                "completed_at": "2026-01-01T00:00:01Z",
                "clock": { "applicability": "not_applicable", "status": "not_applicable" }
            },
        }));
        assert_eq!(
            findings,
            ["/exercise/completed_at: precedes exercise start"]
        );
    }
}
