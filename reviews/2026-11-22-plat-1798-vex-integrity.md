---
id: SR-202611220103
title: "Integrity review of PLAT-1798 FR-144 VEX population amendment"
type: SpecReview
analysis: integrity
scope: "PR #1008 at 7121d53c; spec/functional/FR-144-soup-report.md; agent-ix/spec-objects-security FR-022; quoin FR-156"
review_set: all
---

## Summary

The new behavior conflicts with the source-of-truth schema requirement for `vex_statements`. FR-156's one-export-entry-per-valid-source-statement rule does not override FR-022's duplicate-advisory prohibition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-144-AC-24 and the new no-rejection behavior require processing two statements with the same advisory id, but FR-022 Behavior and AC-6 require that source document to fail validation. The requirement set has no valid fixture satisfying AC-24, so the report's contract and upstream input contract disagree. Either keep duplicates invalid and remove this invalid-input behavior, or revise the owning FR-022/schema contract in a coordinated spec change before implementing it. | spec/functional/FR-144-soup-report.md:49-50,103; agent-ix/spec-objects-security/FR-022:58-60,80; spec/functional/FR-156-vex-export-for-soup-statements.md:21,36,103 |

## Review record

FR-156 states that each stored statement exports as one entry, but it does not permit duplicate advisory ids; FR-022 is explicit that duplicate ids are invalid. No new user story is required for the report change, but the upstream validity contract must agree with any supported duplicate population. Measured with quoin 0.36.8 and quire 0.52.0 (engine 0.62.2).
