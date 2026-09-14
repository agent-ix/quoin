// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The baseline ratchet and the per-PR delta.

use std::collections::BTreeSet;

use quoin_finding_types::{AuditReport, Finding};

/// The baseline key for one finding — `<kind>:<obligation>`.
#[must_use]
pub fn finding_key(finding: &Finding) -> String {
    finding.key()
}

/// Findings not present in the baseline — what a ratchet fails on.
///
/// The baseline is a flat set of `<kind>:<obligation>` keys covering **every**
/// finding kind. It used to be two named buckets, `undischarged` and
/// `suspect`, so `stale-evidence`, `vacuous-evidence`, `method-conformance`,
/// `unknown-method` and `insufficient-multiplicity` could never be baselined
/// and `--ratchet` reported the whole existing backlog for all five — which is
/// the outcome ratchet mode exists to prevent (agent-ix/quoin#105).
///
/// # The one-way property
///
/// Widening the baseline can only shrink the result, and it never invents a
/// finding: every returned finding is one of `report`'s, in `report`'s order.
/// `tests/tc_383_ratchet.rs` states that over generated reports and baselines
/// rather than over examples, because a ratchet that is one-way for the cases
/// somebody thought of is not a ratchet.
#[must_use]
pub fn ratchet<'a>(report: &'a AuditReport, accepted: &[String]) -> Vec<&'a Finding> {
    let accepted: BTreeSet<&str> = accepted.iter().map(String::as_str).collect();
    // Ratchet mode exists because a gate that fails on the whole existing
    // backlog gets disabled within a week. Only NEW violations fail.
    report
        .findings
        .iter()
        .filter(|finding| !accepted.contains(finding.key().as_str()))
        .collect()
}

/// What changed between two audits — the per-PR delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta<'a> {
    /// Findings in `after` whose key is not in `before`.
    pub added: Vec<&'a Finding>,
    /// Findings in `before` whose key is not in `after`.
    pub resolved: Vec<&'a Finding>,
}

/// The delta between two reports, keyed on `<kind>:<obligation>`.
#[must_use]
pub fn delta<'a>(before: &'a AuditReport, after: &'a AuditReport) -> Delta<'a> {
    let before_keys: BTreeSet<String> = before.findings.iter().map(Finding::key).collect();
    let after_keys: BTreeSet<String> = after.findings.iter().map(Finding::key).collect();
    Delta {
        added: after
            .findings
            .iter()
            .filter(|finding| !before_keys.contains(&finding.key()))
            .collect(),
        resolved: before
            .findings
            .iter()
            .filter(|finding| !after_keys.contains(&finding.key()))
            .collect(),
    }
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
    use quoin_finding_types::{AuditReport, Finding, FindingKind, Severity};

    use super::{delta, finding_key, ratchet};

    fn report(kinds: &[(&str, &str)]) -> AuditReport {
        AuditReport {
            findings: kinds
                .iter()
                .map(|(kind, obligation)| Finding::new(*kind, *obligation, Severity::medium(), "s"))
                .collect(),
            healthy: Vec::new(),
            unevaluated: Vec::new(),

            other: quoin_finding_types::OtherMembers::new(),
        }
    }

    #[test]
    fn the_key_is_kind_then_obligation() {
        let finding = Finding::new(
            FindingKind::UNDISCHARGED,
            "FR-1-AC-1",
            Severity::medium(),
            "s",
        );
        assert_eq!(finding_key(&finding), "undischarged:FR-1-AC-1");
    }

    #[test]
    fn an_accepted_key_is_the_only_thing_a_baseline_removes() {
        let report = report(&[("undischarged", "A"), ("suspect-link", "B")]);
        let remaining = ratchet(&report, &["undischarged:A".to_owned()]);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].obligation, "B");
        assert_eq!(
            ratchet(&report, &[]).len(),
            2,
            "an empty baseline accepts nothing"
        );
    }

    #[test]
    fn a_baseline_naming_a_kind_on_another_obligation_removes_nothing() {
        let report = report(&[("undischarged", "A")]);
        assert_eq!(ratchet(&report, &["undischarged:Z".to_owned()]).len(), 1);
    }

    #[test]
    fn the_delta_is_keyed_and_symmetric() {
        let before = report(&[("undischarged", "A"), ("suspect-link", "B")]);
        let after = report(&[("suspect-link", "B"), ("stale-evidence", "C")]);
        let changed = delta(&before, &after);
        assert_eq!(changed.added.len(), 1);
        assert_eq!(changed.added[0].kind, "stale-evidence");
        assert_eq!(changed.resolved.len(), 1);
        assert_eq!(changed.resolved[0].kind, "undischarged");
    }
}
