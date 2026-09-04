---
id: TASK-042
title: "Package manifest derivation and registry pins"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-040"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-075"
    type: references
  - target: "ix://agent-ix/quoin/TC-1372"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1373"
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

# TASK-042: Package manifest derivation and registry pins

## Scope

Derive `semantic/package-manifest.json` per FR-075 and pin per-export schema digests in `registry.json`; resolve `semantic.imports` at install.

## Subtasks

- [ ] `src/semantic/package-manifest.ts`: derive every required field as FR-075 states; validate against the vendored filament-core-data `package-manifest.schema.json` (TC-1372).
- [ ] Registry pins under the module entry's `semantic` key (TC-1373); import resolution and cycle rejection at install (TC-1374); identity parity test (TC-1375); package-form rejections (TC-1376); static scans (TC-1377, TC-1378).

## Deliverables

- Derived manifest writer and registry pins.

## Notes

- quoin#287 may relocate the pins later; keep the writer behind one function.
