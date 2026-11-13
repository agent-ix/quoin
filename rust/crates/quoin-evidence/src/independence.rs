// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Profile-selected evidence lineage and independence policy (FR-094).
//!
//! The question is always "do two evidence lines exist that differ on *every*
//! axis this profile selected". A dimension absent on either side cannot
//! demonstrate separation, so two suites sharing one model, parser or corpus
//! stay visibly common-mode rather than passing by omission.

use std::collections::BTreeSet;

use ix_trace_rs::implements;

use crate::error::EvidenceError;
use crate::ids::{ObligationId, ProfileId, SuiteId};
use crate::types::{
    Binding, EvidenceLineage, IndependenceAssessment, IndependenceDimension,
    IndependenceDimensionAssessment, IndependencePolicy, IndependenceRequirement,
    IndependenceStatus,
};

/// Check one policy against the FR-094 boundary.
///
/// # Errors
///
/// [`EvidenceError::InvalidRecord`] carrying every clause that failed, joined
/// with `; `.
#[implements("FR-094")]
pub fn validate_independence_policy(policy: &IndependencePolicy) -> Result<(), EvidenceError> {
    let mut clauses: Vec<String> = Vec::new();
    if policy.schema_version != 1 {
        clauses.push("schemaVersion: must be 1".to_owned());
    }
    if policy.requirements.is_empty() {
        clauses.push("requirements: must not be empty".to_owned());
    }
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    let mut obligations: BTreeSet<&str> = BTreeSet::new();
    for (index, requirement) in policy.requirements.iter().enumerate() {
        if requirement.id.trim().is_empty() {
            clauses.push(format!("requirements.{index}.id: must not be empty"));
        }
        if requirement.obligation.as_str().trim().is_empty() {
            clauses.push(format!(
                "requirements.{index}.obligation: must not be empty"
            ));
        }
        if requirement.rationale.trim().is_empty() {
            clauses.push(format!("requirements.{index}.rationale: must not be empty"));
        }
        if requirement.dimensions.is_empty() {
            clauses.push(format!(
                "requirements.{index}.dimensions: must not be empty"
            ));
        }
        let unique: BTreeSet<IndependenceDimension> =
            requirement.dimensions.iter().copied().collect();
        if unique.len() != requirement.dimensions.len() {
            clauses.push(format!(
                "requirements.{index}.dimensions: contains duplicate dimensions"
            ));
        }
        if !ids.insert(requirement.id.as_str()) {
            clauses.push(format!(
                "requirements.{index}.id: duplicates another requirement id"
            ));
        }
        if !obligations.insert(requirement.obligation.as_str()) {
            clauses.push(format!(
                "requirements.{index}.obligation: duplicates another obligation requirement"
            ));
        }
    }
    if clauses.is_empty() {
        Ok(())
    } else {
        Err(EvidenceError::invalid(
            "independence policy",
            clauses.join("; "),
        ))
    }
}

/// Check one lineage against the FR-094 boundary.
///
/// # Errors
///
/// [`EvidenceError::InvalidRecord`] when the lineage names nothing at all: an
/// empty object claims to have recorded separation facts while recording none.
#[implements("FR-094")]
pub fn validate_evidence_lineage(lineage: &EvidenceLineage) -> Result<(), EvidenceError> {
    if lineage.is_empty() {
        return Err(EvidenceError::invalid(
            "evidence lineage",
            "record: must name at least one lineage dimension",
        ));
    }
    Ok(())
}

/// Refuse a stale or typoed projection rather than silently evaluating nothing.
///
/// A policy whose obligation was renamed would otherwise report every
/// requirement as vacuously assessed, which is the opposite of what an
/// independence check is for.
///
/// # Errors
///
/// [`EvidenceError::PolicyUnknownObligations`], naming the unknown ids sorted.
pub fn require_known_policy_obligations<'a, I>(
    policy: &IndependencePolicy,
    known: I,
) -> Result<(), EvidenceError>
where
    I: IntoIterator<Item = &'a str>,
{
    let known: BTreeSet<&str> = known.into_iter().collect();
    let unknown: BTreeSet<&str> = policy
        .requirements
        .iter()
        .map(|requirement| requirement.obligation.as_str())
        .filter(|obligation| !known.contains(obligation))
        .collect();
    if unknown.is_empty() {
        return Ok(());
    }
    Err(EvidenceError::PolicyUnknownObligations {
        obligations: unknown.into_iter().collect::<Vec<_>>().join(", "),
    })
}

