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
//! The record types belong to `spec-objects-safety`. Until the quire-rs record
//! extractor serves them, `records` reads the parts of their frontmatter and
//! determination tables that the report needs.

#![forbid(unsafe_code)]

mod corpus;
mod error;
mod evidence;
mod finding;
mod links;
mod policy;
mod queue;
mod records;
mod render;

use std::path::Path;

use serde::Serialize;

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
}

/// Read the records below `scope` and build the report.
///
/// # Errors
///
/// Returns an error when `scope` or a file below it cannot be read.
pub fn advise(scope: &Path) -> Result<Advice, AdviceError> {
    let records = records::Records::read(&corpus::load(scope)?);
    let links = links::Links::build(&records);
    Ok(Advice {
        attribution: ATTRIBUTION,
        policy_cells: policy::cells(&records, &links),
        evidence_gaps: evidence::gaps(&records, &links),
        decision_queue: queue::build(&records, &links),
    })
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
