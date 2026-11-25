---
id: SR-967175026302197768900474181941935084769
title: "EARS conformance — PLAT-1449 quoin"
type: SpecReview
analysis: ears-conformance
scope: "PLAT-1449 quoin PR #1069 against origin/main"
review_set: subset
---

## Summary

Most added FR criteria use scenario-style Given clauses; several behavior bullets and constraints are not EARS statements, but this is an authoring style issue rather than an observed contradiction.

## Verdict

**CONDITIONAL** — Most added FR criteria use scenario-style Given clauses; several behavior bullets and constraints are not EARS statements, but this is an authoring style issue rather than an observed contradiction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Some requirements in FR-235 and FR-236 use direct SHALL statements rather than an EARS-triggered form; criterion statements generally do use Given. | spec/functional/FR-235-quoin-selftest.md:26-58; spec/functional/FR-236-selftest-run-record.md:27-61 |

## Coverage

Reviewed frozen PR head a50fe034ecf05d405658c80e8b582c04940103fa against origin/main. Ratings remain proposed and unsigned; no acceptance decision is inferred. Plan completion: not assessed.
