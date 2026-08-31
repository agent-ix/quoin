---
id: TASK-020
title: "Consume the versioned Quire assurance export"
type: Task
status: done
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

- [x] Add the hand-authored schema, recorded publisher revision, and content digest.
- [x] Implement typed parse/validation with fail-closed premise checking.
- [x] Preserve source revision, module versions, schema digests, and distinct availability states.
- [x] Read required `--export <json>`, `--premises <json>`, and `--audit <json>` inputs without
      invoking Quire, recomputing an audit, or discovering acceptance from ambient modules.
- [x] Validate audit source/export identity before exposing its unchanged FR-032 findings,
      healthy obligations, and unevaluated checks.
- [x] Cover repeated valid import and every invalid/unavailable input class.

## Deliverables

- Validated Quire assurance-export adapter under `src/quire/`.
- Contract fixtures and TC-1256/TC-1257 tests.

## Notes

- Consumes quire-rs #386's committed producer contract at
  `3fe2c7e0e9de445af290603c3728857803b61183`.
- No subprocess belongs in the consumer; the command receives an existing export artifact.
