---
id: TASK-006
title: "Implement the intervention record contract"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/quoin/FR-046"
    type: references
  - target: "ix://agent-ix/quoin/TC-1125"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1133"
    type: verifies
---

# TASK-006: Implement the intervention record contract

## Scope

Add the v1 JSON Schema, TypeScript types, loader, and semantic validator for the
complete FR-046 envelope and cross-field invariants.

## TDD Work

- Write TC-1125..TC-1133 first, including generated invalid combinations for arm,
  effect, qualifier, conclusion, producer, and raw-evidence relationships.
- Implement schema validation and deterministic, path-addressed semantic findings.
- Export the contract through Quoin's existing measurement/evidence library seam.

## Exit Criteria

- TC-1125..TC-1133 pass with `Trace: FR-046-AC-*` tags.
- Valid records round-trip without coercion and every undeclared field is refused.
- No store or report behavior is introduced in this task.

