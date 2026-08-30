---
id: TASK-012
title: "Add red semantic type-fit audit contract tests"
type: Task
track: "Test-first foundation"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-051"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-052"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-053"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-054"
    type: "references"
---

# TASK-012: Add red semantic type-fit audit contract tests

## Status

**done** — the focused red run failed only because the audit module did not exist; the completed
suite now passes all 39 TC-1156..TC-1193 contract cases.

## Scope

Create fixture modules and a focused suite with exact trace tags for TC-1156..TC-1193. Cover clean
and adversarial source identity, duplicate declarations, placeholder contracts, all parse states,
missing instances, occurrence signals, canonical-output tampering, equal-input reruns, and read-only inputs.

## Exit criteria

- The focused suite fails for missing audit behavior, not for a broken fixture.
- Every automated case appears exactly once as a trace tag.
- The fixtures exercise every closed vocabulary and every denominator failure state.
