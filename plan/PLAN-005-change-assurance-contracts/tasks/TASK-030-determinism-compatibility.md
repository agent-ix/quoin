---
id: TASK-030
title: "Determinism and compatibility"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: "ix://agent-ix/quoin/TASK-029"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-065"
    type: "references"
  - target: "ix://agent-ix/quoin/TC-1290"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1291"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1292"
    type: "verifies"
---

# TASK-030: Determinism and compatibility

## Scope

Prove byte-identical receipts under input permutations, preserve existing
evidence and assurance outputs, and enforce non-identity terminology/boundaries.

## Subtasks

- [x] Add permutation, snapshot, prior-store, and prior-report regressions.
- [x] Export the new library without changing existing default command behavior.
- [x] Add static and golden checks for read-only and non-identity language.

## Deliverables

- Public exports and TC-1290..TC-1292 compatibility evidence.
