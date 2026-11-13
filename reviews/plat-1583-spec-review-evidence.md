---
id: SR-17086
title: Evidence review of quoin PR 872 (PLAT-1583, FR-175)
type: SpecReview
analysis: evidence
scope: agent-ix/quoin@5fbc1576 (PR 872); spec/functional/FR-175-anomaly-check-fix-verification-evidence.md, spec/functional/index.md
review_set: subset
date: '2026-11-13'
---

# Evidence review of quoin PR 872 (PLAT-1583, FR-175)

## Summary

Ticket: PLAT-1583. Checked each AC's declared verification method against the behaviour it covers, and that every Behavior branch has a criterion that can fail. The fixtures are spec-artifacts-process documents, and the stores are built in temp dirs.

## Verdict

Changes requested, minor. Test is the right method for AC-1 to AC-10 and Inspection for AC-11, the absence of a copy. AC-2, AC-3 and AC-4 cover the pass/fail/missing triple the ticket asks for, plus `error`, `skip`, absent store, older run and binding. Every AC can fail. Two branches of the result and finding rules have no criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | 'any such entry in any suite's newest run' (failing in one suite wins over passing in another) has no criterion. AC-4 uses one suite only. An implementation that read only the single newest run across all suites would pass every AC. Add an AC with two suites whose newest runs disagree on TC-001. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:56 |
| FND-002 | low | Nothing tests the 'or another Criterion row is passing' clause (two Criterion rows, one passing and one missing), or a criterion-only table whose criterion is `failing`, which should give `fix-test-failing` and also `fix-no-passing-link` under line 77. Add an AC for each, or state the intended output. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:75-79 |
