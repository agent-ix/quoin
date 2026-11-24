// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The finding tokens, all of them the safety module's own.

use serde::Serialize;

/// A finding token. Those of `spec-objects-safety` FR-102, FR-203, FR-204,
/// FR-301, FR-302, FR-303 and FR-107 keep its spelling; `cell-not-in-policy`,
/// `policy-not-found` and `policy-ambiguous` are quoin FR-146's own, and the two decision-attribution tokens are FR-214's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Token {
    /// A level is not in the policy's lists.
    LevelNotInPolicy,
    /// A level is an epistemic marker.
    LevelNotAssessed,
    /// The policy lists no region for a pair of levels it declares.
    CellNotInPolicy,
    /// No residual risk gives the estimate a policy.
    PolicyNotFound,
    /// Residual risks give the estimate more than one policy.
    PolicyAmbiguous,
    /// The post-control estimate assumes no control.
    PostControlWithoutControl,
    /// No requirement satisfies the control.
    ControlWithoutRequirement,
    /// Nothing verifies the control.
    ControlWithoutVerification,
    /// The estimate has no situation.
    EstimateWithoutSituation,
    /// The situation arises from no hazard.
    SituationWithoutHazard,
    /// A post-control estimate that no residual risk references.
    PostControlEstimateWithoutDecision,
    /// A residual risk in scope that an overall evaluation does not consider
    /// (`spec-objects-safety` FR-107).
    ResidualRiskNotConsidered,
    /// An overall evaluation decided over a considered residual risk that is
    /// undecided or not decided as the evaluation (`spec-objects-safety`
    /// FR-107).
    EvaluationAcceptedWithOpenRisk,
    /// The typed decider differs from the author of the commit that set the
    /// decision (quoin FR-214).
    DecidedByDiffersFromCommitAuthor,
    /// The commit that set the decision carries no valid signature (quoin
    /// FR-214).
    DecisionCommitNotSigned,
}

/// A token and the ids or values it names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Finding {
    /// The token.
    pub finding: Token,
    /// What the finding names; empty when the token names nothing.
    pub names: Vec<String>,
}

impl Finding {
    pub(crate) fn bare(finding: Token) -> Self {
        Self {
            finding,
            names: Vec::new(),
        }
    }

    pub(crate) fn naming(finding: Token, names: Vec<String>) -> Self {
        Self { finding, names }
    }
}
