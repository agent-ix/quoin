// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One obligation's pass down the check ladder.
//!
//! The ladder is **one finding per obligation**, in a fixed order, with two
//! exceptions the retained source makes deliberately: the pre-guard
//! `unknown-method` check and the head-commit staleness check do not stop it.
//! An unbound obligation with an uncatalogued method is BOTH undischarged AND
//! unknown-method, and hiding either behind the other cost a reader the
//! ability to see both facts (agent-ix/quoin#165).
//!
//! Each rung is its own function returning `Option<Finding>`, so the order is
//! readable in one screen of [`run`] and no rung can silently fall through
//! into the next one's data.

use std::collections::BTreeSet;

use quoin_combinatorial::js;
use quoin_combinatorial::{
    Configuration, DimensionName, DimensionValue, parse_space, tway_coverage,
};
use quoin_evidence::types::{Binding, Outcome, RunRecord};
use quoin_finding_types::{Finding, FindingKind, Severity, UnevaluatedCheck};
use quoin_quire_types::Obligation;

use super::input::AuditInput;
use super::method::{method_conformance, unknown_method_finding};
use super::mocks::{mocked_bindings, mocked_finding};
use super::scores::{multiplicity_finding, mutation_finding};
use super::{Indexed, Pass, is_insufficient};
use crate::error::AuditorError;

/// One binding and the run that records its suite.
type Paired<'a> = (&'a Binding, &'a RunRecord);

/// `String.prototype.slice(0, n)` — the first `n` UTF-16 code units.
///
/// Every value this is applied to is a hex digest, so it never splits a
/// surrogate pair in practice; it counts code units anyway, because "in
/// practice" is how a port acquires a divergence nobody declared.
fn slice_utf16(text: &str, units: usize) -> &str {
    let mut taken = 0usize;
    for (at, character) in text.char_indices() {
        if taken >= units {
            return text.get(..at).unwrap_or(text);
        }
        taken += character.len_utf16();
    }
    text
}

