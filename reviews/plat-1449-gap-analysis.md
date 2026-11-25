---
id: SR-967175026302197768900474181941935084767
title: "Gap analysis — PLAT-1449 quoin"
type: SpecReview
analysis: gap-analysis
scope: "PLAT-1449 quoin PR #1069 against origin/main"
review_set: subset
---

## Summary

The matrix confirms untagged acceptance criteria in newly renumbered self-test/run-record requirements; the split duplicate IDs exposed criteria that had been falsely covered by shared tags.

## Verdict

**FAIL** — The matrix confirms untagged acceptance criteria in newly renumbered self-test/run-record requirements; the split duplicate IDs exposed criteria that had been falsely covered by shared tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | After renumbering the second FR-230 duplicate to FR-236, FR-236-AC-1 through FR-236-AC-4 have no test binders and static_status=untagged. The origin/main matrix had shown duplicate rows covered by the shared old ID; the PR must preserve valid trace coverage or explicitly leave this gap for the owning follow-up. | spec/functional/FR-236-selftest-run-record.md:65-70; quoin matrix --repo . --json |

## Coverage

Reviewed frozen PR head a50fe034ecf05d405658c80e8b582c04940103fa against origin/main. Ratings remain proposed and unsigned; no acceptance decision is inferred. Plan completion: not assessed.