/// What the FR-094 pairwise rule found over one set of bindings.
///
/// The profile assessment and the class assessment (FR-160) are the same
/// comparison under different headers, so the comparison is made once here and
/// each caller wraps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinesAssessment {
    /// Whether a pair differing on every dimension exists.
    pub status: IndependenceStatus,
    /// One entry per requested dimension, in the requested order.
    pub dimensions: Vec<IndependenceDimensionAssessment>,
    /// The two suites that satisfied it, when one pair did.
    pub satisfied_by: Option<(SuiteId, SuiteId)>,
    /// A sentence naming what was found or what was missing.
    pub summary: String,
}

/// Find two distinct evidence relationships separated on every selected axis.
///
/// The bindings are ordered by suite first, so the pair reported for a given
/// binding set does not depend on the order the caller assembled them in.
#[must_use]
pub fn assess_independence(
    profile: &ProfileId,
    requirement: &IndependenceRequirement,
    bindings: &[Binding],
) -> IndependenceAssessment {
    let borrowed: Vec<&Binding> = bindings.iter().collect();
    let lines = assess_lines(&requirement.id, &requirement.dimensions, &borrowed);
    IndependenceAssessment {
        profile: profile.clone(),
        requirement: requirement.id.clone(),
        obligation: requirement.obligation.clone(),
        status: lines.status,
        dimensions: lines.dimensions,
        satisfied_by: lines.satisfied_by,
        summary: lines.summary,
    }
}

/// The FR-094 pairwise comparison over `bindings`, named by `label` in the
/// summary.
#[must_use]
pub fn assess_lines(
    label: &str,
    requested: &[IndependenceDimension],
    bindings: &[&Binding],
) -> LinesAssessment {
    let mut ordered = bindings.to_vec();
    ordered.sort_by(|a, b| a.suite.as_str().cmp(b.suite.as_str()));

    let dimensions: Vec<IndependenceDimensionAssessment> = requested
        .iter()
        .map(|dimension| {
            let mut values: BTreeSet<String> = BTreeSet::new();
            let mut missing = Vec::new();
            for binding in &ordered {
                match value_for(binding.lineage.as_ref(), *dimension) {
                    Some(value) => {
                        values.insert(value);
                    }
                    None => missing.push(binding.suite.clone()),
                }
            }
            IndependenceDimensionAssessment {
                dimension: *dimension,
                values: values.into_iter().collect(),
                missing_suites: missing,
            }
        })
        .collect();

    let selected = requested
        .iter()
        .map(|dimension| dimension.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    for (left, a) in ordered.iter().enumerate() {
        for b in ordered.iter().skip(left + 1) {
            if a.suite == b.suite {
                continue;
            }
            let separated = requested.iter().all(|dimension| {
                match (
                    value_for(a.lineage.as_ref(), *dimension),
                    value_for(b.lineage.as_ref(), *dimension),
                ) {
                    (Some(first), Some(second)) => first != second,
                    _ => false,
                }
            });
            if !separated {
                continue;
            }
            return LinesAssessment {
                status: IndependenceStatus::Satisfied,
                dimensions,
                satisfied_by: Some((a.suite.clone(), b.suite.clone())),
                summary: format!(
                    "{label} is satisfied by {} and {}; they differ on {selected}.",
                    a.suite, b.suite
                ),
            };
        }
    }

    let detail = dimensions
        .iter()
        .map(|item| {
            let missing = if item.missing_suites.is_empty() {
                String::new()
            } else {
                format!(
                    "; missing on {}",
                    item.missing_suites
                        .iter()
                        .map(SuiteId::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            format!(
                "{}: {} distinct{missing}",
                item.dimension,
                item.values.len()
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    LinesAssessment {
        status: IndependenceStatus::Insufficient,
        dimensions,
        satisfied_by: None,
        summary: format!(
            "{label} requires two evidence lines differing on every selected dimension ({selected}); {detail}."
        ),
    }
}

/// The recorded value for one dimension, trimmed, with blank treated as absent.
///
/// A whitespace-only lineage entry is not a separation fact, and treating it as
/// one would let two suites "differ" on a pair of empty strings.
fn value_for(
    lineage: Option<&EvidenceLineage>,
    dimension: IndependenceDimension,
) -> Option<String> {
    let lineage = lineage?;
    let raw = match dimension {
        IndependenceDimension::Actor => lineage.actor.as_deref(),
        IndependenceDimension::ImplementationToolchain => {
            lineage.implementation_toolchain.as_deref()
        }
        IndependenceDimension::Technique => lineage.technique.as_deref(),
        IndependenceDimension::DataSource => lineage.data_source.as_deref(),
        IndependenceDimension::ReviewPath => lineage.review_path.as_deref(),
    }?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// The obligation ids a policy names, in policy order.
#[must_use]
pub fn policy_obligations(policy: &IndependencePolicy) -> Vec<ObligationId> {
    policy
        .requirements
        .iter()
        .map(|requirement| requirement.obligation.clone())
        .collect()
}
