// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The queue of decisions nobody has made (FR-148).
//!
//! The order is record kind, then id. The policy assigns its regions no
//! priority, so the region is shown and never orders anything.

use serde::Serialize;

use crate::finding::{Finding, Token};
use crate::links::Links;
use crate::policy::resolve;
use crate::records::{Choice, Phase, Records};

const UNDECIDED: &str = "undecided";

/// One undecided record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueueEntry {
    /// The record id.
    pub record: String,
    /// `residual_risk`, `benefit_risk` or `overall_residual_risk_evaluation`.
    pub kind: &'static str,
    /// The estimate id (residual risk only).
    pub estimate: Option<String>,
    /// The record's own policy reference (residual risk only).
    pub policy: Option<String>,
    /// The estimate's severity as entered (residual risk only).
    pub severity: Option<String>,
    /// The estimate's probability as entered (residual risk only).
    pub probability: Option<String>,
    /// The policy's region name, or `null`.
    pub region: Option<String>,
    /// The region's flag as the policy wrote it, or `null`.
    pub outside_acceptable: Option<bool>,
    /// Covered or considered ids (benefit-risk and evaluation).
    pub references: Vec<String>,
    /// Why there is no region.
    pub findings: Vec<Finding>,
}

/// A post-control estimate that no residual risk references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoRecord {
    /// The estimate id.
    pub estimate: String,
    /// `post-control-estimate-without-decision`.
    pub findings: Vec<Finding>,
}

/// The `decision_queue` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Queue {
    /// The undecided records.
    pub queue: Vec<QueueEntry>,
    /// The estimates with no residual-risk record.
    pub no_record: Vec<NoRecord>,
}

fn plain(record: &Choice, kind: &'static str) -> QueueEntry {
    QueueEntry {
        record: record.id.clone(),
        kind,
        estimate: None,
        policy: None,
        severity: None,
        probability: None,
        region: None,
        outside_acceptable: None,
        references: record.references.clone(),
        findings: Vec::new(),
    }
}

fn residual(records: &Records, record: &Choice) -> QueueEntry {
    let mut entry = plain(record, "residual_risk");
    entry.references = Vec::new();
    entry.estimate.clone_from(&record.estimate);
    entry.policy = records.policy_of(record);
    let estimate = record
        .estimate
        .as_ref()
        .and_then(|id| records.estimates.get(id));
    if let Some(estimate) = estimate {
        entry.severity = Some(estimate.severity.clone());
        entry.probability = Some(estimate.probability.clone());
        let policy = entry
            .policy
            .as_ref()
            .and_then(|id| records.policies.get(id));
        if let Some(policy) = policy {
            let found = resolve(policy, &estimate.severity, &estimate.probability);
            entry.findings = found.findings;
            if let Some((region, outside)) = found.region {
                entry.region = Some(region);
                entry.outside_acceptable = Some(outside);
            }
        }
    }
    entry
}

pub(crate) fn build(records: &Records, links: &Links) -> Queue {
    let mut queue = Vec::new();
    queue.extend(
        records
            .residual_risks
            .values()
            .filter(|record| record.decision == UNDECIDED)
            .map(|record| residual(records, record)),
    );
    queue.extend(
        records
            .benefit_risks
            .values()
            .filter(|record| record.decision == UNDECIDED)
            .map(|record| plain(record, "benefit_risk")),
    );
    queue.extend(
        records
            .evaluations
            .values()
            .filter(|record| record.decision == UNDECIDED)
            .map(|record| plain(record, "overall_residual_risk_evaluation")),
    );
    let no_record = records
        .estimates
        .values()
        .filter(|estimate| estimate.phase == Phase::PostControl)
        .filter(|estimate| links.residual_risks_of(&estimate.id).is_empty())
        .map(|estimate| NoRecord {
            estimate: estimate.id.clone(),
            findings: vec![Finding::bare(Token::PostControlEstimateWithoutDecision)],
        })
        .collect();
    Queue { queue, no_record }
}
