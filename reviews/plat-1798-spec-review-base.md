---
id: SR-20261117175261
title: "base review of FR-169 risk summary command"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-169-risk-summary-command.md (FR-169; FR-169-AC-2, FR-169-AC-7, FR-169-AC-28)"
review_set: all
---

## Summary

The changed JSON key contract and linked requirement and retired subset behavior are specific and valid. FR-169-AC-28 has no test trace tag in the computed matrix, so this criterion has no test carrying it. No hash, digest, pin, or tracking-record antipattern was found.

## Scope

| ID | Path | Role | Excerpt |
| --- | --- | --- | --- |
| FR-169-AC-2 | spec/functional/FR-169-risk-summary-command.md | examined | The keys of every row equal the row key table exactly; the keys of every estimate entry equal `estimate`, `severity`, `probability`, `policy_region` exactly; of every control entry `control`, `requirements`, `retired_requirements`, `verifications` exactly; of every residual-risk entry `residual_risk`, `decision`, `decided_by` exactly. |
| FR-169-AC-7 | spec/functional/FR-169-risk-summary-command.md | examined | A control assumed by two post-control estimates of one situation appears once in `controls`; a requirement with a `satisfies` edge and a test with a `verifies` edge to it appear by id in `requirements` and `verifications`; a link to the control in the body of a requirement appears in neither. |
| FR-169-AC-28 | spec/functional/FR-169-risk-summary-command.md | examined | For a control linked by `satisfies` to three requirements in id order, where one has `status: retired`, one has `status: draft`, and one omits `status`, `requirements` lists all three ids in ascending order and `retired_requirements` lists only the retired id. The retired id remains in `requirements`; neither list treats its status as discharge, satisfaction or acceptance. A status value other than the exact lowercase `retired` is not reported as retired. |
| FR-169 | spec/functional/FR-169-risk-summary-command.md | context_only | `quoin risk-summary <scope>` is the command that prints the read-only hazard-to-decision chain, including controls, their implementing requirements and verifications, and residual-risk decisions. |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-169-AC-28 is untagged in the computed Test Matrix, so no test trace tag carries the new retired-requirements behavior. | FR-169-AC-28 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | This is a spec-only PR, so the implementation test and trace tag are deferred until after explicit spec review approval. Before implementation acceptance, add a test that exercises AC-28 and carries the exact `FR-169-AC-28` trace tag, then run the matrix and confirm AC-28 is `tagged`. The current untagged result remains recorded; this deferral does not claim coverage or authorize implementation before spec review approval. |

## Verification record

At PR head `466596fa8e90ce800bcc1483ab927dcc26cfee54`, `quire matrix --scope . --format json` reports FR-169-AC-28 as `untagged` with no binders. Measured with quoin 0.36.8 and quire 0.52.0 (engine 0.62.2).
