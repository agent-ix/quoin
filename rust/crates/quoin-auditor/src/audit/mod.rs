// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence auditor (FR-032).
//!
//! A trace link is currently a string match that never expires. Evidence rots
//! in three ways, none detected before this:
//!
//!  1. **Suspect links** — the statement changed after the evidence was bound.
//!  2. **Stale evidence** — bound to an old commit, a failed run, or a run
//!     that never happened.
//!  3. **Vacuous evidence** — a tagged test that is skipped, xfail, or
//!     assertion-free.
//!
//! **The auditor runs nothing.** It reads the store and reports (ADR-0011
//! invariant 1); the consumer's CI refreshes. That separation is what lets the
//! report be trusted: an auditor that could re-run a suite could also make a
//! finding disappear by re-running it. [`crate::inert`] is where that
//! invariant stops being a comment and becomes something rustc checks.

pub mod input;
pub mod method;
pub mod mocks;
pub mod ratchet;
pub mod scores;

mod ladder;

use std::collections::BTreeMap;

use quoin_combinatorial::js;
use quoin_evidence::types::{
    Binding, FindingRecord, IndependenceAssessment, IndependenceStatus, MockInjection, RunRecord,
};
use quoin_finding_types::{AuditReport, Finding, UnevaluatedCheck};

pub use input::AuditInput;
pub use method::{catalog_methods_matching, method_conformance, unknown_method_finding};
pub use mocks::{MOCK_SUBJECT_FLOOR, mocked_bindings, same_test_symbol, words};
pub use ratchet::{Delta, delta, finding_key, ratchet};
pub use scores::{multiplicity_finding, mutation_finding, scores_for};

use crate::error::AuditorError;

/// What the loop reads, indexed once rather than per obligation.
struct Indexed<'a> {
    bindings_by_obligation: BTreeMap<&'a str, Vec<&'a Binding>>,
    runs_by_suite: BTreeMap<&'a str, &'a RunRecord>,
    scans_by_suite: BTreeMap<&'a str, &'a FindingRecord>,
    injections: &'a [MockInjection],
    inspected_suites: Vec<String>,
    assessed_requirements: BTreeMap<&'a str, &'a IndependenceAssessment>,
    vacuous_suites: Vec<String>,
    independence_by_obligation: BTreeMap<&'a str, &'a str>,
}

impl<'a> Indexed<'a> {
    fn of(input: &'a AuditInput) -> Self {
        // An obligation can be discharged by more than one suite — a unit suite
        // and a mutation suite, say — so the graph is grouped, not indexed.
        // Keying on the obligation alone was what let the second suite's
        // binding overwrite the first (agent-ix/quoin#102).
        let mut bindings_by_obligation: BTreeMap<&str, Vec<&Binding>> = BTreeMap::new();
        for binding in &input.bindings {
            bindings_by_obligation
                .entry(binding.obligation.as_str())
                .or_default()
                .push(binding);
        }
        // `new Map(list.map(…))`: a later entry for the same suite replaces an
        // earlier one, which `BTreeMap::insert` reproduces.
        let mut runs_by_suite = BTreeMap::new();
        for run in &input.runs {
            runs_by_suite.insert(run.suite.as_str(), run);
        }
        let mut scans_by_suite = BTreeMap::new();
        for scan in input.scans.iter().flatten() {
            scans_by_suite.insert(scan.suite.as_str(), scan);
        }
        // Absent means "nobody looked", never "nothing was mocked". The check
        // is reported as unevaluated rather than granting a clean bill (#204).
        let injections: &[MockInjection] = input.injections.as_deref().unwrap_or_default();
        let inspected_suites = input.mock_inspection_suites.clone().unwrap_or_else(|| {
            injections
                .iter()
                .map(|injection| injection.suite.as_str().to_owned())
                .collect()
        });
        let mut assessed_requirements = BTreeMap::new();
        for assessment in input.independence.iter().flatten() {
            assessed_requirements.insert(assessment.requirement.as_str(), assessment);
        }
        let mut independence_by_obligation = BTreeMap::new();
        for requirement in input
            .independence_policy
            .iter()
            .flat_map(|policy| &policy.requirements)
        {
            independence_by_obligation
                .insert(requirement.obligation.as_str(), requirement.id.as_str());
        }
        Self {
            bindings_by_obligation,
            runs_by_suite,
            scans_by_suite,
            injections,
            inspected_suites,
            assessed_requirements,
            // A closed list, so an unanswered suite is not a vacuous one. An
            // absent list means nobody asked; membership means the scan
            // evaluated no rules.
            vacuous_suites: input.vacuous_scan_suites.clone().unwrap_or_default(),
            independence_by_obligation,
        }
    }
}

/// What one obligation's pass produced, before the ladder decides.
struct Pass {
    findings: Vec<Finding>,
    unevaluated: Vec<UnevaluatedCheck>,
    independence: Vec<IndependenceAssessment>,
    /// True only when the ladder ran all the way to the bottom.
    ///
    /// The retained source pushes `healthy` as the LAST statement of the loop
    /// body, so every `continue` skips it — including the one silent exit,
    /// `runBindings.length === 0` at `src/auditor/audit.ts:413`. An obligation
    /// whose every binding is scan-backed therefore has no finding and is
    /// **not** healthy either, and a port that inferred "healthy" from "no
    /// finding" would report a stronger result than the retained code does.
    completed: bool,
}

