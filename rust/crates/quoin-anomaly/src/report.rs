// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin anomaly report` (FR-162, FR-163, FR-164, NFR-035): the anomalies a
//! release still carries, grouped by affected hazard and requirement, and the
//! `--fail-on` gate over them.
//!
//! The report is a view. It prints only what a person wrote in an `Anomaly`
//! document, what git recorded about its commits, and the group and gate facts
//! derived from them. It originates no severity, impact, rationale or verdict,
//! writes no file, and never judges a rationale adequate.

mod render;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;

use crate::carried::read_carried;
use crate::derived::{Derived, fixing_change, rationale_in};
use crate::error::AnomalyError;
use crate::history::{history_of, locate};
use crate::record::{RationaleEntry, Record};

/// A condition the gate tests (FR-164).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateMode {
    /// A listed anomaly whose `safety_impact` is `unassessed`.
    Unassessed,
    /// A carried `impacting` anomaly with no release rationale.
    UnjustifiedImpacting,
}

impl GateMode {
    /// Every mode, in canonical order.
    pub const ALL: [Self; 2] = [Self::Unassessed, Self::UnjustifiedImpacting];

    /// The mode a `--fail-on` value names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == word)
    }

    /// The mode as given to `--fail-on`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unassessed => "unassessed",
            Self::UnjustifiedImpacting => "unjustified-impacting",
        }
    }
}

/// One `assessment` entry, as the record carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssessmentEntry {
    /// `Requirement`, `Hazard` or `None`.
    pub kind: String,
    /// The authored reference token; `null` for a row whose `Target` is `-`.
    pub target: Option<String>,
    /// How the anomaly bears on the target.
    pub impact: String,
    /// The engineer's judgement, verbatim.
    pub rationale: String,
}

/// An `assessment` entry of kind `Requirement` or `Hazard`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Affected {
    /// The authored target id; never resolved against the corpus.
    pub target: String,
    /// `Requirement` or `Hazard`.
    pub kind: String,
    /// How the anomaly bears on the target.
    pub impact: String,
}

/// One listed anomaly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    /// The document id.
    pub id: String,
    /// The authored title; `null` when absent.
    pub title: Option<String>,
    /// The document path.
    pub path: String,
    /// The authored status.
    pub status: String,
    /// The authored resolution, when present.
    pub resolution: Option<String>,
    /// The authored severity, when present.
    pub severity: Option<String>,
    /// The authored safety impact.
    pub safety_impact: String,
    /// Whether `safety_impact` is `unassessed`.
    pub unassessed: bool,
    /// The assessment rows, in document order.
    pub assessment: Vec<AssessmentEntry>,
    /// The requirement and hazard targets of the assessment, in document order.
    pub affected: Vec<Affected>,
    /// The authored release rationale; empty while the section is absent.
    pub release_rationale: Vec<RationaleEntry>,
    /// The fixing change, derived from git (FR-136).
    pub fixing_change: Derived,
    /// The change that last edited the release rationale, derived from git
    /// (FR-163).
    pub rationale_change: Derived,
}

/// One anomaly in a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Member {
    /// The anomaly id.
    pub anomaly: String,
    /// The impact the anomaly states on the group's target.
    pub impact: String,
}

/// The listed anomalies that name one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Group {
    /// The authored target id.
    pub target: String,
    /// The anomalies naming it, by anomaly id.
    pub anomalies: Vec<Member>,
}

/// A listed anomaly that is in no group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ungrouped {
    /// The anomaly id.
    pub anomaly: String,
    /// `unassessed` or `no-affected-id`.
    pub reason: &'static str,
}

/// A document the mapping cannot read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unreadable {
    /// The document path.
    pub path: String,
    /// The mapping error's code.
    pub code: &'static str,
}

/// One gate failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Failure {
    /// The mode that failed.
    pub mode: GateMode,
    /// The anomaly id.
    pub anomaly: String,
    /// The document path.
    pub path: String,
}

/// The `--fail-on` result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gate {
    /// The modes evaluated.
    pub modes: Vec<GateMode>,
    /// Every failure, by anomaly id then mode.
    pub failures: Vec<Failure>,
}

/// The anomaly report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// The listed anomalies, by anomaly id.
    pub anomalies: Vec<ReportRow>,
    /// One group for each hazard a listed anomaly names.
    pub by_hazard: Vec<Group>,
    /// One group for each requirement or criterion a listed anomaly names.
    pub by_requirement: Vec<Group>,
    /// Listed anomalies in no group.
    pub ungrouped: Vec<Ungrouped>,
    /// Documents with a mapping error and so no record.
    pub unreadable: Vec<Unreadable>,
    /// The gate, when `--fail-on` was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,
}

impl Report {
    /// Whether the command exits non-zero: a document was unreadable, or the
    /// gate has a failure.
    #[must_use]
    pub fn failed(&self) -> bool {
        !self.unreadable.is_empty() || self.gate.as_ref().is_some_and(|g| !g.failures.is_empty())
    }

    /// The report as Markdown.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        render::markdown(self)
    }

    /// The report as one pretty-printed JSON object.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error; the report holds only strings, so none
    /// is expected.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

fn is_placeholder(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "" | "n/a" | "none" | "unknown" | "tbd" | "-"
    )
}

