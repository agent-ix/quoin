// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The anomaly diagnostics (FR-136): illegal transition, missing reassessment,
//! missing module and no record.

use std::path::Path;

use serde::Serialize;

use crate::derived::{Derived, fixing_change};
use crate::error::AnomalyError;
use crate::history::{History, StatusEntry, pending_entry, read_history};
use crate::record::{assessment_text, parse_record, status_line, status_word};
use crate::status::Status;

/// A diagnostic's severity; every anomaly diagnostic is an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Fails `quoin anomaly check`.
    Error,
}

impl Severity {
    /// The severity as printed.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
        }
    }
}

/// A stable diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DiagnosticCode {
    /// A status move whose later status is unreachable from the earlier one.
    #[serde(rename = "anomaly.illegal-transition")]
    IllegalTransition,
    /// A reopen whose assessment section never changed.
    #[serde(rename = "anomaly.reassessment-missing")]
    ReassessmentMissing,
    /// A `Hazard` row with the `hazard` object type not installed.
    #[serde(rename = "anomaly.missing-module")]
    MissingModule,
    /// A document the mapping cannot read.
    #[serde(rename = "anomaly.no-record")]
    NoRecord,
}

impl DiagnosticCode {
    /// The code as printed.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IllegalTransition => "anomaly.illegal-transition",
            Self::ReassessmentMissing => "anomaly.reassessment-missing",
            Self::MissingModule => "anomaly.missing-module",
            Self::NoRecord => "anomaly.no-record",
        }
    }

    /// The reason that accompanies the code.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::IllegalTransition => "transition",
            Self::ReassessmentMissing => "reopen",
            Self::MissingModule => "spec-objects-safety",
            Self::NoRecord => "mapping-error",
        }
    }
}

/// One finding about one anomaly document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// The stable code.
    pub code: DiagnosticCode,
    /// Always `error`.
    pub severity: Severity,
    /// 1-based line of the frontmatter `status` field in the working tree.
    pub line: usize,
    /// The commits involved, when the finding is about history.
    pub commits: Vec<String>,
    /// Human-readable detail.
    pub message: String,
}

fn diagnostic(
    code: DiagnosticCode,
    line: usize,
    commits: Vec<String>,
    message: String,
) -> Diagnostic {
    Diagnostic {
        code,
        severity: Severity::Error,
        line,
        commits,
        message,
    }
}

impl Diagnostic {
    /// The reason: `transition`, `reopen`, `spec-objects-safety` or `mapping-error`.
    #[must_use]
    pub fn reason(&self) -> &'static str {
        self.code.reason()
    }
}

/// What the check needs from its environment.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// Whether the `hazard` object type is installed (`spec-objects-safety`).
    pub hazard_installed: bool,
}

fn short(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}

fn describe(entry: &StatusEntry) -> String {
    entry
        .commit
        .as_deref()
        .map_or_else(|| "the working tree".to_owned(), |c| short(c).to_owned())
}

fn transition_diagnostics(
    entries: &[StatusEntry],
    assessments: &[Option<String>],
    line: usize,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (index, pair) in entries.windows(2).enumerate() {
        let [earlier, later] = pair else { continue };
        let (from, to) = (earlier.status, later.status);
        let commits = [&earlier.commit, &later.commit]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        if !from.reaches(to) {
            out.push(diagnostic(
                DiagnosticCode::IllegalTransition,
                line,
                commits.clone(),
                format!(
                    "status moved from {} ({}) to {} ({}), which is not reachable by a legal move",
                    from.as_str(),
                    describe(earlier),
                    to.as_str(),
                    describe(later),
                ),
            ));
        }
        // A reopen is a visible `resolved` -> `assessed` entry; it needs the
        // assessment changed before the next move out of `assessed`.
        if from == Status::Resolved && to == Status::Assessed {
            let reopened = assessments.get(index + 1);
            let next = assessments.get(index + 2);
            if let (Some(at_reopen), Some(at_next)) = (reopened, next)
                && at_reopen == at_next
            {
                out.push(diagnostic(
                    DiagnosticCode::ReassessmentMissing,
                    line,
                    later.commit.iter().cloned().collect(),
                    format!(
                        "reopened at {} but the Safety Impact Assessment was not changed before the next move",
                        describe(later),
                    ),
                ));
            }
        }
    }
    out
}

/// The outcome of checking one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The findings, in the order they were made.
    pub diagnostics: Vec<Diagnostic>,
    /// False when the file is not under git: `statusHistory` does not apply
    /// and no transition was judged.
    pub history_applicable: bool,
    /// The fixing change (FR-136); `None` while the document has no record.
    pub fixing_change: Option<Derived>,
}

/// Check one `Anomaly` document, whose working-tree text is `text` and which
/// lives at `path`.
///
/// Every diagnostic is located at the frontmatter `status` field, whether or
/// not the document yields a record.
///
/// # Errors
///
/// Returns an error when `git` cannot be run or cannot answer a history query.
pub fn check_document(path: &Path, text: &str, context: Context) -> Result<Checked, AnomalyError> {
    let mut out = Vec::new();
    let record = parse_record(text);
    let line = status_line(text);
    if let Err(error) = &record {
        out.push(diagnostic(
            DiagnosticCode::NoRecord,
            line,
            Vec::new(),
            format!(
                "no record: mapping error {} at line {}",
                error.code.as_str(),
                error.line
            ),
        ));
    }
    let history = read_history(path)?;
    let history_applicable = !matches!(history, History::NotApplicable);
    let fixing = record
        .as_ref()
        .ok()
        .map(|record| fixing_change(&history, record));
    if let History::Entries {
        mut entries,
        mut assessments,
    } = history
    {
        let working = status_word(text)
            .as_deref()
            .and_then(Status::parse)
            .map(|status| (pending_entry(status), assessment_text(text)));
        if let Some((pending, assessment)) = working
            && entries
                .last()
                .is_none_or(|last| last.status != pending.status)
        {
            entries.push(pending);
            assessments.push(assessment);
        }
        out.extend(transition_diagnostics(&entries, &assessments, line));
    }
    if let Ok(record) = &record
        && !context.hazard_installed
        && record
            .assessment
            .iter()
            .flatten()
            .any(|row| row.kind == "Hazard")
    {
        out.push(diagnostic(
            DiagnosticCode::MissingModule,
            line,
            Vec::new(),
            "the assessment holds a Hazard row but the `hazard` object type is not installed; install spec-objects-safety".to_owned(),
        ));
    }
    Ok(Checked {
        diagnostics: out,
        history_applicable,
        fixing_change: fixing,
    })
}
