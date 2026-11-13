---
id: SR-20261113052925
title: "Dependency review of PLAT-1601 OSV-Scanner adapter and advisory join"
type: SpecReview
analysis: dependency
scope: "agent-ix/quoin; PR #881; spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/FR-149-osv-scanner-advisory-report-adapter.md"
review_set: all
---

# Dependency review of PLAT-1601 OSV-Scanner adapter and advisory join

## Summary

Reviewed the requirement graph and enablement/feature ordering for the new adapter, join updates, and report updates. The adapter is an enablement dependency for the join and report, while FR-149-AC-7 also makes the adapter criterion depend on join behavior, creating a feature-to-enablement edge that complicates sequencing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-149-AC-7 requires an adapter recording test to assert the FR-142 join's unmatched behavior. FR-142 itself depends on FR-149's adapter, so this criterion makes adapter acceptance depend on the downstream join feature. Keep FR-149-AC-7 to asserting the recorded fallback path, and keep the unmatched-join assertion in FR-142-AC-26; this restores an acyclic implementation order. | spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:116; spec/functional/FR-142-advisory-join-onto-soup-records.md:225 |

## Ordering

The natural order is the producer request and report contract, then the finding adapter, then the advisory join, then the rendered report. No cycle exists among the requirement-level `relationships`, but the cross-requirement AC currently adds the reverse test dependency described above.
