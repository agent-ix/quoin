// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The intervention report: a pure projection, and its markdown rendering.
//!
//! A port of `src/measurement/intervention-report.ts`. Both functions are total
//! and read nothing — no clock, no filesystem — which is why they are the whole
//! of this module and not a seam on one.
//!
//! # Ordering
//!
//! The TypeScript comparator is `a === b ? 0 : a < b ? -1 : 1` over strings,
//! and JavaScript's `<` compares UTF-16 code units while Rust's `Ord` for `str`
//! compares UTF-8 bytes. The two agree on every string whose characters are all
//! in the Basic Multilingual Plane and disagree only when a supplementary
//! character (U+10000 and above) is compared against one in U+E000..=U+FFFF.
//! See `DIVERGENCE.md`; no identity in this domain is written in that range.
//!
//! Both sorts are stable — `Array.prototype.sort` since ES2019, and
//! `slice::sort_by` — so records that tie on both keys keep their input order
//! on both sides.

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::common::identity::{RecordId, WireInstant};
use crate::common::producer::{Producer, Subject};
use crate::common::recorded_evidence::RecordedEvidenceReference;
use crate::common::wire_enum::wire_enum;
use crate::intervention::record::{
    DispositionedEffect, InterventionDisposition, InterventionExperimentRecord, InterventionStatus,
    MeasuredEffect,
};

wire_enum! {
    /// Which list a piece of counterevidence came from.
    ///
    /// `intervention-report.ts:52,55`.
    pub enum CounterevidenceKind {
        /// From `confounders`.
        Confounder => "confounder",
        /// From `interactions`.
        Interaction => "interaction",
    }
}

wire_enum! {
    /// The dispositions that make an effect counterevidence.
    ///
    /// `intervention-report.ts:18,121-126` — `isAdverse` narrows the four-way
    /// [`InterventionDisposition`] to these two, and the entry type carries the
    /// narrowed pair. Carrying the narrow type is what stops a `controlled`
    /// effect reaching the counterevidence list: there is no value of this type
    /// that spells one.
    pub enum AdverseDisposition {
        /// The effect varied across arms and was not held constant.
        Uncontrolled => "uncontrolled",
        /// Whether the effect varied is not known.
        Unknown => "unknown",
    }
}

impl AdverseDisposition {
    /// The adverse reading of a disposition, or `None` if it is not adverse.
    ///
    /// The whole of `isAdverse` (`intervention-report.ts:121-126`), as a parse
    /// rather than a predicate.
    #[must_use]
    pub fn of(disposition: InterventionDisposition) -> Option<Self> {
        match disposition {
            InterventionDisposition::Uncontrolled => Some(Self::Uncontrolled),
            InterventionDisposition::Unknown => Some(Self::Unknown),
            InterventionDisposition::Controlled | InterventionDisposition::NotApplicable => None,
        }
    }
}

/// One piece of counterevidence. `intervention-report.ts:16-20`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counterevidence {
    /// Which list it came from.
    pub kind: CounterevidenceKind,
    /// What the effect is.
    pub description: String,
    /// How it was handled — adverse by construction.
    pub disposition: AdverseDisposition,
}

/// What a record rests on. `intervention-report.ts:12-15`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionEvidence {
    /// The measurements, ordered by `(treatment_id, metric)`.
    pub measured_effects: Vec<MeasuredEffect>,
    /// The retained files, ordered by path.
    pub raw_evidence: Vec<RecordedEvidenceReference>,
}

/// One record, projected for reporting. `intervention-report.ts:3-23`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionReportEntry {
    /// The record's identity.
    pub record_id: RecordId,
    /// When the experiment was observed.
    pub observed_at: WireInstant,
    /// What was measured.
    pub subject: Subject,
    /// Whether the experiment ran to a usable end.
    pub status: InterventionStatus,
    /// What the record claims — the conclusion's statement, and nothing else.
    pub claims: Vec<String>,
    /// What the claim rests on.
    pub evidence: InterventionEvidence,
    /// What argues against it.
    pub counterevidence: Vec<Counterevidence>,
    /// What the experiment could not settle.
    pub gaps: Vec<String>,
    /// Who owns the record.
    pub owner: String,
    /// What the record asks for next.
    pub actions: Vec<String>,
    /// What produced the record.
    pub producer: Producer,
}

/// Projects records into report entries, ordered and with their lists sorted.
///
/// `buildInterventionReport` (`intervention-report.ts:25-65`).
#[must_use]
pub fn build_intervention_report(
    records: &[InterventionExperimentRecord],
) -> Vec<InterventionReportEntry> {
    let mut ordered: Vec<&InterventionExperimentRecord> = records.iter().collect();
    ordered.sort_by(|left, right| {
        left.observed_at
            .as_str()
            .cmp(right.observed_at.as_str())
            .then_with(|| left.record_id.as_str().cmp(right.record_id.as_str()))
    });
    ordered.into_iter().map(entry_for).collect()
}

