---
id: TASK-036
title: "Implement pure governed graph portfolio logic"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-034"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-035"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-067"
    type: references
  - target: "ix://agent-ix/quoin/TC-1305"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1309"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1312"
    type: verifies
---

# TASK-036: Implement pure governed graph portfolio logic

## Scope

Build deterministic graph-quality history/current/comparison views and merge
them with an optional injected FR-062 report bundle. Preserve partitions,
availability, raw identities, incompatible history, and readable siblings.

## Subtasks

- [ ] Isolate corrupt collection reads without changing existing strict store APIs.
- [ ] Reuse FR-044 comparison concepts and add corpus/population premises.
- [ ] Render human and canonical JSON from the same report object.

## Deliverables

- `src/measurement/graph-portfolio.ts` and focused exports.

## Notes

- Pure logic accepts already-produced graph report objects and never traverses graph edges.
