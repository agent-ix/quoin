---
id: TASK-024
title: "Seal graph-analysis boundaries and regressions"
type: Task
status: done
track: Gate
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-023"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-062"
    type: references
  - target: "ix://agent-ix/quoin/TC-1259"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1260"
    type: verifies
---

# TASK-024: Seal graph-analysis boundaries and regressions

## Scope

Add static dependency and golden regression gates proving graph analysis is read-only,
non-scoring, export-driven, and additive to existing evidence, assurance, and measurement outputs.

## Subtasks

- [x] Reject producer, suite, Quire, Git, update-check, network, write, and frontmatter-reader
      dependencies.
- [x] Reject independent graph construction and score/threshold vocabulary in graph reports.
- [x] Pin unchanged evidence-audit, assurance-case, and measurement output without graph invocation.
- [x] Run format, lint, build, Quire validation, focused tests, and the full repository gate.

## Deliverables

- TC-1259 static boundary test.
- TC-1260 additive/non-regression test.
- Verification evidence separating #152 results from two pre-existing external-drift failures.

## Notes

- This is the landing gate for issue #152 and the prerequisite for issue #281.
- The full pinned Vitest run passes 819/819 tests. The reusable skill-contract gate prefers the
  active installed module schema over a stale development checkout, and the suite uses the pinned
  Quire contract build from issue #386.
