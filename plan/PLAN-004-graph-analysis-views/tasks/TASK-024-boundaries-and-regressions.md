---
id: TASK-024
title: "Seal graph-analysis boundaries and regressions"
type: Task
status: not_started
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

- [ ] Reject producer, suite, Quire, Git, network, write, and frontmatter-reader dependencies.
- [ ] Reject independent graph construction and score/threshold vocabulary in graph reports.
- [ ] Pin unchanged evidence-audit, assurance-case, and measurement output without graph invocation.
- [ ] Run format, lint, build, Quire validation, focused tests, and the full repository gate.

## Deliverables

- TC-1259 static boundary test.
- TC-1260 additive/non-regression test.
- Green full verification evidence.

## Notes

- This is the landing gate for issue #152 and the prerequisite for issue #281.
