// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The policy cell of each risk estimate (FR-146).

use serde::Serialize;

use crate::finding::{Finding, Token};
use crate::links::Links;
use crate::records::{Policy, Records};

/// The levels an estimate was entered with and the policy cell they name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CellKey {
    /// The severity level.
    pub severity: String,
    /// The probability level.
    pub probability: String,
}

/// One estimate and the region its policy lists for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyCell {
    /// The estimate id.
    pub estimate: String,
    /// `pre_control` or `post_control`.
    pub phase: &'static str,
    /// The severity as entered.
    pub severity: String,
    /// The probability as entered.
    pub probability: String,
    /// The policy id, or `null`.
    pub policy: Option<String>,
    /// The matched cell, or `null`.
    pub cell: Option<CellKey>,
    /// The policy's region name, or `null`.
    pub region: Option<String>,
    /// The region's flag as the policy wrote it, or `null`.
    pub outside_acceptable: Option<bool>,
    /// Why there is no region.
    pub findings: Vec<Finding>,
}

/// What a policy lists for a pair of levels, or why it lists nothing.
pub(crate) struct Resolution {
    pub(crate) region: Option<(String, bool)>,
    pub(crate) findings: Vec<Finding>,
}

const MARKERS: [&str; 3] = ["unknown", "not_assessed", "not_applicable"];

fn level_finding(level: &str, members: &[String], policy: &str) -> Option<Finding> {
    if MARKERS.contains(&level) {
        Some(Finding::naming(
            Token::LevelNotAssessed,
            vec![level.to_owned()],
        ))
    } else if members.iter().any(|member| member == level) {
        None
    } else {
        Some(Finding::naming(
            Token::LevelNotInPolicy,
            vec![level.to_owned(), policy.to_owned()],
        ))
    }
}

/// The policy's own region for the pair, by lookup only.
pub(crate) fn resolve(policy: &Policy, severity: &str, probability: &str) -> Resolution {
    let findings: Vec<Finding> = [
        level_finding(severity, &policy.severities, &policy.id),
        level_finding(probability, &policy.probabilities, &policy.id),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !findings.is_empty() {
        return Resolution {
            region: None,
            findings,
        };
    }
    let region = policy
        .cells
        .get(&(severity.to_owned(), probability.to_owned()))
        .and_then(|name| Some((name.clone(), *policy.regions.get(name)?)));
    let findings = if region.is_none() {
        vec![Finding::naming(
            Token::CellNotInPolicy,
            vec![
                severity.to_owned(),
                probability.to_owned(),
                policy.id.clone(),
            ],
        )]
    } else {
        Vec::new()
    };
    Resolution { region, findings }
}

/// What the policies a record names list for a pair of levels.
pub(crate) struct Lookup {
    pub(crate) policy: Option<String>,
    pub(crate) region: Option<(String, bool)>,
    pub(crate) findings: Vec<Finding>,
}

/// The one rule that turns the policies a record names into a region: none
/// gives `policy-not-found`, several give `policy-ambiguous` naming them all,
/// and exactly one is looked up as FR-102 does.
pub(crate) fn lookup(
    records: &Records,
    policies: &[String],
    severity: &str,
    probability: &str,
) -> Lookup {
    match policies {
        [] => Lookup {
            policy: None,
            region: None,
            findings: vec![Finding::bare(Token::PolicyNotFound)],
        },
        [id] => match records.policies.get(id) {
            None => Lookup {
                policy: Some(id.clone()),
                region: None,
                findings: vec![Finding::naming(Token::PolicyNotFound, vec![id.clone()])],
            },
            Some(policy) => {
                let found = resolve(policy, severity, probability);
                Lookup {
                    policy: Some(id.clone()),
                    region: found.region,
                    findings: found.findings,
                }
            }
        },
        many => Lookup {
            policy: None,
            region: None,
            findings: vec![Finding::naming(Token::PolicyAmbiguous, many.to_vec())],
        },
    }
}

pub(crate) fn cells(records: &Records, links: &Links) -> Vec<PolicyCell> {
    records
        .estimates
        .values()
        .map(|estimate| {
            let policies = links.policies_of(records, estimate);
            let found = lookup(
                records,
                &policies,
                &estimate.severity,
                &estimate.probability,
            );
            let cell = found.region.as_ref().map(|_| CellKey {
                severity: estimate.severity.clone(),
                probability: estimate.probability.clone(),
            });
            PolicyCell {
                estimate: estimate.id.clone(),
                phase: estimate.phase.as_str(),
                severity: estimate.severity.clone(),
                probability: estimate.probability.clone(),
                policy: found.policy,
                cell,
                outside_acceptable: found.region.as_ref().map(|(_, outside)| *outside),
                region: found.region.map(|(region, _)| region),
                findings: found.findings,
            }
        })
        .collect()
}
