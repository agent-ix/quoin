---
id: SR-967175026302197768900474181941935084776
title: "Risk-complexity review — PLAT-1449 ratings"
type: SpecReview
analysis: risk-complexity
scope: "PLAT-1449 quoin PR #1069 against origin/main"
review_set: subset
---

## Summary

The two proposed ratings cover all functional requirements exactly once, but no rating rationale references an explicit classification rule or evidence set, leaving some grouped judgments difficult to reproduce.

## Verdict

**CONDITIONAL** — The two proposed ratings cover all functional requirements exactly once, but no rating rationale references an explicit classification rule or evidence set, leaving some grouped judgments difficult to reproduce.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FRR-001 groups eight advisory/reporting features and provides failure mode and detection statements, but its rationale applies a broad non-high rationale uniformly without mapping each assessment to a stated classification threshold. | spec/validation/FRR-001-not-high-process-risk.md:34-52 |

## Coverage

Reviewed frozen PR head a50fe034ecf05d405658c80e8b582c04940103fa against origin/main. Ratings remain proposed and unsigned; no acceptance decision is inferred. Plan completion: not assessed.


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The fix addresses the finding in the current diff. |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      decision. Each feature meets the not-high threshold because its failure can
      mislead or inconvenience a person, while the feature itself neither creates
      authoritative verification evidence nor makes an acceptance decision:
      
      - FR-003 can misdescribe a command option, but help text does not execute the
        command or create a record.
      - FR-011 can misdescribe a catalog type, but a listing does not write a record
        or accept a decision.
      - FR-145, FR-146, FR-147, and FR-148 can omit or misstate read-only advice,
        policy estimates, evidence gaps, or undecided items; each leaves the
        consequential decision to a person and writes no accepted decision.
      - FR-189 can produce an incomplete formalization draft, but that draft remains
        unapproved and is not a verification result.
      - FR-221 can misstate the repository's private reporting route, but it does not
        itself disclose a report or authorize public disclosure.
      
      In every case, the failure affects information used by a person; it does not
      silently claim that verification, traceability, discharge, or approval has
      occurred. A person remains responsible for reviewing the information and
      making any consequential decision.
```

+++

## Current diff review — PR head 965259ca6f32889e6f8b763d085cd5fd790a13f1

The previous grouping finding remains fixed: FRR-001 states its not-high threshold and gives a feature-specific explanation for each linked FR. FRR-002's common rationale is consistent with the high-risk grouping and assurance activities, which are organized around the failure effects on represented requirements, evidence, results, and approvals. The detailed per-requirement assessment tables are absent as requested. No new risk-classification finding was identified in this content update.
