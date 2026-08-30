---
id: TASK-008
title: "Record subsystem ownership and reconcile decisions"
type: Task
track: "Architecture records"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-050"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-006"
    type: "depends_on"
---

# TASK-008: Record subsystem ownership and reconcile decisions

## Status

**done** — ownership, the identity-pinned decision ledger, and both proposed local ADRs satisfy TC-1129..TC-1133 and TC-1145..TC-1150.

## Scope

Create `ownership-and-boundaries.md`, `decision-ledger.md`, and two local ADRs. Allocate positive and
negative ownership for Quire, Quoin, `filament-core-data`, modules, and consumers. Reconcile Quire
ADR-0002/0003/0004/0011 and the current specification without changing their external status.

## Exit criteria

- TC-1129..TC-1133 and TC-1145..TC-1150 pass.
- Each external decision records repository, path, status, and reviewed revision/date.
- Quire remains parser/validator/extractor/addressor/byte-splicer; Quoin remains catalog/workflow owner;
  the compiler remains separate.
