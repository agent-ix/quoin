// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One tool output format per module, transcribed into store records.
//!
//! Ported from `src/evidence/adapters/`. The contract is deliberately narrow,
//! and the narrowness IS the contract (ADR-0011 invariant 1 — quoin
//! transcribes; the consumer's CI executes):
//!
//! - **Pure.** [`RunAdapter::parse`] receives the raw text and returns entries.
//!   It reads no file beyond that text, spawns no process, and reaches no
//!   network. The Rust signature enforces what the TypeScript could only state:
//!   a `fn(&str) -> Result<_, EvidenceError>` has no other input to reach for.
//! - **No verdict.** An adapter reports what the tool said. Deciding whether a
//!   result is acceptable belongs to the auditor and the consumer's gate
//!   policy, never here.

mod agent_eval;
mod audit_script;
mod cargo_mutants;
mod contract_conformance;
mod differential_report;
mod entries;
mod junit;
mod registry;
mod sarif;
mod sbom;

pub use agent_eval::parse_agent_eval;
pub use audit_script::parse_audit_script;
pub use cargo_mutants::parse_cargo_mutants;
pub use contract_conformance::{PROTOCOL, parse_contract_conformance};
pub use differential_report::parse_differential_report;
pub use entries::parse_entries;
pub use junit::{parse_junit, qualified_name};
pub use registry::{
    ADAPTERS, AdapterSelection, FINDING_ADAPTERS, FindingAdapter, RunAdapter, adapter_names,
    select_adapter, select_finding_adapter,
};
pub use sarif::{parse_cargo_audit, parse_sarif};
pub use sbom::parse_sbom;

use crate::types::RunEntry;

/// A result the producer reported that quoin's run-entry vocabulary cannot
/// carry.
///
/// [`crate::types::Outcome`] is `pass | fail | skip | error`. A producer that
/// reports "the external tool does not support this case" is saying none of
/// those: it is not a skip (nothing chose to omit it) and not an error (nothing
/// failed). Mapping it onto the nearest word would delete the distinction at
/// the point of intake, permanently and silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrepresentedResult {
    /// The producer's own identity for the result.
    pub symbol: String,
    /// The producer's own state name, verbatim.
    pub state: String,
    /// Why no run-entry outcome carries it.
    pub reason: String,
}

/// What a run-shaped adapter produces.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdapterResult {
    /// The transcribed results.
    pub entries: Vec<RunEntry>,
    /// Results the producer reported that no run-entry outcome represents.
    ///
    /// `None` when every result was transcribable. **Never `Some(vec![])`**: an
    /// adapter with nothing to report omits it rather than asserting an empty
    /// list, so `Some` always means the adapter looked and found some.
    pub unrepresented: Option<Vec<UnrepresentedResult>>,
    /// The evidence kind this format proves, when the FORMAT ITSELF determines
    /// it — and usually it does not.
    ///
    /// Every adapter shipped here leaves this unset rather than mint a fourth
    /// copy of a vocabulary that already exists in three places
    /// (agent-ix/quoin#114). The field exists for an external adapter for a
    /// single-purpose tool that legitimately knows.
    pub evidence_kind: Option<String>,
}

impl AdapterResult {
    /// A result carrying entries and nothing else — what nine of the ten
    /// shipped adapters return.
    #[must_use]
    pub(crate) fn from_entries(entries: Vec<RunEntry>) -> Self {
        Self {
            entries,
            unrepresented: None,
            evidence_kind: None,
        }
    }
}

/// What a finding-shaped adapter produces.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FindingResult {
    /// The transcribed findings. Empty is meaningful: a scan that ran and found
    /// nothing is the whole reason this record type exists.
    pub findings: Vec<crate::types::Finding>,
    /// The scanner that produced them, where the document names it.
    pub tool: Option<String>,
    /// The ruleset, where the document names it.
    pub ruleset: Option<String>,
    /// How many rules the scanner reported evaluating.
    ///
    /// `None` and `Some(0)` are different facts and FR-034 turns on the
    /// difference: the first is a question that cannot be asked, the second is
    /// a vacuous scan.
    pub rules_evaluated: Option<u64>,
}
