// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The read-only risk advice report (FR-145 to FR-148, NFR-031).
//!
//! It reads the safety-module records below a scope and prints the policy cell
//! of each risk estimate, the evidence gaps behind each post-control estimate
//! and the queue of undecided decisions. Every value it prints is one a person
//! entered, a policy cell a person wrote, or a link that exists in the records.
//! It writes nothing and originates no verdict.
//!
//! The record types belong to `spec-objects-safety`. The documents are read
//! with the quire engine's walker and parser through `quoin-quire`, and a
//! record is checked against the `body_extraction` its type's module declares.

#![forbid(unsafe_code)]

mod discrepancies;
mod error;
mod evidence;
mod finding;
mod links;
mod policy;
mod queue;
mod records;
mod render;

use std::path::Path;

use quoin_quire::ModuleSelection;
use quoin_quire::records::read;

use serde::Serialize;

pub use discrepancies::{Discrepancies, Discrepancy, DiscrepancyToken, Refs};
pub use error::{AdviceError, ErrorCode};
pub use evidence::{AnomalyRef, ControlEvidence, EvidenceEntry};
pub use finding::{Finding, Token};
pub use policy::{CellKey, PolicyCell};
pub use queue::{NoRecord, Queue, QueueEntry};

/// The statement of where regions and decisions come from (FR-145).
pub const ATTRIBUTION: &str =
    "Regions come from the project's policy. Decisions come from the named person who made them.";

/// The whole report; its fields are the JSON keys, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Advice {
    /// Where regions and decisions come from.
    pub attribution: &'static str,
    /// One entry per risk estimate (FR-146).
    pub policy_cells: Vec<PolicyCell>,
    /// One entry per post-control estimate (FR-147).
    pub evidence_gaps: Vec<EvidenceEntry>,
    /// The undecided decisions (FR-148).
    pub decision_queue: Queue,
    /// The files the reader could not load, each as `path: reason`. They are
    /// absent from the report, so the caller says so; they are not part of
    /// the rendered output.
    #[serde(skip)]
    pub unreadable: Vec<String>,
}

/// Read the records below `scope`, with the record types of the modules
/// `modules` selects, and build the report.
///
/// # Errors
///
/// Returns an error when `scope` or the modules cannot be read, or when a
/// residual-risk, benefit-risk, overall-evaluation or policy record breaks the
/// contract its type declares.
pub fn advise(scope: &Path, modules: &ModuleSelection) -> Result<Advice, AdviceError> {
    let (records, unreadable) = load(scope, modules, &records::CHECKED, false)?;
    let links = links::Links::build(&records);
    Ok(Advice {
        attribution: ATTRIBUTION,
        policy_cells: policy::cells(&records, &links),
        evidence_gaps: evidence::gaps(&records, &links),
        decision_queue: queue::build(&records, &links),
        unreadable,
    })
}

/// The records below `scope` with the types in `checked` contract-checked, and
/// the files the reader could not load. With `refuse_unreadable`, a file that
/// cannot be read or whose frontmatter does not parse is an error naming it,
/// because the report is a gate that must not pass over a record it never read.
fn load(
    scope: &Path,
    modules: &ModuleSelection,
    checked: &[&str],
    refuse_unreadable: bool,
) -> Result<(records::Records, Vec<String>), AdviceError> {
    let corpus = read(scope, modules, checked).map_err(|error| AdviceError::read(&error))?;
    let unreadable: Vec<String> = corpus
        .unreadable
        .iter()
        .map(|file| format!("{}: {}", file.path.display(), file.reason))
        .collect();
    if refuse_unreadable {
        let malformed = corpus
            .malformed
            .iter()
            .map(|path| format!("{}: frontmatter does not parse", path.display()));
        let all: Vec<String> = unreadable.iter().cloned().chain(malformed).collect();
        if !all.is_empty() {
            return Err(AdviceError::unparseable(&all));
        }
    }
    let records = records::Records::read(&corpus.documents)?;
    Ok((records, unreadable))
}

/// Read the records below `scope` and report where a recorded residual-risk
/// decision disagrees with the region its own policy lists (FR-154, FR-155).
///
/// # Errors
///
/// Returns an error as [`advise`] does.
pub fn discrepancies(
    scope: &Path,
    modules: &ModuleSelection,
) -> Result<Discrepancies, AdviceError> {
    let (records, _) = load(scope, modules, &records::DISCREPANCY_CHECKED, true)?;
    let links = links::Links::build(&records);
    Ok(Discrepancies {
        attribution: ATTRIBUTION,
        findings: discrepancies::build(&records, &links),
    })
}

impl Discrepancies {
    /// The report as Markdown.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        discrepancies::markdown(self)
    }

    /// The report as one pretty-printed JSON object.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization fails.
    pub fn to_json(&self) -> Result<String, AdviceError> {
        serde_json::to_string_pretty(self).map_err(|error| AdviceError::render(&error))
    }
}

impl Advice {
    /// The report as Markdown.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        render::markdown(self)
    }

    /// The report as one pretty-printed JSON object.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization fails.
    pub fn to_json(&self) -> Result<String, AdviceError> {
        serde_json::to_string_pretty(self).map_err(|error| AdviceError::render(&error))
    }
}
