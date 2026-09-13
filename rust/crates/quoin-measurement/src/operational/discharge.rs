// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Whether an exercise discharges an obligation.
//!
//! Ports `operational.ts:174-230` (`operationalDischarge`).
//!
//! # A verdict with a reason, not a boolean with a comment
//!
//! The retained function returns `{ discharged, reason }`, and every caller has
//! to read the boolean to know whether the reason is an explanation or an
//! excuse. Here the two answers are two types: [`Discharged`] is minted only on
//! the path that checked everything, and [`NotDischarged`] is the only thing
//! that carries a reason to explain. The reason strings are the retained ones
//! verbatim — they are what a reader of a report sees today.
//!
//! # Instants are compared as instants
//!
//! Every clock comparison here is between parsed milliseconds, not between the
//! recorded spellings: `2026-08-29T23:21:00.000Z` and `2026-08-29T23:21:00Z`
//! are the same instant and the retained code treats them as one, which is why
//! an obligation stated by hand can be discharged by a produced record at all.

use serde_json::Value;

use crate::date_time::Rfc3339DateTime;
use crate::operational::clock::{DerivedClockStatus, derive_clock_status};
use crate::operational::record::{
    ExerciseClock, ExerciseOutcome, OperationalEvidenceRecord, OperationalExerciseRecord,
    OperationalObligation,
};
use crate::operational::validate::ValidOperationalRecord;

/// Proof that an exercise discharged an obligation.
///
/// Minted in exactly one place: the end of [`operational_discharge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Discharged;

impl Discharged {
    /// The reason the retained code reports. `operational.ts:227`.
    pub const REASON: &'static str = "matched succeeded exercise completed within clock";

    /// The reason this discharge holds.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        Self::REASON
    }
}

/// Why an exercise did not discharge an obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotDischarged {
    reason: String,
}

impl NotDischarged {
    /// The reason, in the retained spelling.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for NotDischarged {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.reason)
    }
}

/// Does this exercise discharge this obligation?
///
/// # Errors
///
/// [`NotDischarged`] carrying the first reason it does not, in the order
/// `operational.ts:178-226` checks them.
pub fn operational_discharge(
    exercise: &ValidOperationalRecord,
    obligation: &OperationalObligation,
) -> Result<Discharged, NotDischarged> {
    let OperationalEvidenceRecord::Exercise(record) = exercise.record() else {
        return no("invalid operational exercise: record is a standing capability");
    };
    if record.base.control_kind != obligation.control_kind {
        return no("control_kind mismatch");
    }
    if record.base.subject != obligation.subject {
        return no("subject mismatch");
    }
    if record.base.scope != obligation.scope {
        return no("scope mismatch");
    }
    if !obligation.accepted_modes.contains(&record.exercise.mode) {
        return no("exercise mode mismatch");
    }
    if record.exercise.outcome != ExerciseOutcome::Succeeded {
        return no(&format!(
            "exercise outcome {}",
            record.exercise.outcome.as_str()
        ));
    }
    let (Some(obligation_start), Some(obligation_deadline)) = (
        millis(obligation.clock.started_at.as_str()),
        millis(obligation.clock.deadline_at.as_str()),
    ) else {
        return no("invalid obligation clock condition");
    };
    if obligation_start > obligation_deadline {
        return no("invalid obligation clock condition");
    }
    let ExerciseClock::WithClock {
        started_at,
        deadline_at,
        completed_at,
        ..
    } = &record.exercise.clock
    else {
        return no(&format!(
            "clock status {}",
            record.exercise.clock.status_str()
        ));
    };
    if millis(started_at.as_str()) != Some(obligation_start)
        || millis(deadline_at.as_str()) != Some(obligation_deadline)
    {
        return no("obligation clock condition mismatch");
    }
    let completed = completed_at
        .as_ref()
        .and_then(|instant| millis(instant.as_str()));
    match completed {
        Some(completed) if completed >= obligation_start && completed <= obligation_deadline => {}
        _ => return no("exercise did not complete within obligation clock"),
    }
    let derived = derived_status(exercise, record);
    if derived != DerivedClockStatus::Met {
        return no(&format!("derived clock status {}", derived.as_str()));
    }
    Ok(Discharged)
}

/// The status re-derived from the record's own clock, on the document.
///
/// The document rather than [`ExerciseClock`] because the derivation's fourth
/// answer, [`DerivedClockStatus::Unknown`], is about instants that are absent
/// or unreadable, and the typed clock has already refused those — deriving from
/// it would be deriving from the answer.
fn derived_status(
    exercise: &ValidOperationalRecord,
    record: &OperationalExerciseRecord,
) -> DerivedClockStatus {
    let observed = millis(record.base.observed_at.as_str());
    exercise
        .document()
        .get("exercise")
        .and_then(|payload| payload.get("clock"))
        .and_then(Value::as_object)
        .map_or(DerivedClockStatus::Unknown, |clock| {
            derive_clock_status(clock, observed)
        })
}

fn millis(text: &str) -> Option<i64> {
    Rfc3339DateTime::parse(text)
        .ok()
        .map(|parsed| parsed.epoch_millis())
}

fn no(reason: &str) -> Result<Discharged, NotDischarged> {
    Err(NotDischarged {
        reason: reason.to_owned(),
    })
}
