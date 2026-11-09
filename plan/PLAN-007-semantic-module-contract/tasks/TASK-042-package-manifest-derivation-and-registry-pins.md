---
id: TASK-042
title: "Package manifest derivation"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-040"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-075"
    type: references
  - target: "ix://agent-ix/quoin/TC-1372"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1374"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1375"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1376"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1377"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1378"
    type: verifies
---

# TASK-042: Package manifest derivation

## Scope

Derive `semantic/package-manifest.json` per FR-075 and resolve `semantic.imports` at install.

## Subtasks

- [x] `src/semantic/package-manifest.ts`: derive every required field as FR-075 states; validate against the published filament-core-data `package-manifest.schema.json` (TC-1372).
- [x] Import resolution and cycle rejection at install (TC-1374); identity parity test (TC-1375); package-form rejections (TC-1376); static scans (TC-1377, TC-1378).

## Deliverables

- Derived manifest writer.

## Notes

- TC-1373 is withdrawn with FR-075-AC-2 (per-export digests removed); this task verifies none of it. The registry entry records no pin (FR-075 CR notes 2026-09-30).
