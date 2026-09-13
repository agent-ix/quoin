// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Where an exercise stands against its deadline, derived rather than believed.
//!
//! Ports `operational.ts:365-421` (`checkClock`) and `operational.ts:471-489`
//! (`deriveClockStatus`).
//!
//! # The recorded status is a claim, and the derived status is the fact
//!
//! A record carries `clock.status`, and intake does not take its word for it:
//! the status is re-derived from the three instants and the record is refused
//! when the two disagree. That is why [`DerivedClockStatus`] exists beside
//! [`crate::operational::record::ClockStatus`] and is not the same type — the
//! recorded status can only be `open`, `met` or `missed`, while a derivation
//! from instants that are absent or unreadable has a fourth answer,
//! [`DerivedClockStatus::Unknown`], which no record may claim.

use serde_json::{Map, Value};

use crate::common::wire_enum::wire_enum;
use crate::date_time::Rfc3339DateTime;

wire_enum! {
    /// Where an exercise stands, as intake derives it. `operational.ts:474`.
    pub enum DerivedClockStatus {
        /// The deadline has not passed and the exercise has not completed.
        Open => "open",
        /// The exercise completed inside the deadline.
        Met => "met",
        /// The deadline passed first.
        Missed => "missed",
        /// There are not enough readable instants to say.
        ///
        /// No record may claim this: the schema's `clock.status` admits only
        /// the other three, so a derivation of `unknown` always disagrees with
        /// whatever was recorded and the record is refused.
        Unknown => "unknown",
    }
}

/// Read an instant, reporting an unreadable one at `pointer`.
///
/// `operational.ts:463-469`. A missing member and an unparseable one are the
/// same finding, because `parseRfc3339DateTime(undefined)` is `null`.
pub(super) fn instant_at(
    value: Option<&Value>,
    pointer: &str,
    findings: &mut Vec<String>,
) -> Option<i64> {
    let millis = instant(value);
    if millis.is_none() {
        findings.push(format!("{pointer}: must be an RFC 3339 date-time"));
    }
    millis
}

/// Read an instant, saying nothing when it is not one.
pub(super) fn instant(value: Option<&Value>) -> Option<i64> {
    value
        .and_then(Value::as_str)
        .and_then(|text| Rfc3339DateTime::parse(text).ok())
        .map(|parsed| parsed.epoch_millis())
}

/// Derive where an exercise stands from its instants alone.
#[must_use]
pub fn derive_clock_status(
    clock: &Map<String, Value>,
    observed: Option<i64>,
) -> DerivedClockStatus {
    let deadline = instant(clock.get("deadline_at"));
    // `undefined` and an unreadable value are different here: an absent
    // `completed_at` means the exercise is still running, which is what makes
    // `open` reachable at all.
    let completed = clock
        .get("completed_at")
        .and_then(|value| instant(Some(value)));
    match (completed, observed, deadline) {
        (Some(completed), _, Some(deadline)) => at_or_before(completed, deadline),
        (None, Some(observed), Some(deadline)) => match at_or_before(observed, deadline) {
            DerivedClockStatus::Met => DerivedClockStatus::Open,
            missed => missed,
        },
        _ => DerivedClockStatus::Unknown,
    }
}

fn at_or_before(instant: i64, deadline: i64) -> DerivedClockStatus {
    if instant <= deadline {
        DerivedClockStatus::Met
    } else {
        DerivedClockStatus::Missed
    }
}

/// Check an exercise's clock against itself and against the derivation.
///
/// `operational.ts:365-421`. A clock that is absent or not an object is not
/// checked here at all: the schema already refused it, and a second refusal
/// from this pass would only duplicate the finding.
pub(super) fn check_clock(
    exercise: &Map<String, Value>,
    observed: Option<i64>,
    findings: &mut Vec<String>,
) {
    let Some(clock) = exercise.get("clock").and_then(Value::as_object) else {
        return;
    };
    match clock.get("applicability").and_then(Value::as_str) {
        Some("not_applicable") => {
            if clock.get("status").and_then(Value::as_str) != Some("not_applicable")
                || clock.len() != 2
            {
                findings.push(
                    "/exercise/clock: not_applicable excludes timestamps and requires matching \
                     status"
                        .to_owned(),
                );
            }
        }
        Some("operational_with_clock") => with_clock(clock, observed, findings),
        _ => {}
    }
}

fn with_clock(clock: &Map<String, Value>, observed: Option<i64>, findings: &mut Vec<String>) {
    let started = instant_at(
        clock.get("started_at"),
        "/exercise/clock/started_at",
        findings,
    );
    let deadline = instant_at(
        clock.get("deadline_at"),
        "/exercise/clock/deadline_at",
        findings,
    );
    let completed = clock
        .get("completed_at")
        .and_then(|value| instant_at(Some(value), "/exercise/clock/completed_at", findings));
    if let (Some(started), Some(deadline)) = (started, deadline)
        && started > deadline
    {
        findings.push("/exercise/clock/deadline_at: precedes clock start".to_owned());
    }
    if let (Some(started), Some(completed)) = (started, completed)
        && completed < started
    {
        findings.push("/exercise/clock/completed_at: precedes clock start".to_owned());
    }
    let derived = derive_clock_status(clock, observed);
    let recorded = clock.get("status").and_then(Value::as_str).unwrap_or("");
    if recorded != derived.as_str() {
        findings.push(format!(
            "/exercise/clock/status: {recorded} disagrees with derived {derived}",
            derived = derived.as_str()
        ));
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
    use serde_json::{Map, Value, json};

    use super::{DerivedClockStatus, derive_clock_status};
    use crate::date_time::Rfc3339DateTime;

    fn clock(value: &Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn observed(text: &str) -> i64 {
        Rfc3339DateTime::parse(text).unwrap().epoch_millis()
    }

    /// Every branch of the derivation, including the one no record may claim.
    #[test]
    fn the_derivation_answers_from_the_instants_alone() {
        let bounded = clock(&json!({
            "started_at": "2026-08-29T23:11:00Z",
            "deadline_at": "2026-08-29T23:21:00.000Z",
            "completed_at": "2026-08-29T23:11:37Z",
        }));
        assert_eq!(derive_clock_status(&bounded, None), DerivedClockStatus::Met);

        let late = clock(&json!({
            "deadline_at": "2026-08-29T23:21:00.000Z",
            "completed_at": "2026-08-29T23:31:00Z",
        }));
        assert_eq!(derive_clock_status(&late, None), DerivedClockStatus::Missed);

        let running = clock(&json!({ "deadline_at": "2026-08-29T23:21:00.000Z" }));
        assert_eq!(
            derive_clock_status(&running, Some(observed("2026-08-29T23:12:00Z"))),
            DerivedClockStatus::Open
        );
        assert_eq!(
            derive_clock_status(&running, Some(observed("2026-08-29T23:41:00Z"))),
            DerivedClockStatus::Missed
        );

        // No deadline, so nothing to be on the right side of.
        assert_eq!(
            derive_clock_status(&clock(&json!({})), None),
            DerivedClockStatus::Unknown
        );
    }

    /// A completion the deadline exactly equals is met, not missed.
    #[test]
    fn the_deadline_itself_is_inside_the_deadline() {
        let exact = clock(&json!({
            "deadline_at": "2026-08-29T23:21:00.000Z",
            "completed_at": "2026-08-29T23:21:00Z",
        }));
        assert_eq!(derive_clock_status(&exact, None), DerivedClockStatus::Met);
    }
}
