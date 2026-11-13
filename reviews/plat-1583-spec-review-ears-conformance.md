---
id: SR-17085
title: EARS-conformance review of quoin PR 872 (PLAT-1583, FR-175)
type: SpecReview
analysis: ears-conformance
scope: agent-ix/quoin@5fbc1576 (PR 872); spec/functional/FR-175-anomaly-check-fix-verification-evidence.md, spec/functional/index.md
review_set: subset
date: '2026-11-13'
---

# EARS-conformance review of quoin PR 872 (PLAT-1583, FR-175)

## Summary

Ticket: PLAT-1583. `quire validate --summary` reports FR-175 grammar-clean (1/1, 0 findings). The engine splits on newlines, so it cannot see a conjoined SHALL on a wrapped bullet. A semantic pass over all 18 Behavior statements found four non-singular ones.

## Verdict

Minor. The trigger and unwanted-behaviour forms are correct (line 77 `If ... then`), no statement uses a vague verb, and the remaining 14 statements each carry one obligation. Split the four statements below so that each maps to one criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three obligations in one bullet: skip a no-record anomaly, skip a dangling row, and a `Quoin SHALL NOT report it again` with a different subject. Split into three statements. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:44-45 |
| FND-002 | low | `SHALL read the evidence store ... and SHALL write nothing to it or to any document` joins a read obligation and a prohibition. Split them. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:47-48 |
| FND-003 | low | Two subjects and two obligations (a passing row yields no finding; an all-passing document yields none of the three). The second follows from the first and line 77, so delete it or make it its own statement. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:80-81 |
| FND-004 | low | `SHALL NOT state that a fix is verified ... and SHALL NOT judge whether a test is adequate` holds two prohibitions. Split them, because AC-10 tests only the first. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:85-86 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | Line 69, added in the fix round, carries two obligations ('that suite SHALL contribute no entry, and an older run of it SHALL NOT stand in') and a note on FR-030-AC-13. Split it when FND-003 of SR-17083 is reworded. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:69 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (b1a6c35e): - The check SHALL skip an anomaly that has no record (`anomaly.no-record`, FR-136). - The check SHALL skip a row whose `Target` resolves to nothing in the corpus. - Quoin SHALL NOT report a dangling reference again, because `quire validate` reports it. |
| FND-002 | still-open | Line 56 still reads 'The check SHALL read the evidence store under spec/evidence/ of the scope and SHALL write nothing to it or to any document'. It is two obligations and was not split. |
| FND-003 | fixed | Round 1 (b1a6c35e): - A row whose result is `passing` SHALL yield no finding. |
| FND-004 | fixed | Round 1 (b1a6c35e): - The check SHALL NOT state that a fix is verified, accepted or acceptable. - The check SHALL NOT judge whether a test is adequate for the fix. |
