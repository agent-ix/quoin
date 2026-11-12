---
id: TASK-025
title: "Strict canonical integrity core"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-063"
    type: "references"
---

# TASK-025: Strict canonical integrity core

## Scope

Build shared schemas and the strict raw JSON, RFC 8785, and ordering
primitives used by all three records; no digest primitive exists.

## Subtasks

- [x] Write red vector, malformed-input, closed-schema, and mutation properties.
- [x] Implement duplicate-preserving raw parsing and I-JSON validation.
- [x] Implement canonical bytes without reusing generic sorted JSON; the BLAKE3 digest helpers are removed.

## Deliverables

- Versioned schema assets, typed models, integrity library, and TC-1261..TC-1267 tests.

## Notes

- This freezes the shared byte contract and unblocks TASK-026 and TASK-027.
