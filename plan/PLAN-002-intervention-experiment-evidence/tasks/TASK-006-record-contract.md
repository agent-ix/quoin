---
id: TASK-006
title: "Implement the intervention record contract"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/quoin/FR-056"
    type: references
  - target: "ix://agent-ix/quoin/TC-1195"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1203"
    type: verifies
---

# TASK-006: Implement the intervention record contract

## Scope

Add the v1 JSON Schema, TypeScript types, loader, and semantic validator for the
complete FR-056 envelope and cross-field invariants.

## TDD Work

- Write TC-1195..TC-1203 first, including generated invalid combinations for arm,
  effect, qualifier, conclusion, producer, and raw-evidence relationships.
- Implement schema validation and deterministic, path-addressed semantic findings.
- Export the contract through Quoin's existing measurement/evidence library seam.

## Exit Criteria

- TC-1195..TC-1203 pass with `Trace: FR-056-AC-*` tags.
- Valid records round-trip without coercion and every undeclared field is refused.
- No store or report behavior is introduced in this task.
