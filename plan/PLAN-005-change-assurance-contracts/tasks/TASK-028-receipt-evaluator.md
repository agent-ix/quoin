---
id: TASK-028
title: "Pure receipt evaluator"
type: Task
status: done
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-025"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/TASK-027"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-065"
    type: "references"
---

# TASK-028: Pure receipt evaluator

## Scope

Build the side-effect-free receipt schema, selection normalization, binding
checks, outcome precedence, and proof-state evaluator.

## Subtasks

- [x] Write generated permutation and mismatch properties before implementation.
- [x] Preserve duplicate/unknown selection evidence before lookup construction.
- [x] Implement exact reason ordering and valid/invalid/incomplete aggregation.

## Deliverables

- Pure evaluator and TC-1281..TC-1286 tests.
