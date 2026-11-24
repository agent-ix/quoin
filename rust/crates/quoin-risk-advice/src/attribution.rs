// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Attribution of each recorded decision to the commit that last set it
//! (FR-214).
//!
//! The typed `Decided by` is text anyone can write. The commit that set the
//! decision, its author and its signature state are what git recorded. The
//! signature state is the reduction FR-136 states; trust is not judged here.

use serde::Serialize;

use quoin_anomaly::{Derived, Signature, value_change};

use crate::error::AdviceError;
use crate::finding::{Finding, Token};
use crate::records::{Choice, Records};

const ACCEPTED: &str = "accepted";
const NOT_ACCEPTED: &str = "not_accepted";

/// One decided record and what git recorded about its decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Attribution {
    /// The record id.
    pub record: String,
    /// `residual_risk`, `benefit_risk` or `overall_residual_risk_evaluation`.
    pub kind: &'static str,
    /// The decision word as written.
    pub decision: String,
    /// The `Decided by` cell as written, or `null`.
    pub decided_by: Option<String>,
    /// The commit that last set the decision.
    pub decision_change: Derived,
    /// The mismatch findings.
    pub findings: Vec<Finding>,
}

/// The `Decision` cell of the first data row of the `Determination` table in a
/// document's text, trimmed.
pub(crate) fn decision_cell(text: &str) -> Option<String> {
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim() != "## Determination")
        .skip(1)
        .take_while(|line| !line.starts_with('#'))
        .skip_while(|line| !line.trim_start().starts_with('|'));
    let header = lines.next()?;
    lines.next()?;
    let row = lines.next()?;
    let column = cells(header).iter().position(|name| name == "Decision")?;
    cells(row).into_iter().nth(column)
}

fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

fn entry(record: &Choice, kind: &'static str) -> Result<Attribution, AdviceError> {
    let decision_change = value_change(&record.path, &record.decision, decision_cell)
        .map_err(|error| AdviceError::git(&error))?;
    let mut findings = Vec::new();
    if let Derived::Present(change) = &decision_change {
        if let Some(decider) = &record.decided_by
            && !decider.trim().eq_ignore_ascii_case(change.author.trim())
        {
            findings.push(Finding::naming(
                Token::DecidedByDiffersFromCommitAuthor,
                vec![decider.clone(), change.author.clone()],
            ));
        }
        if change.signature != Signature::Signed {
            findings.push(Finding::naming(
                Token::DecisionCommitNotSigned,
                vec![change.commit.clone()],
            ));
        }
    }
    Ok(Attribution {
        record: record.id.clone(),
        kind,
        decision: record.decision.clone(),
        decided_by: record.decided_by.clone(),
        decision_change,
        findings,
    })
}

pub(crate) fn build(records: &Records) -> Result<Vec<Attribution>, AdviceError> {
    let groups = [
        (&records.residual_risks, "residual_risk"),
        (&records.benefit_risks, "benefit_risk"),
        (&records.evaluations, "overall_residual_risk_evaluation"),
    ];
    let mut out = Vec::new();
    for (map, kind) in groups {
        for record in map.values() {
            if record.decision == ACCEPTED || record.decision == NOT_ACCEPTED {
                out.push(entry(record, kind)?);
            }
        }
    }
    Ok(out)
}
