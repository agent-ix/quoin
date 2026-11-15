---
id: SR-20261115183519
title: "Spec review (base) — PLAT-1760 FR-115 typed stale-evidence cause"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md; PLAT-1760"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-115"
    type: "reviews"
---

## Summary

Ticket: PLAT-1760. A base checklist review of the FR-115 change in quoin PR #931:
the Outputs bullet on `findings` (edited), the new "Typed cause of
`stale-evidence`" bullet, and FR-115-AC-17..19. The review set is a subset:
base plus ears-conformance, integrity, dependency, evidence and
risk-complexity, and, at the dispatcher's request, spec-object-review. Each
has its own SpecReview. No AssuranceProfile applies: AP-201 covers finding
scoring and AP-202 covers Jev lens evaluation, so neither covers the evidence
matrix. Direct review was therefore used. `quire validate` could not run
because approval was required.

## Verdict

**CONDITIONAL**. The ids are well formed and contiguous (AC-17..19 follow
AC-16), every new AC has a `Test` verification, and no hash, pin or tracking
record was added. Two of the new ACs are not atomic, and one of them has an
oracle that cannot be built.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-115-AC-19 bundles three obligations: no `cause` key on non-stale kinds, consumers can distinguish the conditions, and the other fields are unchanged. Two of them cannot be tested as written. "A consumer distinguishes the two stale conditions from the `cause` field alone" is a claim about consumers, not the system. "Identical to the same fixture without `cause`" has no buildable baseline for the auditor's output, because both producers always set `cause`. It can only be compared at `matrix.build`, with `cause` stripped from the input. | spec/functional/FR-115-evidence-backed-test-matrix.md:318 |
| FND-002 | low | FR-115-AC-18 is compound. Its second sentence, a passing symbol staying `bound`, is a separate regression obligation that FR-115-AC-4 and FR-032-AC-1 already state. It should be its own AC or dropped. Its correctness defect is code-review FND-001. | spec/functional/FR-115-evidence-backed-test-matrix.md:317 |

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-115-AC-19..25 split the obligations into observable criteria, including two matrix.build inputs differing only by cause. |
| FND-002 | fixed | The passing-symbol condition was removed from the failed-run AC and is covered separately with its mock-inspection precondition. |
