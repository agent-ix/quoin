---
id: SR-2026112016223475846355006
title: "PLAT-1834 quoin PR 1029 spec-review EARS"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-200-dossier-risk-management-plan-content.md, spec/functional/FR-191-dossier-command.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-200"
    type: references
---

# PLAT-1834 quoin PR 1029 spec-review EARS

## Summary

Ticket: PLAT-1834. `quire validate --scope .` (quire 0.52.0, engine 0.62.2) on FR-200 and FR-191, compared with the same command on origin/main: main reports no grammar warning for either file; the PR head reports seven, all in FR-200 and all introduced by this PR. FR-191 edits are grammar-clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Five rewritten Behavior bullets join two obligations with one subject ("SHALL ... and SHALL NOT ..."; "SHALL produce ... and the four plan sections SHALL state ..."; "SHALL produce ... and SHALL show no example ..."), flagged `ears:non-singular`. The main-branch FR-200 split these one per statement; split them back. | spec/functional/FR-200-dossier-risk-management-plan-content.md:68-71,74 |
| FND-002 | low | The Description sentence's second clause "SHALL state the absence of a plan record when none exists" is flagged `ears:unclassifiable` and `ears:missing-subject`. Give it its own subject and an EARS form ("If no plan record exists, then the document SHALL state ..."). | spec/functional/FR-200-dossier-risk-management-plan-content.md:22-25 |

## Verdict

PASS WITH LOW FINDINGS. The new conditional statements ("If a plan record exists, then ... SHALL NOT produce `no_plan_record`", "If a plan references no policy in scope, then ...") are well-formed unwanted-behaviour EARS and each maps to one AC.

## Examined

- FR-200 Description and Behavior (examined)
- FR-191 lines 109, 126-127, AC-22 (examined)