/// `list.join(", ")` over suite names.
fn join_suites(bindings: &[&&Binding]) -> String {
    bindings
        .iter()
        .map(|binding| binding.suite.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Run one obligation down the ladder.
pub(super) fn run(
    input: &AuditInput,
    indexed: &Indexed<'_>,
    obligation: &Obligation,
    bindings: &[&Binding],
) -> Result<Pass, AuditorError> {
    let mut pass = Pass {
        findings: Vec::new(),
        unevaluated: Vec::new(),
        independence: Vec::new(),
        completed: false,
    };
    let catalog = input.catalog.as_ref();

    // ── 0. Unknown method ──
    // A pure statement-vs-catalog comparison: it needs no bindings, no runs and
    // no evidence store, so it is asked BEFORE the binding guard. Behind it,
    // the one check that pays off on day one of adoption could never fire on an
    // unadopted repository, whose every obligation is unbound.
    if let Some(unknown) = unknown_method_finding(obligation, catalog) {
        pass.findings.push(unknown);
    }

    if bindings.is_empty() {
        pass.findings.push(undischarged(obligation));
        return Ok(pass);
    }
    if let Some(finding) = suspect_link(obligation, bindings) {
        pass.findings.push(finding);
        return Ok(pass);
    }
    if let Some(finding) = unrecorded_evidence(indexed, obligation, bindings) {
        pass.findings.push(finding);
        return Ok(pass);
    }

    // ── Mocked confirmation (#204) ──
    if let Some(check) = uninspected_suites(indexed, obligation, bindings) {
        pass.unevaluated.push(check);
    }
    let mocked = mocked_bindings(obligation, bindings, indexed.injections);
    // Reported only when EVERY binding is mocked. One suite standing in a
    // dependency while another exercises the real path is ordinary test design.
    if !mocked.is_empty()
        && mocked.len() == bindings.len()
        && let Some(finding) = mocked_finding(obligation, &mocked)
    {
        pass.findings.push(finding);
        return Ok(pass);
    }

    if let Some(finding) = vacuous_scan(indexed, obligation, bindings) {
        pass.findings.push(finding);
        return Ok(pass);
    }

    // ── Profile-selected independence ──
    // A property of the obligation→evidence relationships, not a guessed
    // property of roles, method names, or tool vendors. Asked only for exact
    // obligations the profile projected into the policy.
    let assessment = indexed
        .independence_by_obligation
        .get(obligation.id.as_str())
        .and_then(|id| indexed.assessed_requirements.get(*id).copied())
        .filter(|_| input.independence_policy.is_some());
    if let Some(assessment) = assessment {
        pass.independence.push(assessment.clone());
        if is_insufficient(assessment) {
            pass.findings.push(Finding::new(
                FindingKind::INSUFFICIENT_INDEPENDENCE,
                &obligation.id,
                Severity::medium(),
                assessment.summary.clone(),
            ));
            return Ok(pass);
        }
    }

    // Every remaining check reasons over run entries, so a binding backed only
    // by a scan has nothing more to answer here.
    let paired: Vec<Paired<'_>> = bindings
        .iter()
        .filter_map(|binding| {
            indexed
                .runs_by_suite
                .get(binding.suite.as_str())
                .map(|run| (*binding, *run))
        })
        .collect();
    if paired.is_empty() {
        return Ok(pass);
    }

    // Does NOT stop the ladder: an older run is a fact about freshness, and the
    // checks below are facts about content.
    if let Some(head) = input.head_commit.as_deref().filter(|head| !head.is_empty())
        && let Some(finding) = behind_head(obligation, &paired, head)
    {
        pass.findings.push(finding);
    }
    if let Some(finding) = vacuous_run(obligation, &paired) {
        pass.findings.push(finding);
        return Ok(pass);
    }
    if let Some(finding) = combinatorial_gap(obligation, &paired)? {
        pass.findings.push(finding);
        return Ok(pass);
    }
    if let Some(finding) = method_conformance(obligation, &paired, catalog) {
        pass.findings.push(finding);
        return Ok(pass);
    }
    if let Some(finding) = multiplicity_finding(obligation, bindings, input) {
        pass.findings.push(finding);
        return Ok(pass);
    }
    let run_bindings: Vec<&Binding> = paired.iter().map(|(binding, _)| *binding).collect();
    let runs: Vec<&RunRecord> = paired.iter().map(|(_, run)| *run).collect();
    if let Some(finding) = mutation_finding(obligation, &run_bindings, &runs, input) {
        pass.findings.push(finding);
        return Ok(pass);
    }
    pass.completed = true;
    Ok(pass)
}

/// Nothing is bound to this obligation.
fn undischarged(obligation: &Obligation) -> Finding {
    Finding::new(
        FindingKind::UNDISCHARGED,
        &obligation.id,
        // Medium, not high: an unwritten test is ordinary work in progress.
        // What is high is evidence that *claims* to exist and does not hold.
        Severity::medium(),
        format!("no evidence is bound to {}", obligation.id),
    )
}

/// ── 1. Suspect link ──
///
/// Reported when ANY binding predates the current statement. A sibling suite
/// that re-bound after the reword does not absolve the one that did not: that
/// binding still claims to discharge a statement it never saw.
fn suspect_link(obligation: &Obligation, bindings: &[&Binding]) -> Option<Finding> {
    let suspect: Vec<&&Binding> = bindings
        .iter()
        .filter(|binding| binding.statement_hash_at_binding.as_str() != obligation.statement_hash)
        .collect();
    let first = suspect.first()?;
    Some(Finding::new(
        FindingKind::SUSPECT_LINK,
        &obligation.id,
        // The highest-value single finding in the traceability design: the
        // requirement moved and the evidence did not follow, while the matrix
        // still reads as covered.
        Severity::high(),
        format!(
            "{id} was reworded after its evidence was bound in {suites} (bound against \
             {was}\u{2026}, now {now}\u{2026}). Re-affirm or re-verify.",
            id = obligation.id,
            suites = join_suites(&suspect),
            was = slice_utf16(first.statement_hash_at_binding.as_str(), 12),
            now = slice_utf16(&obligation.statement_hash, 12)
        ),
    ))
}

/// ── 2. Stale evidence ──
///
/// A suite that recorded a SCAN has evidence; it simply is not run-shaped.
fn unrecorded_evidence(
    indexed: &Indexed<'_>,
    obligation: &Obligation,
    bindings: &[&Binding],
) -> Option<Finding> {
    let unrecorded: Vec<&&Binding> = bindings
        .iter()
        .filter(|binding| {
            !indexed.runs_by_suite.contains_key(binding.suite.as_str())
                && !indexed.scans_by_suite.contains_key(binding.suite.as_str())
        })
        .collect();
    if unrecorded.is_empty() {
        return None;
    }
    Some(Finding::new(
        FindingKind::STALE_EVIDENCE,
        &obligation.id,
        Severity::high(),
        format!(
            "{id} is bound to {suites}, which {verb} no recorded run. The binding claims \
             evidence that is not in the store.",
            id = obligation.id,
            suites = join_suites(&unrecorded),
            verb = if unrecorded.len() == 1 { "has" } else { "have" }
        ),
    ))
}

/// Run-backed suites with no current mock inspection.
///
/// An absent inspection means "nobody looked", never "nothing was mocked", so
/// the result is recorded under `unevaluated` rather than granting a clean
/// bill (#204).
fn uninspected_suites(
    indexed: &Indexed<'_>,
    obligation: &Obligation,
    bindings: &[&Binding],
) -> Option<UnevaluatedCheck> {
    let mut uninspected: Vec<&str> = bindings
        .iter()
        // Finding-shaped scans do not execute test source and therefore cannot
        // inject the behavior under verification.
        .filter(|binding| indexed.runs_by_suite.contains_key(binding.suite.as_str()))
        .map(|binding| binding.suite.as_str())
        .filter(|suite| !indexed.inspected_suites.iter().any(|seen| seen == suite))
        .collect::<BTreeSet<&str>>()
        .into_iter()
        .collect();
    if uninspected.is_empty() {
        return None;
    }
    uninspected.sort_by(|left, right| js::compare(left, right));
    Some(UnevaluatedCheck {
        check: UnevaluatedCheck::MOCKED_CONFIRMATION.to_owned(),
        obligation: obligation.id.clone(),
        suites: uninspected
            .iter()
            .map(|suite| (*suite).to_owned())
            .collect(),
        reason: format!(
            "no current mock inspection exists for {suites}. Run quoin evidence \
             inspect-mocks for each suite at HEAD.",
            suites = uninspected.join(", ")
        ),
        // The auditor writes exactly the four members it declares; the map is
        // `quoin-finding-types`' passthrough for a member added upstream.
        other: quoin_finding_types::OtherMembers::new(),
    })
}

/// ── Finding-shaped vacuity ──
///
/// A scan that ran every rule and reported nothing is a CLEAN RESULT, and is
/// precisely the evidence FR-034 exists to preserve. What proves nothing is a
/// scan that evaluated NO RULES: it reports zero findings too, and from the
/// findings list alone the two are identical.
fn vacuous_scan(
    indexed: &Indexed<'_>,
    obligation: &Obligation,
    bindings: &[&Binding],
) -> Option<Finding> {
    let mut named: Vec<String> = bindings
        .iter()
        .filter_map(|binding| indexed.scans_by_suite.get(binding.suite.as_str()))
        .filter(|scan| {
            indexed
                .vacuous_suites
                .iter()
                .any(|suite| suite == scan.suite.as_str())
        })
        .map(|scan| format!("{suite} ({tool})", suite = scan.suite, tool = scan.tool))
        .collect();
    if named.is_empty() {
        return None;
    }
    named.sort_by(|left, right| js::compare(left, right));
    Some(Finding::new(
        FindingKind::VACUOUS_EVIDENCE,
        &obligation.id,
        Severity::high(),
        format!(
            "{id} rests on a scan that evaluated no rules: {list}. It reported no finding \
             because it looked for nothing.",
            id = obligation.id,
            list = named.join(", ")
        ),
    ))
}

/// Runs recorded at a commit other than the one being audited.
///
/// # Divergence
///
/// The retained code filters `bindings` while indexing a `runs` array built
/// from `runBindings`. Whenever any binding is scan-backed the filter walks
/// past `runs.length`, `runs[i]` is `undefined`, and the retained code throws
/// a `TypeError`. This pairs each binding with its own run, which is exactly
/// what the adjacent vacuity block's own comment says must be done — the same
/// defect was found and fixed there and not here. `DIVERGENCE.md` §1.
fn behind_head(obligation: &Obligation, paired: &[Paired<'_>], head: &str) -> Option<Finding> {
    let behind: Vec<&Paired<'_>> = paired
        .iter()
        .filter(|(_, run)| run.commit.as_str() != head)
        .collect();
    if behind.is_empty() {
        return None;
    }
    Some(Finding::new(
        FindingKind::STALE_EVIDENCE,
        &obligation.id,
        // Medium: an older run is normal between releases. It becomes
        // actionable at a gate, which is the consuming workflow's policy.
        Severity::medium(),
        format!(
            "{id} rests on {runs}, not at HEAD ({head}).",
            id = obligation.id,
            runs = behind
                .iter()
                .map(|(binding, run)| format!(
                    "a {suite} run at {commit}",
                    suite = binding.suite,
                    commit = slice_utf16(run.commit.as_str(), 12)
                ))
                .collect::<Vec<_>>()
                .join(" and "),
            head = slice_utf16(head, 12)
        ),
    ))
}

/// ── 3. Vacuous evidence ──
///
/// Vacuous only when EVERY symbol in EVERY suite was skipped or absent. One
/// suite that genuinely ran is evidence; reporting the obligation as vacuous
/// because a second suite skipped would be the false alarm that gets the check
/// switched off.
fn vacuous_run(obligation: &Obligation, paired: &[Paired<'_>]) -> Option<Finding> {
    let mut vacuous: Vec<String> = Vec::new();
    let mut bound_symbols = 0usize;
    for (binding, run) in paired {
        bound_symbols += binding.symbols.len();
        for symbol in &binding.symbols {
            let entry = run
                .entries
                .iter()
                .find(|entry| entry.symbol.as_str() == symbol.as_str());
            // A symbol the run never reported is vacuous in the strongest
            // sense: the binding names evidence the suite did not produce.
            if entry.is_none_or(|entry| entry.outcome == Outcome::Skip) {
                vacuous.push(format!("{suite}:{symbol}", suite = binding.suite));
            }
        }
    }
    if vacuous.is_empty() || vacuous.len() != bound_symbols {
        return None;
    }
    vacuous.sort_by(|left, right| js::compare(left, right));
    Some(Finding::new(
        FindingKind::VACUOUS_EVIDENCE,
        &obligation.id,
        Severity::high(),
        format!(
            "every symbol bound to {id} was skipped or absent from its run: {list}. The row \
             reads as covered and nothing was verified.",
            id = obligation.id,
            list = vacuous.join(", ")
        ),
    ))
}

/// ── Combinatorial coverage ──
///
/// Only for an obligation whose statement declares a configuration space
/// (quire-rs FR-061). [`parse_space`] returning [`None`] IS the test for that,
/// so there is no second flag to keep in agreement with the first.
fn combinatorial_gap(
    obligation: &Obligation,
    paired: &[Paired<'_>],
) -> Result<Option<Finding>, AuditorError> {
    let Some(space) = parse_space(&obligation.statement) else {
        return Ok(None);
    };
    let configs: Vec<Configuration> = paired
        .iter()
        .flat_map(|(_, run)| run.entries.iter())
        .filter_map(|entry| entry.config.as_ref())
        .map(|config| {
            config
                .iter()
                .map(|(name, value)| {
                    (
                        DimensionName::new(name.clone()),
                        DimensionValue::new(value.clone()),
                    )
                })
                .collect()
        })
        .collect();
    let coverage = tway_coverage(&space, &configs)?;
    if coverage.covered >= coverage.demanded {
        return Ok(None);
    }
    let more = coverage.gaps.len().saturating_sub(10);
    Ok(Some(Finding::new(
        FindingKind::COMBINATORIAL_GAP,
        &obligation.id,
        // Medium: an incomplete covering array is ordinary work in progress.
        // What would be high is a run claiming completeness it does not have,
        // which is what the gap list makes impossible to do quietly.
        Severity::medium(),
        format!(
            "{id} demands {demanded} {strength}-way combinations and its runs reached \
             {covered}. Never run: {gaps}{tail}",
            id = obligation.id,
            demanded = coverage.demanded,
            strength = coverage.strength,
            covered = coverage.covered,
            // Named, not counted. A percentage says how much is missing; the
            // list says which combinations to run, which is the difference
            // between a number and an action.
            gaps = coverage
                .gaps
                .iter()
                .take(10)
                .map(quoin_combinatorial::TupleKey::as_str)
                .collect::<Vec<_>>()
                .join("; "),
            tail = if more > 0 {
                format!(" (and {more} more)")
            } else {
                String::new()
            }
        ),
    )))
}
