// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The operational report: a pure projection, and its markdown rendering.
//!
//! A port of `src/measurement/operational-report.ts`. The ordering note in
//! [`crate::intervention::report`] applies here unchanged — same comparator,
//! same UTF-16-versus-UTF-8 divergence, recorded once in `DIVERGENCE.md`.

use serde::{Deserialize, Serialize};

use crate::common::identity::{RecordId, WireInstant};
use crate::common::recorded_evidence::RecordedEvidenceReference;
use crate::operational::record::{
    CapabilityStatus, ExerciseOutcome, OperationalControlKind, OperationalEvidenceRecord,
    OperationalRecordShape,
};

/// One record, projected for reporting. `operational-report.ts:3-15`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalReportEntry {
    /// The record's identity.
    pub record_id: RecordId,
    /// When the evidence was observed.
    pub observed_at: WireInstant,
    /// Which shape the record is.
    pub record_shape: OperationalRecordShape,
    /// The kind of control.
    pub control_kind: OperationalControlKind,
    /// What the record claims.
    pub claims: Vec<String>,
    /// What the claim rests on, other than the retained files.
    pub evidence: Vec<String>,
    /// What argues against it.
    pub counterevidence: Vec<String>,
    /// What is unsettled — the record's own gaps, plus any the projection adds.
    pub gaps: Vec<String>,
    /// Who owns the record.
    pub owner: String,
    /// What the record asks for next.
    pub actions: Vec<String>,
    /// The retained files, ordered by path.
    pub raw_evidence: Vec<RecordedEvidenceReference>,
}

/// Projects records into report entries.
///
/// `buildOperationalReport` (`operational-report.ts:17-85`).
///
/// The projection is a three-way verdict per record — a claim, a gap, or
/// counterevidence — and both shapes reach it by their own route. The routes
/// are `match`es on the shape and on the status, so a status added to
/// [`CapabilityStatus`] or [`ExerciseOutcome`] is a compile error here rather
/// than a branch that quietly falls to the `else`.
#[must_use]
pub fn build_operational_report(
    records: &[OperationalEvidenceRecord],
) -> Vec<OperationalReportEntry> {
    let mut ordered: Vec<&OperationalEvidenceRecord> = records.iter().collect();
    ordered.sort_by(|left, right| {
        let (left, right) = (left.base(), right.base());
        left.observed_at
            .as_str()
            .cmp(right.observed_at.as_str())
            .then_with(|| left.record_id.as_str().cmp(right.record_id.as_str()))
    });
    ordered.into_iter().map(entry_for).collect()
}

/// One record's entry.
fn entry_for(record: &OperationalEvidenceRecord) -> OperationalReportEntry {
    let base = record.base();
    let mut claims: Vec<String> = Vec::new();
    let mut evidence: Vec<String> = Vec::new();
    let mut counterevidence: Vec<String> = Vec::new();
    let mut gaps = base.gaps.clone();

    match record {
        OperationalEvidenceRecord::StandingCapability(capability_record) => {
            let capability = &capability_record.capability;
            match capability.status {
                CapabilityStatus::Available => {
                    claims.push(format!(
                        "{} control {} is available for {}",
                        base.control_kind, capability.control_id, base.scope.service
                    ));
                    evidence.push(format!(
                        "surface {}; coverage {}",
                        capability.surface, capability.coverage
                    ));
                }
                CapabilityStatus::Unknown => {
                    gaps.push(format!(
                        "capability state is unknown for {}",
                        capability.control_id
                    ));
                }
                // `operational-report.ts:45-48`'s `else`: `unavailable` and
                // `not_applicable` both read as counterevidence, and both name
                // the status they carry.
                status @ (CapabilityStatus::Unavailable | CapabilityStatus::NotApplicable) => {
                    counterevidence
                        .push(format!("{} capability is {status}", capability.control_id));
                }
            }
        }
        OperationalEvidenceRecord::Exercise(exercise_record) => {
            let exercise = &exercise_record.exercise;
            let clock = &exercise.clock;
            let clock_status = clock.status_str();
            if exercise.outcome == ExerciseOutcome::Succeeded
                && (clock.is_not_applicable() || clock_status == "met")
            {
                claims.push(format!(
                    "{} control {} was exercised successfully",
                    base.control_kind, exercise.control_id
                ));
                evidence.push(format!(
                    "{} exercise completed {}; clock {clock_status}",
                    exercise.mode, exercise.completed_at
                ));
            } else if clock_status == "open" {
                gaps.push(format!(
                    "{} exercise is {}; clock {clock_status}",
                    exercise.control_id, exercise.outcome
                ));
            } else {
                counterevidence.push(format!(
                    "{} exercise is {}; clock {clock_status}",
                    exercise.control_id, exercise.outcome
                ));
            }
        }
    }

    let mut raw_evidence = base.raw_evidence.clone();
    raw_evidence.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));

    OperationalReportEntry {
        record_id: base.record_id.clone(),
        observed_at: base.observed_at.clone(),
        record_shape: record.record_shape(),
        control_kind: base.control_kind,
        claims,
        evidence,
        counterevidence,
        gaps,
        owner: base.owner.clone(),
        actions: base.actions.clone(),
        raw_evidence,
    }
}

/// Renders report entries as markdown.
///
/// `renderOperationalReport` (`operational-report.ts:87-127`). Byte-identical,
/// including the retained files being appended to the evidence section rather
/// than given one of their own, and `#### Actions` carrying no placeholder when
/// the list is empty.
#[must_use]
pub fn render_operational_report(entries: &[OperationalReportEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = vec!["## Operational evidence".to_owned(), String::new()];
    for entry in entries {
        lines.push(format!("### {}", entry.record_id));
        lines.push(String::new());

        let rendered_evidence: Vec<String> = entry
            .evidence
            .iter()
            .cloned()
            .chain(
                entry
                    .raw_evidence
                    .iter()
                    .map(|raw| format!("{} — {}", raw.path, raw.digest)),
            )
            .collect();
        let sections: [(&str, &[String], &str); 4] = [
            ("Claims", &entry.claims, "No affirmative claim."),
            ("Evidence", &rendered_evidence, "No affirmative evidence."),
            (
                "Counterevidence",
                &entry.counterevidence,
                "No declared counterevidence.",
            ),
            ("Gaps", &entry.gaps, "No declared gaps."),
        ];
        for (heading, values, empty) in sections {
            lines.push(format!("#### {heading}"));
            lines.push(String::new());
            if values.is_empty() {
                lines.push(empty.to_owned());
            } else {
                lines.extend(values.iter().map(|item| format!("- {item}")));
            }
            lines.push(String::new());
        }

        lines.push("#### Owner".to_owned());
        lines.push(String::new());
        lines.push(entry.owner.clone());
        lines.push(String::new());
        lines.push("#### Actions".to_owned());
        lines.push(String::new());
        lines.extend(entry.actions.iter().map(|item| format!("- {item}")));
        lines.push(String::new());
    }
    lines.join("\n")
}
