// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The fix-verification evidence findings (FR-175).
//!
//! A fixed anomaly's `## Fix Verification` rows name what demonstrates the fix.
//! This module states what the evidence store holds against those names: a
//! result is derived from the newest run of each suite, over the entries whose
//! trace ids match the row's `Target`. It reads and reports; it never writes
//! and it never says a fix is good (NFR-035).

use std::collections::BTreeMap;
use std::path::Path;

use quoin_evidence::store::{latest_run, list_recorded_suites};
use quoin_evidence::{DiskEvidence, Outcome};

use crate::check::{Diagnostic, DiagnosticCode, diagnostic};
use crate::error::AnomalyError;
use crate::record::{Record, VerificationRow, parse_record};
use crate::refs::{CorpusIds, corpus_ids};
use crate::status::Status;

/// The result of one fix-verification row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixResult {
    /// A matching entry of a suite's newest run failed or errored.
    Failing,
    /// No matching entry failed and one passed.
    Passing,
    /// No matching entry, only skipped entries, or no store.
    Missing,
}

/// One entry of a suite's newest run: the trace ids it carries and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reported {
    trace_ids: Vec<String>,
    outcome: Outcome,
}

/// What the check reads besides the anomaly: the store's results, the ids of
/// the corpus, and which test cases each criterion names.
#[derive(Debug, Clone)]
pub struct Verification {
    results: Vec<Reported>,
    skipped: Vec<String>,
    corpus: CorpusIds,
    criterion_tests: BTreeMap<String, Vec<String>>,
}

impl Verification {
    /// Read the evidence store and the corpus under `scope`.
    ///
    /// `criterion_tests` maps a criterion id to the test case ids its
    /// obligation names (quire's `target_ids`, FR-053-AC-11); the caller takes
    /// them from the obligation records and this crate never reads a
    /// Verification cell.
    ///
    /// # Errors
    ///
    /// Returns an error when `scope` cannot be read or the store cannot be
    /// listed.
    pub fn read(
        scope: &Path,
        criterion_tests: BTreeMap<String, Vec<String>>,
    ) -> Result<Self, AnomalyError> {
        let (results, skipped) = read_results(scope)?;
        Ok(Self {
            results,
            skipped,
            corpus: corpus_ids(scope)?,
            criterion_tests,
        })
    }

    /// The store-relative paths of the run files that could not be read and
    /// were skipped (FR-030-AC-13), sorted. Each leaves its suite without an
    /// entry.
    #[must_use]
    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }

    /// The result for the trace ids `ids`.
    #[must_use]
    pub fn result_of(&self, ids: &[&str]) -> FixResult {
        let mut passed = false;
        for entry in &self.results {
            if !entry.trace_ids.iter().any(|t| ids.contains(&t.as_str())) {
                continue;
            }
            match entry.outcome {
                Outcome::Fail | Outcome::Error => return FixResult::Failing,
                Outcome::Pass => passed = true,
                Outcome::Skip => {}
            }
        }
        if passed {
            FixResult::Passing
        } else {
            FixResult::Missing
        }
    }

    /// The findings for one `Anomaly` document, in row order. A document with no
    /// record, or one that is not a fixed anomaly, yields none.
    #[must_use]
    pub fn diagnostics(&self, text: &str) -> Vec<Diagnostic> {
        let Ok(record) = parse_record(text) else {
            return Vec::new();
        };
        if !is_fixed(&record) {
            return Vec::new();
        }
        record
            .fix_verification
            .iter()
            .flatten()
            .filter_map(|row| self.finding(&record, row))
            .collect()
    }

    fn finding(&self, record: &Record, row: &VerificationRow) -> Option<Diagnostic> {
        let target = row.target.as_deref()?;
        if !self.corpus.contains(target) {
            return None;
        }
        let (kind, result) = match row.kind.as_str() {
            "Test" => ("Test", self.result_of(&[target])),
            "Criterion" if self.corpus.criteria.contains(target) => {
                let mut ids = vec![target];
                ids.extend(
                    self.criterion_tests
                        .get(target)
                        .into_iter()
                        .flatten()
                        .map(String::as_str),
                );
                ("Criterion", self.result_of(&ids))
            }
            _ => return None,
        };
        let (code, what) = match (result, kind) {
            (FixResult::Failing, _) => (
                DiagnosticCode::FixResultFailing,
                "has a failing result: the newest run of a suite records fail or error for it",
            ),
            (FixResult::Missing, "Test") => (
                DiagnosticCode::FixTestMissingResult,
                "has no result: no entry of the newest run of any suite carries it as a pass, fail or error",
            ),
            (FixResult::Passing | FixResult::Missing, _) => return None,
        };
        let mut finding = diagnostic(
            code,
            record.status_line,
            Vec::new(),
            format!("{kind} row {what}"),
        );
        finding.target = Some(target.to_owned());
        Some(finding)
    }
}

/// Whether the anomaly is resolved or closed as `fixed`: the only anomalies
/// the check evaluates.
fn is_fixed(record: &Record) -> bool {
    matches!(record.status(), Some(Status::Resolved | Status::Closed))
        && record.resolution.as_deref() == Some("fixed")
}

/// Whether the document is a fixed anomaly with a `Criterion` row, whose
/// matching needs the obligations' test case ids.
#[must_use]
pub fn evaluates_criteria(text: &str) -> bool {
    parse_record(text).is_ok_and(|record| {
        is_fixed(&record)
            && record
                .fix_verification
                .iter()
                .flatten()
                .any(|row| row.kind == "Criterion" && row.target.is_some())
    })
}

/// The entries of the newest run of each suite, and the run files skipped as
/// unreadable. A suite with an unreadable run file contributes no entry: the
/// file has no timestamp, so it cannot be ruled out as the newest and an older
/// run must not stand in for it. As in `quoin evidence`, an unreadable file is
/// skipped and reported, while a failure to read the store is an error.
fn read_results(scope: &Path) -> Result<(Vec<Reported>, Vec<String>), AnomalyError> {
    let store = DiskEvidence::new(scope);
    let suites = list_recorded_suites(&store).map_err(|error| evidence_error(&error))?;
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    for suite in suites {
        let mut unreadable = Vec::new();
        let newest =
            latest_run(&store, &suite, &mut unreadable).map_err(|error| evidence_error(&error))?;
        if !unreadable.is_empty() {
            skipped.append(&mut unreadable);
            continue;
        }
        if let Some(newest) = newest {
            out.extend(newest.entries.into_iter().map(|entry| Reported {
                trace_ids: entry.trace_ids.unwrap_or_default(),
                outcome: entry.outcome,
            }));
        }
    }
    skipped.sort();
    skipped.dedup();
    Ok((out, skipped))
}

fn evidence_error(error: &quoin_evidence::EvidenceError) -> AnomalyError {
    AnomalyError {
        code: crate::error::ErrorCode::Io,
        message: error.to_string(),
    }
}