/// One record's entry.
fn entry_for(record: &InterventionExperimentRecord) -> InterventionReportEntry {
    let mut measured_effects = record.measured_effects.clone();
    measured_effects.sort_by(|left, right| {
        left.treatment_id
            .as_str()
            .cmp(right.treatment_id.as_str())
            .then_with(|| left.metric.as_str().cmp(right.metric.as_str()))
    });

    let mut raw_evidence = record.raw_evidence.clone();
    raw_evidence.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));

    let mut counterevidence: Vec<Counterevidence> =
        adverse(&record.interactions, CounterevidenceKind::Interaction)
            .chain(adverse(
                &record.confounders,
                CounterevidenceKind::Confounder,
            ))
            .collect();
    counterevidence.sort_by(|left, right| {
        left.kind
            .as_str()
            .cmp(right.kind.as_str())
            .then_with(|| left.description.cmp(&right.description))
    });

    InterventionReportEntry {
        record_id: record.record_id.clone(),
        observed_at: record.observed_at.clone(),
        subject: record.subject.clone(),
        status: record.status,
        claims: vec![record.conclusion.statement.clone()],
        evidence: InterventionEvidence {
            measured_effects,
            raw_evidence,
        },
        counterevidence,
        gaps: record.gaps.clone(),
        owner: record.owner.clone(),
        actions: record.actions.clone(),
        producer: record.producer.clone(),
    }
}

/// The adverse members of one list, tagged with which list they came from.
fn adverse(
    effects: &[DispositionedEffect],
    kind: CounterevidenceKind,
) -> impl Iterator<Item = Counterevidence> + '_ {
    effects.iter().filter_map(move |effect| {
        AdverseDisposition::of(effect.disposition).map(|disposition| Counterevidence {
            kind,
            description: effect.description.clone(),
            disposition,
        })
    })
}

/// Renders report entries as markdown.
///
/// `renderInterventionReport` (`intervention-report.ts:67-116`). Byte-identical
/// to it, including the two places the TypeScript emits no placeholder — an
/// entry with neither measured effects nor raw evidence leaves `#### Evidence`
/// empty, and an entry with no actions leaves `#### Actions` empty. The
/// committed oracle capture holds both (case `single_bare`).
///
/// There is no trailing newline: the TypeScript joins with `"\n"` rather than
/// appending one, and the last pushed element is an empty string, so the text
/// ends in exactly one newline after the last action line.
#[must_use]
pub fn render_intervention_report(entries: &[InterventionReportEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = vec!["## Intervention experiments".to_owned(), String::new()];
    for entry in entries {
        lines.push(format!("### {}", entry.record_id));
        lines.push(String::new());
        lines.push("#### Claims".to_owned());
        lines.push(String::new());
        lines.extend(entry.claims.iter().map(|claim| format!("- {claim}")));
        lines.push(String::new());
        lines.push("#### Evidence".to_owned());
        lines.push(String::new());
        for effect in &entry.evidence.measured_effects {
            let mut line = String::new();
            // `write!` to a String cannot fail; the result is discarded rather
            // than unwrapped because `unwrap_used` is a workspace lint.
            let _ = write!(
                line,
                "- {}/{}: baseline {}, treatment {}, effect {} {}",
                effect.treatment_id,
                effect.metric,
                effect.baseline_value,
                effect.treatment_value,
                effect.effect,
                effect.unit
            );
            lines.push(line);
        }
        lines.extend(entry.evidence.raw_evidence.iter().map(|raw| {
            format!(
                "- {} — {}; {} bytes; {}",
                raw.path, raw.digest, raw.size_bytes, raw.media_type
            )
        }));
        lines.push(String::new());
        lines.push("#### Counterevidence".to_owned());
        lines.push(String::new());
        if entry.counterevidence.is_empty() {
            lines.push("No declared counterevidence.".to_owned());
        } else {
            lines.extend(entry.counterevidence.iter().map(|item| {
                format!(
                    "- {}: {} ({})",
                    item.kind, item.description, item.disposition
                )
            }));
        }
        lines.push(String::new());
        lines.push("#### Gaps".to_owned());
        lines.push(String::new());
        if entry.gaps.is_empty() {
            lines.push("No declared gaps.".to_owned());
        } else {
            lines.extend(entry.gaps.iter().map(|gap| format!("- {gap}")));
        }
        lines.push(String::new());
        lines.push("#### Owner".to_owned());
        lines.push(String::new());
        lines.push(entry.owner.clone());
        lines.push(String::new());
        lines.push("#### Actions".to_owned());
        lines.push(String::new());
        lines.extend(entry.actions.iter().map(|action| format!("- {action}")));
        lines.push(String::new());
    }
    lines.join("\n")
}
