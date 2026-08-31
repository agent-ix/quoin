---
id: TASK-025
title: "Strict canonical integrity core"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-063"
    type: "references"
  - target: "ix://agent-ix/quoin/TC-1261"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1262"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1263"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1264"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1265"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1266"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1267"
    type: "verifies"
---

# TASK-025: Strict canonical integrity core

## Scope

Build shared schemas and the strict raw JSON, RFC 8785, BLAKE3, ordering, and
digest-validation primitives used by all three records.

## Subtasks

- [ ] Write red vector, malformed-input, closed-schema, and mutation properties.
- [ ] Implement duplicate-preserving raw parsing and I-JSON validation.
- [ ] Implement canonical bytes and BLAKE3 digest helpers without reusing generic sorted JSON.

## Deliverables

- Versioned schema assets, typed models, integrity library, and TC-1261..TC-1267 tests.

## Notes

- This freezes the shared byte contract and unblocks TASK-026 and TASK-027.
