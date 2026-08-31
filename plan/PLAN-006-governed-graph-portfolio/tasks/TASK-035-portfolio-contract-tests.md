---
id: TASK-035
title: "Add red governed graph portfolio contract tests"
type: Task
status: not_started
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-067"
    type: references
  - target: "ix://agent-ix/quoin/TC-1305"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1306"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1307"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1308"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1309"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1310"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1311"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1312"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1313"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1314"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1315"
    type: verifies
---

# TASK-035: Add red governed graph portfolio contract tests

## Scope

Create TC/AC-tagged tests for graph-quality current/history rows, partitions,
availability, compatibility, raw digest resolution, injected report identity,
local failure isolation, permutations, old-store compatibility, and boundaries.

## Subtasks

- [ ] Arrange repository/store fixtures with valid and corrupt sibling collections.
- [ ] Use a minimal injected FR-062 fake object without defining a public graph model.

## Deliverables

- `tests/graph-portfolio.test.ts` with property-driven permutation coverage.

## Notes

- The suite must distinguish measurement state from graph availability.
