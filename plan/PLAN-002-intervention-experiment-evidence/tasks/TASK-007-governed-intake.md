---
id: TASK-007
title: "Implement governed intervention intake"
type: Task
status: done
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-006"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-047"
    type: references
  - target: "ix://agent-ix/quoin/TC-1134"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1139"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1146"
    type: verifies
---

# TASK-007: Implement governed intervention intake

## Scope

Extend the measurement store and record command with definition-gated,
raw-byte-verified intervention intake and stable refusal reasons.

## TDD Work

- Write TC-1134..TC-1139 and TC-1146 against temporary same-filesystem stores.
- Resolve the active plan definition before any write and verify every raw path,
  byte size, digest, and safe-root constraint.
- Reuse canonical serialization and same-directory atomic rename behavior for
  idempotency and collision protection.

## Exit Criteria

- Invalid input reports every applicable path/reason and leaves no partial entry.
- Identical input is byte-idempotent; same-id/different-content is refused.
- All terminal and cause-not-established states remain independently queryable.
