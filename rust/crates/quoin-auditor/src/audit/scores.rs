// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Multiplicity and mutation score — the two checks keyed on criticality.

use std::collections::BTreeSet;

use crate::jsvalue::number_text;
use quoin_evidence::types::{Binding, Outcome, RunEntry, RunRecord};
use quoin_finding_types::{Finding, FindingKind, Severity};
use quoin_quire_types::Obligation;

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
    class_floor: Option<(&str, f64)>,
) -> Option<Finding> {
    let Floor {
        ratio: floor,
        source,
    } = floor_for(obligation, input, class_floor)?;

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
                "{id} is {source}, which demands a mutation score of at \
                 least {floor}, and no run bound to it records one.",
                id = obligation.id,
                floor = number_text(floor)
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
            "{id} is {source}, which demands a mutation score of at least \
             {floor}; its weakest bound symbol scores {worst}.",
            id = obligation.id,
            floor = number_text(floor),
            worst = number_text(worst)
        ),
    ))
}

/// Who set a mutation ratio: the obligation's criticality or its software
/// safety class (FR-160). Displays as the sentence fragment a finding uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorSource<'a> {
    /// The obligation's FR-053 criticality value.
    Criticality(&'a str),
    /// The software safety class its requirement states.
    Class(&'a str),
}

impl std::fmt::Display for FloorSource<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Criticality(value) => write!(formatter, "criticality {value}"),
            Self::Class(value) => write!(formatter, "class {value}"),
        }
    }
}

/// The mutation ratio an obligation must reach, and what set it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floor<'a> {
    /// The ratio, in `[0, 1]`.
    pub ratio: f64,
    /// Who set it.
    pub source: FloorSource<'a>,
}

/// The mutation floor an obligation's criticality or class demands, if either
/// declares one.
///
/// The two axes are separate and each selects its own floor. When both apply
/// the higher ratio is evaluated and the finding names the source that set it;
/// a tie goes to criticality, which is the floor that was there first.
#[must_use]
pub fn floor_for<'a>(
    obligation: &'a Obligation,
    input: &'a AuditInput,
    class_floor: Option<(&'a str, f64)>,
) -> Option<Floor<'a>> {
    let by_criticality = obligation
        .criticality
        .as_deref()
        .filter(|value| !value.is_empty())
        .and_then(|criticality| {
            let ratio = *input.mutation_floor.as_ref()?.get(criticality)?;
            Some(Floor {
                ratio,
                source: FloorSource::Criticality(criticality),
            })
        });
    let by_class = class_floor.map(|(class, ratio)| Floor {
        ratio,
        source: FloorSource::Class(class),
    });
    match (by_criticality, by_class) {
        (Some(criticality), Some(class)) if class.ratio > criticality.ratio => Some(class),
        (Some(criticality), _) => Some(criticality),
        (None, class) => class,
    }
}

/// Most `surviving-mutant` findings one obligation reports.
const MAX_SURVIVOR_FINDINGS: usize = 25;

/// One `surviving-mutant` finding per recorded survivor of each mutation-scored
/// symbol, in a suite bound to this obligation, whose score is below `floor`.
///
/// **Attribution is by suite, not by test.** A survivor here sits in code
/// exercised by a suite bound to the obligation; the evidence does not say which
/// test in that suite covers the obligation's behaviour. The finding carries the
/// locus (`path`, `line`, `symbol`) and the mutation so `spec-correctness` can
/// aim a regenerated property at it. Symbols that meet the floor report nothing.
/// Failing symbols come first, worst score first, so the cap cuts the least
/// informative survivors. Read from the same entries as [`scores_for`], on the
/// same `metric`; a skipped entry counts for nothing.
#[must_use]
pub fn surviving_mutant_findings(
    obligation: &Obligation,
    bindings: &[&Binding],
    runs: &[&RunRecord],
    floor: f64,
) -> Vec<Finding> {
    let bound: BTreeSet<&str> = bindings
        .iter()
        .map(|binding| binding.suite.as_str())
        .collect();
    let mut failing: Vec<(f64, &RunEntry)> = Vec::new();
    for run in runs.iter().filter(|run| bound.contains(run.suite.as_str())) {
        for entry in &run.entries {
            if entry.metric.as_deref() != Some(quoin_evidence::types::MUTATION_SCORE_METRIC)
                || entry.outcome == Outcome::Skip
            {
                continue;
            }
            if let Some(score) = entry.score
                && score < floor
            {
                failing.push((score, entry));
            }
        }
    }
    failing.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.symbol.as_str().cmp(right.1.symbol.as_str()))
    });

    let mut found = Vec::new();
    for (score, entry) in failing {
        let symbol = entry.symbol.as_str();
        let path = symbol.split_once("::").map_or(symbol, |(file, _)| file);
        for survivor in entry.survivors.as_deref().unwrap_or_default() {
            if found.len() >= MAX_SURVIVOR_FINDINGS {
                return found;
            }
            let mut finding = Finding::new(
                FindingKind::SURVIVING_MUTANT,
                &obligation.id,
                Severity::medium(),
                format!(
                    "`{mutation}` in {symbol} (score {score}, floor {floor}) survived the tests \
                     of a suite bound to {id}.",
                    mutation = survivor.mutation,
                    score = number_text(score),
                    floor = number_text(floor),
                    id = obligation.id
                ),
            );
            finding.path = Some(path.to_owned());
            finding.line = survivor.line;
            finding.symbol = Some(symbol.to_owned());
            finding.remedy = Some(format!(
                "Check whether {id}'s tests exercise this behaviour; strengthen them or the \
                 criterion's wording so this mutation fails.",
                id = obligation.id
            ));
            found.push(finding);
        }
    }
    found
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
