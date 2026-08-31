---
id: TASK-034
title: "Implement lossless governed graph producer adapters"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-033"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-066"
    type: references
  - target: "ix://agent-ix/quoin/TC-1296"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1300"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1303"
    type: verifies
---

# TASK-034: Implement lossless governed graph producer adapters

## Scope

Implement the exact adapter registry, Quire premise validation/handoff, graph-
quality record and attestation validation, SHA-256 identities, raw attachment,
bijective observations, and reuse of FR-044 collection validation/write behavior.

## Subtasks

- [ ] Keep parsing/validation pure and separate from file reads and store writes.
- [ ] Export only producer-contract and measurement types; no graph-analysis model.

## Deliverables

- `src/measurement/graph-adapters.ts` plus narrow exports from the measurement index.

## Notes

- No subprocess, network, Git, producer, or frontmatter dependency is permitted.
