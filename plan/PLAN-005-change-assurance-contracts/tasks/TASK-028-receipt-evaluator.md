---
id: TASK-028
title: "Pure receipt evaluator"
type: Task
status: not_started
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-025"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/TASK-027"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-065"
    type: "references"
  - target: "ix://agent-ix/quoin/TC-1281"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1282"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1283"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1284"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1285"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1286"
    type: "verifies"
---

# TASK-028: Pure receipt evaluator

## Scope

Build the side-effect-free receipt schema, selection normalization, binding
checks, outcome precedence, and proof-state evaluator.

## Subtasks

- [ ] Write generated permutation and mismatch properties before implementation.
- [ ] Preserve duplicate/unknown selection evidence before lookup construction.
- [ ] Implement exact reason ordering and valid/invalid/incomplete aggregation.

## Deliverables

- Pure evaluator and TC-1281..TC-1286 tests.