/// Audit the store against the obligations of the day.
///
/// Every check is deterministic and reads only what it was handed. There is no
/// clock, no filesystem walk and no subprocess in here — the caller assembles
/// the inputs, which is what makes the whole thing testable without a
/// repository.
///
/// # Errors
///
/// [`AuditorErrorCode::DemandTooLarge`](crate::AuditorErrorCode::DemandTooLarge)
/// when an obligation declares a configuration space demanding more tuples
/// than [`quoin_combinatorial::MAX_DEMANDED_TUPLES`]. The retained code throws
/// at the same point; refusing is what keeps a mistyped statement from
/// allocating the machine.
pub fn audit(input: &AuditInput) -> Result<AuditReport, AuditorError> {
    let indexed = Indexed::of(input);

    let mut obligations: Vec<&quoin_quire_types::Obligation> = input.obligations.iter().collect();
    obligations.sort_by(|left, right| js::compare(&left.id, &right.id));

    let mut findings: Vec<Finding> = Vec::new();
    let mut healthy: Vec<String> = Vec::new();
    let mut unevaluated: Vec<UnevaluatedCheck> = Vec::new();
    let mut independence: Vec<IndependenceAssessment> = Vec::new();

    for obligation in obligations {
        let mut bindings: Vec<&Binding> = indexed
            .bindings_by_obligation
            .get(obligation.id.as_str())
            .cloned()
            .unwrap_or_default();
        bindings.sort_by(|left, right| js::compare(left.suite.as_str(), right.suite.as_str()));

        let pass = ladder::run(input, &indexed, obligation, &bindings)?;
        // Healthy means NOTHING was found for this obligation — including the
        // pre-guard unknown-method check, which does not stop the ladder.
        let clean = pass.completed && pass.findings.is_empty() && pass.unevaluated.is_empty();
        findings.extend(pass.findings);
        unevaluated.extend(pass.unevaluated);
        independence.extend(pass.independence);
        if clean {
            healthy.push(obligation.id.clone());
        }
    }

    // Plain comparison, not `localeCompare`: the report is meant to be
    // reproducible, and `localeCompare` without an explicit locale depends on
    // the runtime's ICU data (agent-ix/quoin#106).
    findings.sort_by(|left, right| {
        js::compare(&left.obligation, &right.obligation)
            .then_with(|| js::compare(&left.kind, &right.kind))
    });
    unevaluated.sort_by(|left, right| {
        js::compare(&left.obligation, &right.obligation)
            .then_with(|| js::compare(&left.check, &right.check))
    });
    healthy.sort_by(|left, right| js::compare(left, right));

    let independence = input.independence_policy.as_ref().map(|policy| {
        // Every requirement the policy states appears in the report, including
        // the ones no obligation loop reached — a requirement whose obligation
        // is not derived today still has an answer, and dropping it would
        // report a narrower policy than the one that was supplied.
        let reported: Vec<String> = independence
            .iter()
            .map(|assessment| assessment.requirement.clone())
            .collect();
        let mut independence = independence.clone();
        for requirement in &policy.requirements {
            if reported.contains(&requirement.id) {
                continue;
            }
            if let Some(missed) = indexed.assessed_requirements.get(requirement.id.as_str()) {
                independence.push((*missed).clone());
            }
        }
        independence.sort_by(|left, right| {
            js::compare(left.obligation.as_str(), right.obligation.as_str())
        });
        independence
    });

    // `independence` is written into the report's passthrough map rather than
    // a declared field: `quoin-graph-analysis`' captured corpus holds an
    // `independence` member that is not an assessment at all, so declaring the
    // type there would make a shared reader refuse input the retained
    // implementation accepted. See `quoin_finding_types::AuditReport::other`.
    // The assessments are typed on this side of the boundary, which is the
    // side that knows what it wrote.
    let mut other = quoin_finding_types::OtherMembers::new();
    if let Some(independence) = independence {
        other.insert(
            "independence".to_owned(),
            serde_json::to_value(independence)
                .map_err(|error| AuditorError::report_not_serialisable(&error))?,
        );
    }

    Ok(AuditReport {
        findings,
        healthy,
        unevaluated,
        other,
    })
}

/// `assessment.status === "insufficient"`.
fn is_insufficient(assessment: &IndependenceAssessment) -> bool {
    assessment.status == IndependenceStatus::Insufficient
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use quoin_evidence::types::IndependenceAssessment;

    /// The reachability question behind
    /// [`AuditorErrorCode::ReportNotSerialisable`](crate::AuditorErrorCode::ReportNotSerialisable),
    /// held open rather than assumed: every member populated, `satisfiedBy`
    /// included, and the value still writes and reads back unchanged.
    ///
    /// Trace: FR-032-AC-2
    /// Provenance: quoin#383
    #[test]
    fn the_independence_the_report_carries_is_serialisable() {
        let source = serde_json::json!({
            "profile": "AP-1",
            "requirement": "IND-1",
            "obligation": "FR-001-AC-1",
            "status": "satisfied",
            "dimensions": [
                {"dimension": "actor", "values": ["a", "b"], "missingSuites": ["unit"]},
                {"dimension": "review-path", "values": [], "missingSuites": []}
            ],
            "satisfiedBy": ["unit", "integration"],
            "summary": "two lines differ on every requested dimension."
        });
        let assessment: IndependenceAssessment =
            serde_json::from_value(source.clone()).expect("the store's own shape reads");
        // Anti-vacuity: an assessment with nothing in it would serialise too.
        assert!(assessment.satisfied_by.is_some());
        assert_eq!(assessment.dimensions.len(), 2);
        assert_eq!(
            serde_json::to_value(vec![assessment]).expect("no input reaches the error path"),
            serde_json::Value::Array(vec![source])
        );
    }
}
