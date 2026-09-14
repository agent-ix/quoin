// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The audit report envelope (quoin#383).

use serde::{Deserialize, Serialize};

use crate::OtherMembers;
use crate::finding::Finding;

/// A check that could not be run, separate from both findings and clean results.
///
/// The distinction is the whole point: an absent mock inspection means "nobody
/// looked", never "nothing was mocked", and folding it into either bucket
/// would hand a caller a clean bill it never earned (agent-ix/quoin#204).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnevaluatedCheck {
    /// Which check could not run.
    ///
    /// A `String` for the same reason as [`Finding::kind`]: the retained
    /// declaration is a one-member union, erased before it reaches the wire,
    /// and a second member added upstream must read here rather than fail.
    /// [`UnevaluatedCheck::MOCKED_CONFIRMATION`] is the one it holds today.
    pub check: String,
    /// The obligation the check was about.
    pub obligation: String,
    /// The suites that could not be answered for, sorted.
    pub suites: Vec<String>,
    /// Why, in one sentence, including what to run.
    pub reason: String,
    /// Every member the producer wrote that this type does not declare.
    ///
    /// `src/graph-analysis/input.ts:60` validates this shape with a zod
    /// `.passthrough()` and copies the rest through unchanged (quoin#385).
    #[serde(flatten)]
    pub other: OtherMembers,
}

impl UnevaluatedCheck {
    /// The only check the auditor records as unevaluated today.
    pub const MOCKED_CONFIRMATION: &'static str = "mocked-confirmation";
}

/// The audit result, ordered so the same input yields the same report.
///
/// # Unknown fields are accepted
///
/// Same reader posture as [`Finding`]: this deserialises whatever the auditor
/// wrote, including fields a later version adds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditReport {
    /// Every finding, sorted by obligation then kind.
    pub findings: Vec<Finding>,
    /// Obligations with a binding whose hash still matches, sorted.
    pub healthy: Vec<String>,
    /// Checks that could not run, sorted by obligation then check.
    pub unevaluated: Vec<UnevaluatedCheck>,
    /// Every member the producer wrote that this type does not declare —
    /// `independence` among them.
    ///
    /// `src/graph-analysis/input.ts:66` reads the report as a zod
    /// `.passthrough()` (quoin#385), and quoin#383 tried declaring
    /// `independence: Option<Vec<IndependenceAssessment>>` here because the
    /// producer writes it under `if (input.independencePolicy)`. The workspace
    /// gate refused: `quoin-graph-analysis`' captured corpus holds
    /// `independence: [{"obligation": "...", "axes": ["author"]}]`, which is
    /// not an assessment at all, and the retained reader accepted it because
    /// nothing between the producer and the view validates the member.
    ///
    /// Same ruling as [`crate::Severity`]: where the retained data is wider
    /// than the declared type, the retained data wins. The producer writes its
    /// assessments in here, typed on its own side of the boundary; no reader
    /// in this workspace reads them back.
    #[serde(flatten)]
    pub other: OtherMembers,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{AuditReport, UnevaluatedCheck};
    use crate::OtherMembers;

    #[test]
    fn a_report_without_a_policy_writes_no_independence_key() {
        // `report.independence` is assigned only under `if
        // (input.independencePolicy)`, so an absent key is the retained shape
        // and a `null` would be a byte the store has never held.
        let report = AuditReport {
            findings: Vec::new(),
            healthy: vec!["FR-001-AC-1".to_owned()],
            unevaluated: Vec::new(),
            other: OtherMembers::new(),
        };
        assert_eq!(
            serde_json::to_string(&report).unwrap(),
            r#"{"findings":[],"healthy":["FR-001-AC-1"],"unevaluated":[]}"#
        );
    }

    /// Carried from quoin#385: `independence` is undeclared and survives in
    /// `other`, in the two shapes the workspace actually holds — the
    /// assessment the producer writes, and the `{obligation, axes}` the
    /// graph-analysis corpus holds. A declared field would refuse the second.
    #[test]
    fn a_report_carries_members_it_does_not_declare() {
        let source = r#"{"findings":[],"healthy":["FR-1-AC-1"],"unevaluated":[{"check":"mocked-confirmation","obligation":"FR-1-AC-1","suites":["unit"],"reason":"r","inspectedAt":null}],"independence":[{"obligation":"FR-1-AC-1","axes":["author"]}]}"#;
        let report: AuditReport =
            serde_json::from_str(source).expect("the auditor's own report shape must deserialize");
        assert!(
            report.other.contains_key("independence"),
            "{:?}",
            report.other
        );
        assert_eq!(report.unevaluated[0].other.len(), 1);
        assert_eq!(report.unevaluated[0].check, "mocked-confirmation");
        assert_eq!(
            serde_json::to_value(&report).expect("writes"),
            serde_json::from_str::<serde_json::Value>(source).expect("reads"),
            "a member the auditor wrote must survive the round trip"
        );
    }

    #[test]
    fn an_unevaluated_check_round_trips_its_four_keys() {
        let check = UnevaluatedCheck {
            check: UnevaluatedCheck::MOCKED_CONFIRMATION.to_owned(),
            obligation: "FR-001-AC-1".to_owned(),
            suites: vec!["unit".to_owned()],
            reason: "no current mock inspection exists for unit.".to_owned(),
            other: OtherMembers::new(),
        };
        let text = serde_json::to_string(&check).unwrap();
        assert_eq!(
            serde_json::from_str::<UnevaluatedCheck>(&text).unwrap(),
            check
        );
        assert!(text.contains(r#""check":"mocked-confirmation""#));
    }
}
