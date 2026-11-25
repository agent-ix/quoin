---
id: SR-967175026302197768900474181941935084783
title: "Evidence review — validation report"
type: SpecReview
analysis: evidence
scope: "agent-ix/quoin; FR-217 and FR-220 acceptance criteria in spec/functional/FR-217-validation-report.md and spec/functional/FR-220-external-tool-configuration-index.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-217", type: references }
  - { target: "ix://agent-ix/quoin/FR-220", type: references }
---

## Summary

Ran `quoin advise --repo . --json` and reviewed the recommendations for FR-217 and FR-220. All twelve obligations are authored as `Test`; the advisor recommends test-class methods for each and reports no mismatches or inconclusive recommendations.

## Verdict

**FAIL** — the authored method is appropriate, but FR-217-AC-8 and FR-220-AC-2..4 lack test binders in the computed matrix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The required Test evidence for FR-217-AC-8 is absent; the matrix has no binder for the quire-cli/quire-rs isolation behavior. | FR-217-AC-8 |
| FND-002 | high | The required Test evidence for FR-220-AC-2 is absent. | FR-220-AC-2 |
| FND-003 | high | The required Test evidence for FR-220-AC-3 is absent. | FR-220-AC-3 |
| FND-004 | high | The required Test evidence for FR-220-AC-4 is absent. | FR-220-AC-4 |
## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | FR-217-AC-8 is the separate Quire-side behavior tracked by PLAT-1642, which remains open; this PLAT-1641 PR does not claim that cross-repository criterion. |
| FND-002 | fixed | Verified on PR head 79eaa3e8. |
| FND-003 | fixed | Verified on PR head 79eaa3e8. |
| FND-004 | fixed | Verified on PR head 79eaa3e8. |

| FND-005 | fixed | The computed matrix now reports FR-236-AC-8 tagged to tc_230_run_record_retains_versioned_anomaly_set_and_absence_state; the required Test evidence is criterion-bound. |
+++ [reviewer dispositions]

```yaml
dispositions:
- fnd: FND-001
  outcome: deferred
  reason: FR-217-AC-8 is the separate Quire-side behavior tracked by PLAT-1642, which
    remains open; this PLAT-1641 PR does not claim that cross-repository criterion.
- fnd: FND-002
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-2")]'
- fnd: FND-003
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-3")]'
- fnd: FND-004
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-4")]'
- fnd: FND-005
  outcome: fixed
  after_excerpt: >-
    Given a self-test run, the retained record preserves the producing binary's anomaly-set provided state, producer version, and authored anomaly documents; the producer version equals toolVersion, and provided-empty differs from absent.
```

+++

## New findings (replacement PR #1077 review)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The required Test evidence for FR-236-AC-8 is absent; the matrix reports no binder. | FR-236-AC-8 |

The current matrix command was `quoin matrix --repo . --json`; FR-236-AC-8 reports `static_status: untagged`, `binders: []`, and “no evidence is bound to FR-236-AC-8”. The existing `tc_230_run_record_serializes_run_context_and_case_outcomes` test is tagged to FR-236-AC-1, AC-2, and AC-4, not AC-8; its current assertions do not cover all AC-8 behavior.
