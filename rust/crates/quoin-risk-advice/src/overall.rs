// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The overall-evaluation checks of `spec-objects-safety` FR-107 (FR-213).
//!
//! A finding states that two things people wrote disagree. The report never
//! derives an evaluation, creates one, or states a product accepted.

use serde::Serialize;

use crate::finding::{Finding, Token};
use crate::records::{Choice, Records};

const ACCEPTED: &str = "accepted";
const UNDECIDED: &str = "undecided";
const NOT_ACCEPTED: &str = "not_accepted";

/// One overall evaluation and the findings that name it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OverallCheck {
    /// The evaluation id.
    pub evaluation: String,
    /// Its `Product` cell as written, or `null`.
    pub product: Option<String>,
    /// Its decision word as written.
    pub decision: String,
    /// The findings naming it.
    pub findings: Vec<Finding>,
}

fn check(records: &Records, evaluation: &Choice) -> OverallCheck {
    let considered = |id: &String| evaluation.references.contains(id);
    let not_considered: Vec<String> = records
        .residual_risks
        .keys()
        .filter(|id| !considered(id))
        .cloned()
        .collect();
    let open: Vec<String> = if evaluation.decision == ACCEPTED {
        records
            .residual_risks
            .values()
            .filter(|risk| considered(&risk.id))
            .filter(|risk| risk.decision == UNDECIDED || risk.decision == NOT_ACCEPTED)
            .map(|risk| risk.id.clone())
            .collect()
    } else {
        Vec::new()
    };
    let mut findings = Vec::new();
    if !not_considered.is_empty() {
        findings.push(Finding::naming(
            Token::ResidualRiskNotConsidered,
            not_considered,
        ));
    }
    if !open.is_empty() {
        findings.push(Finding::naming(Token::EvaluationAcceptedWithOpenRisk, open));
    }
    OverallCheck {
        evaluation: evaluation.id.clone(),
        product: evaluation.subject.clone(),
        decision: evaluation.decision.clone(),
        findings,
    }
}

pub(crate) fn build(records: &Records) -> Vec<OverallCheck> {
    records
        .evaluations
        .values()
        .map(|evaluation| check(records, evaluation))
        .collect()
}