/// An entry counts as a release rationale when its author and rationale each
/// satisfy the value rule of spec-artifacts-process FR-016. Whether it is
/// adequate is never read.
fn counts_as_rationale(entry: &RationaleEntry) -> bool {
    !is_placeholder(&entry.author) && !is_placeholder(&entry.rationale)
}

/// The path as shown: as given under the scope, without a leading `./`.
fn shown(path: &Path) -> String {
    let path = path.strip_prefix(".").unwrap_or(path);
    path.to_string_lossy().replace('\\', "/")
}

fn row_of(path: &Path, record: Record) -> Result<ReportRow, AnomalyError> {
    let located = locate(path)?;
    let fixing = fixing_change(&history_of(located.as_ref()), &record);
    let rationale = rationale_in(located.as_ref(), &record);
    let assessment = record
        .assessment
        .iter()
        .flatten()
        .map(|row| AssessmentEntry {
            kind: row.kind.clone(),
            target: row.target.clone(),
            impact: row.impact.clone(),
            rationale: row.rationale.clone(),
        })
        .collect::<Vec<_>>();
    let affected = assessment
        .iter()
        .filter(|entry| matches!(entry.kind.as_str(), "Requirement" | "Hazard"))
        .filter_map(|entry| {
            Some(Affected {
                target: entry.target.clone()?,
                kind: entry.kind.clone(),
                impact: entry.impact.clone(),
            })
        })
        .collect();
    Ok(ReportRow {
        unassessed: record.safety_impact == "unassessed",
        id: record.id,
        title: record.title,
        path: shown(path),
        status: record.status_word,
        resolution: record.resolution,
        severity: record.severity,
        safety_impact: record.safety_impact,
        assessment,
        affected,
        release_rationale: record.release_rationale.unwrap_or_default(),
        fixing_change: fixing,
        rationale_change: rationale,
    })
}

fn groups(rows: &[ReportRow], kind: &str) -> Vec<Group> {
    let mut by_target: BTreeMap<&str, Vec<Member>> = BTreeMap::new();
    for row in rows {
        let mut seen = BTreeSet::new();
        for affected in row.affected.iter().filter(|a| a.kind == kind) {
            if seen.insert(affected.target.as_str()) {
                by_target
                    .entry(affected.target.as_str())
                    .or_default()
                    .push(Member {
                        anomaly: row.id.clone(),
                        impact: affected.impact.clone(),
                    });
            }
        }
    }
    by_target
        .into_iter()
        .map(|(target, anomalies)| Group {
            target: target.to_owned(),
            anomalies,
        })
        .collect()
}

fn gate(rows: &[ReportRow], modes: &[GateMode]) -> Gate {
    let modes = modes.iter().copied().collect::<BTreeSet<_>>();
    let mut failures = Vec::new();
    for row in rows {
        for &mode in &modes {
            let fails = match mode {
                GateMode::Unassessed => row.unassessed,
                GateMode::UnjustifiedImpacting => {
                    row.safety_impact == "impacting"
                        && (row.status != "closed" || row.resolution.as_deref() == Some("wont-fix"))
                        && !row.release_rationale.iter().any(counts_as_rationale)
                }
            };
            if fails {
                failures.push(Failure {
                    mode,
                    anomaly: row.id.clone(),
                    path: row.path.clone(),
                });
            }
        }
    }
    failures.sort_by(|a, b| (&a.anomaly, a.mode, &a.path).cmp(&(&b.anomaly, b.mode, &b.path)));
    Gate {
        modes: modes.into_iter().collect(),
        failures,
    }
}

/// Build the report over the `Anomaly` documents under `scope`, and evaluate
/// the gate when `modes` is not empty.
///
/// # Errors
///
/// Returns an error when `scope` cannot be read or `git` cannot be run.
pub fn build_report(scope: &Path, modes: &[GateMode]) -> Result<Report, AnomalyError> {
    let read = read_carried(scope)?;
    let unreadable = read
        .unreadable
        .iter()
        .map(|entry| Unreadable {
            path: shown(&entry.path),
            code: entry.code.as_str(),
        })
        .collect();
    let mut rows = Vec::new();
    for entry in read.carried {
        rows.push(row_of(&entry.path, entry.record)?);
    }
    rows.sort_by(|a, b| (&a.id, &a.path).cmp(&(&b.id, &b.path)));
    let by_hazard = groups(&rows, "Hazard");
    let by_requirement = groups(&rows, "Requirement");
    let grouped = by_hazard
        .iter()
        .chain(&by_requirement)
        .flat_map(|group| &group.anomalies)
        .map(|member| member.anomaly.as_str())
        .collect::<BTreeSet<_>>();
    let ungrouped = rows
        .iter()
        .filter(|row| !grouped.contains(row.id.as_str()))
        .map(|row| Ungrouped {
            anomaly: row.id.clone(),
            reason: if row.unassessed {
                "unassessed"
            } else {
                "no-affected-id"
            },
        })
        .collect();
    let gate = (!modes.is_empty()).then(|| gate(&rows, modes));
    Ok(Report {
        anomalies: rows,
        by_hazard,
        by_requirement,
        ungrouped,
        unreadable,
        gate,
    })
}
