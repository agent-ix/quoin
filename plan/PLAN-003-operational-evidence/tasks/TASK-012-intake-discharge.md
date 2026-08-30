---
id: TASK-012
title: "Implement operational intake and clock discharge"
type: Task
status: todo
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-011"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-049"
    type: references
  - target: "ix://agent-ix/quoin/TC-1156"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1162"
    type: verifies
---

# TASK-012: Implement operational intake and clock discharge

## Scope

Extend governed measurement intake and obligation evaluation for both operational
shapes, raw-byte integrity, stable refusals, and timestamp-derived clock discharge.

## TDD Work

- Write TC-1156..TC-1162 against temporary same-filesystem stores.
- Reuse governing-plan resolution, canonical bytes, atomic rename, idempotency, and
  collision protection from the measurement store.
- Match kind, subject, scope, accepted exercise mode, success outcome, and derived
  `met` status before discharging any clocked obligation.

## Exit Criteria

- Invalid input and raw mismatch write nothing and return every named reason.
- Both shapes and all exercise outcomes remain independently queryable.
- Capability existence alone and every non-matching/adverse exercise do not discharge.

