---
id: TASK-012
title: "Implement operational intake and clock discharge"
type: Task
status: done
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-011"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-060"
    type: references
  - target: "ix://agent-ix/quoin/TC-1232"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1238"
    type: verifies
---

# TASK-012: Implement operational intake and clock discharge

## Scope

Extend governed measurement intake and obligation evaluation for both operational
shapes, raw-byte integrity, stable refusals, and timestamp-derived clock discharge.

## TDD Work

- Write TC-1232..TC-1238 against temporary same-filesystem stores.
- Reuse governing-plan resolution and canonical bytes, and publish by atomic
  no-replace commit with pair-aware idempotency and collision protection.
- Match kind, subject, scope, accepted exercise mode, success outcome, and exact
  obligation clock condition before deriving timely completion and discharging.

## Exit Criteria

- Invalid input and raw mismatch write nothing and return every named reason.
- Both shapes and all exercise outcomes remain independently queryable.
- Capability existence alone and every non-matching/adverse exercise do not discharge.
