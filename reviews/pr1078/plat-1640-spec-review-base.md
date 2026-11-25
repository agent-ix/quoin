---
id: SR-20261117175250
title: "Base spec review of PLAT-1640 Quoin ratings coverage"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-235-quoin-selftest.md; changed behavior and acceptance criteria"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-235", type: references}
---

## Summary

Ticket: PLAT-1640. The amendment assigns the coverage command to Quoin, defines high-risk coverage from an existing acceptance-criterion trace marker, distinguishes non-high release evidence from unrated features, and makes malformed or unknown markers visible. The local behavior is stated in FR-235-AC-4 and AC-11 and has a focused test binding. The ticket-level cross-repository zero-uncovered acceptance condition remains unmet; that status is recorded in the gap-analysis artifact.

## Verdict

PASS for the amended FR-235 requirements. The new criteria are specific, testable and consistent with the behavior described. Existing FR-235 criterion IDs remain sequential; no duplicate or malformed IDs were found. No hash, digest, pin, or tracking-record antipattern was introduced. The requirement statement itself was not changed, so EARS conformance analysis does not apply.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope

- FR-235 behavior bullets for ratings coverage, release-evidence status, unrated features, invalid markers, and retained-run evidence (examined).
- FR-235-AC-4 and FR-235-AC-11 (examined).
- FR-235-AC-1 through AC-3 and AC-5 through AC-10 (context only; unchanged).
- FR-235 dependency list (examined).
