---
id: TASK-037
title: "Wire report mappings and the stable FR-062 graph interface"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-036"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-066"
    type: references
  - target: "ix://agent-ix/quoin/FR-067"
    type: references
  - target: "ix://agent-ix/quoin/TC-1295"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1311"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1315"
    type: verifies
---

# TASK-037: Wire report mappings and the stable FR-062 graph interface

## Scope

Parse graph-export and changed-seed mappings, reject conflicts before reads,
and inject #152's stable report functions into the portfolio command without
reconstructing relationships or defining a second graph report contract.

## Subtasks

- [ ] Bind only after #152 confirms its exported interface.
- [ ] Preserve existing `quoin report` behavior when graph flags are absent.

## Deliverables

- `src/commands/report.ts` wiring and integration tests.

## Notes

- This is the sole cross-lane seam; no changes are made in #152's worktree.
