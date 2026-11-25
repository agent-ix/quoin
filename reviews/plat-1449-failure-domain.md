---
id: SR-967175026302197768900474181941935084774
title: "Failure-domain review — PLAT-1449 quoin"
type: SpecReview
analysis: failure-domain
scope: "PLAT-1449 quoin PR #1069 against origin/main"
review_set: subset
---

## Summary

The self-test requirement explicitly covers timeout, unsupported platforms, missing evidence stores, and invalid stderr encoding; no additional topological failure domain was identified in this pass.

## Verdict

**CONDITIONAL** — The self-test requirement explicitly covers timeout, unsupported platforms, missing evidence stores, and invalid stderr encoding; no additional topological failure domain was identified in this pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed frozen PR head a50fe034ecf05d405658c80e8b582c04940103fa against origin/main. Ratings remain proposed and unsigned; no acceptance decision is inferred. Plan completion: not assessed.
