---
id: SR-16395
title: "Spec review of quoin PR 777 (FR-071-CON-2, matrix, PLAN-007)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@a4ffd114; spec/functional/FR-071-typed-properties-mapping.md, spec/matrix.md, plan/PLAN-007-semantic-module-contract/plan.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-071"
    type: "reviews"
---

# Spec review of quoin PR 777

## Summary

Ticket: PLAT-1364. The PR restates FR-071-CON-2 from "quoin publishes the goldens under
`tests/fixtures/semantic-module/`" to "quoin reads them from `quire-fixtures` and holds no
copy". It updates three matrix rows (FR-072, FR-104, TC-1352) and one PLAN-007 sentence.
The CON-2 statement is a single, testable, ubiquitous SHALL that matches the code. The
EARS-relevant change is that one statement, and it conforms. The plan edit is correct.

## Verdict

**PASS WITH FINDINGS (one medium).** The restated constraint is sound. The matrix rows the PR
left alone still say quoin publishes the fixtures, and one row the PR edited still uses
vendoring language.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Matrix rows contradict the restated FR-071-CON-2. TC-1353..TC-1358, TC-1715 and TC-1717..TC-1730 still say "fixture(s) published here". The FR-104 row, which this PR edited, still says "TC-1729 and TC-1730 run from the vendored fixture once quire-rs re-vendors it". The FR-072 row, also edited, still says "TC-1716 awaits agent-ix/quire-rs#432", although TC-1716 is withdrawn (line 955). A reader or agent following these rows would believe quoin owns and publishes the goldens and might restore a local copy | spec/matrix.md:947-969; spec/matrix.md:133; spec/matrix.md:86 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | d4daf114: TC-1353..TC-1358, TC-1715 and TC-1717..TC-1730 say "fixture(s) held in `quire-fixtures`"; the FR-104 row's re-vendoring sentence and the FR-072 row's stale TC-1716 clause are removed; `spec/matrix.md` has no "published here", "re-vendor" or "vendored fixture" text left |
