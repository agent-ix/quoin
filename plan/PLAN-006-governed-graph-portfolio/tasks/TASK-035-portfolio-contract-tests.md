---
id: TASK-035
title: "Add red governed graph portfolio contract tests"
type: Task
status: done
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

## Status

**done** — the red run failed all 11 behavior groups only because the portfolio
API was absent; the implemented suite now passes 12/12 tests including the
TC-1316 retained-adapter stakeholder flow.

## Scope

Create TC/AC-tagged tests for graph-quality current/history rows, partitions,
availability, compatibility, raw digest resolution, injected report identity,
local failure isolation, permutations, old-store compatibility, and boundaries.

## Subtasks

- [x] Arrange repository/store fixtures with valid and corrupt sibling collections.
- [x] Use a minimal injected FR-062 fake object without defining a public graph model.

## Deliverables

- `tests/graph-portfolio.test.ts` with property-driven permutation coverage.

## Notes

- The suite must distinguish measurement state from graph availability.
