// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The ratchet's one-way property, over generated reports and baselines.
//!
//! Trace: quoin#383-AC-2 — the ratchet's one-way property is a test, not a
//! convention.
//! Provenance: quoin#383
//!
//! A ratchet exists so that a gate fails on NEW findings only. Three things
//! have to hold for that to be true of every input rather than of the examples
//! somebody thought of, which is why these are properties and not cases:
//!
//!  1. **Sound** — every finding it reports is one of the report's own, in the
//!     report's own order. A ratchet that reordered or invented findings would
//!     send a reader to a finding that is not there.
//!  2. **One-way** — widening the baseline can only shrink the result. If
//!     accepting one more key could ever ADD a finding, a team could baseline a
//!     backlog and watch the gate get louder.
//!  3. **Total** — baselining every key leaves nothing, and baselining nothing
//!     leaves everything. Those are the two endpoints the other two properties
//!     are measured between.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;

use proptest::prelude::*;
use quoin_auditor::{delta, finding_key, ratchet};
use quoin_finding_types::{AuditReport, Finding, FindingKind, Severity};

/// A finding drawn from the real kind vocabulary and a small obligation space.
///
/// Small on purpose: keys have to COLLIDE for the baseline to be interesting.
/// Drawn from `FindingKind::ALL` rather than from arbitrary strings for the
/// same reason — a generator whose keys never repeat would exercise the empty
/// baseline a thousand times over.
fn a_finding() -> impl Strategy<Value = Finding> {
    (
        0..FindingKind::ALL.len(),
        "FR-00[1-5]-AC-[1-3]",
        prop_oneof![
            Just(Severity::low()),
            Just(Severity::medium()),
            Just(Severity::high())
        ],
        "[a-z ]{0,24}",
    )
        .prop_map(|(kind, obligation, severity, summary)| {
            Finding::new(FindingKind::ALL[kind], obligation, severity, summary)
        })
}

/// A report of up to twelve findings.
fn a_report() -> impl Strategy<Value = AuditReport> {
    proptest::collection::vec(a_finding(), 0..12).prop_map(|findings| AuditReport {
        findings,
        healthy: Vec::new(),
        unevaluated: Vec::new(),

        other: quoin_finding_types::OtherMembers::new(),
    })
}

/// A baseline: some keys from the report, plus some that match nothing.
fn a_baseline(report: &AuditReport) -> impl Strategy<Value = Vec<String>> + use<> {
    let keys: Vec<String> = report.findings.iter().map(finding_key).collect();
    (
        proptest::sample::subsequence(keys.clone(), 0..=keys.len()),
        proptest::collection::vec("stale-evidence:XX-[0-9]{3}", 0..3),
    )
        .prop_map(|(mut chosen, noise)| {
            chosen.extend(noise);
            chosen
        })
}

proptest! {
    /// Sound: the result is a subsequence of the report's findings.
    #[test]
    fn tc_383_030_the_ratchet_only_ever_reports_findings_the_report_holds(
        (report, accepted) in a_report().prop_flat_map(|report| {
            let baseline = a_baseline(&report);
            (Just(report), baseline)
        })
    ) {
        let remaining = ratchet(&report, &accepted);
        prop_assert!(remaining.len() <= report.findings.len());

        // Subsequence, not "subset": order is part of the answer, because the
        // gate prints this list and a reader follows it top to bottom.
        let mut source = report.findings.iter();
        for finding in &remaining {
            let found = source.by_ref().any(|candidate| std::ptr::eq(candidate, *finding));
            prop_assert!(found, "the ratchet reordered or invented a finding");
        }

        // And nothing it kept was accepted.
        let accepted_set: BTreeSet<&str> = accepted.iter().map(String::as_str).collect();
        for finding in &remaining {
            prop_assert!(!accepted_set.contains(finding_key(finding).as_str()));
        }
    }

    /// One-way: a wider baseline can only shrink the result.
    #[test]
    fn tc_383_031_widening_the_baseline_can_only_shrink_the_result(
        (report, accepted, extra) in a_report().prop_flat_map(|report| {
            let baseline = a_baseline(&report);
            let more = a_baseline(&report);
            (Just(report), baseline, more)
        })
    ) {
        let narrow = ratchet(&report, &accepted);
        let mut wider = accepted.clone();
        wider.extend(extra);
        let wide = ratchet(&report, &wider);

        prop_assert!(
            wide.len() <= narrow.len(),
            "accepting more keys made the gate louder"
        );
        let narrow_keys: Vec<String> = narrow.iter().map(|finding| finding_key(finding)).collect();
        for finding in &wide {
            prop_assert!(
                narrow_keys.contains(&finding_key(finding)),
                "a wider baseline produced a finding the narrower one did not"
            );
        }
    }

    /// Total: the two endpoints.
    #[test]
    fn tc_383_032_the_endpoints_are_everything_and_nothing(report in a_report()) {
        prop_assert_eq!(ratchet(&report, &[]).len(), report.findings.len());

        let every: Vec<String> = report.findings.iter().map(finding_key).collect();
        prop_assert!(ratchet(&report, &every).is_empty());
    }

    /// The delta partitions: nothing is both added and resolved, and a report
    /// against itself changes nothing.
    #[test]
    fn tc_383_033_the_delta_is_a_partition_and_is_empty_against_itself(
        before in a_report(),
        after in a_report(),
    ) {
        let change = delta(&before, &after);
        let added: BTreeSet<String> = change.added.iter().map(|f| finding_key(f)).collect();
        let resolved: BTreeSet<String> = change.resolved.iter().map(|f| finding_key(f)).collect();
        prop_assert!(added.is_disjoint(&resolved));

        let none = delta(&after, &after);
        prop_assert!(none.added.is_empty() && none.resolved.is_empty());
    }
}

/// The generator really does produce colliding keys.
///
/// Without this the four properties above could all hold vacuously on a corpus
/// of reports whose every key is unique, where a baseline is either empty or
/// total and the one-way property has nothing to say.
#[test]
fn tc_383_034_the_generator_produces_reports_whose_keys_collide() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let mut collided = 0usize;
    let mut findings_seen = 0usize;
    for _ in 0..256 {
        let report = a_report()
            .new_tree(&mut runner)
            .expect("the strategy produces a value")
            .current();
        findings_seen += report.findings.len();
        let keys: BTreeSet<String> = report.findings.iter().map(finding_key).collect();
        if keys.len() < report.findings.len() {
            collided += 1;
        }
    }
    assert!(
        findings_seen >= 256,
        "anti-vacuity floor: the generator must produce findings, saw {findings_seen}"
    );
    assert!(
        collided >= 8,
        "anti-vacuity floor: at least eight of 256 reports must carry a repeated \
         key, or the baseline never has to choose; saw {collided}"
    );
}
