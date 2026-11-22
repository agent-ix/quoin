---
id: SR-20261117175265
title: "evidence review of FR-169 risk summary command"
type: SpecReview
analysis: evidence
scope: "agent-ix/quoin; spec/functional/FR-169-risk-summary-command.md (FR-169; FR-169-AC-2, FR-169-AC-7, FR-169-AC-28)"
review_set: all
---

## Summary

The deterministic advisor recommends Test class methods for FR-169-AC-7 and FR-169-AC-28, matching the authored Test class. It has no applicable rule for FR-169-AC-2, so Test is a human choice that should be recorded as such.

## Scope

| ID | Path | Role | Excerpt |
| --- | --- | --- | --- |
| FR-169-AC-2 | spec/functional/FR-169-risk-summary-command.md | examined | The keys of every row equal the row key table exactly; the keys of every estimate entry equal `estimate`, `severity`, `probability`, `policy_region` exactly; of every control entry `control`, `requirements`, `retired_requirements`, `verifications` exactly; of every residual-risk entry `residual_risk`, `decision`, `decided_by` exactly. |
| FR-169-AC-7 | spec/functional/FR-169-risk-summary-command.md | examined | A control assumed by two post-control estimates of one situation appears once in `controls`; a requirement with a `satisfies` edge and a test with a `verifies` edge to it appear by id in `requirements` and `verifications`; a link to the control in the body of a requirement appears in neither. |
| FR-169-AC-28 | spec/functional/FR-169-risk-summary-command.md | examined | For a control linked by `satisfies` to three requirements in id order, where one has `status: retired`, one has `status: draft`, and one omits `status`, `requirements` lists all three ids in ascending order and `retired_requirements` lists only the retired id. The retired id remains in `requirements`; neither list treats its status as discharge, satisfaction or acceptance. A status value other than the exact lowercase `retired` is not reported as retired. |
| FR-169 | spec/functional/FR-169-risk-summary-command.md | context_only | `quoin risk-summary <scope>` is the command that prints the read-only hazard-to-decision chain, including controls, their implementing requirements and verifications, and residual-risk decisions. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-169-AC-2 has no applicable catalog recommendation; its authored Test method is a human choice rather than a catalog-backed recommendation. | FR-169-AC-2 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Retain the human-selected `Test` method: AC-2 checks exact JSON key equality, which is directly testable, and the current matrix reports AC-2 as `tagged`. `quoin advise` provides no catalog recommendation, so this records a human method choice rather than catalog advice. No requirement amendment is needed. |

## Verification record

At PR head `466596fa8e90ce800bcc1483ab927dcc26cfee54`, `quoin advise --repo . --json` reports AC-2 as inconclusive with no recommendation; AC-7 and AC-28 have Test-class recommendations. `quire matrix --scope . --format json` reports AC-2 as `tagged`. Measured with quoin 0.36.8 and quire 0.52.0 (engine 0.62.2).
