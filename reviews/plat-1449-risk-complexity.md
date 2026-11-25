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
