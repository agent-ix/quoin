---
id: TASK-037
title: "Wire report mappings and the stable FR-062 graph interface"
type: Task
status: done
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

## Status

**done** — the report command accepts explicit export/premises/audit triples
and changed seeds, validates every mapping before reads, and injects #152's
stable analyzers from corrected dependency commit `b8112fc`. The combined
#152/#281 seam suite passes 67/67 and the legacy no-graph command is unchanged.

## Scope

Parse graph-export, graph-premises, graph-audit, and changed-seed mappings,
reject conflicts before reads, reject partial triples without graph reads, and
inject #152's stable report functions into the portfolio command without
reconstructing relationships or defining a second graph report contract.

## Subtasks

- [x] Bind only after #152 confirms its exported interface.
- [x] Preserve existing `quoin report` behavior when graph flags are absent.

## Deliverables

- `src/commands/report.ts` wiring and integration tests.

## Notes

- This is the sole cross-lane seam; no changes are made in #152's worktree.
