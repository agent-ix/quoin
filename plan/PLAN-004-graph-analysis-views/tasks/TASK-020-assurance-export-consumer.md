---
id: TASK-020
title: "Consume the versioned Quire assurance export"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-062"
    type: references
  - target: "ix://agent-ix/quoin/TC-1256"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1257"
    type: verifies
---

# TASK-020: Consume the versioned Quire assurance export

## Scope

Vendor the published assurance-v1 schema and producer provenance, validate the complete payload,
enforce accepted format/module/schema premises, and expose immutable typed records only after every
premise succeeds.

## Subtasks

- [ ] Add the hand-authored schema, recorded publisher revision, and content digest.
- [ ] Implement typed parse/validation with fail-closed premise checking.
- [ ] Preserve source revision, module versions, schema digests, and distinct availability states.
- [ ] Cover repeated valid import and every invalid/unavailable input class.

## Deliverables

- Validated Quire assurance-export adapter under `src/quire/`.
- Contract fixtures and TC-1256/TC-1257 tests.

## Notes

- Blocked until quire-rs #386 publishes the actual schema and producer contract.
- No subprocess belongs in the consumer; the command receives an existing export artifact.
