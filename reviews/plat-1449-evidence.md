---
id: SR-967175026302197768900474181941935084773
title: "Evidence review — PLAT-1449 Quoin"
type: SpecReview
analysis: evidence
scope: "agent-ix/quoin#1069 at 47b3a4cf7b565bdf73fa4ebd166418d4f7c5be97 against origin/main"
review_set: subset
---

## Summary

The run-record serialization test binds FR-236-AC-1, AC-2, and AC-4. AC-3 is explicitly assigned as a cross-repository validation-kit follow-up in PLAT-1641 because quoin selftest does not invoke external tools.

## Verdict

**CONDITIONAL** — Current Quoin behavior has focused evidence for the criteria it owns; AC-3 remains assigned to PLAT-1641.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-236-AC-1 through AC-4 declare Test verification but have no matrix binders; there is no current evidence path for the required fields, failed-run retention, external-tool reference, and digest exclusions. | spec/functional/FR-236-selftest-run-record.md:65-70 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The serialization test now binds AC-1, AC-2, and AC-4; AC-3 remains assigned to PLAT-1641 because Quoin selftest does not invoke external tools. |

## Coverage

Reviewed the current public PR head `47b3a4cf7b565bdf73fa4ebd166418d4f7c5be97` against `origin/main`. The focused serialization test covers run context, pass and not-executed outcomes, and the specified digest exclusions. AC-3 is not claimed as implemented in Quoin and remains assigned to PLAT-1641. Ratings remain proposed and unsigned. Plan completion: not assessed.
