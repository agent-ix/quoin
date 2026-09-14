// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Multiplicity and mutation score — the two checks keyed on criticality.

use std::collections::BTreeSet;

use quoin_evidence::types::{Binding, Outcome, RunRecord};
use quoin_finding_types::{Finding, FindingKind, Severity};
use quoin_quire_types::Obligation;
use quoin_store::json::number::format_number;

use super::input::AuditInput;

/// Fault-detection scores among the runs bound to this obligation.
///
/// **Filtered on the entry's own `metric`, never on the tool name.**
/// `RunEntry::score` is deliberately generic — its own contract says "a
/// mutation score, a coverage percentage, a measured latency" — so reading
/// every scored entry would compare a p95 latency in milliseconds against a
/// floor of `0.8` and report the obligation as failing. The first draft did
/// exactly that; the first fix scoped by tool name was a tool allowlist by
/// another name. The adapter now names what it measured at the point of
/// recording (agent-ix/quoin#138), and an entry without a `metric` is not a
/// mutation score — no fallback read of the tool string.
///
/// Public for the advisor (agent-ix/quoin#158), which needs the same answer to
/// decide whether anything has measured that the tests discriminate. One
/// definition, so the auditor's finding and the advisor's recommendation
/// cannot disagree about what a score is.
#[must_use]
pub fn scores_for(bindings: &[&Binding], runs: &[&RunRecord]) -> Vec<f64> {
    let bound: BTreeSet<&str> = bindings
        .iter()
        .map(|binding| binding.suite.as_str())
        .collect();
    let mut scores = Vec::new();
    for run in runs {
        if !bound.contains(run.suite.as_str()) {
            continue;
        }
        for entry in &run.entries {
            if entry.metric.as_deref() != Some(quoin_evidence::types::MUTATION_SCORE_METRIC) {
                continue;
            }
            // `skip` carries no measurement: a skipped symbol's absent score is
            // not a zero, and treating it as one would fail an obligation for a
            // test nobody ran rather than for a test that failed to
            // discriminate.
            if let Some(score) = entry.score
                && entry.outcome != Outcome::Skip
            {
                scores.push(score);
            }
        }
    }
    scores
}

/// `Math.min(...values)` over a list the caller has already found non-empty.
///
/// `Math.min` propagates `NaN`, and every comparison against a `NaN` is false,
/// so a `NaN` score leaves the floor unmet-but-unreported rather than minting
/// a finding from a number nobody recorded.
fn math_min(values: &[f64]) -> f64 {
    values.iter().fold(f64::INFINITY, |smallest, &value| {
        if value.is_nan() || smallest.is_nan() {
            f64::NAN
        } else if value < smallest {
            value
        } else {
            smallest
        }
    })
}

/// Criticality can demand two independent methods.
///
/// "Independent" means two different suites: two symbols in one suite share a
/// harness, a fixture set and a failure mode, so counting them as two methods
/// would let one broken assumption look like corroboration.
#[must_use]
pub fn multiplicity_finding(
    obligation: &Obligation,
    bindings: &[&Binding],
    input: &AuditInput,
) -> Option<Finding> {
    let demanding = input.multiplicity_requires.as_deref().unwrap_or_default();
    if demanding.is_empty() {
        return None;
    }
    let criticality = obligation
        .criticality
        .as_deref()
        .filter(|value| !value.is_empty())?;
    if !demanding.iter().any(|value| value == criticality) {
        return None;
    }

    // A real measurement. While `bind()` keyed on the obligation alone this set
    // could never hold more than one suite, so the finding fired on every
    // demanding obligation and no amount of evidence could clear it (#102).
    //
    // `new Set(...)` preserves insertion order, and the summary joins it: the
    // suites are listed in binding order, which the caller has already sorted.
    let mut suites: Vec<&str> = Vec::new();
    for binding in bindings {
        if !suites.contains(&binding.suite.as_str()) {
            suites.push(binding.suite.as_str());
        }
    }
    if suites.len() >= 2 {
        return None;
    }

    Some(Finding::new(
        FindingKind::INSUFFICIENT_MULTIPLICITY,
        &obligation.id,
        Severity::medium(),
        format!(
            "{id} is criticality {criticality}, which requires two independent methods, but \
             all its evidence comes from {suites}.",
            id = obligation.id,
            suites = suites.join(", ")
        ),
    ))
}

/// Criticality can demand that the tests have been shown to detect faults.
///
/// **Why this check exists at all.** Every other quality signal in this
/// program is a proxy — does the criterion use a vague verb, does it name a
/// concrete object. Those are word lists over an open vocabulary, and the
/// corpus already showed what that is worth: 1,201 distinct verb stems, of
/// which a built-in list of 13 covered 14.5%.
///
/// A mutation score is the direct answer to the question those proxies
/// approximate — *does the test discriminate the behaviour the criterion
/// describes?*
///
/// **quoin does not run the mutation tool** (ADR-0011: the consumer's CI
/// does). This reads a score somebody else recorded, which is why the absence
/// of one is reported rather than filled in.
#[must_use]
pub fn mutation_finding(
    obligation: &Obligation,
    bindings: &[&Binding],
    runs: &[&RunRecord],
    input: &AuditInput,
) -> Option<Finding> {
    let criticality = obligation
        .criticality
        .as_deref()
        .filter(|value| !value.is_empty())?;
    let floor = *input.mutation_floor.as_ref()?.get(criticality)?;

    let scores = scores_for(bindings, runs);
    if scores.is_empty() {
        // Distinct from `undischarged`: this obligation may be thoroughly
        // tested and still have nothing saying the tests detect anything. A
        // demanded threshold that cannot be evaluated is not a threshold met.
        return Some(Finding::new(
            FindingKind::UNMEASURED_MUTATION_SCORE,
            &obligation.id,
            Severity::medium(),
            format!(
                "{id} is criticality {criticality}, which demands a mutation score of at \
                 least {floor}, and no run bound to it records one.",
                id = obligation.id,
                floor = format_number(floor)
            ),
        ));
    }

    // The WORST score, not the mean. Averaging lets a well-tested symbol carry
    // a symbol whose mutants all survive, which is the case the threshold
    // exists to find.
    let worst = math_min(&scores);
    if worst >= floor {
        return None;
    }
    Some(Finding::new(
        FindingKind::INSUFFICIENT_MUTATION_SCORE,
        &obligation.id,
        Severity::medium(),
        format!(
            "{id} is criticality {criticality}, which demands a mutation score of at least \
             {floor}; its weakest bound symbol scores {worst}.",
            id = obligation.id,
            floor = format_number(floor),
            worst = format_number(worst)
        ),
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::math_min;

    #[test]
    fn the_minimum_propagates_a_nan_rather_than_ignoring_it() {
        assert!(math_min(&[0.9, f64::NAN]).is_nan());
        assert!((math_min(&[0.9, 0.4, 1.0]) - 0.4).abs() < f64::EPSILON);
    }
}
