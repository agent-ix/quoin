---
id: TASK-033
title: "Add red governed graph adapter contract tests"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-066"
    type: references
---

# TASK-033: Add red governed graph adapter contract tests

## Status

**done** — the red suite failed only because `graph-adapters` did not exist;
the implemented focused suite passes 21/21 tests covering TC-1293..TC-1304.

## Scope

Create retained Quire and graph-quality fixtures plus exact TC/AC-tagged tests
for registry selection, closed schemas, canonical identities, length checks,
attestation/plan premises, bijective normalization, state classes, idempotence,
and static no-execution boundaries.

## Subtasks

- [x] Capture one healthy fixture and one independently mutated fixture per failure class.
- [x] Prove the initial suite fails only because adapter behavior is absent.

## Deliverables

- `tests/graph-adapters.test.ts` and retained fixture bytes.

## Notes

- Do not run Quire or quire-code-rs; fixtures are contract input, not producer simulation.
