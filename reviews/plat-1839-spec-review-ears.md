---
id: SR-17094
title: "EARS conformance review of PLAT-1839 FR-211"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-211-module-lag-warning.md; PR agent-ix/quoin#1027"
review_set: subset
---

## Summary

Ticket: PLAT-1839. EARS review of the requirement statements added in FR-211 (branch copy). The primary SHALL sentence in Description is a well-formed ubiquitous requirement with a single response ("warn for each default module whose installed release is older than the release the running CLI pins"). AC-1 and AC-2 are atomic and testable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "`quoin module doctor` SHALL report the registry state and exit zero when only lag warnings are present" is compound (two responses) and "report the registry state" is undefined: one implementer prints every registry row, another a one-line summary (the branch does the latter). | spec/functional/FR-211-module-lag-warning.md:39-40 |
| FND-002 | low | FR-211-AC-3 bundles two commands (`module doctor` and `catalog validate`) in one criterion, so a failure cannot be attributed to one surface from the matrix row. | spec/functional/FR-211-module-lag-warning.md:48 |

## Verdict

One ambiguous compound statement and one compound AC; the headline requirement conforms.

## Dispositions

Round 1, head 35474e19.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Replaced by three single-response statements: print the warnings, print one summary line with the count, exit zero when only warnings. |
| FND-002 | fixed | Split into FR-212-AC-5 (module doctor) and FR-212-AC-6 (catalog validate). |
