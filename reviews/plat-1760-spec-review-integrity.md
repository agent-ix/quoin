---
id: SR-20261115183521
title: "Integrity — PLAT-1760 FR-115 typed stale-evidence cause"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md, spec/functional/FR-032-evidence-auditor.md (context); PLAT-1760"
review_set: subset
---

## Summary

Ticket: PLAT-1760. Consistency, atomicity and allocation check of the FR-115
change in PR #931 against FR-115's own `stale` and `suspect` definitions and
FR-032's stale and vacuity criteria (AC-3, AC-4, AC-17). The atomicity
findings are in the base review and are not repeated here.

## Verdict

**CONDITIONAL**, with two medium findings and one low. Precedence, status
vocabulary and `summary` are explicitly left unchanged, so the change raises
no conflict with AC-4 or AC-9. The defects are a second valid reading of
AC-17, a drifted restatement of the failed-run rule, and the auditor contract
being changed from the wrong FR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-115-AC-17 says "A bound symbol with no recorded run", but the producer condition is per suite. The Description says "the bound suite has no recorded run", and `unrecorded_evidence` also ignores a suite with a recorded scan. A bound symbol that a recorded run does not report is `vacuous-evidence`, which maps to `suspect` (FR-032-AC-4, FR-115 status table), not `stale`/`missing-run`. The AC has two readings that lead to different fixtures and different expected statuses. | spec/functional/FR-115-evidence-backed-test-matrix.md:316 |
| FND-002 | low | The failed-run condition is restated differently from its owner. FR-115 (line 222, and AC-18) says "the newest recorded outcome for the bound symbol is `fail` or `error`". FR-032-AC-17 says the suite's newest recorded run (`latest_runs`) reports `fail` or `error` for a bound symbol. Read per symbol, an older run could be selected when the newest run omits the symbol. | spec/functional/FR-115-evidence-backed-test-matrix.md:317 |
| FND-003 | medium | The change adds a field to the auditor's JSON `Finding`, and FR-115-AC-17..19 test the auditor's own report. That output belongs to FR-032, as FR-115-CON-2 says. FR-032 is unchanged: its Outputs still read "Findings, each with a kind, an obligation, a severity and a summary", and it has no AC or CR note for `cause`. The obligation sits in the consumer's FR, not the producer's. | spec/functional/FR-115-evidence-backed-test-matrix.md:222-224 |

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-032-AC-18 now states the missing-run condition per bound suite. |
| FND-002 | fixed | FR-115 now uses FR-032's suite-newest-run condition for failed-run. |
| FND-003 | fixed | FR-032 now owns the producer contract through its SHALL behavior, AC-18..21 and change note; FR-115 no longer defines the producer's report. |
